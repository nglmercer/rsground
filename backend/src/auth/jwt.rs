use std::sync::LazyLock;

use actix_web::HttpRequest;
use chrono::{Duration, TimeDelta, Utc};
use serde::{Deserialize, Serialize};

use crate::http_errors::HttpErrors;
use crate::utils::ArcStr;

/// The JWT secret is intentionally configurable, but a local default keeps the
/// guest-only development server usable after a fresh checkout. Deployments
/// should always provide `JWT_SECRET` through their environment.
pub static JWT_SECRET: LazyLock<String> = LazyLock::new(|| {
    std::env::var("JWT_SECRET")
        .ok()
        .filter(|secret| !secret.trim().is_empty())
        .unwrap_or_else(|| {
            log::warn!(
                "JWT_SECRET is not set; using a local development secret. Set it before deployment."
            );
            "rsground-local-development-secret".to_owned()
        })
});
const JWT_EXP: TimeDelta = Duration::hours(12);

#[derive(Debug, Serialize, Deserialize)]
pub struct RgUserData {
    pub id: ArcStr,
    pub name: ArcStr,
    pub is_guest: bool,
    pub exp: i64,
}

impl RgUserData {
    pub fn new(id: ArcStr, name: ArcStr, is_guest: bool) -> Self {
        let exp = Utc::now() + JWT_EXP;

        Self {
            id,
            name,
            is_guest,
            exp: exp.timestamp(),
        }
    }

    pub fn encode(self) -> Result<String, jsonwebtoken::errors::Error> {
        encode(self)
    }
}

pub fn encode(data: RgUserData) -> Result<String, jsonwebtoken::errors::Error> {
    jsonwebtoken::encode::<RgUserData>(
        &jsonwebtoken::Header::default(),
        &data,
        &jsonwebtoken::EncodingKey::from_secret(JWT_SECRET.as_bytes()),
    )
    .inspect_err(|err| log::error!("Error encoding jwt: {err}"))
}

pub fn decode(token: impl AsRef<str>) -> Option<RgUserData> {
    let token_data = jsonwebtoken::decode::<RgUserData>(
        token.as_ref(),
        &jsonwebtoken::DecodingKey::from_secret(JWT_SECRET.as_bytes()),
        &jsonwebtoken::Validation::default(),
    )
    .inspect_err(|err| {
        log::error!("Error al decodificar JWT: {err}");
    })
    .ok()?;

    if Utc::now().timestamp() >= token_data.claims.exp {
        return None;
    }

    Some(token_data.claims)
}

pub fn get_auth_token(req: &HttpRequest) -> Option<&str> {
    req.headers()
        .get("Authorization")
        .and_then(|auth| auth.to_str().ok())
        .and_then(|auth| auth.strip_prefix("Bearer "))
}

pub fn get_user_info(req: &HttpRequest) -> Result<RgUserData, HttpErrors> {
    let token = get_auth_token(req).ok_or_else(|| HttpErrors::NoTokenProvided)?;
    decode(token).ok_or(HttpErrors::InvalidJWT)
}
