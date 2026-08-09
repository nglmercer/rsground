use std::io;

use actix_web::{App, HttpServer};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    dotenvy::dotenv().ok();

    log::info!("Iniciando servidor Actix-Web");

    let bind_address =
        std::env::var("RSGROUND_BIND").unwrap_or_else(|_| "127.0.0.1:8080".to_owned());
    backend::validate_configuration(&bind_address)
        .map_err(|message| io::Error::new(io::ErrorKind::InvalidInput, message))?;
    backend::initialize();
    let app_data = backend::new_app_data();
    log::info!("Listening on http://{bind_address}");

    HttpServer::new(move || {
        let app_data = app_data.clone();

        App::new()
            .wrap(backend::cors())
            .configure(move |config| app_data.configure(config))
    })
    .bind(bind_address)?
    .run()
    .await
}
