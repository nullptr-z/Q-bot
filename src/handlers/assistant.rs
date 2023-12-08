use std::sync::Arc;

use anyhow::{anyhow, Result};
use askama_axum::IntoResponse;
use axum::{
    extract::{Multipart, State},
    Json,
};
use llm_sdk::{
    ChatCompletionMessage, ChatCompletionRequest, LlmSdk, SpeechRequest, WhisperRequest,
};
use serde_json::json;
use tokio::fs;
use uuid::Uuid;

use crate::{audio_path, audio_url, error::AppError, extractors::AppContext, AppState};

pub async fn assistant_handler(
    context: AppContext,
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, AppError> {
    let Some(field) = multipart.next_field().await? else {
        return Err(anyhow!("expected an audio field"))?;
    };

    let data = match field.name() {
        Some(name) if name == "audio" => field.bytes().await?,
        _ => return Err(anyhow!("expected an audio field"))?,
    };

    let llm = &state.llm;
    let input = transcript(llm, data.to_vec()).await?;
    let output = chat_completion(llm, &input).await?;
    let audio_url = speech(llm, &context.device_id, &output).await?;

    Ok(Json(
        json!({"len": data.len(),"request":input,"response":output,"audio_url":audio_url}),
    ))
}

async fn transcript(llm: &LlmSdk, data: Vec<u8>) -> anyhow::Result<String> {
    let req = WhisperRequest::transcription(data);
    let res = llm.whisper(req).await?;

    Ok(res.text)
}

async fn chat_completion(llm: &LlmSdk, prompt: &str) -> anyhow::Result<String> {
    let messages = vec![
        ChatCompletionMessage::new_system("我是助手Q,有什么事情尽管问我", ""),
        ChatCompletionMessage::new_user(prompt, "zheng"),
    ];

    let req = ChatCompletionRequest::new(messages);
    let mut res = llm.chat_completion(req).await?;
    let text = res
        .choices
        .pop()
        .ok_or_else(|| anyhow!("expect at least one choice"))?
        .message
        .content
        .ok_or_else(|| anyhow!("expect content but no content available"))?;

    Ok(text)
}

async fn speech(llm: &LlmSdk, device_id: &str, text: &str) -> anyhow::Result<String> {
    let mut req = SpeechRequest::new(text);
    let audio_stream = llm.speech(req).await?;
    let uuid = Uuid::new_v4().to_string();
    let path = audio_path(&device_id, &uuid);
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            fs::create_dir_all(path.parent().unwrap()).await?;
        }
    }
    fs::write(path, audio_stream).await?;

    Ok(audio_url(device_id, &uuid))
}
