use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    web, Error, FromRequest, HttpMessage,
};
use futures::future::LocalBoxFuture;

use crate::extractors::auth::AuthenticateData;

pub struct AuthenticateMiddleware<S> {
    service: std::rc::Rc<S>,
}

impl<S: 'static, B> Service<ServiceRequest> for AuthenticateMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let svc = self.service.clone();

        Box::pin(async move {
            let (req, mut payload) = req.into_parts();
            let auth_data =
                match crate::extractors::auth::AuthenticateData::from_request(&req, &mut payload)
                    .await
                {
                    Ok(v) => v,
                    Err(e) => return Err(e),
                };
            let Some(ctx) = req.app_data() else {
                return Err(actix_web::error::ErrorExpectationFailed("app context missing"));
            };

            //check that is the last token
            let Some(last_token) = crate::model::users::db_get_last_token_id(
                &auth_data.1.sub,
                ctx,
                std::time::Duration::from_secs(10),
            )
            .await? else {
                return Err(actix_web::error::ErrorForbidden("no active token registration"));
            };

            if last_token.token_id.ne(&auth_data.1.jti) {
                return Err(actix_web::error::ErrorForbidden("invalid token"));
            }

            //go further through the call chain
            req.extensions_mut().insert(auth_data);
            let req = ServiceRequest::from_parts(req, payload);

            let res = svc.call(req).await?;
            Ok(res)
        })
    }
}

pub struct AuthenticateFactory;

impl<S: 'static, B> Transform<S, ServiceRequest> for AuthenticateFactory
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = AuthenticateMiddleware<S>;
    type Future = std::future::Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        std::future::ready(Ok(AuthenticateMiddleware {
            service: std::rc::Rc::new(service),
        }))
    }
}

pub struct AuthorizeMiddleware<S> {
    app_code: &'static str,
    method_code: &'static str,
    service: std::rc::Rc<S>,
}

impl<S: 'static, B> Service<ServiceRequest> for AuthorizeMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let svc = self.service.clone();
        let app_code = self.app_code;
        let method_code = self.method_code;

        Box::pin(async move {
            let (req, mut payload) = req.into_parts();
            let user_id =
                crate::extractors::auth::AuthenticateData::from_request(&req, &mut payload)
                    .await
                    .map(|AuthenticateData(_, v)| v.sub)?;

            let Some(ctx) = req.app_data::<web::Data<crate::AppContext>>() else {
                return Err(actix_web::error::ErrorExpectationFailed("app context missing"));
            };

            //check if user is allowed on transaction
            let check = crate::model::users::db_check_authorization(
                &user_id,
                app_code,
                method_code,
                &ctx,
                std::time::Duration::from_secs(10),
            )
            .await?;

            if !check {
                return Err(actix_web::error::ErrorUnauthorized("insufficient rights"));
            }

            //go further through the call chain
            let req = ServiceRequest::from_parts(req, payload);
            let res = svc.call(req).await?;
            Ok(res)
        })
    }
}

pub struct AuthorizeFactory {
    app_code: &'static str,
    method_code: &'static str,
}

impl AuthorizeFactory {
    pub fn new(app_code: &'static str, method_code: &'static str) -> Self {
        Self {
            app_code,
            method_code,
        }
    }
}

impl<S: 'static, B> Transform<S, ServiceRequest> for AuthorizeFactory
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = AuthorizeMiddleware<S>;
    type Future = std::future::Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        std::future::ready(Ok(AuthorizeMiddleware {
            app_code: self.app_code,
            method_code: self.method_code,
            service: std::rc::Rc::new(service),
        }))
    }
}
