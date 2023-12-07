use askama_axum::IntoResponse;

use axum::response::sse::{Event, Sse};
use futures::stream;
use std::{convert::Infallible, time::Duration};
use tokio_stream::StreamExt as _;
use tracing::info;

// SSE 服务器向客户端推消息
pub async fn chats_handler() -> impl IntoResponse {
    info!("connected");

    // A `Stream` that repeats an event every second
    //
    // You can also create streams from tokio channels using the wrappers in
    // https://docs.rs/tokio-stream
    let stream = stream::repeat_with(|| {
        Event::default().data(r#"<li class="text-red-300">Hi the is Chats!</li>"#)
    })
    .map(Ok::<_, Infallible>)
    .throttle(Duration::from_secs(10));

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(1))
            .text("keep-alive-text"),
    )
}
