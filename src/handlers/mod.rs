mod assistant;
mod chats;
mod common;

pub use assistant::*;
pub use chats::*;
pub use common::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use strum::{Display, EnumString};

// 几种事件
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum AssistantEvent {
    Processing(AssistantStep),
    Finish(AssistantStep),
    Error(String),
    Complete(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, EnumString, Display)]
#[serde(rename_all = "snake_case")]
pub enum AssistantStep {
    UploadAudio,
    Transcription,
    ChatCompletion,
    Speech,
}

impl AssistantEvent {
    pub fn processing(step: AssistantStep) -> serde_json::Value {
        serde_json::to_value(Self::Processing(step)).unwrap()
    }

    pub fn finish(step: AssistantStep) -> Value {
        serde_json::to_value(AssistantEvent::Finish(step)).unwrap()
    }

    pub fn error(message: impl Into<String>) -> Value {
        serde_json::to_value(AssistantEvent::Error(message.into())).unwrap()
    }

    pub fn complete(data: impl Into<String>) -> Value {
        serde_json::to_value(AssistantEvent::Complete(data.into())).unwrap()
    }
}
