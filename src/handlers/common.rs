use askama::Template;
use axum::response::IntoResponse;

#[derive(Debug, Template)]
#[template(path = "index.html.jinja")]
struct IndexTemplate {}

pub async fn index_page() -> impl IntoResponse {
    IndexTemplate {}
}
