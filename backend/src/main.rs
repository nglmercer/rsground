use actix_cors::Cors;
use actix_web::{App, HttpServer};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    dotenv::dotenv().ok();

    backend::initialize();

    let app_data = backend::new_app_data();

    log::info!("Iniciando servidor Actix-Web");

    let bind_address =
        std::env::var("RSGROUND_BIND").unwrap_or_else(|_| "127.0.0.1:8080".to_owned());
    log::info!("Listening on http://{bind_address}");

    HttpServer::new(move || {
        App::new()
            .wrap(
                Cors::default()
                    .allow_any_origin()
                    .allow_any_method()
                    .allow_any_header(),
            )
            .configure(|config| app_data.configure(config))
    })
    .bind(bind_address)?
    .run()
    .await
}
