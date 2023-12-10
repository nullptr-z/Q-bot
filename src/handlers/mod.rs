mod assistant;
mod chats;
mod common;
use askama::Template;
pub use assistant::*;
pub use chats::*;
use chrono::Local;
pub use common::*;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

use crate::tools::{DrawImageResult, WriteCodeResult};

// 几种事件
#[derive(Debug, Clone, Serialize, Deserialize, Template)]
#[template(path = "event/signal.html.jinja")]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum AssistantEvent {
    Processing(AssistantStep),
    Finish(AssistantStep),
    Error(String),
    Complete,
}

#[derive(Debug, Clone, Serialize, Deserialize, Template)]
#[template(path = "event/chat_reply.html.jinja")]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
enum ChatReplyEvent {
    Speech(SpeechResult),
    Image(DrawImageResult),
    Markdown(WriteCodeResult),
}

#[derive(Debug, Clone, Serialize, Deserialize, Template)]
#[template(path = "event/chat_input.html.jinja")]
struct ChatInputEvent {
    message: String,
    datetime: String,
    avatar: String,
    name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, EnumString, Display)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum AssistantStep {
    UploadAudio,
    Transcription,
    ChatCompletion,
    Speech,
    DraImage,
    WriteCode,
}

#[derive(Debug, Clone, Serialize, Deserialize, Template)]
#[template(path = "blocks/speech.html.jinja")]
pub struct SpeechResult {
    text: String,
    url: String,
}

impl ChatInputEvent {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            datetime: Local::now().format("%d/%m/%Y %H:%M:%S").to_string(),
            avatar: "https://i.pravatar.cc/128".into(),
            name: "user".into(),
        }
    }
}

impl SpeechResult {
    fn new(text: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            url: url.into(),
        }
    }
}

impl From<AssistantEvent> for String {
    fn from(value: AssistantEvent) -> Self {
        value.render().unwrap()
    }
}

impl From<SpeechResult> for String {
    fn from(value: SpeechResult) -> Self {
        ChatReplyEvent::from(value).render().unwrap()
    }
}

impl From<DrawImageResult> for String {
    fn from(value: DrawImageResult) -> Self {
        ChatReplyEvent::from(value).render().unwrap()
    }
}

impl From<WriteCodeResult> for String {
    fn from(value: WriteCodeResult) -> Self {
        ChatReplyEvent::from(value).render().unwrap()
    }
}

impl From<ChatInputEvent> for String {
    fn from(value: ChatInputEvent) -> Self {
        value.render().unwrap()
    }
}

impl From<SpeechResult> for ChatReplyEvent {
    fn from(value: SpeechResult) -> Self {
        Self::Speech(value)
    }
}

impl From<DrawImageResult> for ChatReplyEvent {
    fn from(value: DrawImageResult) -> Self {
        Self::Image(value)
    }
}

impl From<WriteCodeResult> for ChatReplyEvent {
    fn from(value: WriteCodeResult) -> Self {
        Self::Markdown(value)
    }
}
