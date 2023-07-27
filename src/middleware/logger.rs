use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, FromRequest,
};
use futures::future::LocalBoxFuture;

pub struct LoggerMiddleware<S> {
    service: std::rc::Rc<S>,
}

impl<S: 'static, B> Service<ServiceRequest> for LoggerMiddleware<S>
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

            let user_agent = match req.headers().get(actix_web::http::header::USER_AGENT) {
                Some(v) => v.to_str().unwrap_or_default(),
                None => "null",
            };

            let log_text = format!("Request: {{ method: {}, version: {:?}, path: {}, user_agent: [{}], real_ip: {}, remote_ip: {} }}", req.method(), req.version(), req.path(), user_agent,
            req.connection_info().realip_remote_addr().unwrap_or_default(), req.connection_info().peer_addr().unwrap_or_default());

            let user_id =
                match crate::extractors::auth::AuthenticateData::from_request(&req, &mut payload)
                    .await
                {
                    Ok(v) => v.1.sub,
                    Err(_) => "null".into(),
                };

            let req = ServiceRequest::from_parts(req, payload);
            let res = svc.call(req).await.map_err(|err| {
                log::error!("{} -> {}", log_text, err);
                err
            })?;
            let status = res.status();
            if status.is_client_error() || status.is_server_error() {
                log::error!(
                    "{} -> {{ user_id: {}, response_status: {} }}",
                    log_text,
                    user_id,
                    status
                );
            } else {
                log::info!(
                    "{} -> {{ user_id: {}, response_status: {} }}",
                    log_text,
                    user_id,
                    status
                );
            };

            Ok(res)
        })
    }
}

pub struct LoggerFactory;

impl<S: 'static, B> Transform<S, ServiceRequest> for LoggerFactory
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = LoggerMiddleware<S>;
    type Future = std::future::Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        std::future::ready(Ok(LoggerMiddleware {
            service: std::rc::Rc::new(service),
        }))
    }
}
