use actix_web::{web, HttpMessage};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct AuthClaims {
    pub iss: String,     // issuer
    pub sub: String,     // subject - the user
    pub jti: uuid::Uuid, // unique identifier
    pub iat: i64,        // issued time
    pub exp: i64,        // expiry time
}

impl AuthClaims {
    pub const JWT_ALGORITH: jwt::AlgorithmID = jwt::AlgorithmID::RS512;

    pub fn new(
        iss: String,
        sub: String,
        jti: uuid::Uuid,
        iat: chrono::DateTime<chrono::Utc>,
        exp: chrono::DateTime<chrono::Utc>,
    ) -> Self {
        Self {
            iss,
            sub,
            jti,
            iat: iat.timestamp(),
            exp: exp.timestamp(),
        }
    }

    pub fn create_token(
        &self,
        ctx: &web::Data<crate::AppContext>,
    ) -> Result<String, actix_web::Error> {
        let alg = jwt::Algorithm::new_rsa_pem_signer(
            Self::JWT_ALGORITH,
            ctx.rsa_keys
                .get_private_key()
                .private_key_to_pem()
                .unwrap()
                .as_ref(),
        )
        .map_err(actix_web::error::ErrorExpectationFailed)?;
        let header = serde_json::json!({"alg": alg.name()});
        let claims = serde_json::json!(self);
        let token = jwt::encode(&header, &claims, &alg)
            .map_err(actix_web::error::ErrorExpectationFailed)?;
        Ok(token)
    }

    pub fn decode_token(
        token: &str,
        ctx: &web::Data<crate::AppContext>,
    ) -> Result<Self, actix_web::Error> {
        let alg = jwt::Algorithm::new_rsa_pem_verifier(
            Self::JWT_ALGORITH,
            ctx.rsa_keys
                .get_public_key()
                .public_key_to_pem()
                .unwrap()
                .as_ref(),
        )
        .map_err(actix_web::error::ErrorExpectationFailed)?;
        let verifier = jwt::Verifier::create()
            .leeway(5)
            .build()
            .map_err(actix_web::error::ErrorExpectationFailed)?;
        let raw_claims = verifier
            .verify(token, &alg)
            .map_err(actix_web::error::ErrorUnauthorized)?;
        let claims: AuthClaims = serde_json::from_value(raw_claims)?;

        //return
        Ok(claims)
    }
}

impl Into<crate::model::users::UserLastAuthToken> for AuthClaims {
    fn into(self) -> crate::model::users::UserLastAuthToken {
        crate::model::users::UserLastAuthToken {
            id: None,
            user_id: self.sub,
            token_id: self.jti,
            mod_timp: chrono::NaiveDateTime::from_timestamp_opt(self.iat, 0)
                .unwrap_or(chrono::Utc::now().naive_utc()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AuthenticateData(pub String, pub AuthClaims);

impl actix_web::FromRequest for AuthenticateData {
    type Error = actix_web::error::Error;
    type Future = std::future::Ready<Result<Self, Self::Error>>;

    fn from_request(req: &actix_web::HttpRequest, _: &mut actix_web::dev::Payload) -> Self::Future {
        //check if athentication data was added to extensions
        if let Some(data) = req.extensions().get::<Self>() {
            return std::future::ready(Ok(data.clone()));
        }

        //get authentication data from other sources and verifiy
        let token = if let Some(t) = req
            .cookie(crate::Consts::AUTH_COOKIE_NAME)
            .map(|v| v.value().to_owned())
        {
            t
        } else if let Some(Ok(t)) = req
            .headers()
            .get(crate::Consts::AUTH_HEADER_NAME)
            .map(|v| v.to_str().map(ToOwned::to_owned))
        {
            t
        } else if let Ok(Some(t)) = crate::helper::get_req_query_params(req).map(|v| {
            v.get(crate::Consts::AUTH_COOKIE_NAME)
                .map(ToOwned::to_owned)
        }) {
            t
        } else {
            return std::future::ready(Err(actix_web::error::ErrorUnauthorized(
                "missing authentication token",
            )));
        };

        let Some(ctx) = req.app_data::<web::Data<crate::AppContext>>() else {
            return std::future::ready(Err(actix_web::error::ErrorInternalServerError(
                "no app context",
            )));
        };

        let claims = match AuthClaims::decode_token(&token, &ctx) {
            Ok(v) => v,
            Err(e) => return std::future::ready(Err(e)),
        };

        std::future::ready(Ok(Self(token, claims)))
    }
}

impl From<AuthenticateData> for AuthClaims {
    fn from(v: AuthenticateData) -> Self {
        let AuthenticateData(_, claims) = v;
        claims
    }
}

#[cfg(test)]
mod tests {
    use super::AuthClaims;

    #[test]
    fn jwt() {
        let ctx = crate::init_app_data().unwrap();

        let iat = chrono::Utc::now();
        let claims = AuthClaims {
            iss: "http://localhost".into(),
            sub: "C12153".into(),
            jti: uuid::Uuid::new_v4(),
            iat: iat.timestamp(),
            exp: iat.timestamp(),
        };

        let token = claims.create_token(&ctx).unwrap();
        let result = AuthClaims::decode_token(&token, &ctx).unwrap();
        assert_eq!(claims, result);

        let exp = iat
            .checked_sub_signed(chrono::Duration::days(3))
            .unwrap_or(iat);
        let claims = AuthClaims {
            iss: "http://localhost".into(),
            sub: "C12153".into(),
            jti: uuid::Uuid::new_v4(),
            iat: exp.timestamp(),
            exp: exp.timestamp(),
        };

        let token = claims.create_token(&ctx).unwrap();
        let result = AuthClaims::decode_token(&token, &ctx);
        assert!(result.is_err());
    }
}
