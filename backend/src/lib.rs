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
    std::env::var(env::MAX_USERS)
        .ok()
        .and_then(|value| value.parse().ok())
        .filter(|limit: &usize| *limit > 0)
        .unwrap_or(constants::limits::DEFAULT_MAX_USERS)
}

pub fn initialize() {
    LazyLock::force(&auth::jwt::JWT_SECRET);
}

pub fn validate_configuration(bind_address: &str) -> Result<(), String> {
    let production = std::env::var(env::ENVIRONMENT).is_ok_and(|value| {
        value.eq_ignore_ascii_case(environment::PRODUCTION)
            || value.eq_ignore_ascii_case(environment::PRODUCTION_ALIAS)
    });
    let public_bind = bind_address
        .parse::<SocketAddr>()
        .map(|address| !address.ip().is_loopback())
        .unwrap_or(true);

    if production || public_bind {
        auth::jwt::validate_deployment_secret()?;
    }

    if (production || public_bind)
        && !std::env::var(env::CORS_ORIGINS)
            .ok()
            .is_some_and(|origins| {
                let origins = origins
                    .split(',')
                    .map(str::trim)
                    .filter(|origin| !origin.is_empty())
                    .collect::<Vec<_>>();
                !origins.is_empty() && origins.iter().all(|origin| *origin != http::CORS_WILDCARD)
            })
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
