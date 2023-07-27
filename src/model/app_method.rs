use actix_web::web;
use dbpool::generics::{ColumnDefault, GenericWrapper};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, time::Duration};

use crate::AppContext;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct AppMethod {
    pub id: Option<uuid::Uuid>,
    pub app_code: String,
    pub method_code: String,
    pub descr: String,
    pub mod_de: Option<String>,
    pub mod_timp: Option<chrono::NaiveDateTime>,
}

impl TryFrom<tokio_postgres::row::Row> for AppMethod {
    type Error = dbpool::error::ErrorReport;

    fn try_from(row: tokio_postgres::Row) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.try_get("id")?,
            app_code: row.try_get("app_code")?,
            method_code: row.try_get("method_code")?,
            descr: row.try_get("descr")?,
            mod_de: row.try_get("mod_de")?,
            mod_timp: row.try_get("mod_timp")?,
        })
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct AppCode {
    pub app_code: String,
}

impl TryFrom<tokio_postgres::Row> for AppCode {
    type Error = dbpool::error::ErrorReport;

    fn try_from(row: tokio_postgres::Row) -> Result<Self, Self::Error> {
        Ok(Self {
            app_code: row.try_get("app_code")?,
        })
    }
}

pub async fn db_get_app_code_list(
    ctx: &web::Data<AppContext>,
    timeout: Duration,
) -> Result<Vec<AppCode>, actix_web::Error> {
    let db = &ctx.pgsql_pool;
    let sql = ctx
        .general
        .get_sql("pgsql_api_app_mthd_get_app_code_list.sql")?;

    let callable =
        |conn| async move { dbpool::pgsql::connection_get(&conn, sql.as_str(), None, None).await };

    let res: Vec<AppCode> = db
        .conn_get(callable, timeout)
        .await
        .map_err(|err| actix_web::error::ErrorExpectationFailed(err))?;
    Ok(res)
}

pub async fn db_get_methods_all(
    ctx: &web::Data<AppContext>,
    timeout: Duration,
) -> Result<Vec<AppMethod>, actix_web::Error> {
    let db = &ctx.pgsql_pool;
    let sql = ctx
        .general
        .get_sql("pgsql_api_app_mthd_get_method_all.sql")?;
    let callable =
        |conn| async move { dbpool::pgsql::connection_get(&conn, sql.as_str(), None, None).await };

    let res: Vec<AppMethod> = db
        .conn_get(callable, timeout)
        .await
        .map_err(|err| actix_web::error::ErrorExpectationFailed(err))?;
    Ok(res)
}

pub async fn db_get_methods_by_app_code(
    app_zone: &AppCode,
    ctx: &web::Data<AppContext>,
    timeout: Duration,
) -> Result<Vec<AppMethod>, actix_web::Error> {
    let db = &ctx.pgsql_pool;
    let sql = ctx
        .general
        .get_sql("pgsql_api_app_mthd_get_method_list_for_app_code.sql")?;
    let param_types = &[postgres_types::Type::TEXT];
    let param_values: &[&(dyn postgres_types::ToSql + Sync)] = &[&app_zone.app_code];

    let callable = |conn| async move {
        dbpool::pgsql::connection_get(&conn, sql.as_str(), Some(param_types), Some(param_values))
            .await
    };

    let res: Vec<AppMethod> = db
        .conn_get(callable, timeout)
        .await
        .map_err(|err| actix_web::error::ErrorExpectationFailed(err))?;
    Ok(res)
}

pub async fn db_methods_all_down_xlsx(
    mod_de: &str,
    ctx: &web::Data<AppContext>,
    timeout: Duration,
) -> Result<std::path::PathBuf, actix_web::Error> {
    let db = &ctx.pgsql_pool;
    let temp_dir = &ctx.general.temp_dir;
    let file_path = temp_dir.join(&format!("d-{}-{}.xlsx", mod_de, uuid::Uuid::new_v4()));
    let file_path_ref = file_path.as_path();
    if file_path_ref.exists() {
        std::fs::remove_file(file_path_ref)?;
    }
    let sql = ctx
        .general
        .get_sql("pgsql_api_app_mthd_get_method_all.sql")?;

    let callable = |conn| async move {
        dbpool::pgsql::download_to_xlsx(
            &conn,
            sql.as_str(),
            None,
            None,
            file_path_ref,
            Some("DATA"),
        )
        .await
    };

    let _ = db
        .conn_run(callable, timeout)
        .await
        .map_err(|err| actix_web::error::ErrorExpectationFailed(err))?;
    Ok(file_path)
}

pub async fn db_methods_all_up_xlsx(
    mod_de: &str,
    file_path: &std::path::Path,
    sheet_name: Option<&str>,
    ctx: &web::Data<AppContext>,
    timeout: Duration,
) -> Result<usize, actix_web::Error> {
    let db = &ctx.pgsql_pool;
    let batch_size = db.get_batch_size();
    if !file_path.exists() {
        return Err(actix_web::error::ErrorExpectationFailed(
            "upload file not found",
        ));
    }

    let upsert_constraint = &["app_code", "method_code"];
    let mut restricted_cols: HashMap<String, ColumnDefault<GenericWrapper>> = HashMap::new();
    restricted_cols.insert("id".into(), ColumnDefault::ByDatabase);
    restricted_cols.insert("mod_de".into(), ColumnDefault::Value(mod_de.into()));
    restricted_cols.insert(
        "mod_timp".into(),
        ColumnDefault::Formula("current_timestamp"),
    );

    let callable = |mut conn| async move {
        dbpool::pgsql::upload_from_xlsx_file(
            &mut conn,
            batch_size,
            "portal",
            "tbl_int_app_transactions",
            Some(upsert_constraint),
            Some(&restricted_cols),
            file_path,
            sheet_name,
        )
        .await
    };

    let res = db
        .conn_run(callable, timeout)
        .await
        .map_err(|err| actix_web::error::ErrorExpectationFailed(err))?;
    Ok(res)
}

pub async fn db_methods_all_down_csv(
    mod_de: &str,
    ctx: &web::Data<AppContext>,
    timeout: Duration,
) -> Result<std::path::PathBuf, actix_web::Error> {
    let db = &ctx.pgsql_pool;
    let temp_dir = &ctx.general.temp_dir;
    let file_path = temp_dir.join(&format!("d-{}-{}.csv", mod_de, uuid::Uuid::new_v4()));
    let file_path_ref = file_path.as_path();
    if file_path_ref.exists() {
        std::fs::remove_file(file_path_ref)?;
    }
    let sql = ctx
        .general
        .get_sql("pgsql_api_app_mthd_get_method_all.sql")?;

    let callable = |conn| async move {
        dbpool::pgsql::download_to_csv(&conn, sql.as_str(), None, None, file_path_ref).await
    };

    let _ = db
        .conn_run(callable, timeout)
        .await
        .map_err(|err| actix_web::error::ErrorExpectationFailed(err))?;
    Ok(file_path)
}

pub async fn db_methods_all_up_txt(
    mod_de: &str,
    file_path: &std::path::Path,
    file_column_delimiter: u8,
    file_column_quote_char: Option<u8>,
    file_quote_char_escape: Option<u8>,
    ctx: &web::Data<AppContext>,
    timeout: Duration,
) -> Result<usize, actix_web::Error> {
    let db = &ctx.pgsql_pool;
    let batch_size = db.get_batch_size();
    if !file_path.exists() {
        return Err(actix_web::error::ErrorExpectationFailed(
            "upload file not found",
        ));
    }

    let upsert_constraint = &["app_code", "method_code"];
    let mut restricted_cols: HashMap<String, ColumnDefault<GenericWrapper>> = HashMap::new();
    restricted_cols.insert("id".into(), ColumnDefault::ByDatabase);
    restricted_cols.insert("mod_de".into(), ColumnDefault::Value(mod_de.into()));
    restricted_cols.insert(
        "mod_timp".into(),
        ColumnDefault::Formula("current_timestamp"),
    );

    let callable = |mut conn| async move {
        dbpool::pgsql::upload_from_text_file(
            &mut conn,
            batch_size,
            "portal",
            "tbl_int_app_transactions",
            Some(upsert_constraint),
            Some(&restricted_cols),
            file_path,
            file_column_delimiter,
            file_column_quote_char,
            file_quote_char_escape,
        )
        .await
    };

    let res = db
        .conn_run(callable, timeout)
        .await
        .map_err(|err| actix_web::error::ErrorExpectationFailed(err))?;
    Ok(res)
}

pub async fn db_methods_by_app_code_down_xlsx(
    app_zone: &AppCode,
    mod_de: &str,
    ctx: &web::Data<AppContext>,
    timeout: Duration,
) -> Result<std::path::PathBuf, actix_web::Error> {
    let db = &ctx.pgsql_pool;
    let temp_dir = &ctx.general.temp_dir;
    let file_path = temp_dir.join(&format!("{}-{}.xlsx", mod_de, uuid::Uuid::new_v4()));
    let file_path_ref = file_path.as_path();
    if file_path_ref.exists() {
        std::fs::remove_file(file_path_ref)?;
    }
    let sql = ctx
        .general
        .get_sql("pgsql_api_app_mthd_get_method_list_for_app_code.sql")?;
    let param_types = &[postgres_types::Type::TEXT];
    let param_values: &[&(dyn postgres_types::ToSql + Sync)] = &[&app_zone.app_code];

    let callable = |conn| async move {
        dbpool::pgsql::download_to_xlsx(
            &conn,
            sql.as_str(),
            Some(param_types),
            Some(param_values),
            file_path_ref,
            Some("DATA"),
        )
        .await
    };

    let _ = db
        .conn_run(callable, timeout)
        .await
        .map_err(|err| actix_web::error::ErrorExpectationFailed(err))?;
    Ok(file_path)
}

pub async fn db_methods_by_app_code_up_xlsx(
    app_zone: &AppCode,
    mod_de: &str,
    file_path: &std::path::Path,
    sheet_name: Option<&str>,
    ctx: &web::Data<AppContext>,
    timeout: Duration,
) -> Result<usize, actix_web::Error> {
    let db = &ctx.pgsql_pool;
    let batch_size = db.get_batch_size();
    if !file_path.exists() {
        return Err(actix_web::error::ErrorExpectationFailed(
            "upload file not found",
        ));
    }

    let upsert_constraint = &["app_code", "method_code"];
    let mut restricted_cols: HashMap<String, ColumnDefault<GenericWrapper>> = HashMap::new();
    restricted_cols.insert("id".into(), ColumnDefault::ByDatabase);
    restricted_cols.insert(
        "app_code".into(),
        ColumnDefault::Value(GenericWrapper::from(app_zone.app_code.as_str())),
    );
    restricted_cols.insert("mod_de".into(), ColumnDefault::Value(mod_de.into()));
    restricted_cols.insert(
        "mod_timp".into(),
        ColumnDefault::Formula("current_timestamp"),
    );

    let callable = |mut conn| async move {
        dbpool::pgsql::upload_from_xlsx_file(
            &mut conn,
            batch_size,
            "portal",
            "tbl_int_app_transactions",
            Some(upsert_constraint),
            Some(&restricted_cols),
            file_path,
            sheet_name,
        )
        .await
    };

    let res = db
        .conn_run(callable, timeout)
        .await
        .map_err(|err| actix_web::error::ErrorExpectationFailed(err))?;
    Ok(res)
}

pub async fn db_methods_by_app_code_down_csv(
    app_zone: &AppCode,
    mod_de: &str,
    ctx: &web::Data<AppContext>,
    timeout: Duration,
) -> Result<std::path::PathBuf, actix_web::Error> {
    let db = &ctx.pgsql_pool;
    let temp_dir = &ctx.general.temp_dir;
    let file_path = temp_dir.join(&format!("{}-{}.csv", mod_de, uuid::Uuid::new_v4()));
    let file_path_ref = file_path.as_path();
    if file_path_ref.exists() {
        std::fs::remove_file(file_path_ref)?;
    }
    let sql = ctx
        .general
        .get_sql("pgsql_api_app_mthd_get_method_all.sql")?;
    let param_types = &[postgres_types::Type::TEXT];
    let param_values: &[&(dyn postgres_types::ToSql + Sync)] = &[&app_zone.app_code];

    let callable = |conn| async move {
        dbpool::pgsql::download_to_csv(
            &conn,
            sql.as_str(),
            Some(param_types),
            Some(param_values),
            file_path_ref,
        )
        .await
    };

    let _ = db
        .conn_run(callable, timeout)
        .await
        .map_err(|err| actix_web::error::ErrorExpectationFailed(err))?;
    Ok(file_path)
}

pub async fn db_methods_by_app_code_up_txt(
    app_zone: &AppCode,
    mod_de: &str,
    file_path: &std::path::Path,
    file_column_delimiter: u8,
    file_column_quote_char: Option<u8>,
    file_quote_char_escape: Option<u8>,
    ctx: &web::Data<AppContext>,
    timeout: Duration,
) -> Result<usize, actix_web::Error> {
    let db = &ctx.pgsql_pool;
    let batch_size = db.get_batch_size();
    if !file_path.exists() {
        return Err(actix_web::error::ErrorExpectationFailed(
            "upload file not found",
        ));
    }

    let upsert_constraint = &["app_code", "method_code"];
    let mut restricted_cols: HashMap<String, ColumnDefault<GenericWrapper>> = HashMap::new();
    restricted_cols.insert("id".into(), ColumnDefault::ByDatabase);
    restricted_cols.insert(
        "app_code".into(),
        ColumnDefault::Value(GenericWrapper::from(app_zone.app_code.as_str())),
    );
    restricted_cols.insert("mod_de".into(), ColumnDefault::Value(mod_de.into()));
    restricted_cols.insert(
        "mod_timp".into(),
        ColumnDefault::Formula("current_timestamp"),
    );

    let callable = |mut conn| async move {
        dbpool::pgsql::upload_from_text_file(
            &mut conn,
            batch_size,
            "portal",
            "tbl_int_app_transactions",
            Some(upsert_constraint),
            Some(&restricted_cols),
            file_path,
            file_column_delimiter,
            file_column_quote_char,
            file_quote_char_escape,
        )
        .await
    };

    let res = db
        .conn_run(callable, timeout)
        .await
        .map_err(|err| actix_web::error::ErrorExpectationFailed(err))?;
    Ok(res)
}

pub async fn db_method_get_by_id(
    id: &uuid::Uuid,
    ctx: &web::Data<AppContext>,
    timeout: Duration,
) -> Result<Option<AppMethod>, actix_web::Error> {
    let db = &ctx.pgsql_pool;
    let sql = ctx
        .general
        .get_sql("pgsql_api_app_mthd_single_get_by_id.sql")?;
    let param_types = &[postgres_types::Type::UUID];
    let param_values: &[&(dyn postgres_types::ToSql + Sync)] = &[&id];

    let callable = |conn| async move {
        dbpool::pgsql::connection_get(&conn, sql.as_str(), Some(param_types), Some(param_values))
            .await
    };

    let res: Vec<AppMethod> = db
        .conn_get(callable, timeout)
        .await
        .map_err(|err| actix_web::error::ErrorExpectationFailed(err))?;
    Ok(res.get(0).map(ToOwned::to_owned))
}

pub async fn db_method_delete_by_id(
    id: &uuid::Uuid,
    ctx: &web::Data<AppContext>,
    timeout: Duration,
) -> Result<usize, actix_web::Error> {
    let db = &ctx.pgsql_pool;
    let sql = ctx
        .general
        .get_sql("pgsql_api_app_mthd_single_delete_by_id.sql")?;
    let param_types = &[postgres_types::Type::UUID];
    let param_values: &[&(dyn postgres_types::ToSql + Sync)] = &[&id];

    let callable = |conn| async move {
        dbpool::pgsql::connection_run(&conn, sql.as_str(), Some(param_types), Some(param_values))
            .await
    };

    let res = db
        .conn_run(callable, timeout)
        .await
        .map_err(|err| actix_web::error::ErrorExpectationFailed(err))?;
    Ok(res)
}

pub async fn db_method_single_upsert(
    method: &AppMethod,
    mod_de: &str,
    ctx: &web::Data<AppContext>,
    timeout: Duration,
) -> Result<Option<AppMethod>, actix_web::Error> {
    let db = &ctx.pgsql_pool;
    let sql = ctx
        .general
        .get_sql("pgsql_api_app_mthd_single_upsert.sql")?;
    let param_types = &[
        postgres_types::Type::TEXT,
        postgres_types::Type::TEXT,
        postgres_types::Type::TEXT,
        postgres_types::Type::TEXT,
    ];
    let param_values: &[&(dyn postgres_types::ToSql + Sync)] = &[
        &method.app_code,
        &method.method_code,
        &method.descr,
        &mod_de,
    ];

    let callable = |conn| async move {
        dbpool::pgsql::connection_get(&conn, sql.as_str(), Some(param_types), Some(param_values))
            .await
    };

    let res: Vec<AppMethod> = db
        .conn_get(callable, timeout)
        .await
        .map_err(|err| actix_web::error::ErrorExpectationFailed(err))?;
    Ok(res.get(0).map(ToOwned::to_owned))
}

#[cfg(test)]
mod tests {
    use super::AppCode;

    #[actix_web::test]
    async fn get_app_code_list() {
        let ctx = crate::init_app_data().unwrap();
        let res = super::db_get_app_code_list(&ctx, std::time::Duration::from_secs(10))
            .await
            .unwrap();
        assert!(res.contains(&super::AppCode {
            app_code: "portal".to_string()
        }));
    }

    #[actix_web::test]
    async fn methods_list_all() {
        let ctx = crate::init_app_data().unwrap();
        let res = super::db_get_methods_all(&ctx, std::time::Duration::from_secs(10))
            .await
            .unwrap();
        assert!(res.len() > 0);
    }

    #[actix_web::test]
    async fn methods_list_by_app_code() {
        let ctx = crate::init_app_data().unwrap();
        let app_zone = AppCode {
            app_code: "portal".into(),
        };
        let res =
            super::db_get_methods_by_app_code(&app_zone, &ctx, std::time::Duration::from_secs(10))
                .await
                .unwrap();
        assert!(res.len() > 0);
    }

    #[actix_web::test]
    async fn methods_all_xlsx() {
        let ctx = crate::init_app_data().unwrap();
        let res =
            super::db_methods_all_down_xlsx("catalin", &ctx, std::time::Duration::from_secs(20))
                .await
                .unwrap();
        assert!(res.exists());

        let file_path = res.as_path();
        let res = super::db_methods_all_up_xlsx(
            "catalin",
            file_path,
            Some("DATA"),
            &ctx,
            std::time::Duration::from_secs(20),
        )
        .await
        .unwrap();
        assert!(res > 0);
        std::fs::remove_file(file_path).unwrap();
    }

    #[actix_web::test]
    async fn methods_all_csv() {
        let ctx = crate::init_app_data().unwrap();
        let res =
            super::db_methods_all_down_csv("catalin", &ctx, std::time::Duration::from_secs(20))
                .await
                .unwrap();
        assert!(res.exists());

        let file_path = res.as_path();
        let res = super::db_methods_all_up_txt(
            "catalin",
            file_path,
            b',',
            Some(b'"'),
            Some(b'"'),
            &ctx,
            std::time::Duration::from_secs(20),
        )
        .await
        .unwrap();
        assert!(res > 0);
        std::fs::remove_file(file_path).unwrap();
    }

    #[actix_web::test]
    async fn methods_by_app_code_xlsx() {
        let ctx = crate::init_app_data().unwrap();
        let app_zone = super::AppCode {
            app_code: "portal".into(),
        };
        let res = super::db_methods_by_app_code_down_xlsx(
            &app_zone,
            "catalin",
            &ctx,
            std::time::Duration::from_secs(20),
        )
        .await
        .unwrap();
        assert!(res.exists());

        let file_path = res.as_path();
        let res = super::db_methods_by_app_code_up_xlsx(
            &app_zone,
            "catalin",
            file_path,
            Some("DATA"),
            &ctx,
            std::time::Duration::from_secs(20),
        )
        .await
        .unwrap();
        assert!(res > 0);
        std::fs::remove_file(file_path).unwrap();
    }

    #[actix_web::test]
    async fn methods_by_app_code_csv() {
        let ctx = crate::init_app_data().unwrap();
        let app_zone = super::AppCode {
            app_code: "portal".into(),
        };
        let res = super::db_methods_by_app_code_down_csv(
            &app_zone,
            "catalin",
            &ctx,
            std::time::Duration::from_secs(20),
        )
        .await
        .unwrap();
        assert!(res.exists());

        let file_path = res.as_path();
        let res = super::db_methods_by_app_code_up_txt(
            &app_zone,
            "catalin",
            file_path,
            b',',
            Some(b'"'),
            Some(b'"'),
            &ctx,
            std::time::Duration::from_secs(20),
        )
        .await
        .unwrap();
        assert!(res > 0);
        std::fs::remove_file(file_path).unwrap();
    }

    #[actix_web::test]
    async fn method_single() {
        let ctx = crate::init_app_data().unwrap();
        let mut method = super::AppMethod {
            id: None,
            app_code: "portal".into(),
            method_code: "testare".into(),
            descr: "testare".into(),
            mod_de: None,
            mod_timp: None,
        };

        method = super::db_method_single_upsert(
            &method,
            "catalin",
            &ctx,
            std::time::Duration::from_secs(10),
        )
        .await
        .unwrap()
        .unwrap();
        assert!(method.id.is_some());

        let res = super::db_method_get_by_id(
            &method.id.unwrap(),
            &ctx,
            std::time::Duration::from_secs(10),
        )
        .await
        .unwrap()
        .unwrap();
        assert_eq!(method.id, res.id);

        let res = super::db_method_delete_by_id(
            &method.id.unwrap(),
            &ctx,
            std::time::Duration::from_secs(10),
        )
        .await
        .unwrap();
        assert!(res > 0);
    }
}
