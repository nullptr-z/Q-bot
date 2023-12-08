use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
};
use axum_extra::extract::CookieJar;

#[derive(Debug, Clone)]
pub struct AppContext {
    pub device_id: String,
}

#[async_trait]
impl<S> FromRequestParts<S> for AppContext
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let jar = CookieJar::from_request_parts(parts, state).await.unwrap();

        if let Some(device_id) = jar.get("device_id") {
            Ok(AppContext {
                device_id: device_id.value().to_string(),
            })
        } else {
            Err((StatusCode::BAD_REQUEST, "Cookie `device_id` is missing"))
        }
    }
}
