use askama_axum::IntoResponse;

use axum::{
    extract::State,
    response::sse::{Event, Sse},
};
use std::{convert::Infallible, sync::Arc, time::Duration};
use tokio::sync::broadcast;
use tokio_stream::{wrappers::BroadcastStream, StreamExt as _};

use crate::{extractors::AppContext, AppState};

const MAX_EVENTS: usize = 128;

// SSE 服务器向客户端推消息
pub async fn chats_handler(context: AppContext, state: State<Arc<AppState>>) -> impl IntoResponse {
    let device_id = &context.device_id;

    let rx = if let Some(tx) = state.senders.get(device_id) {
        tx.subscribe()
    } else {
        let (tx, rx) = broadcast::channel(MAX_EVENTS);
        state.senders.insert(device_id.to_string(), tx);

        rx
    };

    let stream = BroadcastStream::new(rx)
        .filter_map(|v| v.ok())
        .map(|v| Event::default().data(serde_json::to_string(&v).unwrap()))
        .map(Ok::<_, Infallible>);

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(1))
            .text("keep-alive-text"),
    )
}
