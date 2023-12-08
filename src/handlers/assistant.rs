use anyhow::{anyhow, Result};
use askama_axum::IntoResponse;
use axum::{extract::Multipart, Json};
use serde_json::json;

use crate::error::AppError;

pub async fn assistant_handler(mut multipart: Multipart) -> Result<impl IntoResponse, AppError> {
    let Some(field) = multipart.next_field().await? else {
        return Err(anyhow!("expected an audio field"))?;
    };

    let data = match field.name() {
        Some(name) if name == "audio" => field.bytes().await?,
        _ => return Err(anyhow!("expected an audio field"))?,
    };

    Ok(Json(json!({"len": data.len()})))
}
