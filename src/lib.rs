pub mod error;
mod extractors;
pub mod handlers;
pub mod tools;

use std::path::{Path, PathBuf};

use clap::Parser;
use dashmap::DashMap;
use llm_sdk::LlmSdk;
use tokio::sync::broadcast;

#[derive(Debug)]
pub struct AppState {
    pub llm: LlmSdk,
    pub signals: DashMap<String, broadcast::Sender<String>>,
    pub chats: DashMap<String, broadcast::Sender<String>>,
}

#[derive(Debug, Parser)]
#[clap(name = "qboy")]
pub struct Args {
    #[clap(short, long, default_value = "8080")]
    pub port: u16,
    #[clap(short, long, default_value = ".certs")]
    pub cert_path: String,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            llm: LlmSdk::new(
                "https://api.openai.com/v1",
                std::env::var("OPENAI_API_KEY").unwrap(),
                3,
            ),
            signals: DashMap::new(),
            chats: DashMap::new(),
        }
    }
}

pub fn audio_path(device_id: &str, name: &str) -> PathBuf {
    Path::new("/tmp/qbot/audio")
        .join(device_id)
        .join(format!("{}.mp3", name))
}

pub fn audio_url(device_id: &str, name: &str) -> String {
    format!("/assets/audio/{}/{}.mp3", device_id, name)
}

pub fn image_path(device_id: &str, name: &str) -> PathBuf {
    Path::new("/tmp/qbot/image")
        .join(device_id)
        .join(format!("{}.png", name))
}

pub fn image_url(device_id: &str, name: &str) -> String {
    format!("/assets/image/{}/{}.png", device_id, name)
}
