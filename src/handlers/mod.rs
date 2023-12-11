mod assistant;
mod chats;
mod common;

pub use assistant::*;
pub use chats::*;
pub use common::*;

use crate::tools::{DrawImageResult, WriteCodeResult};
use askama::Template;
use chrono::Local;
use derive_more::From;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

// 几种事件
#[derive(Debug, Clone, From)]
pub enum AssistantEvent {
    Signal(SignalEvent),
    Input(ChatInputEvent),
    InputSkeleton(ChatInputSkeletonEvent),
    Reply(ChatReplyEvent),
    ReplySkeleton(ChatReplySkeletonEvent),
}

#[derive(Debug, Clone, Serialize, Deserialize, Template)]
#[template(path = "event/signal.html.jinja")]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum SignalEvent {
    Processing(AssistantStep),
    Finish(AssistantStep),
    Error(String),
    Complete,
}

#[derive(Debug, Clone, Serialize, Deserialize, Template)]
#[template(path = "event/chat_input.html.jinja")]
pub struct ChatInputEvent {
    id: String,
    content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Template)]
#[template(path = "event/chat_input_skeleton.html.jinja")]
struct ChatInputSkeletonEvent {
    id: String,
    datetime: String,
    avatar: String,
    name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Template)]
#[template(path = "event/chat_reply.html.jinja")]
struct ChatReplyEvent {
    id: String,
    data: ChatReplyData,
}

#[derive(Debug, Clone, Serialize, Deserialize, From)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
enum ChatReplyData {
    Speech(SpeechResult),
    Image(DrawImageResult),
    Markdown(WriteCodeResult),
}

#[derive(Debug, Clone, Serialize, Deserialize, Template)]
#[template(path = "event/chat_reply_skeleton.html.jinja")]
struct ChatReplySkeletonEvent {
    id: String,
    avatar: String,
    name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Template)]
#[template(path = "blocks/speech.html.jinja")]
pub struct SpeechResult {
    text: String,
    url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, EnumString, Display)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum AssistantStep {
    UploadAudio,
    Transcription,
    Thinking,
    ChatCompletion,
    Speech,
    DraImage,
    WriteCode,
}

impl ChatInputEvent {
    fn new(id: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            content: message.into(),
        }
    }
}

impl ChatInputSkeletonEvent {
    fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            datetime: Local::now().format("%d/%m/%Y %H:%M%S").to_string(),
            avatar: "https://i.pravatar.cc/300".to_string(),
            name: "userName".to_string(),
        }
    }
}

impl ChatReplyEvent {
    fn new(id: impl Into<String>, data: impl Into<ChatReplyData>) -> Self {
        Self {
            id: id.into(),
            data: data.into(),
        }
    }
}

impl ChatReplySkeletonEvent {
    fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            avatar: "/public/images/q-bot.png".to_string(),
            name: "Q".to_string(),
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

    fn new_text_only(text: impl Into<String>) -> Self {
        Self::new(text, "".to_string())
    }

    pub fn rende(&self) -> String {
        self.render().unwrap()
    }
}

impl From<AssistantEvent> for String {
    fn from(event: AssistantEvent) -> Self {
        match event {
            AssistantEvent::Signal(v) => v.into(),
            AssistantEvent::InputSkeleton(v) => v.into(),
            AssistantEvent::Input(v) => v.into(),
            AssistantEvent::ReplySkeleton(v) => v.into(),
            AssistantEvent::Reply(v) => v.into(),
        }
    }
}

impl From<SignalEvent> for String {
    fn from(value: SignalEvent) -> Self {
        value.render().unwrap()
    }
}

impl From<ChatInputEvent> for String {
    fn from(value: ChatInputEvent) -> Self {
        value.render().unwrap()
    }
}

impl From<ChatInputSkeletonEvent> for String {
    fn from(value: ChatInputSkeletonEvent) -> Self {
        value.render().unwrap()
    }
}

impl From<ChatReplyEvent> for String {
    fn from(value: ChatReplyEvent) -> Self {
        value.render().unwrap()
    }
}

impl From<ChatReplySkeletonEvent> for String {
    fn from(value: ChatReplySkeletonEvent) -> Self {
        value.render().unwrap()
    }
}
