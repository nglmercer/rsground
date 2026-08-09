mod auth;
mod collab;
pub mod constants;
mod health;
mod http_errors;
mod project;
mod state;
mod utils;
mod ws;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::LazyLock;

use actix_cors::Cors;
use auth::github;
use auth::routes::OAuthData;
use constants::{defaults, env, environment, http};
use state::AppState;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct AppData {
    app_state: actix_web::web::Data<AppState>,
    oauth_data: actix_web::web::Data<OAuthData>,
}

impl AppData {
    pub fn configure(&self, config: &mut actix_web::web::ServiceConfig) {
        config
            .app_data(self.app_state.clone())
            .app_data(self.oauth_data.clone())
            .configure(configure_routes);
    }
}

pub fn new_app_data() -> AppData {
    AppData {
        app_state: actix_web::web::Data::new(AppState {
            manager: Mutex::new(project::ProjectManager::new()).into(),
            usernames: Mutex::new(HashMap::new()).into(),
            max_users: configured_max_users(),
        }),
        oauth_data: actix_web::web::Data::new(OAuthData {
            client: github::get_oauth_client(),
        }),
    }
}

fn configured_max_users() -> usize {
    configured_limit(std::env::var(env::MAX_USERS).ok().as_deref())
}

fn configured_limit(value: Option<&str>) -> usize {
    value
        .and_then(|value| value.parse().ok())
        .filter(|limit: &usize| *limit > 0)
        .unwrap_or(constants::limits::DEFAULT_MAX_USERS)
}

pub fn initialize() {
    LazyLock::force(&auth::jwt::JWT_SECRET);
}

pub fn validate_configuration(bind_address: &str) -> Result<(), String> {
    let production = is_production(std::env::var(env::ENVIRONMENT).ok().as_deref());
    let public_bind = is_public_bind(bind_address);

    if production || public_bind {
        auth::jwt::validate_deployment_secret()?;
    }

    if (production || public_bind)
        && !has_explicit_cors_origins(std::env::var(env::CORS_ORIGINS).ok().as_deref())
    {
        return Err(format!(
            "{} must contain explicit non-wildcard origins for deployment",
            env::CORS_ORIGINS
        ));
    }

    rsground_runner::Runner::validate_environment().map_err(|error| {
        log::error!("Runner configuration is invalid: {error}");
        "The runner sandbox is not available".to_owned()
    })?;

    Ok(())
}

fn is_production(value: Option<&str>) -> bool {
    value.is_some_and(|value| {
        value.eq_ignore_ascii_case(environment::PRODUCTION)
            || value.eq_ignore_ascii_case(environment::PRODUCTION_ALIAS)
    })
}

fn is_public_bind(bind_address: &str) -> bool {
    bind_address
        .parse::<SocketAddr>()
        .map(|address| !address.ip().is_loopback())
        .unwrap_or(true)
}

fn has_explicit_cors_origins(value: Option<&str>) -> bool {
    value.is_some_and(|origins| {
        let origins = origins
            .split(',')
            .map(str::trim)
            .filter(|origin| !origin.is_empty())
            .collect::<Vec<_>>();
        !origins.is_empty() && origins.iter().all(|origin| *origin != http::CORS_WILDCARD)
    })
}

pub fn cors() -> Cors {
    let origins = std::env::var(env::CORS_ORIGINS)
        .ok()
        .map(|value| {
            value
                .split(',')
                .map(str::trim)
                .filter(|origin| !origin.is_empty())
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
        .filter(|origins| !origins.is_empty())
        .unwrap_or_else(|| {
            defaults::CORS_ORIGINS
                .into_iter()
                .map(str::to_owned)
                .collect()
        });

    let mut cors = Cors::default()
        .allowed_methods([http::GET_METHOD, http::POST_METHOD, http::OPTIONS_METHOD])
        .allowed_headers([
            http::AUTHORIZATION_HEADER,
            http::CONTENT_TYPE_HEADER,
            http::PROJECT_PASSWORD_HEADER,
        ])
        .supports_credentials();

    for origin in origins {
        cors = cors.allowed_origin(&origin);
    }

    cors
}

fn configure_routes(config: &mut actix_web::web::ServiceConfig) {
    config
        .service(health::health)
        .service(auth::routes::auth)
        .service(auth::routes::callback)
        .service(auth::routes::login_guest)
        .service(auth::routes::me)
        .service(auth::routes::update_name)
        .service(project::routes::create_project)
        .service(project::routes::fork_project)
        .service(project::routes::get_project)
        .service(ws::routes::websocket);
}

#[cfg(test)]
mod tests {
    use super::{configured_limit, has_explicit_cors_origins, is_production, is_public_bind};
    use crate::constants::limits;

    #[test]
    fn uses_only_positive_numeric_user_limits() {
        assert_eq!(configured_limit(None), limits::DEFAULT_MAX_USERS);
        assert_eq!(configured_limit(Some("invalid")), limits::DEFAULT_MAX_USERS);
        assert_eq!(configured_limit(Some("0")), limits::DEFAULT_MAX_USERS);
        assert_eq!(configured_limit(Some("25")), 25);
    }

    #[test]
    fn detects_production_and_public_bindings() {
        assert!(is_production(Some("production")));
        assert!(is_production(Some("PROD")));
        assert!(!is_production(Some("development")));
        assert!(!is_production(None));

        assert!(!is_public_bind("127.0.0.1:8080"));
        assert!(!is_public_bind("[::1]:8080"));
        assert!(is_public_bind("0.0.0.0:8080"));
        assert!(is_public_bind("not-an-address"));
    }

    #[test]
    fn requires_non_wildcard_cors_origins_for_public_configuration() {
        assert!(!has_explicit_cors_origins(None));
        assert!(!has_explicit_cors_origins(Some("")));
        assert!(!has_explicit_cors_origins(Some("*, http://localhost:3000")));
        assert!(has_explicit_cors_origins(Some("http://localhost:3000")));
        assert!(has_explicit_cors_origins(Some("http://a, http://b")));
    }
}
