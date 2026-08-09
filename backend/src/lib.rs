mod auth;
mod collab;
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
        }),
        oauth_data: actix_web::web::Data::new(OAuthData {
            client: github::get_oauth_client(),
        }),
    }
}

pub fn initialize() {
    LazyLock::force(&auth::jwt::JWT_SECRET);
}

pub fn validate_configuration(bind_address: &str) -> Result<(), String> {
    let production = std::env::var("RSGROUND_ENV").is_ok_and(|value| {
        value.eq_ignore_ascii_case("production") || value.eq_ignore_ascii_case("prod")
    });
    let public_bind = bind_address
        .parse::<SocketAddr>()
        .map(|address| !address.ip().is_loopback())
        .unwrap_or(true);

    if production || public_bind {
        auth::jwt::validate_deployment_secret()?;
    }

    if (production || public_bind)
        && !std::env::var("RSGROUND_CORS_ORIGINS")
            .ok()
            .is_some_and(|origins| origins.split(',').any(|origin| !origin.trim().is_empty()))
    {
        return Err("RSGROUND_CORS_ORIGINS must be set for deployment".to_owned());
    }

    rsground_runner::Runner::validate_environment().map_err(|error| {
        log::error!("Runner configuration is invalid: {error}");
        "The runner sandbox is not available".to_owned()
    })?;

    Ok(())
}

pub fn cors() -> Cors {
    let origins = std::env::var("RSGROUND_CORS_ORIGINS")
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
            vec![
                "http://localhost:3000".to_owned(),
                "http://127.0.0.1:3000".to_owned(),
            ]
        });

    let mut cors = Cors::default()
        .allowed_methods(["GET", "POST", "OPTIONS"])
        .allowed_headers(["Authorization", "Content-Type", "X-Project-Password"])
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
