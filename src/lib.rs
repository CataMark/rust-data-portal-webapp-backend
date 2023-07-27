pub mod extractors;
pub mod handlers;
pub mod helper;
pub mod middleware;
pub mod model;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub struct Consts;

impl Consts {
    pub const AUTH_COOKIE_NAME: &str = "atk";
    pub const AUTH_HEADER_NAME: &str = "X-Auth-Token";
    pub const EMAIL_REGEX_PATT: &str =
        r"^([a-z0-9_+]([a-z0-9_+.]*[a-z0-9_+])?)@([a-z0-9]+([\-\.]{1}[a-z0-9]+)*\.[a-z]{2,6})";
    pub const DB_APP_CODE: &str = "portal";
    pub const TXT_FILE_COLUMN_DELIM: &str = "^[,;\t|/]{1}$";
    pub const TXT_FILE_COLUMN_QUOTE: &str = r#"^["'|\\/]{1}$"#;
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct AuthQueryParam {
    pub atk: String, //field name must match AUTH_COOKIE_NAME constant(see above)
}

pub struct GeneralSettings {
    pub is_in_dev: bool,
    pub app_domain: String,
    pub app_path: String,
    pub sql_dir: PathBuf,
    pub static_files_dir: PathBuf,
    pub temp_dir: PathBuf,
}

impl GeneralSettings {
    pub fn get_sql(&self, file_name: &str) -> Result<String, actix_web::Error> {
        let file_path = self.sql_dir.join(file_name);
        let res = std::fs::read_to_string(file_path)?;
        Ok(res)
    }
}

pub struct AppContext {
    pub general: GeneralSettings,
    pub rsa_keys: utils::rsakeys::RsaKeys,
    pub pgsql_pool: dbpool::pgsql::Pool,
    pub mailer: utils::mailer::Mailer,
}

pub fn init_logger() -> Result<flexi_logger::LoggerHandle, Box<dyn std::error::Error + Send + Sync>>
{
    //init logger
    let file_specs = flexi_logger::FileSpec::default()
        .directory(&crate::helper::get_env("GEN_TEMP_DIRECTORY")?)
        .basename("log")
        .suffix("log");

    let logger = flexi_logger::Logger::try_with_str("info")?
        .log_to_file(file_specs)
        .duplicate_to_stderr(flexi_logger::Duplicate::Warn)
        .write_mode(flexi_logger::WriteMode::Async)
        .format_for_files(flexi_logger::opt_format)
        .rotate(
            flexi_logger::Criterion::Age(flexi_logger::Age::Day),
            flexi_logger::Naming::Timestamps,
            flexi_logger::Cleanup::KeepLogFiles(7),
        )
        .start()?;
    Ok(logger)
}

pub fn init_app_data(
) -> Result<actix_web::web::Data<AppContext>, Box<dyn std::error::Error + Send + Sync>> {
    // init paths constants
    let is_in_dev: bool = crate::helper::get_env("GEN_UNDER_DEVELOPMENT")?.parse()?;
    let app_domain = crate::helper::get_env("GEN_APPLICATION_DOMAIN")?;
    let app_path = crate::helper::get_env("GEN_APPLICATION_PATH")?;
    let sql_resource_dir = crate::helper::get_env("GEN_SQL_RESOURCE_DIR")?;
    let static_files_dir = crate::helper::get_env("GEN_STATIC_FILES_DIR")?;
    let temp_dir = crate::helper::get_env("GEN_TEMP_DIRECTORY")?;
    let paths = GeneralSettings {
        is_in_dev,
        app_domain,
        app_path,
        sql_dir: crate::helper::get_exist_path(sql_resource_dir.as_str())?,
        static_files_dir: crate::helper::get_exist_path(static_files_dir.as_str())?,
        temp_dir: crate::helper::get_exist_path(temp_dir.as_str())?,
    };

    // init RSA KEYS
    let rsa_pass = crate::helper::get_env("RSA_PASS")?;
    let rsa_priv_path = crate::helper::get_env("RSA_PRIV_KEY_PATH")?;
    let rsa_publ_path = crate::helper::get_env("RSA_PUB_KEY_PATH")?;
    let rsa_keys = utils::rsakeys::RsaKeys::init(&rsa_pass, &rsa_priv_path, &rsa_publ_path)?;

    // init PGSQL db pool
    let pgsql_conn_string = crate::helper::get_env("PGSQL_CONN_STRING")?;
    let pgsql_max_conn = crate::helper::get_env("PGSQL_POOL_MAX_CONN")?.parse()?;
    let pgsql_batch_size = crate::helper::get_env("PGSQL_BATCH_INSERTS")?.parse()?;
    let pgsql_pool =
        dbpool::pgsql::Pool::init(pgsql_conn_string, None, pgsql_max_conn, pgsql_batch_size)?;

    // init mail client
    let mail_config = utils::mailer::Config {
        from_addrs: lettre::message::Mailbox::new(
            Some(crate::helper::get_env("MAIL_FROM_NAME")?),
            crate::helper::get_env("MAIL_FROM_ADRS")?.parse::<lettre::Address>()?,
        ),
        reply_to: lettre::message::Mailbox::new(
            Some(crate::helper::get_env("MAIL_FROM_NAME")?),
            crate::helper::get_env("MAIL_REPLY_TO")?.parse::<lettre::Address>()?,
        ),
        server: crate::helper::get_env("MAIL_SMTP_SERVER")?,
        port: crate::helper::get_env("MAIL_SMTP_PORT")?.parse()?,
        user_name: crate::helper::get_env("MAIL_SMTP_USER")?,
        password: crate::helper::get_env("MAIL_SMTP_PASS")?,
        template_dir_path: crate::helper::get_env("MAIL_TEMPLATE_DIR")?,
        template_name_format: crate::helper::get_env("MAIL_TEMPLATE_NAME_FORMAT")?,
        languages: crate::helper::get_env("MAIL_LANGS")?
            .split(',')
            .filter(|v| !v.is_empty())
            .map(ToString::to_string)
            .collect(),
        default_language: crate::helper::get_env("MAIL_LANG_DEFAULT")?.to_lowercase(),
    };
    let mailer = utils::mailer::Mailer::init(mail_config);

    Ok(actix_web::web::Data::new(AppContext {
        general: paths,
        rsa_keys,
        pgsql_pool,
        mailer,
    }))
}

fn config_public(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(
        actix_web::web::resource("/static/{filename:.*}")
            .route(actix_web::web::get().to(crate::handlers::other::static_files)),
    )
    .service(
        actix_web::web::resource("/rsa/keys")
            .route(actix_web::web::get().to(crate::handlers::other::rsa_public)),
    );
}

fn config_auth(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(
        actix_web::web::resource("/login")
            .route(actix_web::web::post().to(crate::handlers::auth::user_login)),
    )
    .service(
        actix_web::web::resource("/auth")
            .wrap(crate::middleware::auth::AuthenticateFactory)
            .route(actix_web::web::get().to(crate::handlers::auth::authenticate)),
    )
    .service(
        actix_web::web::resource("/auth/isauth")
            .route(actix_web::web::get().to(crate::handlers::auth::is_authenticated)),
    )
    .service(
        actix_web::web::resource("/auth/user")
            .wrap(crate::middleware::auth::AuthenticateFactory)
            .route(actix_web::web::get().to(crate::handlers::auth::get_auth_user)),
    )
    .service(
        actix_web::web::resource("/auth/methods")
            .wrap(crate::middleware::auth::AuthenticateFactory)
            .route(actix_web::web::get().to(crate::handlers::auth::get_allowed_methods)),
    )
    .service(
        actix_web::web::resource("/logout")
            .wrap(crate::middleware::auth::AuthenticateFactory)
            .route(actix_web::web::get().to(crate::handlers::auth::user_logout)),
    );
}

fn config_users(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(
        actix_web::web::scope("/users")
            .wrap(crate::middleware::auth::AuthenticateFactory)
            .service(
                actix_web::web::resource("/get/{user_id}")
                    .wrap(crate::middleware::auth::AuthorizeFactory::new(
                        "portal",
                        "user_single_get",
                    ))
                    .route(actix_web::web::get().to(crate::handlers::users::user_get_single)),
            )
            .service(
                actix_web::web::resource("/list")
                    .wrap(crate::middleware::auth::AuthorizeFactory::new(
                        "portal",
                        "user_all_list",
                    ))
                    .route(
                        actix_web::web::get()
                            .to(|| async { actix_web::HttpResponse::Ok().body("user list") }),
                    ),
            ),
    );
}

fn config_app_method(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(
        actix_web::web::scope("/app_methods")
            .wrap(crate::middleware::auth::AuthenticateFactory)
            .service(
                actix_web::web::resource("")
                    .route(
                        actix_web::web::get()
                            .to(crate::handlers::app_method::app_method_list)
                            .wrap(crate::middleware::auth::AuthorizeFactory::new(
                                "portal",
                                "app_method_list_all",
                            )),
                    )
                    .route(
                        actix_web::web::post()
                            .to(crate::handlers::app_method::app_method_single_upsert)
                            .wrap(crate::middleware::auth::AuthorizeFactory::new(
                                "portal",
                                "app_method_upsert_single",
                            )),
                    ),
            )
            .service(
                actix_web::web::resource("/app_codes")
                    .wrap(crate::middleware::auth::AuthorizeFactory::new(
                        "portal",
                        "app_method_app_codes",
                    ))
                    .route(
                        actix_web::web::get()
                            .to(crate::handlers::app_method::app_method_get_app_code_list),
                    ),
            )
            .service(
                actix_web::web::resource("/xlsx")
                    .route(
                        actix_web::web::get()
                            .to(crate::handlers::app_method::app_method_down_xlsx)
                            .wrap(crate::middleware::auth::AuthorizeFactory::new(
                                "portal",
                                "app_method_list_all",
                            )),
                    )
                    .route(
                        actix_web::web::post()
                            .to(crate::handlers::app_method::app_method_up_xlsx)
                            .wrap(crate::middleware::auth::AuthorizeFactory::new(
                                "portal",
                                "app_method_upsert_all",
                            )),
                    ),
            )
            .service(
                actix_web::web::resource("/csv")
                    .route(
                        actix_web::web::get()
                            .to(crate::handlers::app_method::app_method_down_csv)
                            .wrap(crate::middleware::auth::AuthorizeFactory::new(
                                "portal",
                                "app_method_list_all",
                            )),
                    )
                    .route(
                        actix_web::web::post()
                            .to(crate::handlers::app_method::app_method_up_txt)
                            .wrap(crate::middleware::auth::AuthorizeFactory::new(
                                "portal",
                                "app_method_upsert_all",
                            )),
                    ),
            )
            .service(
                actix_web::web::resource("/{id}")
                    .route(
                        actix_web::web::get()
                            .to(crate::handlers::app_method::app_method_get_single_by_id)
                            .wrap(crate::middleware::auth::AuthorizeFactory::new(
                                "portal",
                                "app_method_get_single_by_id",
                            )),
                    )
                    .route(
                        actix_web::web::delete()
                            .to(crate::handlers::app_method::app_method_delete_by_id)
                            .wrap(crate::middleware::auth::AuthorizeFactory::new(
                                "portal",
                                "app_method_del_single_by_id",
                            )),
                    ),
            ),
    );
}

pub fn init_app_service(
    app_data: actix_web::web::Data<AppContext>,
) -> actix_web::App<
    impl actix_web::dev::ServiceFactory<
        actix_web::dev::ServiceRequest,
        Config = (),
        Response = actix_web::dev::ServiceResponse,
        Error = actix_web::Error,
        InitError = (),
    >,
> {
    let app_path = app_data.general.app_path.clone();

    actix_web::App::new()
        .app_data(app_data)
        .wrap(crate::middleware::logger::LoggerFactory)
        .wrap(actix_web::middleware::NormalizePath::trim())
        .service(actix_web::web::scope(&app_path).configure(|cfg| {
            config_public(cfg);
            config_auth(cfg);
            config_users(cfg);
            config_app_method(cfg);
        }))
        .route(
            "/",
            actix_web::web::get().to(|| async { "Hello from the Server" }),
        )
        .default_service(actix_web::web::route().method(actix_web::http::Method::GET))
}
