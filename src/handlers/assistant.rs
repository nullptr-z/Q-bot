use super::{AssistantEvent, AssistantStep, SignalEvent, SpeechResult};
use crate::{
    audio_path, audio_url,
    error::AppError,
    extractors::AppContext,
    handlers::{ChatInputEvent, ChatInputSkeletonEvent, ChatReplyEvent, ChatReplySkeletonEvent},
    image_path, image_url,
    tools::{
        tool_completion_request, AnswerCodeArgs, AssistantTool, DrawImageArgs, DrawImageResult,
        WriteCodeArgs, WriteCodeResult,
    },
    AppState,
};
use anyhow::{anyhow, Result};
use askama_axum::IntoResponse;
use axum::{
    extract::{Multipart, State},
    Json,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use comrak::{markdown_to_html_with_plugins, plugins::syntect::SyntectAdapter};
use llm_sdk::{
    ChatCompletionChoice, ChatCompletionMessage, ChatCompletionRequest, CreateImageRequestBuilder,
    ImageResponseFormat, LlmSdk, SpeechRequest, WhisperRequestBuilder, WhisperRequestType,
};
use serde_json::json;
use std::sync::Arc;
use tokio::{fs, sync::broadcast};
use tracing::info;
use uuid::Uuid;

pub async fn assistant_handler(
    context: AppContext,
    State(state): State<Arc<AppState>>,
    multipart: Multipart,
) -> Result<impl IntoResponse, AppError> {
    let device_id = &context.device_id;
    info!("start assist for {}", device_id);

    // chat content sender
    let event_sender = state
        .events
        .get(device_id)
        .ok_or_else(|| anyhow!("device_id not found for chat sender"))?
        .clone();

    let llm = &state.llm;

    let _ = match process(&event_sender, llm, device_id, multipart).await {
        Err(err) => {
            event_sender.send(error(err.to_string()).into())?;
            return Ok(Json(json!({"status":"error"})));
        }
        _ => {}
    };

    Ok(Json(json!({"status":"done"})))
}

async fn process(
    event_sender: &broadcast::Sender<AssistantEvent>,
    llm: &LlmSdk,
    device_id: &str,
    mut multipart: Multipart,
) -> Result<()> {
    let id = Uuid::new_v4().to_string();

    event_sender.send(in_audio_upload())?;

    let Some(field) = multipart.next_field().await? else {
        return Err(anyhow!("expected an audio field"))?;
    };
    let data = match field.name() {
        Some(name) if name == "audio" => field.bytes().await?,
        _ => return Err(anyhow!("expected an audio field"))?,
    };

    info!("audio buffer size {}", data.len());

    // 语音转文字
    event_sender.send(in_transcription())?;
    event_sender.send(ChatInputSkeletonEvent::new(&id).into())?;
    let input = transcript(llm, data.to_vec()).await?;
    info!("> input {}", &input);
    event_sender.send(ChatInputEvent::new(&id, &input).into())?;

    // choice, 选择模型
    event_sender.send(in_thinking())?;
    event_sender.send(ChatReplySkeletonEvent::new(&id).into())?;
    let choice = chat_completion_with_tools(llm, &input).await?;

    match choice.finish_reason {
        llm_sdk::FinishReason::Stop => {
            let output = choice
                .message
                .content
                .ok_or_else(|| anyhow!("expect content but no content available"))?;
            info!("> output {}", &output);

            // 回复内容转成语音
            event_sender.send(in_speech())?;
            let speech_ret = SpeechResult::new_text_only(&output);
            event_sender.send(ChatReplyEvent::new(&id, speech_ret).into())?;

            let speech_ret = speech(llm, device_id, &output).await?;
            event_sender.send(complete())?;
            event_sender.send(ChatReplyEvent::new(&id, speech_ret).into())?;
        }
        llm_sdk::FinishReason::ToolCalls => {
            let tool_call = &choice.message.tool_calls[0].function;
            println!("tool call name {:?}", tool_call.name);
            let tool = tool_call.name.parse().unwrap_or(AssistantTool::Answer);
            match tool {
                AssistantTool::DrawImage => {
                    let args: DrawImageArgs = serde_json::from_str(&tool_call.arguments)?;

                    event_sender.send(in_draw_image())?;
                    let ret = DrawImageResult::new("", &args.prompt);
                    event_sender.send(ChatReplyEvent::new(&id, ret).into())?;

                    let ret = draw_image(llm, device_id, args).await?;
                    event_sender.send(complete())?;
                    event_sender.send(ChatReplyEvent::new(&id, ret).into())?;
                }
                AssistantTool::WriteCode => {
                    event_sender.send(in_write_code())?;
                    let ret = write_code(llm, serde_json::from_str(&tool_call.arguments).unwrap())
                        .await?;

                    event_sender.send(complete())?;
                    event_sender.send(ChatReplyEvent::new(&id, ret).into())?;
                }
                AssistantTool::Answer => {
                    event_sender.send(in_chat_completion())?;
                    let output =
                        answer(llm, serde_json::from_str(&tool_call.arguments).unwrap()).await?;

                    event_sender.send(complete())?;
                    let speech_ret = SpeechResult::new_text_only(&output);
                    event_sender.send(ChatReplyEvent::new(&id, speech_ret).into())?;

                    // 回复内容转成语音
                    event_sender.send(in_speech())?;
                    let ret = speech(llm, device_id, &output).await?;
                    event_sender.send(complete())?;
                    event_sender.send(ChatReplyEvent::new(&id, ret).into())?;
                }
            }
        }
        _ => {}
    }

    Ok(())
}

/// reply question, answer
async fn chat_completion(
    llm: &LlmSdk,
    messages: Vec<ChatCompletionMessage>,
) -> anyhow::Result<String> {
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

/// chat tools prompt
async fn chat_completion_with_tools(
    llm: &LlmSdk,
    prompt: &str,
) -> anyhow::Result<ChatCompletionChoice> {
    let req = tool_completion_request(prompt, "zheng");
    let mut res = llm.chat_completion(req).await?;

    let choice = res
        .choices
        .pop()
        .ok_or_else(|| anyhow!("expect at least one choice"));

    choice
}

/// speech convert to word
async fn transcript(llm: &LlmSdk, audio_buffer: Vec<u8>) -> anyhow::Result<String> {
    let req = WhisperRequestBuilder::default()
        .file(audio_buffer)
        .prompt("If audio language is Chinese, please use simplified chinese")
        .request_type(WhisperRequestType::Transcription)
        .build()?;
    let res = llm.whisper(req).await?;

    Ok(res.text)
}

// word convert to speech
async fn speech(llm: &LlmSdk, device_id: &str, text: &str) -> anyhow::Result<SpeechResult> {
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

    Ok(SpeechResult::new(text, audio_url(device_id, &uuid)))
}

/// coding prompt
async fn answer(llm: &LlmSdk, args: AnswerCodeArgs) -> anyhow::Result<String> {
    let messages = vec![
        ChatCompletionMessage::new_system("I can help answer anything you'r like to chat.", "Q"),
        ChatCompletionMessage::new_user(args.prompt, "zheng"),
    ];

    chat_completion(llm, messages).await
}

/// coding prompt
async fn write_code(llm: &LlmSdk, args: WriteCodeArgs) -> anyhow::Result<WriteCodeResult> {
    let messages = vec![
        ChatCompletionMessage::new_system(
            "I'm an expert on coding, I'll write code for you in markdown format based on your prompt",
            "Q",
        ),
        ChatCompletionMessage::new_user(args.prompt, "zheng"),
    ];

    let md = chat_completion(llm, messages).await?;

    Ok(WriteCodeResult {
        content: md2html(&md),
    })
}

fn md2html(md: &str) -> String {
    let adapter = SyntectAdapter::new(Some("Solarized (dark)"));
    let options = comrak::Options::default();
    let mut plugins = comrak::Plugins::default();

    plugins.render.codefence_syntax_highlighter = Some(&adapter);
    markdown_to_html_with_plugins(md, &options, &plugins)
}

async fn draw_image(
    llm: &LlmSdk,
    device_id: &str,
    args: DrawImageArgs,
) -> anyhow::Result<DrawImageResult> {
    let req = CreateImageRequestBuilder::default()
        .prompt(args.prompt)
        .response_format(ImageResponseFormat::B64Json)
        .build()
        .unwrap();
    let mut res_image = llm.create_image(req).await?;
    let image = res_image
        .data
        .pop()
        .ok_or_else(|| anyhow!("expect at least one data"))?;
    let buffer_image = STANDARD.decode(&image.b64_json.unwrap())?;

    let uuid = Uuid::new_v4().to_string();
    let path = image_path(&device_id, &uuid);
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            fs::create_dir_all(path.parent().unwrap()).await?;
        }
    }
    fs::write(path, buffer_image).await?;

    Ok(DrawImageResult::new(
        image_url(device_id, &uuid),
        image.revised_prompt,
    ))
}

fn in_audio_upload() -> AssistantEvent {
    SignalEvent::Processing(AssistantStep::UploadAudio).into()
}

fn in_transcription() -> AssistantEvent {
    SignalEvent::Processing(AssistantStep::Transcription).into()
}

fn in_thinking() -> AssistantEvent {
    SignalEvent::Processing(AssistantStep::Thinking).into()
}

fn in_chat_completion() -> AssistantEvent {
    SignalEvent::Processing(AssistantStep::ChatCompletion).into()
}

fn in_speech() -> AssistantEvent {
    SignalEvent::Processing(AssistantStep::Speech).into()
}

fn in_draw_image() -> AssistantEvent {
    SignalEvent::Processing(AssistantStep::DraImage).into()
}

fn in_write_code() -> AssistantEvent {
    SignalEvent::Processing(AssistantStep::WriteCode).into()
}

fn complete() -> AssistantEvent {
    SignalEvent::Complete.into()
}

fn error(msg: impl Into<String>) -> AssistantEvent {
    SignalEvent::Error(msg.into()).into()
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
