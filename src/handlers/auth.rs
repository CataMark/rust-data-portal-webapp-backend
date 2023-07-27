use actix_web::{web, FromRequest, HttpResponse};

use crate::{extractors::auth::AuthClaims, AppContext};

pub async fn user_login(
    ctx: web::Data<AppContext>,
    data: web::Json<crate::model::users::LoginData>,
) -> Result<HttpResponse, actix_web::Error> {
    //get user account from request
    let Some(user) = crate::model::users::db_get_single(data.user_id.as_str(), &ctx, std::time::Duration::from_secs(10))
        .await
        .map_err(|e| actix_web::error::ErrorExpectationFailed(e))? else {
        return Err(actix_web::error::ErrorUnauthorized("user not found"))
    };

    //get last token data
    let last_token = crate::model::users::db_get_last_token_id(
        &user.user_id,
        &ctx,
        std::time::Duration::from_secs(10),
    )
    .await?;

    //check if login reuest is not too early
    if let Some(token) = last_token {
        let now = chrono::Utc::now();
        let diff = now
            .signed_duration_since(chrono::DateTime::<chrono::Utc>::from_utc(
                token.mod_timp,
                chrono::Utc,
            ))
            .num_minutes();
        if diff <= 10 {
            return Err(actix_web::error::ErrorNotAcceptable(format!(
                "wait-minutes:{}",
                diff
            )));
        }
    }

    //prepare claims for new token
    let iat = chrono::Utc::now();
    let exp = iat
        .checked_add_signed(chrono::Duration::minutes(10))
        .unwrap_or(iat);

    let claims = AuthClaims {
        iss: ctx.general.app_domain.clone(),
        sub: user.user_id,
        jti: uuid::Uuid::new_v4(),
        iat: iat.timestamp(),
        exp: exp.timestamp(),
    };

    let jwt = claims.create_token(&ctx)?;
    let to_addrs = vec![lettre::message::Mailbox::new(
        Some(format!("{} {}", user.first_name, user.last_name)),
        user.email
            .parse()
            .map_err(|e| actix_web::error::ErrorExpectationFailed(e))?,
    )];
    let subject = "Portal CDG - Autentificare".to_string();
    let message = format!(
        r#"
        <span>Pentru autentificarea in portalul CdG va rog accesati urmatorul link: 
        <a href="{0}{1}/auth?{2}={3}">autentificare</a>.</span><br/>
        <span>Acest link expira in 10 minute</span>
    "#,
        ctx.general.app_domain,
        ctx.general.app_path,
        crate::Consts::AUTH_COOKIE_NAME,
        jwt
    );

    //send login link to user
    let _ = ctx
        .mailer
        .send(to_addrs, None, &subject, &message, None, None)
        .map_err(|e| actix_web::error::ErrorExpectationFailed(e))?;

    //save new token data
    let _ = crate::model::users::db_persist_last_token_id(
        &claims.into(),
        &ctx,
        std::time::Duration::from_secs(10),
    )
    .await?;

    //return response
    Ok(HttpResponse::Ok().finish())
}

pub async fn authenticate(
    ctx: web::Data<AppContext>,
    auth_data: crate::extractors::auth::AuthenticateData,
) -> Result<HttpResponse, actix_web::Error> {
    //get token from request query string
    let mut claims = auth_data.1;
    //create new long lived authentication token
    let iat = chrono::Utc::now();
    let exp = iat
        .checked_add_signed(chrono::Duration::days(90))
        .unwrap_or(iat);

    claims.jti = uuid::Uuid::new_v4();
    claims.iat = iat.timestamp();
    claims.exp = exp.timestamp();

    let jwt = claims.create_token(&ctx)?;
    let app_path: &str = if ctx.general.is_in_dev {
        "/"
    } else {
        &ctx.general.app_path
    };
    //save new token data
    let _ = crate::model::users::db_persist_last_token_id(
        &claims.into(),
        &ctx,
        std::time::Duration::from_secs(10),
    )
    .await?;
    //send response
    Ok(HttpResponse::Found()
        .append_header((crate::Consts::AUTH_HEADER_NAME, jwt.as_str()))
        .cookie(
            actix_web::cookie::Cookie::build(crate::Consts::AUTH_COOKIE_NAME, jwt)
                .path(app_path)
                .http_only(true)
                .secure(true)
                .expires(time::OffsetDateTime::now_utc().checked_add(time::Duration::days(90)))
                .finish(),
        )
        //.append_header((actix_web::http::header::LOCATION, app_path))
        .finish())
}

pub async fn is_authenticated(req: actix_web::HttpRequest) -> HttpResponse {
    let res = match crate::extractors::auth::AuthenticateData::from_request(
        &req,
        &mut actix_web::dev::Payload::None,
    )
    .await
    {
        Ok(_) => true,
        Err(_) => false,
    };
    HttpResponse::Ok().body(res.to_string())
}

pub async fn get_auth_user(
    ctx: web::Data<AppContext>,
    auth_data: crate::extractors::auth::AuthenticateData,
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = crate::extractors::auth::AuthClaims::from(auth_data).sub;
    let Some(user) = crate::model::users::db_get_single(&user_id, &ctx, std::time::Duration::from_secs(10)).await? else {
        return Err(actix_web::error::ErrorExpectationFailed("no auth user"));
    };
    Ok(HttpResponse::Ok().json(user))
}

pub async fn get_allowed_methods(
    ctx: web::Data<AppContext>,
    auth_data: crate::extractors::auth::AuthenticateData,
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = crate::extractors::auth::AuthClaims::from(auth_data).sub;
    let res = crate::model::users::db_get_allowed_transaction_list(
        &user_id,
        &ctx,
        std::time::Duration::from_secs(10),
    )
    .await?;
    Ok(HttpResponse::Ok().json(res))
}

pub async fn user_logout(
    ctx: web::Data<AppContext>,
    auth_data: crate::extractors::auth::AuthenticateData,
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = crate::extractors::auth::AuthClaims::from(auth_data).sub;
    let new_token = crate::model::users::UserLastAuthToken {
        id: None,
        user_id,
        token_id: uuid::Uuid::new_v4(),
        mod_timp: chrono::Utc::now().naive_utc(),
    };
    let app_path: &str = if ctx.general.is_in_dev {
        "/"
    } else {
        &ctx.general.app_path
    };

    //save new token data
    let _ = crate::model::users::db_persist_last_token_id(
        &new_token,
        &ctx,
        std::time::Duration::from_secs(10),
    )
    .await?;

    //send response
    Ok(HttpResponse::Found()
        .append_header((crate::Consts::AUTH_HEADER_NAME, ""))
        .cookie(
            actix_web::cookie::Cookie::build(crate::Consts::AUTH_COOKIE_NAME, "")
                .path(app_path)
                .http_only(true)
                .secure(true)
                .max_age(time::Duration::seconds(0))
                .expires(time::OffsetDateTime::now_utc().checked_sub(time::Duration::days(365)))
                .finish(),
        )
        .finish())
}
