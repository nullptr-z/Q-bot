use askama::Template;
use axum::response::IntoResponse;
use axum_extra::extract::{cookie::Cookie, CookieJar};
use uuid::Uuid;

#[derive(Debug, Template)]
#[template(path = "index.html.jinja")]
struct IndexTemplate {}

pub async fn index_page(jar: CookieJar) -> impl IntoResponse {
    let jar = match jar.get("device_id") {
        Some(_) => jar,
        None => {
            let device_id = Uuid::new_v4().to_string();
            let cookie = Cookie::build("device_id", device_id)
                .path("/")
                .secure(true)
                .permanent()
                .finish();
            jar.add(cookie)
        }
    };

    (jar, IndexTemplate {})
}
