use super::{AssistantEvent, AssistantStep, SpeechResult};
use crate::{
    audio_path, audio_url,
    error::AppError,
    extractors::AppContext,
    handlers::ChatInputEvent,
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
use comrak::markdown_to_html;
use llm_sdk::{
    ChatCompletionChoice, ChatCompletionMessage, ChatCompletionRequest, CreateImageRequest,
    CreateImageRequestBuilder, ImageResponseFormat, LlmSdk, SpeechRequest, WhisperRequest,
    WhisperRequestBuilder, WhisperRequestType,
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

    // state of process sender
    let signal_sender = state
        .signals
        .get(device_id)
        .ok_or_else(|| anyhow!("device_id not found for signal sender"))?
        .clone();

    // chat content sender
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

    // 语音转文字
    signal_sender.send(in_transcription())?;
    let input = transcript(llm, data.to_vec()).await?;
    chat_sender.send(ChatInputEvent::new(&input).into())?;
    info!("> input {}", &input);

    // 内容发送给聊天API
    signal_sender.send(in_chat_completion())?;
    let choice = chat_completion_with_tools(llm, &input).await?;

    match choice.finish_reason {
        llm_sdk::FinishReason::Stop => {
            let output = choice
                .message
                .content
                .ok_or_else(|| anyhow!("expect content but no content available"))?;
            info!("> output {}", &output);

            // 回复内容转成语音
            signal_sender.send(in_speech())?;
            let speech = speech(llm, device_id, &output).await?;

            signal_sender.send(complete())?;
            chat_sender.send(speech.into())?;
        }
        llm_sdk::FinishReason::ToolCalls => {
            let tool_call = &choice.message.tool_calls[0].function;
            println!("tool call name {:?}", tool_call.name);
            let tool = tool_call.name.parse().unwrap_or(AssistantTool::Answer);
            match tool {
                AssistantTool::DrawImage => {
                    signal_sender.send(in_draw_image())?;
                    let ret = draw_image(
                        llm,
                        device_id,
                        serde_json::from_str(&tool_call.arguments).unwrap(),
                    )
                    .await?;

                    signal_sender.send(complete())?;
                    chat_sender.send(ret.into())?;
                }
                AssistantTool::WriteCode => {
                    signal_sender.send(in_write_code())?;
                    let ret = write_code(llm, serde_json::from_str(&tool_call.arguments).unwrap())
                        .await?;

                    signal_sender.send(complete())?;
                    chat_sender.send(ret.into())?;
                }
                AssistantTool::Answer => {
                    signal_sender.send(in_chat_completion())?;
                    let output =
                        answer(llm, serde_json::from_str(&tool_call.arguments).unwrap()).await?;

                    // 回复内容转成语音
                    signal_sender.send(in_speech())?;
                    let speech = speech(llm, device_id, &output).await?;

                    signal_sender.send(complete())?;
                    chat_sender.send(speech.into())?;
                }
            }
        }
        _ => {}
    }

    // chat_sender.send(
    //     format!(
    //         "
    //             <li><audio controls autoplay>
    //                 <source src='{}' type='audio/mp3'>
    //             </audio></li>
    //             <p>Q: {}</p>
    //             </br>
    //         ",
    //         audio_url.url, output
    //     )
    //     .into(),
    // )?;

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
        .prompt("If audio language is Chinese, please use Simplified Chinese")
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

    let output = chat_completion(llm, messages).await?;
    let md = markdown_to_html(&output, &comrak::ComrakOptions::default());

    Ok(WriteCodeResult { content: md })
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

fn in_audio_upload() -> String {
    AssistantEvent::Processing(AssistantStep::UploadAudio).to_string()
}

fn in_transcription() -> String {
    AssistantEvent::Processing(AssistantStep::Transcription).to_string()
}

fn in_chat_completion() -> String {
    AssistantEvent::Processing(AssistantStep::ChatCompletion).to_string()
}

fn in_speech() -> String {
    AssistantEvent::Processing(AssistantStep::Speech).to_string()
}

fn in_draw_image() -> String {
    AssistantEvent::Processing(AssistantStep::DraImage).to_string()
}

fn in_write_code() -> String {
    AssistantEvent::Processing(AssistantStep::WriteCode).to_string()
}

fn complete() -> String {
    AssistantEvent::Complete.to_string()
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
