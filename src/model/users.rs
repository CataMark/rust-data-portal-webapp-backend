use crate::AppContext;
use actix_web::web;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct User {
    pub user_id: String,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub mod_de: Option<String>,
    pub mod_timp: Option<NaiveDateTime>,
}

impl TryFrom<tokio_postgres::row::Row> for User {
    type Error = dbpool::error::ErrorReport;

    fn try_from(row: tokio_postgres::Row) -> Result<Self, Self::Error> {
        Ok(Self {
            user_id: row.try_get("user_id")?,
            first_name: row.try_get("first_name")?,
            last_name: row.try_get("last_name")?,
            email: row.try_get("email")?,
            mod_de: row.try_get("mod_de")?,
            mod_timp: row.try_get("mod_timp")?,
        })
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct UserTransaction {
    pub user_id: String,
    pub app_code: String,
    pub method_code: String,
}

impl TryFrom<tokio_postgres::Row> for UserTransaction {
    type Error = dbpool::error::ErrorReport;

    fn try_from(row: tokio_postgres::row::Row) -> Result<Self, Self::Error> {
        Ok(Self {
            user_id: row.try_get("user_id")?,
            app_code: row.try_get("app_code")?,
            method_code: row.try_get("method_code")?,
        })
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct UserLastAuthToken {
    pub id: Option<uuid::Uuid>,
    pub user_id: String,
    pub token_id: uuid::Uuid,
    pub mod_timp: chrono::NaiveDateTime,
}

impl TryFrom<tokio_postgres::Row> for UserLastAuthToken {
    type Error = dbpool::error::ErrorReport;

    fn try_from(row: tokio_postgres::row::Row) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.try_get("id")?,
            user_id: row.try_get("user_id")?,
            token_id: row.try_get("token_id")?,
            mod_timp: row.try_get("mod_timp")?,
        })
    }
}

#[derive(Serialize, Deserialize)]
pub struct LoginData {
    pub user_id: String,
}

pub async fn db_check_authorization(
    user_id: &str,
    app_code: &str,
    method_code: &str,
    ctx: &web::Data<AppContext>,
    timeout: Duration,
) -> Result<bool, actix_web::Error> {
    let db = &ctx.pgsql_pool;
    let sql = ctx
        .general
        .get_sql("pgsql_api_user_authorization_check.sql")?;
    let param_types: &[postgres_types::Type] = &[
        postgres_types::Type::TEXT,
        postgres_types::Type::TEXT,
        postgres_types::Type::TEXT,
    ];
    let param_values: &[&(dyn postgres_types::ToSql + Sync)] = &[&user_id, &app_code, &method_code];

    let callable = |conn| async move {
        dbpool::pgsql::connection_get(&conn, sql.as_str(), Some(param_types), Some(param_values))
            .await
    };

    let rows: Vec<dbpool::generics::GenericSqlRow<String, dbpool::generics::GenericWrapper>> = db
        .conn_get(callable, timeout)
        .await
        .map_err(actix_web::error::ErrorExpectationFailed)?;

    let res = match rows.get(0) {
        Some(m) => match m.as_ref().get_index(0) {
            Some((_, dbpool::generics::GenericWrapper::Bool(v))) => *v,
            _ => false,
        },
        None => false,
    };
    Ok(res)
}

pub async fn db_get_allowed_transaction_list(
    user_id: &str,
    ctx: &web::Data<AppContext>,
    timeout: Duration,
) -> Result<Vec<UserTransaction>, actix_web::Error> {
    let db = &ctx.pgsql_pool;
    let sql = ctx
        .general
        .get_sql("pgsql_api_user_get_auth_transaction_list.sql")?;
    let param_types: &[postgres_types::Type] = &[postgres_types::Type::TEXT];
    let param_values: &[&(dyn postgres_types::ToSql + Sync)] = &[&user_id];

    let callable = |conn| async move {
        dbpool::pgsql::connection_get(&conn, sql.as_str(), Some(param_types), Some(param_values))
            .await
    };

    let res: Vec<UserTransaction> = db
        .conn_get(callable, timeout)
        .await
        .map_err(actix_web::error::ErrorExpectationFailed)?;

    Ok(res)
}

pub async fn db_persist_single(
    user: &User,
    mod_de: &str,
    ctx: &web::Data<AppContext>,
    timeout: Duration,
) -> Result<Option<User>, actix_web::Error> {
    let db = &ctx.pgsql_pool;
    let sql = ctx.general.get_sql("pgsql_api_user_single_upsert.sql")?;
    let param_types: &[postgres_types::Type] = &[
        postgres_types::Type::TEXT,
        postgres_types::Type::TEXT,
        postgres_types::Type::TEXT,
        postgres_types::Type::TEXT,
        postgres_types::Type::TEXT,
    ];
    let param_values: &[&(dyn postgres_types::ToSql + Sync)] = &[
        &user.user_id,
        &user.first_name,
        &user.last_name,
        &user.email,
        &mod_de,
    ];

    let callable = |conn| async move {
        dbpool::pgsql::connection_get(&conn, sql.as_str(), Some(param_types), Some(param_values))
            .await
    };
    let rows: Vec<User> = db
        .conn_get(callable, timeout)
        .await
        .map_err(actix_web::error::ErrorExpectationFailed)?;
    let res = rows.get(0).map(|v| v.to_owned());
    Ok(res)
}

pub async fn db_get_single(
    user_id: &str,
    ctx: &web::Data<AppContext>,
    timeout: Duration,
) -> Result<Option<User>, actix_web::Error> {
    let db = &ctx.pgsql_pool;
    let sql = ctx.general.get_sql("pgsql_api_user_get_single.sql")?;
    let param_types: &[postgres_types::Type] = &[postgres_types::Type::TEXT];
    let param_values: &[&(dyn postgres_types::ToSql + Sync)] = &[&user_id];

    let callable = |conn| async move {
        dbpool::pgsql::connection_get(&conn, sql.as_str(), Some(param_types), Some(param_values))
            .await
    };
    let rows: Vec<User> = db
        .conn_get(callable, timeout)
        .await
        .map_err(actix_web::error::ErrorExpectationFailed)?;
    let res = rows.get(0).map(|v| v.to_owned());
    Ok(res)
}

pub async fn db_get_all(
    ctx: &web::Data<AppContext>,
    timeout: Duration,
) -> Result<Vec<User>, actix_web::Error> {
    let db = &ctx.pgsql_pool;
    let sql = ctx.general.get_sql("pgsql_api_user_get_all.sql")?;

    let callable =
        |conn| async move { dbpool::pgsql::connection_get(&conn, sql.as_str(), None, None).await };
    let res = db
        .conn_get(callable, timeout)
        .await
        .map_err(actix_web::error::ErrorExpectationFailed)?;
    Ok(res)
}

pub async fn db_get_last_token_id(
    user_id: &str,
    ctx: &web::Data<AppContext>,
    timeout: Duration,
) -> Result<Option<UserLastAuthToken>, actix_web::Error> {
    let db = &ctx.pgsql_pool;
    let sql = ctx.general.get_sql("pgsql_api_user_auth_get_token.sql")?;
    let param_types: &[postgres_types::Type] = &[postgres_types::Type::TEXT];
    let param_values: &[&(dyn postgres_types::ToSql + Sync)] = &[&user_id];

    let callable = |conn| async move {
        dbpool::pgsql::connection_get(&conn, sql.as_str(), Some(param_types), Some(param_values))
            .await
    };
    let rows: Vec<UserLastAuthToken> = db
        .conn_get(callable, timeout)
        .await
        .map_err(actix_web::error::ErrorExpectationFailed)?;
    let res = rows.get(0).map(|v| v.to_owned());
    Ok(res)
}

pub async fn db_persist_last_token_id(
    token_data: &UserLastAuthToken,
    ctx: &web::Data<AppContext>,
    timeout: Duration,
) -> Result<UserLastAuthToken, actix_web::Error> {
    let db = &ctx.pgsql_pool;
    let sql = ctx
        .general
        .get_sql("pgsql_api_user_auth_persist_token.sql")?;
    let param_types: &[postgres_types::Type] = &[
        postgres_types::Type::TEXT,
        postgres_types::Type::UUID,
        postgres_types::Type::TIMESTAMP,
    ];
    let param_values: &[&(dyn postgres_types::ToSql + Sync)] = &[
        &token_data.user_id,
        &token_data.token_id,
        &token_data.mod_timp,
    ];
    let callable = |conn| async move {
        dbpool::pgsql::connection_get(&conn, sql.as_str(), Some(param_types), Some(param_values))
            .await
    };
    let rows: Vec<UserLastAuthToken> = db
        .conn_get(callable, timeout)
        .await
        .map_err(actix_web::error::ErrorExpectationFailed)?;
    let Some(res) = rows.get(0).map(|v| v.to_owned()) else {
        return Err(actix_web::error::ErrorExpectationFailed("could not persist authentication token id"));
    };
    Ok(res)
}

#[cfg(test)]
mod tests {
    #[actix_web::test]
    async fn check_authorisation() {
        let ctx = crate::init_app_data().unwrap();
        let res = super::db_check_authorization(
            "catalin",
            "portal",
            "user_single_get",
            &ctx,
            std::time::Duration::from_secs(10),
        )
        .await
        .unwrap();
        assert_eq!(true, res);
    }

    #[actix_web::test]
    async fn get_transaction_list() {
        let ctx = crate::init_app_data().unwrap();
        let res = super::db_get_allowed_transaction_list(
            "catalin",
            &ctx,
            std::time::Duration::from_secs(10),
        )
        .await
        .unwrap();

        let check = super::UserTransaction {
            user_id: "catalin".into(),
            app_code: "portal".into(),
            method_code: "user_single_get".into(),
        };
        assert!(res.contains(&check))
    }

    #[actix_web::test]
    async fn persist_single() {
        let ctx = crate::init_app_data().unwrap();
        let user = super::User {
            user_id: "catalin".into(),
            first_name: "Catalin".into(),
            last_name: "Any".into(),
            email: "mail@example.com".into(),
            mod_de: None,
            mod_timp: None,
        };

        let res =
            super::db_persist_single(&user, "catalin", &ctx, std::time::Duration::from_secs(10))
                .await
                .unwrap()
                .unwrap();
        assert_eq!(user.first_name, res.first_name);
    }

    #[actix_web::test]
    async fn get_all() {
        let ctx = crate::init_app_data().unwrap();
        let res = super::db_get_all(&ctx, std::time::Duration::from_secs(10))
            .await
            .unwrap();
        assert!(res.len() >= 1)
    }

    #[actix_web::test]
    async fn user_get_single() {
        let ctx = crate::init_app_data().unwrap();
        let res = super::db_get_single("catalin", &ctx, std::time::Duration::from_secs(10))
            .await
            .unwrap()
            .unwrap();
        assert_eq!("catalin", res.user_id)
    }

    #[actix_web::test]
    async fn persist_last_token_id() {
        let ctx = crate::init_app_data().unwrap();

        let iat = chrono::Utc::now()
            .checked_sub_signed(chrono::Duration::minutes(20))
            .unwrap();
        let exp = iat
            .checked_add_signed(chrono::Duration::minutes(10))
            .unwrap();
        let claims = crate::extractors::auth::AuthClaims {
            iss: ctx.general.app_domain.clone(),
            sub: "catalin".into(),
            jti: uuid::Uuid::new_v4(),
            iat: iat.timestamp(),
            exp: exp.timestamp(),
        };
        let res = super::db_persist_last_token_id(
            &claims.into(),
            &ctx,
            std::time::Duration::from_secs(10),
        )
        .await
        .unwrap();
        assert_eq!(iat.date_naive(), res.mod_timp.date());
    }
}
