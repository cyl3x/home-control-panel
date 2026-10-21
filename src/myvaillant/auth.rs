//! Login against the Vaillant Keycloak, ported from myPyllant (signalkraft/myPyllant v0.9.18).
//!
//! There is no public OAuth client, so this mimics the app: open the login page with a PKCE
//! challenge, post the credentials to the scraped form action and exchange the code from
//! the redirect to the app's custom URL scheme for a token.

use base64::prelude::*;
use chrono::{DateTime, TimeDelta, Utc};
use ring::rand::SecureRandom;
use std::num::NonZeroU32;
use ureq::Agent;
use ureq::http::Response;
use url::Url;

use super::Error;

const AUTH_BASE_URL: &str = "https://identity.vaillant-group.com/auth/realms";
const ALTCHA_CHALLENGE_URL: &str = "https://identity.vaillant-group.com/api/altcha/challenge";
const CLIENT_ID: &str = "myvaillant";
const REDIRECT_URI: &str = "enduservaillant.page.link://login";

/// Upper bound for the ALTCHA proof of work, a one byte key prefix needs 256 tries on average
const ALTCHA_MAX_COUNTER: u32 = 1 << 20;

/// Refresh tokens this long before they expire, same as the Home Assistant integration
const EXPIRY_MARGIN: TimeDelta = TimeDelta::minutes(3);

#[derive(Clone)]
pub struct Credentials {
    pub username: String,
    pub password: String,
    pub realm: String,
}

impl core::fmt::Debug for Credentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("<hidden>")
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Token {
    pub access_token: String,
    refresh_token: Option<String>,
    expires_at: DateTime<Utc>,
}

impl Token {
    pub fn expires_soon(&self) -> bool {
        self.expires_at - Utc::now() < EXPIRY_MARGIN
    }
}

#[derive(serde::Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: u64,
}

impl From<TokenResponse> for Token {
    fn from(response: TokenResponse) -> Self {
        Self {
            access_token: response.access_token,
            refresh_token: response.refresh_token,
            expires_at: Utc::now()
                + TimeDelta::seconds(i64::try_from(response.expires_in).unwrap_or(i64::MAX)),
        }
    }
}

#[derive(serde::Deserialize)]
struct AltchaChallenge {
    parameters: serde_json::Map<String, serde_json::Value>,
    signature: serde_json::Value,
}

/// Bulex, Glow-worm and DemirDöküm have one realm for all countries.
pub fn realm(brand: &str, country: Option<&str>) -> String {
    match (brand, country) {
        ("bulex" | "glow-worm" | "demirdokum", _) | (_, None) => format!("{brand}-b2c"),
        (_, Some(country)) => format!("{brand}-{country}-b2c"),
    }
}

/// Log in with username and password.
///
/// # Errors
/// Returns an error if the credentials are rejected or the login page changed.
pub fn login(agent: &Agent, credentials: &Credentials) -> Result<Token, Error> {
    let (verifier, challenge) = pkce()?;
    let code = authorization_code(agent, credentials, &challenge)?;

    request_token(
        agent,
        &credentials.realm,
        &[
            ("grant_type", "authorization_code"),
            ("client_id", CLIENT_ID),
            ("code", &code),
            ("code_verifier", &verifier),
            ("redirect_uri", REDIRECT_URI),
        ],
    )
}

/// Get a new token using the refresh token of the given one.
///
/// # Errors
/// Returns an error if the refresh token is missing, expired or revoked.
pub fn refresh(agent: &Agent, realm: &str, token: &Token) -> Result<Token, Error> {
    let refresh_token = token
        .refresh_token
        .as_deref()
        .ok_or_else(|| Error::Authentication("no refresh token".to_string()))?;

    request_token(
        agent,
        realm,
        &[
            ("refresh_token", refresh_token),
            ("client_id", CLIENT_ID),
            ("grant_type", "refresh_token"),
        ],
    )
}

fn authorization_code(
    agent: &Agent,
    credentials: &Credentials,
    challenge: &str,
) -> Result<String, Error> {
    // Start with a fresh Keycloak session, leftovers of a previous login could break the form
    agent.cookie_jar_lock().clear();

    let mut response = agent
        .get(format!(
            "{AUTH_BASE_URL}/{}/protocol/openid-connect/auth",
            credentials.realm
        ))
        .query("response_type", "code")
        .query("client_id", CLIENT_ID)
        .query("code", "code_challenge")
        .query("redirect_uri", REDIRECT_URI)
        .query("code_challenge_method", "S256")
        .query("code_challenge", challenge)
        .call()?;

    // An existing session redirects right away
    if let Some(code) = code_from_location(&response) {
        return Ok(code);
    }

    if !response.status().is_success() {
        return Err(Error::Authentication(format!(
            "login page returned {}, check brand and country",
            response.status()
        )));
    }

    let html = response.body_mut().read_to_string()?;
    let login_url = login_form_url(&html, &credentials.realm)
        .ok_or_else(|| Error::Authentication("could not find the login form".to_string()))?;

    let altcha = solve_altcha(agent)
        .inspect_err(|err| log::warn!("MyVaillant: continuing login without ALTCHA: {err}"))
        .ok();

    let mut form = vec![
        ("username", credentials.username.as_str()),
        ("password", credentials.password.as_str()),
        ("credentialId", ""),
    ];

    if let Some(altcha) = &altcha {
        form.push(("altcha", altcha));
    }

    let response = agent.post(&login_url).send_form(form)?;

    code_from_location(&response).ok_or_else(|| {
        Error::Authentication(format!(
            "login returned {} without a code, check username and password",
            response.status()
        ))
    })
}

fn request_token(agent: &Agent, realm: &str, form: &[(&str, &str)]) -> Result<Token, Error> {
    let url = format!("{AUTH_BASE_URL}/{realm}/protocol/openid-connect/token");
    let mut response = agent.post(&url).send_form(form.iter().copied())?;

    if !response.status().is_success() {
        return Err(Error::Authentication(format!(
            "token request returned {}: {}",
            response.status(),
            response.body_mut().read_to_string().unwrap_or_default()
        )));
    }

    Ok(response.body_mut().read_json::<TokenResponse>()?.into())
}

/// The authorization code is passed to the app via `enduservaillant.page.link://login?code=...`
fn code_from_location<B>(response: &Response<B>) -> Option<String> {
    let location = response.headers().get("location")?.to_str().ok()?;

    Url::parse(location)
        .ok()?
        .query_pairs()
        .find(|(key, _)| key == "code")
        .map(|(_, code)| code.into_owned())
}

/// Find the action of the login form, which carries the session of this login attempt.
fn login_form_url(html: &str, realm: &str) -> Option<String> {
    let prefix = format!("{AUTH_BASE_URL}/{realm}/login-actions/authenticate?");
    let start = html.find(&prefix)?;
    let end = html[start..]
        .find('"')
        .map_or(html.len(), |end| start + end);

    Some(html[start..end].replace("&amp;", "&"))
}

/// Code verifier and S256 code challenge for PKCE.
fn pkce() -> Result<(String, String), Error> {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    const LENGTH: usize = 128;

    let rng = ring::rand::SystemRandom::new();
    let mut verifier = String::with_capacity(LENGTH);
    let mut bytes = [0u8; LENGTH];

    while verifier.len() < LENGTH {
        rng.fill(&mut bytes)
            .map_err(|_| Error::Authentication("no secure random source".to_string()))?;

        let missing = LENGTH - verifier.len();

        // Reject bytes above the largest multiple of the charset length to avoid a modulo bias
        verifier.extend(
            bytes
                .iter()
                .map(|byte| usize::from(*byte))
                .filter(|byte| *byte < CHARSET.len() * 4)
                .map(|byte| char::from(CHARSET[byte % CHARSET.len()]))
                .take(missing),
        );
    }

    let challenge = code_challenge(&verifier);

    Ok((verifier, challenge))
}

fn code_challenge(verifier: &str) -> String {
    BASE64_URL_SAFE_NO_PAD.encode(ring::digest::digest(
        &ring::digest::SHA256,
        verifier.as_bytes(),
    ))
}

fn solve_altcha(agent: &Agent) -> Result<String, Error> {
    let mut response = agent.get(ALTCHA_CHALLENGE_URL).call()?;

    if !response.status().is_success() {
        return Err(Error::Authentication(format!(
            "ALTCHA challenge returned {}",
            response.status()
        )));
    }

    altcha_solution(response.body_mut().read_json()?)
}

/// Solve the ALTCHA proof of work of the login page and package it like the browser widget does.
///
/// The challenge parameters are sent back in the received order, which `serde_json`
/// keeps thanks to its `preserve_order` feature.
fn altcha_solution(challenge: AltchaChallenge) -> Result<String, Error> {
    let parameter = |key: &str| challenge.parameters.get(key);
    let hex_parameter = |key: &str| {
        parameter(key)
            .and_then(serde_json::Value::as_str)
            .and_then(hex_decode)
            .ok_or_else(|| Error::Parsing(format!("invalid ALTCHA parameter \"{key}\"")))
    };

    let nonce = hex_parameter("nonce")?;
    let salt = hex_parameter("salt")?;
    let key_prefix = hex_parameter("keyPrefix")?;
    let cost = parameter("cost")
        .and_then(serde_json::Value::as_u64)
        .and_then(|cost| u32::try_from(cost).ok())
        .and_then(NonZeroU32::new)
        .ok_or_else(|| Error::Parsing("invalid ALTCHA parameter \"cost\"".to_string()))?;
    let key_length = parameter("keyLength")
        .and_then(serde_json::Value::as_u64)
        .and_then(|length| usize::try_from(length).ok())
        .filter(|length| (1..=1024).contains(length))
        .unwrap_or(32);
    let algorithm = match parameter("algorithm").and_then(serde_json::Value::as_str) {
        Some("PBKDF2/SHA-512") => ring::pbkdf2::PBKDF2_HMAC_SHA512,
        Some("PBKDF2/SHA-384") => ring::pbkdf2::PBKDF2_HMAC_SHA384,
        _ => ring::pbkdf2::PBKDF2_HMAC_SHA256,
    };

    let mut password = [nonce.as_slice(), &[0; 4]].concat();
    let counter_offset = nonce.len();
    let mut derived = vec![0; key_length];

    for counter in 0..ALTCHA_MAX_COUNTER {
        password[counter_offset..].copy_from_slice(&counter.to_be_bytes());
        ring::pbkdf2::derive(algorithm, cost, &salt, &password, &mut derived);

        if derived.starts_with(&key_prefix) {
            let payload = serde_json::json!({
                "challenge": {
                    "parameters": challenge.parameters,
                    "signature": challenge.signature,
                },
                "solution": {
                    "counter": counter,
                    "derivedKey": hex_encode(&derived),
                    "time": 0,
                },
            });

            return Ok(BASE64_STANDARD.encode(payload.to_string()));
        }
    }

    Err(Error::Authentication(
        "ALTCHA challenge not solvable".to_string(),
    ))
}

fn hex_decode(hex: &str) -> Option<Vec<u8>> {
    if !hex.len().is_multiple_of(2) {
        return None;
    }

    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(hex.get(i..i + 2)?, 16).ok())
        .collect()
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Expected values are generated with myPyllant's implementation

    #[test]
    fn altcha_solution_matches_mypyllant() {
        // Parameters are out of alphabetical order on purpose, their order must be kept
        let challenge = r#"{"signature":"abc123","parameters":{"nonce":"0f1e2d3c4b5a69788796a5b4c3d2e1f0","algorithm":"PBKDF2/SHA-256","salt":"deadbeefcafebabe","cost":50,"keyPrefix":"00","keyLength":32}}"#;

        assert_eq!(
            altcha_solution(serde_json::from_str(challenge).unwrap()).unwrap(),
            "eyJjaGFsbGVuZ2UiOnsicGFyYW1ldGVycyI6eyJub25jZSI6IjBmMWUyZDNjNGI1YTY5Nzg4Nzk2YTViNGMzZDJlMWYwIiwiYWxnb3JpdGhtIjoiUEJLREYyL1NIQS0yNTYiLCJzYWx0IjoiZGVhZGJlZWZjYWZlYmFiZSIsImNvc3QiOjUwLCJrZXlQcmVmaXgiOiIwMCIsImtleUxlbmd0aCI6MzJ9LCJzaWduYXR1cmUiOiJhYmMxMjMifSwic29sdXRpb24iOnsiY291bnRlciI6ODIsImRlcml2ZWRLZXkiOiIwMDVkNDdiNzEzMDRjZTM5ZDU2NzMzMjZmYmYyYjAyZTVhODgwMDMzYTIzYmViN2ZhMTY3MGNjMmI2ZTJmZTNjIiwidGltZSI6MH19"
        );
    }

    #[test]
    fn altcha_solution_supports_sha512() {
        let challenge = r#"{"parameters":{"algorithm":"PBKDF2/SHA-512","cost":20,"keyLength":64,"keyPrefix":"0a","nonce":"11223344","salt":"aabbccdd"},"signature":"xyz"}"#;

        assert_eq!(
            altcha_solution(serde_json::from_str(challenge).unwrap()).unwrap(),
            "eyJjaGFsbGVuZ2UiOnsicGFyYW1ldGVycyI6eyJhbGdvcml0aG0iOiJQQktERjIvU0hBLTUxMiIsImNvc3QiOjIwLCJrZXlMZW5ndGgiOjY0LCJrZXlQcmVmaXgiOiIwYSIsIm5vbmNlIjoiMTEyMjMzNDQiLCJzYWx0IjoiYWFiYmNjZGQifSwic2lnbmF0dXJlIjoieHl6In0sInNvbHV0aW9uIjp7ImNvdW50ZXIiOjU5MiwiZGVyaXZlZEtleSI6IjBhZGNhM2Q4MzhjOGJlOGQ5Mzg1NGU0MzZlNDRiZGExNDcwMzhmYjFlYzhkNDE2MjJjZTVmZWZiNzFhZTg1ODA1Mzk5ODAxYTYwYzZlMmEyNjA5ZTY4N2ZhYTdlNGEzZjY1ZjQ5ZWQyNjA0YzE2NTM1YWVhY2Q5OWJiOGJlNTBmIiwidGltZSI6MH19"
        );
    }

    #[test]
    fn pkce_matches_mypyllant() {
        let (verifier, challenge) = pkce().unwrap();

        assert_eq!(verifier.len(), 128);
        assert!(verifier.chars().all(|c| c.is_ascii_alphanumeric()));
        assert_eq!(challenge, code_challenge(&verifier));
        assert_eq!(
            code_challenge(&("AbC123".repeat(21) + "xy")),
            "45xlJ8oX4hZYX8Wo_Hav0fkF9uLgt_sj37SaXgx9nQM"
        );
    }

    #[test]
    fn finds_login_form_url() {
        let html = r#"<form id="kc-form-login" action="https://identity.vaillant-group.com/auth/realms/vaillant-germany-b2c/login-actions/authenticate?session_code=abc&amp;execution=def&amp;client_id=myvaillant&amp;tab_id=ghi" method="post">"#;

        assert_eq!(
            login_form_url(html, "vaillant-germany-b2c").as_deref(),
            Some(
                "https://identity.vaillant-group.com/auth/realms/vaillant-germany-b2c/login-actions/authenticate?session_code=abc&execution=def&client_id=myvaillant&tab_id=ghi"
            )
        );
        assert_eq!(login_form_url(html, "vaillant-austria-b2c"), None);
    }

    #[test]
    fn reads_code_from_app_redirect() {
        let response = Response::builder()
            .header(
                "location",
                "enduservaillant.page.link://login?state=x&session_state=y&code=a.b-c",
            )
            .body(())
            .unwrap();

        assert_eq!(code_from_location(&response).as_deref(), Some("a.b-c"));
        assert_eq!(code_from_location(&Response::new(())), None);
    }

    #[test]
    fn builds_realm() {
        assert_eq!(realm("vaillant", Some("germany")), "vaillant-germany-b2c");
        assert_eq!(realm("bulex", Some("belgium")), "bulex-b2c");
        assert_eq!(realm("sdbg", None), "sdbg-b2c");
    }
}
