use base64::{engine::general_purpose, Engine as _};
use serde::Serialize;
use std::time::Duration;

const MAX_AVATAR_IMAGE_BYTES: usize = 1024 * 1024;

#[derive(Serialize)]
struct SystemInfo {
    os: String,
    arch: String,
    version: String,
}

#[tauri::command]
fn get_system_info() -> SystemInfo {
    SystemInfo {
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    }
}

#[tauri::command]
async fn cache_avatar_image(url: String) -> Result<String, String> {
    let url = url.trim().to_string();
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err("头像链接必须是 http 或 https".to_string());
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|_| "头像下载客户端初始化失败".to_string())?;
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|_| "头像图片下载失败".to_string())?;
    if !response.status().is_success() {
        return Err("头像图片下载失败".to_string());
    }

    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("image/png")
        .to_string();
    if !content_type.starts_with("image/") {
        return Err("头像链接返回的不是图片".to_string());
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|_| "头像图片读取失败".to_string())?;
    if bytes.len() > MAX_AVATAR_IMAGE_BYTES {
        return Err("头像图片超过缓存大小限制".to_string());
    }

    let encoded = general_purpose::STANDARD.encode(bytes);
    Ok(format!("data:{};base64,{}", content_type, encoded))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .invoke_handler(tauri::generate_handler![
            get_system_info,
            cache_avatar_image
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
