#[actix_web::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    //init logger
    let _logger = cdg_portal::init_logger()?;

    //stating the app
    log::info!("Starting server");

    // run server
    let app_data = cdg_portal::init_app_data().map_err(|err| {
        log::error!("Initializing app data -> {}", err);
        err
    })?;

    actix_web::HttpServer::new(move || cdg_portal::init_app_service(app_data.clone()))
        .bind(("0.0.0.0", 3001))
        .map_err(|err| {
            log::error!("Bind server to port error -> {}", err);
            err
        })?
        .run()
        .await
        .map_err(|err| {
            log::error!("Runing server error -> {}", err);
            err.into()
        })
}
