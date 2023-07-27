use data_encoding::BASE64URL_NOPAD;
use std::{collections::HashMap, path::PathBuf, str::FromStr};

#[derive(Debug)]
pub struct TempFile {
    pub path: PathBuf,
}

impl Drop for TempFile {
    fn drop(&mut self) {
        if self.path.exists() {
            let _ = std::fs::remove_file(self.path.as_path());
        }
    }
}

pub fn get_env(key: &str) -> Result<String, String> {
    dotenv::var(key).map_err(|e| format!("{}: {}", e, key))
}

pub fn get_exist_path(v: &str) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    let p = PathBuf::from_str(v)?;
    if !p.try_exists()? {
        return Err(format!("path doesn't exists: {}", v).into());
    }
    Ok(p)
}

pub fn get_req_query_params(
    req: &actix_web::HttpRequest,
) -> Result<HashMap<String, String>, actix_web::error::Error> {
    let res = serde_urlencoded::from_str::<HashMap<String, String>>(req.query_string())?;
    Ok(res)
}

pub fn from_req_base64_query_params(
    req: &actix_web::HttpRequest,
) -> Result<HashMap<String, String>, actix_web::error::Error> {
    let raw = get_req_query_params(req)?;
    let mut res: HashMap<String, String> = HashMap::new();
    for (k, v) in raw {
        let d = BASE64URL_NOPAD
            .decode(v.as_bytes())
            .map_err(|e| actix_web::error::ErrorBadRequest(e))?;
        let s = String::from_utf8(d).map_err(|e| actix_web::error::ErrorExpectationFailed(e))?;
        res.insert(k, s);
    }
    Ok(res)
}

pub fn base64_decode(v: &[u8]) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let vec = BASE64URL_NOPAD.decode(v)?;
    let res = String::from_utf8(vec)?;
    Ok(res)
}
