use askama_axum::IntoResponse;

use axum::{
    extract::State,
    response::sse::{Event, Sse},
};
use dashmap::DashMap;
use std::{convert::Infallible, sync::Arc, time::Duration};
use tokio::sync::broadcast;
use tokio_stream::{wrappers::BroadcastStream, StreamExt as _};
use tracing::info;

use crate::{extractors::AppContext, AppState};

const MAX_EVENTS: usize = 128;

pub async fn chats_handler(context: AppContext, state: State<Arc<AppState>>) -> impl IntoResponse {
    sse_handler(context, &state.chats).await
}

pub async fn signal_handler(context: AppContext, state: State<Arc<AppState>>) -> impl IntoResponse {
    sse_handler(context, &state.signals).await
}

// SSE 服务器向客户端推消息
async fn sse_handler(
    context: AppContext,
    map: &DashMap<String, broadcast::Sender<String>>,
) -> impl IntoResponse {
    let device_id = &context.device_id;
    info!("user {} connect", device_id);

    let device_id = &context.device_id;
    let rx = if let Some(tx) = map.get(device_id) {
        tx.subscribe()
    } else {
        let (tx, rx) = broadcast::channel(MAX_EVENTS);
        map.insert(device_id.to_string(), tx);

        rx
    };

    let stream = BroadcastStream::new(rx)
        .filter_map(|v| v.ok())
        .map(|v| Event::default().data(v))
        .map(Ok::<_, Infallible>);

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(1))
            .text("keep-alive-text"),
    )
}
