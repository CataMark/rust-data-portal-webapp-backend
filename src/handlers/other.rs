use crate::AppContext;
use actix_web::{http::header::ContentType, web, HttpRequest, HttpResponse};

pub async fn rsa_public(ctx: web::Data<AppContext>) -> Result<HttpResponse, actix_web::Error> {
    let key = match ctx.rsa_keys.get_public_key().public_key_to_pem_pkcs1() {
        Ok(k) => String::from_utf8_lossy(&k).into_owned(),
        Err(er) => return Err(actix_web::error::ErrorExpectationFailed(er)),
    };

    Ok(HttpResponse::Ok()
        .content_type(ContentType::plaintext())
        .body(key))
}

pub async fn static_files(req: HttpRequest) -> Result<actix_files::NamedFile, std::io::Error> {
    let Some(ctx) = req.app_data::<web::Data<AppContext>>() else {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "app context not found"));
    };
    let file_name = req.match_info().query("filename");
    actix_files::NamedFile::open_async(ctx.general.static_files_dir.join(file_name)).await
}
