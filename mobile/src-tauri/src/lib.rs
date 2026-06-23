use base64::{engine::general_purpose, Engine as _};
use serde::Serialize;
use std::fmt::Display;
use std::time::Duration;

const MAX_AVATAR_IMAGE_BYTES: usize = 1024 * 1024;
const ANDROID_LOG_TAG: &str = "HongFuMobile";

// Android release 包启动失败时，Rust panic 默认只留下 SIGABRT 栈；
// 这里直接写入 logcat，方便真机安装后定位 Tauri 初始化阶段错误。
#[cfg(target_os = "android")]
mod native_log {
    use std::ffi::CString;
    use std::os::raw::{c_char, c_int};

    const ANDROID_LOG_ERROR: c_int = 6;

    #[link(name = "log")]
    extern "C" {
        fn __android_log_print(
            priority: c_int,
            tag: *const c_char,
            format: *const c_char,
            ...
        ) -> c_int;
    }

    pub fn error(tag: &str, message: &str) {
        let tag = CString::new(tag.replace('\0', " "))
            .unwrap_or_else(|_| CString::new("HongFuMobile").unwrap());
        let message = CString::new(message.replace('\0', " "))
            .unwrap_or_else(|_| CString::new("日志内容包含非法字符").unwrap());
        let format = CString::new("%s").unwrap();

        unsafe {
            __android_log_print(
                ANDROID_LOG_ERROR,
                tag.as_ptr(),
                format.as_ptr(),
                message.as_ptr(),
            );
        }
    }
}

#[cfg(not(target_os = "android"))]
mod native_log {
    pub fn error(tag: &str, message: &str) {
        eprintln!("[{tag}] {message}");
    }
}

fn log_native_error(message: impl Display) {
    native_log::error(ANDROID_LOG_TAG, &message.to_string());
}

// 在 Tauri 初始化前安装 panic hook，保证插件配置、IPC 权限等早期异常也能输出中文日志。
fn install_native_panic_hook() {
    std::panic::set_hook(Box::new(|panic_info| {
        log_native_error(format!("手机端原生层发生 panic：{panic_info}"));
    }));
}

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
    install_native_panic_hook();

    // 不直接 unwrap Tauri 运行结果，先把初始化错误写进 logcat，再让应用按原生异常退出。
    let result = tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(|_app| {
            #[cfg(mobile)]
            {
                _app.handle().plugin(tauri_plugin_geolocation::init())?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_system_info,
            cache_avatar_image
        ])
        .run(tauri::generate_context!());

    if let Err(error) = result {
        let message = format!("手机端 Tauri 应用运行失败：{error:?}");
        log_native_error(&message);
        panic!("{message}");
    }
}
