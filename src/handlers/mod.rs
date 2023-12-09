mod assistant;
mod chats;
mod common;

use askama::Template;
pub use assistant::*;
pub use chats::*;
pub use common::*;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

// 几种事件
#[derive(Debug, Clone, Serialize, Deserialize, Template)]
#[template(path = "signal.html.jinja")]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum AssistantEvent {
    Processing(AssistantStep),
    Finish(AssistantStep),
    Error(String),
    Complete,
}

#[derive(Debug, Clone, Serialize, Deserialize, EnumString, Display)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum AssistantStep {
    UploadAudio,
    Transcription,
    ChatCompletion,
    Speech,
}

impl AssistantEvent {
    pub fn processing(step: AssistantStep) -> String {
        Self::Processing(step).to_string()
    }

    pub fn finish(step: AssistantStep) -> String {
        AssistantEvent::Finish(step).to_string()
    }

    pub fn error(message: impl Into<String>) -> String {
        AssistantEvent::Error(message.into()).to_string()
    }

    pub fn complete() -> String {
        AssistantEvent::Complete.to_string()
    }
}
