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
use tokio::{fs, sync::broadcast};
use tracing::info;
use uuid::Uuid;

use crate::{audio_path, audio_url, error::AppError, extractors::AppContext, AppState};

use super::{AssistantEvent, AssistantStep};

pub async fn assistant_handler(
    context: AppContext,
    State(state): State<Arc<AppState>>,
    multipart: Multipart,
) -> Result<impl IntoResponse, AppError> {
    let device_id = &context.device_id;
    info!("start assist for {}", device_id);

    // 信号，连接状态
    let signal_sender = state
        .signals
        .get(device_id)
        .ok_or_else(|| anyhow!("device_id not found for signal sender"))?
        .clone();

    // 聊天内容
    let chat_sender = state
        .chats
        .get(device_id)
        .ok_or_else(|| anyhow!("device_id not found for chat sender"))?
        .clone();

    let llm = &state.llm;

    let _ = match process(&signal_sender, &chat_sender, llm, device_id, multipart).await {
        Err(err) => {
            signal_sender.send(error(err.to_string()))?;
            return Ok(Json(json!({"status":"error"})));
        }
        _ => {}
    };

    Ok(Json(json!({"status":"done"})))
}

async fn process(
    signal_sender: &broadcast::Sender<String>,
    chat_sender: &broadcast::Sender<String>,
    llm: &LlmSdk,
    device_id: &str,
    mut multipart: Multipart,
) -> Result<()> {
    signal_sender.send(in_audio_upload())?;

    let Some(field) = multipart.next_field().await? else {
        return Err(anyhow!("expected an audio field"))?;
    };
    let data = match field.name() {
        Some(name) if name == "audio" => field.bytes().await?,
        _ => return Err(anyhow!("expected an audio field"))?,
    };

    info!("audio buffer size {}", data.len());

    signal_sender.send(in_transcription())?;
    // 语音转文字
    let input = transcript(llm, data.to_vec()).await?;
    info!("> input {}", &input);
    chat_sender.send(format!("<p>A: {}</p></br>", input).into())?;

    // 内容发送给聊天API
    signal_sender.send(in_chat_completion())?;
    let output = chat_completion(llm, &input).await?;
    info!("> output {}", &output);

    // 回复内容转成语音
    signal_sender.send(in_speech())?;
    let audio_url = speech(llm, device_id, &output).await?;
    info!("audio_url {:?}", audio_url);

    signal_sender.send(complete())?;
    chat_sender.send(
        format!(
            "
                <li><audio controls autoplay>
                    <source src='{}' type='audio/mp3'>
                </audio></li>
                <p>Q: {}</p>
                </br>
            ",
            audio_url, output
        )
        .into(),
    )?;

    Ok(())
}

async fn transcript(llm: &LlmSdk, data: Vec<u8>) -> anyhow::Result<String> {
    let req = WhisperRequest::transcription(data);
    let res = llm.whisper(req).await?;

    Ok(res.text)
}

async fn chat_completion(llm: &LlmSdk, prompt: &str) -> anyhow::Result<String> {
    let messages = vec![
        ChatCompletionMessage::new_system("Hi! I's Q", ""),
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
    AssistantEvent::processing(AssistantStep::UploadAudio)
}

fn in_transcription() -> String {
    AssistantEvent::processing(AssistantStep::Transcription)
}

fn in_chat_completion() -> String {
    AssistantEvent::processing(AssistantStep::ChatCompletion)
}

fn in_speech() -> String {
    AssistantEvent::processing(AssistantStep::Speech)
}

#[allow(dead_code)]
fn finish_upload_audio() -> String {
    AssistantEvent::finish(AssistantStep::UploadAudio)
}

#[allow(dead_code)]
fn finish_transcription() -> String {
    AssistantEvent::finish(AssistantStep::Transcription)
}

#[allow(dead_code)]
fn finish_chat_completion() -> String {
    AssistantEvent::finish(AssistantStep::ChatCompletion)
}

#[allow(dead_code)]
fn finish_speech() -> String {
    AssistantEvent::finish(AssistantStep::Speech)
}

fn complete() -> String {
    AssistantEvent::complete()
}

fn error(msg: impl Into<String>) -> String {
    AssistantEvent::Error(msg.into()).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_render() {
        let event: String = error("error").into();
        assert_eq!(
            event,
            r#"\n    <p class=\"text-red-600\">  Error error </p>\n  "#
        );
    }
}
