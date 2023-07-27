#[actix_web::test]
async fn test_login_request() {
    let payload = cdg_portal::model::users::LoginData {
        user_id: "catalin".into(),
    };

    let app_data = cdg_portal::init_app_data().unwrap();
    let app = actix_web::test::init_service(actix_web::App::new().app_data(app_data).route(
        "/",
        actix_web::web::post().to(cdg_portal::handlers::auth::user_login),
    ))
    .await;
    let req = actix_web::test::TestRequest::post()
        .set_json(payload)
        .to_request();
    let resp = actix_web::test::call_service(&app, req).await;
    let status = resp.status();
    let bytes = actix_web::body::to_bytes(resp.into_body())
        .await
        .unwrap()
        .to_vec();
    let body = String::from_utf8(bytes).unwrap();
    assert!(status.is_success(), "{}", body);
}
