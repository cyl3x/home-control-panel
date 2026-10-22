//! Login against the AWS Cognito user pool of Envi IQ with SRP, like the Amplify library does.
//!
//! The SRP math is ported from pycognito (<https://github.com/NabuCasa/pycognito>, v2024.5.1),
//! which follows amazon-cognito-identity-js.

use std::collections::HashMap;

use base64::prelude::*;
use chrono::{DateTime, TimeDelta, Utc};
use num_bigint::BigUint;
use ring::hmac;
use ring::rand::SecureRandom;
use ureq::Agent;

use super::Error;

const COGNITO_URL: &str = "https://cognito-idp.eu-central-1.amazonaws.com/";

/// User pool and app client of the installer UI served by the Envi IQ box
const USER_POOL_ID: &str = "eu-central-1_f5oo3Vntf";
const CLIENT_ID: &str = "2h7tepepqtitmtcr4b0obh4ls";

/// The 3072 bit group of RFC 5054, with generator 2
const N_HEX: &str = concat!(
    "FFFFFFFFFFFFFFFFC90FDAA22168C234C4C6628B80DC1CD129024E088A67CC74",
    "020BBEA63B139B22514A08798E3404DDEF9519B3CD3A431B302B0A6DF25F1437",
    "4FE1356D6D51C245E485B576625E7EC6F44C42E9A637ED6B0BFF5CB6F406B7ED",
    "EE386BFB5A899FA5AE9F24117C4B1FE649286651ECE45B3DC2007CB8A163BF05",
    "98DA48361C55D39A69163FA8FD24CF5F83655D23DCA3AD961C62F356208552BB",
    "9ED529077096966D670C354E4ABC9804F1746C08CA18217C32905E462E36CE3B",
    "E39E772C180E86039B2783A2EC07A28FB5C55DF06F4C52C9DE2BCBF695581718",
    "3995497CEA956AE515D2261898FA051015728E5A8AAAC42DAD33170D04507A33",
    "A85521ABDF1CBA64ECFB850458DBEF0A8AEA71575D060C7DB3970F85A6E1E4C7",
    "ABF5AE8CDB0933D71E8C94E04A25619DCEE3D2261AD2EE6BF12FFA06D98A0864",
    "D87602733EC86A64521F2B18177B200CBBE117577A615D6C770988C0BAD946E2",
    "08E24FA074E5AB3143DB5BFCE0FD108E4B82D120A93AD2CAFFFFFFFFFFFFFFFF",
);

const HKDF_INFO: &[u8] = b"Caldera Derived Key";

/// Refresh tokens this long before they expire
const EXPIRY_MARGIN: TimeDelta = TimeDelta::minutes(3);

#[derive(Clone)]
pub struct Credentials {
    pub username: String,
    pub password: String,
}

impl core::fmt::Debug for Credentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("<hidden>")
    }
}

#[derive(Clone)]
pub struct Token {
    /// The API authorizes with the ID token, not the access token
    pub id_token: String,
    refresh_token: Option<String>,
    expires_at: DateTime<Utc>,
}

impl Token {
    pub fn expires_soon(&self) -> bool {
        self.expires_at - Utc::now() < EXPIRY_MARGIN
    }
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
struct AuthResponse {
    challenge_name: Option<String>,
    #[serde(default)]
    challenge_parameters: HashMap<String, String>,
    authentication_result: Option<AuthenticationResult>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
struct AuthenticationResult {
    id_token: String,
    refresh_token: Option<String>,
    expires_in: u64,
}

/// Log in with username and password.
///
/// # Errors
/// Returns an error if the credentials are rejected or Cognito asks for another challenge.
pub fn login(agent: &Agent, credentials: &Credentials) -> Result<Token, Error> {
    let srp = Srp::new()?;

    let response = call(
        agent,
        "InitiateAuth",
        &serde_json::json!({
            "AuthFlow": "USER_SRP_AUTH",
            "ClientId": CLIENT_ID,
            "AuthParameters": {
                "USERNAME": credentials.username,
                "SRP_A": format!("{:x}", srp.big_a),
            },
        }),
    )?;

    if response.challenge_name.as_deref() != Some("PASSWORD_VERIFIER") {
        return Err(unexpected_challenge(response.challenge_name));
    }

    let parameter = |key: &str| {
        response
            .challenge_parameters
            .get(key)
            .map(String::as_str)
            .ok_or_else(|| Error::Parsing(format!("challenge parameter \"{key}\" is missing")))
    };

    let timestamp = timestamp(Utc::now());
    let secret_block = parameter("SECRET_BLOCK")?;
    let signature = srp.password_signature(
        parameter("USER_ID_FOR_SRP")?,
        &credentials.password,
        parameter("SRP_B")?,
        parameter("SALT")?,
        secret_block,
        &timestamp,
    )?;

    let response = call(
        agent,
        "RespondToAuthChallenge",
        &serde_json::json!({
            "ChallengeName": "PASSWORD_VERIFIER",
            "ClientId": CLIENT_ID,
            "ChallengeResponses": {
                "USERNAME": parameter("USERNAME")?,
                "TIMESTAMP": timestamp,
                "PASSWORD_CLAIM_SECRET_BLOCK": secret_block,
                "PASSWORD_CLAIM_SIGNATURE": signature,
            },
        }),
    )?;

    into_token(response, None)
}

/// Get a new token using the refresh token of the given one.
///
/// # Errors
/// Returns an error if the refresh token is missing, expired or revoked.
pub fn refresh(agent: &Agent, token: &Token) -> Result<Token, Error> {
    let refresh_token = token
        .refresh_token
        .as_deref()
        .ok_or_else(|| Error::Authentication("no refresh token".to_string()))?;

    let response = call(
        agent,
        "InitiateAuth",
        &serde_json::json!({
            "AuthFlow": "REFRESH_TOKEN_AUTH",
            "ClientId": CLIENT_ID,
            "AuthParameters": { "REFRESH_TOKEN": refresh_token },
        }),
    )?;

    // Cognito only hands out a refresh token on login
    into_token(response, token.refresh_token.clone())
}

fn into_token(response: AuthResponse, refresh_token: Option<String>) -> Result<Token, Error> {
    let Some(result) = response.authentication_result else {
        return Err(unexpected_challenge(response.challenge_name));
    };

    Ok(Token {
        id_token: result.id_token,
        refresh_token: result.refresh_token.or(refresh_token),
        expires_at: Utc::now()
            + TimeDelta::seconds(i64::try_from(result.expires_in).unwrap_or(i64::MAX)),
    })
}

/// Challenges like MFA or a new password have to be finished in the app.
fn unexpected_challenge(challenge: Option<String>) -> Error {
    Error::Authentication(format!(
        "unexpected challenge {}, log in with the app once",
        challenge.as_deref().unwrap_or("none")
    ))
}

fn call(agent: &Agent, action: &str, body: &serde_json::Value) -> Result<AuthResponse, Error> {
    let mut response = agent
        .post(COGNITO_URL)
        .header("Content-Type", "application/x-amz-json-1.1")
        .header(
            "X-Amz-Target",
            format!("AWSCognitoIdentityProviderService.{action}"),
        )
        .send(body.to_string())?;

    // Errors look like {"__type":"NotAuthorizedException","message":"Incorrect username or password."}
    if !response.status().is_success() {
        return Err(Error::Authentication(format!(
            "{action} returned {}: {}",
            response.status(),
            response.body_mut().read_to_string().unwrap_or_default()
        )));
    }

    Ok(serde_json::from_reader(response.body_mut().as_reader())?)
}

/// Like `Fri Oct 2 09:05:07 UTC 2026`, the day is not padded.
fn timestamp(now: DateTime<Utc>) -> String {
    now.format("%a %b %-d %H:%M:%S UTC %Y").to_string()
}

struct Srp {
    n: BigUint,
    g: BigUint,
    k: BigUint,
    a: BigUint,
    big_a: BigUint,
}

impl Srp {
    fn new() -> Result<Self, Error> {
        let mut bytes = [0u8; 128];
        ring::rand::SystemRandom::new()
            .fill(&mut bytes)
            .map_err(|_| Error::Authentication("no secure random source".to_string()))?;

        Self::with_a(&BigUint::from_bytes_be(&bytes))
    }

    fn with_a(a: &BigUint) -> Result<Self, Error> {
        let n = BigUint::parse_bytes(N_HEX.as_bytes(), 16).expect("N is valid hex");
        let g = BigUint::from(2u32);
        let k = BigUint::from_bytes_be(&sha256(&[pad(&n), pad(&g)].concat()));
        let a = a % &n;
        let big_a = g.modpow(&a, &n);

        if big_a == BigUint::ZERO {
            return Err(Error::Authentication(
                "SRP safety check for A failed".to_string(),
            ));
        }

        Ok(Self { n, g, k, a, big_a })
    }

    /// Sign the secret block of the challenge with the key derived from the password.
    fn password_signature(
        &self,
        user_id: &str,
        password: &str,
        srp_b: &str,
        salt: &str,
        secret_block: &str,
        timestamp: &str,
    ) -> Result<String, Error> {
        let big_b = parse_hex(srp_b)?;
        if &big_b % &self.n == BigUint::ZERO {
            return Err(Error::Authentication(
                "SRP safety check for B failed".to_string(),
            ));
        }

        let u = BigUint::from_bytes_be(&sha256(&[pad(&self.big_a), pad(&big_b)].concat()));
        if u == BigUint::ZERO {
            return Err(Error::Authentication(
                "SRP safety check for u failed".to_string(),
            ));
        }

        let pool_name = USER_POOL_ID
            .split_once('_')
            .map_or(USER_POOL_ID, |(_, name)| name);
        let user_password = sha256(format!("{pool_name}{user_id}:{password}").as_bytes());
        let x = BigUint::from_bytes_be(&sha256(&[pad(&parse_hex(salt)?), user_password].concat()));

        // S = (B - k * g^x) ^ (a + u * x) mod N, adding N keeps the base positive
        let kgx = &self.k * self.g.modpow(&x, &self.n) % &self.n;
        let base = (&big_b % &self.n + &self.n - kgx) % &self.n;
        let s = base.modpow(&(&self.a + &u * &x), &self.n);

        let key = hkdf(&pad(&s), &pad(&u));
        let secret_block_bytes = BASE64_STANDARD
            .decode(secret_block)
            .map_err(|err| Error::Parsing(format!("invalid secret block: {err}")))?;
        let message = [
            pool_name.as_bytes(),
            user_id.as_bytes(),
            &secret_block_bytes,
            timestamp.as_bytes(),
        ]
        .concat();

        let signature = hmac::sign(&hmac::Key::new(hmac::HMAC_SHA256, &key), &message);

        Ok(BASE64_STANDARD.encode(signature.as_ref()))
    }
}

/// Big endian bytes, with a leading zero byte if the highest bit is set, so it reads as positive.
fn pad(value: &BigUint) -> Vec<u8> {
    let bytes = value.to_bytes_be();

    if bytes[0] & 0x80 == 0 {
        bytes
    } else {
        [&[0], bytes.as_slice()].concat()
    }
}

fn parse_hex(hex: &str) -> Result<BigUint, Error> {
    BigUint::parse_bytes(hex.as_bytes(), 16)
        .ok_or_else(|| Error::Parsing(format!("invalid hex number \"{hex}\"")))
}

fn sha256(data: &[u8]) -> Vec<u8> {
    ring::digest::digest(&ring::digest::SHA256, data)
        .as_ref()
        .to_vec()
}

/// HKDF with SHA256 and a single block, truncated to 16 bytes.
fn hkdf(ikm: &[u8], salt: &[u8]) -> Vec<u8> {
    let prk = hmac::sign(&hmac::Key::new(hmac::HMAC_SHA256, salt), ikm);
    let okm = hmac::sign(
        &hmac::Key::new(hmac::HMAC_SHA256, prk.as_ref()),
        &[HKDF_INFO, &[1]].concat(),
    );

    okm.as_ref()[..16].to_vec()
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    // Expected values are generated with pycognito's implementation

    #[test]
    fn password_signature_matches_pycognito() {
        let srp = Srp::with_a(&parse_hex(&"1f".repeat(128)).unwrap()).unwrap();
        let big_b = srp
            .g
            .modpow(&parse_hex(&"abc123".repeat(40)).unwrap(), &srp.n);

        assert!(format!("{:x}", srp.k).starts_with("538282c4354742d7cbbde2359fcf67f9"));
        assert!(format!("{:x}", srp.big_a).starts_with("3714ef8f70efe61f13883ff4e72a7880"));
        assert_eq!(
            srp.password_signature(
                "0b1c-uuid",
                "S3cret!pass",
                &format!("{big_b:x}"),
                "8f2e01",
                &BASE64_STANDARD.encode((0..64).collect::<Vec<u8>>()),
                "Fri Oct 2 09:05:07 UTC 2026",
            )
            .unwrap(),
            "X6ylnU7ayBuA5vdsW2tGc8Q8sA8pWbKXV3q48DSua3Q="
        );
    }

    #[test]
    fn rejects_zero_b() {
        let srp = Srp::with_a(&BigUint::from(12345u32)).unwrap();

        assert!(
            srp.password_signature("user", "password", N_HEX, "01", "", "")
                .is_err()
        );
    }

    #[test]
    fn formats_timestamp_without_padded_day() {
        assert_eq!(
            timestamp(Utc.with_ymd_and_hms(2026, 10, 2, 9, 5, 7).unwrap()),
            "Fri Oct 2 09:05:07 UTC 2026"
        );
        assert_eq!(
            timestamp(Utc.with_ymd_and_hms(2026, 10, 22, 13, 0, 0).unwrap()),
            "Thu Oct 22 13:00:00 UTC 2026"
        );
    }

    #[test]
    fn pads_values_with_highest_bit_set() {
        assert_eq!(pad(&BigUint::from(0x7fu32)), vec![0x7f]);
        assert_eq!(pad(&BigUint::from(0x8f2e01u32)), vec![0, 0x8f, 0x2e, 0x01]);
        assert_eq!(pad(&BigUint::ZERO), vec![0]);
    }
}
