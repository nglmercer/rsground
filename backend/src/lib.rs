mod auth;
mod collab;
mod health;
mod http_errors;
mod project;
mod state;
mod utils;
mod ws;

use std::collections::HashMap;
use std::sync::LazyLock;

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
