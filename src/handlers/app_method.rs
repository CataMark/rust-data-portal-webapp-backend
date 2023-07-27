use crate::{extractors::multipart::MultipartFormData, AppContext};
use actix_web::{web, FromRequest, HttpRequest, HttpResponse};
use std::time::Duration;

pub async fn app_method_get_app_code_list(
    ctx: web::Data<AppContext>,
) -> Result<HttpResponse, actix_web::Error> {
    let res = crate::model::app_method::db_get_app_code_list(&ctx, Duration::from_secs(10)).await?;
    Ok(HttpResponse::Ok().json(res))
}

/// optional query parameter for app_code is "q"
pub async fn app_method_list(req: HttpRequest) -> Result<HttpResponse, actix_web::Error> {
    let Some(ctx) = req.app_data() else {
        return Err(actix_web::error::ErrorExpectationFailed("app context not found"));
    };
    let query = crate::helper::get_req_query_params(&req)?;
    let res = match query.get("q").map(|v| crate::model::app_method::AppCode {
        app_code: v.to_owned(),
    }) {
        Some(app_zone) => {
            crate::model::app_method::db_get_methods_by_app_code(
                &app_zone,
                &ctx,
                Duration::from_secs(10),
            )
            .await?
        }
        None => crate::model::app_method::db_get_methods_all(&ctx, Duration::from_secs(10)).await?,
    };
    Ok(HttpResponse::Ok().json(res))
}

pub async fn app_method_get_single_by_id(
    ctx: web::Data<AppContext>,
    id: web::Path<uuid::Uuid>,
) -> Result<HttpResponse, actix_web::Error> {
    let res =
        crate::model::app_method::db_method_get_by_id(&id, &ctx, Duration::from_secs(10)).await?;
    Ok(HttpResponse::Ok().json(res))
}

pub async fn app_method_delete_by_id(
    ctx: web::Data<AppContext>,
    id: web::Path<uuid::Uuid>,
) -> Result<HttpResponse, actix_web::Error> {
    let res = crate::model::app_method::db_method_delete_by_id(&id, &ctx, Duration::from_secs(10))
        .await?;
    Ok(if res > 0 {
        HttpResponse::Ok().finish()
    } else {
        HttpResponse::NoContent().body(format!("element not found"))
    })
}

pub async fn app_method_single_upsert(
    ctx: web::Data<AppContext>,
    method: web::Json<crate::model::app_method::AppMethod>,
    auth_data: crate::extractors::auth::AuthenticateData,
) -> Result<HttpResponse, actix_web::Error> {
    let mod_de = crate::extractors::auth::AuthClaims::from(auth_data);
    let res = crate::model::app_method::db_method_single_upsert(
        &method,
        &mod_de.sub,
        &ctx,
        Duration::from_secs(10),
    )
    .await?;
    Ok(HttpResponse::Ok().json(res))
}

/// optional query parameter for app_code is "q"
pub async fn app_method_down_xlsx(
    req: HttpRequest,
) -> Result<actix_files::NamedFile, actix_web::Error> {
    let Some(ctx) = req.app_data() else {
        return Err(actix_web::error::ErrorExpectationFailed("app context not found"));
    };
    let mod_de = crate::extractors::auth::AuthenticateData::from_request(
        &req,
        &mut actix_web::dev::Payload::None,
    )
    .await?
    .1;
    let query = crate::helper::get_req_query_params(&req)?;
    let (tmp_file, content_disposition) =
        match query.get("q").map(|v| crate::model::app_method::AppCode {
            app_code: v.to_owned(),
        }) {
            Some(app_zone) => (
                crate::helper::TempFile {
                    path: crate::model::app_method::db_methods_by_app_code_down_xlsx(
                        &app_zone,
                        &mod_de.sub,
                        &ctx,
                        Duration::from_secs(30),
                    )
                    .await?,
                },
                actix_web::http::header::ContentDisposition {
                    disposition: actix_web::http::header::DispositionType::Attachment,
                    parameters: vec![actix_web::http::header::DispositionParam::Filename(
                        format!("app_transactions_for_{}.xlsx", app_zone.app_code),
                    )],
                },
            ),
            None => (
                crate::helper::TempFile {
                    path: crate::model::app_method::db_methods_all_down_xlsx(
                        &mod_de.sub,
                        &ctx,
                        Duration::from_secs(30),
                    )
                    .await?,
                },
                actix_web::http::header::ContentDisposition {
                    disposition: actix_web::http::header::DispositionType::Attachment,
                    parameters: vec![actix_web::http::header::DispositionParam::Filename(
                        "app_transactions_all.xlsx".into(),
                    )],
                },
            ),
        };

    actix_files::NamedFile::open_async(tmp_file.path.as_path())
        .await
        .map(|f| f.set_content_disposition(content_disposition))
        .map_err(|err| actix_web::Error::from(err))
}

/// optional fields:
/// - "app_code", type String
/// - "sheet_name", type String
pub async fn app_method_up_xlsx(
    ctx: web::Data<AppContext>,
    auth_data: crate::extractors::auth::AuthenticateData,
    payload: actix_multipart::Multipart,
) -> Result<HttpResponse, actix_web::Error> {
    let mod_de = crate::extractors::auth::AuthClaims::from(auth_data);
    let file_prefix = format!("u-{}", mod_de.sub);
    let temp_dir = ctx.general.temp_dir.as_path();

    let form_data = MultipartFormData::from_multipart(
        temp_dir,
        &file_prefix,
        payload,
        Some(1),
        Some(&["xlsx"]),
    )
    .await
    .map_err(|err| actix_web::error::ErrorExpectationFailed(err))?;

    if form_data.file_paths.len() != 1 {
        return Err(actix_web::error::ErrorBadRequest(
            "no file with 'xlsx' extension loaded",
        ));
    }

    let app_code = form_data.fields.get("app_code");
    let sheet_name = form_data.fields.get("sheet_name");
    let file_path = form_data.file_paths.first().unwrap();

    let res = if let Some(v) = app_code {
        let app_zone = crate::model::app_method::AppCode {
            app_code: v.to_owned(),
        };
        crate::model::app_method::db_methods_by_app_code_up_xlsx(
            &app_zone,
            &mod_de.sub,
            &file_path.path,
            sheet_name.map(String::as_str),
            &ctx,
            Duration::from_secs(60),
        )
        .await?
    } else {
        crate::model::app_method::db_methods_all_up_xlsx(
            &mod_de.sub,
            &file_path.path,
            sheet_name.map(String::as_str),
            &ctx,
            Duration::from_secs(60),
        )
        .await?
    };

    Ok(HttpResponse::Ok().body(res.to_string()))
}

/// optional query parameter for app_code is "q"
pub async fn app_method_down_csv(
    req: HttpRequest,
) -> Result<actix_files::NamedFile, actix_web::Error> {
    let Some(ctx) = req.app_data() else {
        return Err(actix_web::error::ErrorExpectationFailed("app context not found"));
    };
    let mod_de = crate::extractors::auth::AuthenticateData::from_request(
        &req,
        &mut actix_web::dev::Payload::None,
    )
    .await?
    .1;
    let query = crate::helper::get_req_query_params(&req)?;
    let (tmp_file, content_disposition) =
        match query.get("q").map(|v| crate::model::app_method::AppCode {
            app_code: v.to_owned(),
        }) {
            Some(app_zone) => (
                crate::helper::TempFile {
                    path: crate::model::app_method::db_methods_by_app_code_down_csv(
                        &app_zone,
                        &mod_de.sub,
                        &ctx,
                        Duration::from_secs(30),
                    )
                    .await?,
                },
                actix_web::http::header::ContentDisposition {
                    disposition: actix_web::http::header::DispositionType::Attachment,
                    parameters: vec![actix_web::http::header::DispositionParam::Filename(
                        format!("app_transactions_for_{}.csv", app_zone.app_code),
                    )],
                },
            ),
            None => (
                crate::helper::TempFile {
                    path: crate::model::app_method::db_methods_all_down_csv(
                        &mod_de.sub,
                        &ctx,
                        Duration::from_secs(30),
                    )
                    .await?,
                },
                actix_web::http::header::ContentDisposition {
                    disposition: actix_web::http::header::DispositionType::Attachment,
                    parameters: vec![actix_web::http::header::DispositionParam::Filename(
                        "app_transactions_all.csv".into(),
                    )],
                },
            ),
        };

    actix_files::NamedFile::open_async(tmp_file.path.as_path())
        .await
        .map(|f| f.set_content_disposition(content_disposition))
        .map_err(|err| actix_web::Error::from(err))
}

/// mandatory fields:
/// - **column_delimiter**, validated by regex `^[,;\t|/]{1}$`
///
/// optional fields:
/// - **column_quote**, validated by regex `^["'|\\/]{1}$`
/// - **column_quote_escape**, validated by regex `^["'|\\/]{1}$`
/// - **app_code**, type String
pub async fn app_method_up_txt(
    ctx: web::Data<AppContext>,
    auth_data: crate::extractors::auth::AuthenticateData,
    payload: actix_multipart::Multipart,
) -> Result<HttpResponse, actix_web::Error> {
    let mod_de = crate::extractors::auth::AuthClaims::from(auth_data);
    let file_prefix = format!("u-{}", mod_de.sub);
    let temp_dir = ctx.general.temp_dir.as_path();

    let form_data = MultipartFormData::from_multipart(
        temp_dir,
        &file_prefix,
        payload,
        Some(1),
        Some(&["xlsx", "txt"]),
    )
    .await
    .map_err(|err| actix_web::error::ErrorExpectationFailed(err))?;

    if form_data.file_paths.len() != 1 {
        return Err(actix_web::error::ErrorBadRequest(
            "no file with 'csv' or 'txt' extension loaded",
        ));
    }

    let column_delimiter_regex = regex::Regex::new(crate::Consts::TXT_FILE_COLUMN_DELIM)
        .map_err(|err| actix_web::error::ErrorExpectationFailed(err))?;
    let column_delimiter = if let Some(v) = form_data.fields.get("column_delimiter") {
        if !column_delimiter_regex.is_match(v) {
            return Err(actix_web::error::ErrorBadRequest(
                "value supplied for field 'column_delimiter' is not correct",
            ));
        }
        v.as_bytes()[0]
    } else {
        return Err(actix_web::error::ErrorBadRequest(
            "no value was supplied for mandatory field 'column_delimiter'",
        ));
    };
    let column_quote_regex = regex::Regex::new(crate::Consts::TXT_FILE_COLUMN_QUOTE)
        .map_err(|err| actix_web::error::ErrorExpectationFailed(err))?;
    let column_quote = if let Some(v) = form_data.fields.get("column_quote") {
        if !column_quote_regex.is_match(v) {
            return Err(actix_web::error::ErrorBadRequest(
                "value supplied for field 'column_quote' is not correct",
            ));
        }
        Some(v.as_bytes()[0])
    } else {
        None
    };
    let column_quote_escape = if let Some(v) = form_data.fields.get("column_quote_escape") {
        if !column_quote_regex.is_match(v) {
            return Err(actix_web::error::ErrorBadRequest(
                "value supplied for field 'column_quote_escape' is not correct",
            ));
        }
        Some(v.as_bytes()[0])
    } else {
        None
    };
    let app_code = form_data.fields.get("app_code");
    let file_path = form_data.file_paths.first().unwrap();

    let res = if let Some(v) = app_code {
        let app_zone = crate::model::app_method::AppCode {
            app_code: v.to_owned(),
        };
        crate::model::app_method::db_methods_by_app_code_up_txt(
            &app_zone,
            &mod_de.sub,
            &file_path.path,
            column_delimiter,
            column_quote,
            column_quote_escape,
            &ctx,
            Duration::from_secs(60),
        )
        .await?
    } else {
        crate::model::app_method::db_methods_all_up_txt(
            &mod_de.sub,
            &file_path.path,
            column_delimiter,
            column_quote,
            column_quote_escape,
            &ctx,
            Duration::from_secs(60),
        )
        .await?
    };

    Ok(HttpResponse::Ok().body(res.to_string()))
}
