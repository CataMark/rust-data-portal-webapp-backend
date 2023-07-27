use crate::AppContext;
use actix_web::{web, HttpResponse};
use std::time::Duration;

pub async fn user_get_single(
    // req: HttpRequest,
    ctx: web::Data<AppContext>,
    param_raw: web::Path<String>,
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = param_raw.into_inner();

    let Some(user) = crate::model::users::db_get_single(&user_id, &ctx, Duration::from_secs(10)).await? else {
        return Ok(HttpResponse::NoContent().finish());
    };
    Ok(HttpResponse::Ok().json(user))
}

pub async fn user_get_allowed_transaction_list(
    ctx: web::Data<AppContext>,
    param_raw: web::Path<String>,
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = param_raw.into_inner();

    let vals = crate::model::users::db_get_allowed_transaction_list(
        &user_id,
        &ctx,
        Duration::from_secs(10),
    )
    .await?;

    Ok(HttpResponse::Ok().json(vals))
}

#[cfg(test)]
mod tests {
    #[actix_web::test]
    async fn get_single() {
        let ctx = crate::init_app_data().unwrap();
        let resp = super::user_get_single(ctx, actix_web::web::Path::from("catalin".to_string()))
            .await
            .unwrap();
        let body = actix_web::body::MessageBody::try_into_bytes(resp.into_body()).unwrap();
        let user: crate::model::users::User = serde_json::from_slice(&body.to_vec()).unwrap();
        assert_eq!("catalin", user.user_id)
    }

    #[actix_web::test]
    async fn get_allowed_transactions() {
        let ctx = crate::init_app_data().unwrap();
        let resp = super::user_get_allowed_transaction_list(
            ctx,
            actix_web::web::Path::from("catalin".to_string()),
        )
        .await
        .unwrap();
        let body = actix_web::body::MessageBody::try_into_bytes(resp.into_body()).unwrap();
        let transactions: Vec<crate::model::users::UserTransaction> =
            serde_json::from_slice(&body.to_vec()).unwrap();
        assert!(transactions.len() >= 1)
    }
}
