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
use serde_json::{json, Value};
use tokio::fs;
use tracing::info;
use uuid::Uuid;

use crate::{audio_path, audio_url, error::AppError, extractors::AppContext, AppState};

use super::{AssistantEvent, AssistantStep};

pub async fn assistant_handler(
    context: AppContext,
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, AppError> {
    let device_id = &context.device_id;
    println!("【 &context.device_id 】==> {:?}", device_id);
    let tx = state
        .senders
        .get(device_id)
        .ok_or_else(|| anyhow!("device_id not found"))?
        .clone();
    info!("start assist for {}", device_id);

    tx.send(serde_json::Value::String(in_audio_upload()))?;

    let Some(field) = multipart.next_field().await? else {
        return Err(anyhow!("expected an audio field"))?;
    };

    let data = match field.name() {
        Some(name) if name == "audio" => field.bytes().await?,
        _ => return Err(anyhow!("expected an audio field"))?,
    };

    tx.send(serde_json::Value::String(in_transcription()))?;
    let llm = &state.llm;
    let input = transcript(llm, data.to_vec()).await?;

    tx.send(serde_json::Value::String(in_chat_completion()))?;
    let output = chat_completion(llm, &input).await?;

    tx.send(serde_json::Value::String(in_speech()))?;
    let audio_url = speech(llm, &context.device_id, &output).await?;

    tx.send(serde_json::Value::String(complete(output.clone())))?;

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
    let req = SpeechRequest::new(text);
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

fn in_audio_upload() -> String {
    serde_json::to_string(&AssistantEvent::Processing(AssistantStep::UploadAudio)).unwrap()
}

fn in_transcription() -> String {
    serde_json::to_string(&AssistantEvent::Processing(AssistantStep::Transcription)).unwrap()
}

fn in_chat_completion() -> String {
    serde_json::to_string(&AssistantEvent::Processing(AssistantStep::ChatCompletion)).unwrap()
}

fn in_speech() -> String {
    serde_json::to_string(&AssistantEvent::Processing(AssistantStep::Speech)).unwrap()
}

#[allow(dead_code)]
fn finish_upload_audio() -> String {
    serde_json::to_string(&AssistantEvent::Finish(AssistantStep::UploadAudio)).unwrap()
}

#[allow(dead_code)]
fn finish_transcription() -> String {
    serde_json::to_string(&AssistantEvent::Finish(AssistantStep::Transcription)).unwrap()
}

#[allow(dead_code)]
fn finish_chat_completion() -> String {
    serde_json::to_string(&AssistantEvent::Finish(AssistantStep::ChatCompletion)).unwrap()
}

#[allow(dead_code)]
fn finish_speech() -> String {
    serde_json::to_string(&AssistantEvent::Finish(AssistantStep::Speech)).unwrap()
}

fn complete(data: impl Into<String>) -> String {
    serde_json::to_string(&AssistantEvent::complete(data)).unwrap()
}

fn error(msg: impl Into<String>) -> String {
    serde_json::to_string(&AssistantEvent::Error(msg.into())).unwrap()
}
