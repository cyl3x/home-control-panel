use std::collections::HashMap;
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::PathBuf;

use chrono::{DateTime, Local, Utc};
use serde::{Deserialize, Serialize};

use super::auth::Token;

/// API responses and the token persisted across restarts, so restarting the app does not
/// run into the quota or log in again. Only readable by the user, as it holds the token.
#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Cache {
    #[serde(skip)]
    path: Option<PathBuf>,
    /// Responses of another account are discarded
    account: String,
    /// Raw responses by URL
    responses: HashMap<String, Response>,
    paused_until: Option<DateTime<Local>>,
    token: Option<Token>,
}

#[derive(Serialize, Deserialize)]
struct Response {
    fetched_at: DateTime<Utc>,
    body: serde_json::Value,
}

impl Cache {
    /// Load the cache from the given file, without a path nothing is persisted.
    pub fn load(path: Option<PathBuf>, account: &str) -> Self {
        let cache = path
            .as_ref()
            .and_then(|path| std::fs::read_to_string(path).ok())
            .and_then(|text| {
                serde_json::from_str::<Self>(&text)
                    .inspect_err(|err| log::warn!("MyVaillant: ignoring broken cache: {err}"))
                    .ok()
            })
            .filter(|cache| cache.account == account)
            .unwrap_or_else(|| Self {
                account: account.to_string(),
                ..Self::default()
            });

        Self { path, ..cache }
    }

    pub fn get(&self, url: &str) -> Option<(&serde_json::Value, DateTime<Utc>)> {
        self.responses
            .get(url)
            .map(|response| (&response.body, response.fetched_at))
    }

    pub fn insert(&mut self, url: &str, body: serde_json::Value) {
        self.responses.insert(
            url.to_string(),
            Response {
                fetched_at: Utc::now(),
                body,
            },
        );
        self.save();
    }

    pub fn remove(&mut self, url: &str) {
        if self.responses.remove(url).is_some() {
            self.save();
        }
    }

    pub fn paused_until(&self) -> Option<DateTime<Local>> {
        self.paused_until.filter(|until| *until > Local::now())
    }

    pub fn pause_until(&mut self, until: DateTime<Local>) {
        self.paused_until = Some(until);
        self.save();
    }

    pub const fn token(&self) -> Option<&Token> {
        self.token.as_ref()
    }

    pub fn set_token(&mut self, token: Option<Token>) {
        self.token = token;
        self.save();
    }

    fn save(&self) {
        let Some(path) = &self.path else {
            return;
        };

        let result = (|| {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            // Write to a temporary file first, so a crash never leaves a half written cache
            let temporary = path.with_extension("json.tmp");
            std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .mode(0o600)
                .open(&temporary)?
                .write_all(&serde_json::to_vec(self).map_err(std::io::Error::other)?)?;
            std::fs::rename(&temporary, path)
        })();

        if let Err(err) = result {
            log::warn!(
                "MyVaillant: failed to write cache {}: {err}",
                path.display()
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeDelta;

    use super::*;

    fn path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "home-control-panel-{}-{name}.json",
            std::process::id()
        ))
    }

    #[test]
    fn persists_responses_per_account() {
        let path = path("responses");
        let mut cache = Cache::load(Some(path.clone()), "me@vaillant-germany-b2c");
        cache.insert(
            "https://example.com/homes",
            serde_json::json!([{ "systemId": "sys" }]),
        );

        let cache = Cache::load(Some(path.clone()), "me@vaillant-germany-b2c");
        let (body, fetched_at) = cache.get("https://example.com/homes").unwrap();
        assert_eq!(body[0]["systemId"], "sys");
        assert!(Utc::now() - fetched_at < TimeDelta::minutes(1));

        let other = Cache::load(Some(path.clone()), "someone@vaillant-germany-b2c");
        assert!(other.get("https://example.com/homes").is_none());

        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn persists_quota_pause() {
        let path = path("pause");
        let until = Local::now() + TimeDelta::hours(1);
        Cache::load(Some(path.clone()), "me").pause_until(until);

        assert_eq!(
            Cache::load(Some(path.clone()), "me").paused_until(),
            Some(until)
        );

        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn persists_token_readable_only_by_the_user() {
        use std::os::unix::fs::PermissionsExt;

        let path = path("token");
        let token: Token = serde_json::from_value(serde_json::json!({
            "access_token": "access",
            "refresh_token": "refresh",
            "expires_at": "2030-01-01T00:00:00Z",
        }))
        .unwrap();
        Cache::load(Some(path.clone()), "me").set_token(Some(token));

        let cache = Cache::load(Some(path.clone()), "me");
        assert_eq!(
            cache.token().map(|token| token.access_token.as_str()),
            Some("access")
        );
        assert_eq!(
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );

        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn ignores_missing_and_broken_files() {
        let path = path("broken");
        assert!(Cache::load(Some(path.clone()), "me").get("url").is_none());

        std::fs::write(&path, "{ broken").unwrap();
        assert!(Cache::load(Some(path.clone()), "me").get("url").is_none());

        std::fs::remove_file(path).unwrap();
    }
}
