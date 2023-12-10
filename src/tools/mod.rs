use askama::Template;
use llm_sdk::{ChatCompletionMessage, ChatCompletionRequest, Tool};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use strum::EnumString;

#[derive(Debug, Clone, Serialize, Deserialize, EnumString)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub(crate) enum AssistantTool {
    /// draw a picture based  on user's input
    DrawImage,
    /// write code based on user's input
    WriteCode,
    // reply question
    Answer,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub(crate) struct DrawImageArgs {
    pub(crate) prompt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Template)]
#[template(path = "blocks/image.html.jinja")]
pub(crate) struct DrawImageResult {
    pub(crate) url: String,
    pub(crate) prompt: String,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub(crate) struct WriteCodeArgs {
    pub(crate) prompt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Template)]
#[template(path = "blocks/markdown.html.jinja")]
pub(crate) struct WriteCodeResult {
    pub(crate) content: String,
}

/// chat model
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub(crate) struct AnswerCodeArgs {
    pub(crate) prompt: String,
}

pub(crate) fn tool_completion_request(
    prompt: impl Into<String>,
    name: &str,
) -> ChatCompletionRequest {
    let messages = vec![
        ChatCompletionMessage::new_system("I can do to help you?", "Q"),
        ChatCompletionMessage::new_user(prompt.into(), name),
    ];
    ChatCompletionRequest::new_with_tools(messages, all_tools())
}

pub(crate) fn all_tools() -> Vec<Tool> {
    vec![
        // Tool::new_function::<DrawImageArgs>("draw_image", "Draw an image based on the prompt."),
        Tool::new_function::<WriteCodeArgs>("write_code", "Write code based on the prompt."),
        Tool::new_function::<WriteCodeArgs>("answer", "Just reply based on the prompt."),
    ]
}

impl DrawImageResult {
    pub fn new(url: impl Into<String>, prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            url: url.into(),
        }
    }
}

// impl From<DrawImageResult> for String {
//     fn from(value: DrawImageResult) -> Self {
//         value.render().unwrap()
//     }
// }

// impl From<WriteCodeResult> for String {
//     fn from(value: WriteCodeResult) -> Self {
//         value.render().unwrap()
//     }
// }
