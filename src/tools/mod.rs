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
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub(crate) struct DrawImageArgs {
    pub(crate) prompt: String,
}

#[derive(Debug, Clone, Deserialize, Template)]
#[template(path = "blocks/image.html.jinja")]
pub(crate) struct DrawImageResponse {
    pub(crate) url: String,
    pub(crate) prompt: String,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub(crate) struct WriteCodeArgs {
    pub(crate) prompt: String,
}

#[derive(Debug, Clone, Deserialize, Template)]
#[template(path = "blocks/markdown.html.jinja")]
pub(crate) struct WriteCodeResult {
    pub(crate) content: String,
}

pub(crate) fn tool_completion_request(
    prompt: impl Into<String>,
    name: &str,
) -> ChatCompletionRequest {
    let messages = vec![
        ChatCompletionMessage::new_system("I can help to identify which tool to use, if no proper tool could be used, I'll directly reply the message with pure text", "Q"),
        ChatCompletionMessage::new_user(prompt.into(), name),
    ];
    ChatCompletionRequest::new_with_tools(messages, all_tools())
}

pub(crate) fn all_tools() -> Vec<Tool> {
    let tools = vec![
        // Tool::new_function::<DrawImageArgs>("draw_image", "Draw an image based on the prompt."),
        Tool::new_function::<WriteCodeArgs>("write_code", "Write code based on the prompt."),
    ];

    tools
}

impl DrawImageResponse {
    pub fn new(url: impl Into<String>, prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            url: url.into(),
        }
    }
}

impl From<DrawImageResponse> for String {
    fn from(value: DrawImageResponse) -> Self {
        value.render().unwrap()
    }
}

impl From<WriteCodeResult> for String {
    fn from(value: WriteCodeResult) -> Self {
        value.render().unwrap()
    }
}
