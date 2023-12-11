use crate::{extractors::AppContext, AppState};
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

use super::AssistantEvent;

const MAX_EVENTS: usize = 128;

pub async fn events_handler(context: AppContext, state: State<Arc<AppState>>) -> impl IntoResponse {
    sse_handler(context, &state.events).await
}

// SSE 服务器向客户端推消息
async fn sse_handler(
    context: AppContext,
    map: &DashMap<String, broadcast::Sender<AssistantEvent>>,
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
        .map(|v| {
            let (event, id) = match &v {
                AssistantEvent::Signal(_) => ("signal", "".to_string()),
                AssistantEvent::Input(_) => ("input", "".to_string()),
                AssistantEvent::InputSkeleton(v) => ("input_skeleton", v.id.to_string()),
                AssistantEvent::Reply(v) => ("reply", v.id.to_string()),
                AssistantEvent::ReplySkeleton(v) => ("reply_skeleton", v.id.to_string()),
            };

            let data: String = v.into();
            Event::default().data(data).event(event).id(id)
        })
        .map(Ok::<_, Infallible>);

    let keep_alive = axum::response::sse::KeepAlive::new()
        .interval(Duration::from_secs(1))
        .text("keep-alive-text");
    Sse::new(stream).keep_alive(keep_alive)
}
