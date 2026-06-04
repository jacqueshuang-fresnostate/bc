//! HTTP 服务主入口，加载环境变量并启动 Axum 应用

mod app;
mod domain;
mod error;
mod response;
mod routes;
mod services;

use std::{
    collections::BTreeSet,
    error::Error,
    path::{Path, PathBuf},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let loaded_env_files = load_local_env_files()?;

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "bc_backend=debug,tower_http=debug".into()),
        )
        .init();

    for env_file in loaded_env_files {
        tracing::info!(path = %env_file.display(), "已加载本地环境变量文件");
    }

    let port = std::env::var("PORT").unwrap_or_else(|_| "28080".to_string());
    let address = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&address).await?;

    tracing::info!(address = %address, "后台接口服务已开始监听");
    axum::serve(listener, app::router_from_env().await?).await?;

    Ok(())
}

/// 处理 load_local_env_files 的具体内部流程。
fn load_local_env_files() -> Result<Vec<PathBuf>, Box<dyn Error + Send + Sync>> {
    let current_dir = std::env::current_dir()?;
    let original_env_keys = std::env::vars()
        .map(|(key, _)| key)
        .collect::<BTreeSet<_>>();
    let mut loaded_files = Vec::new();

    for path in local_env_file_candidates(&current_dir) {
        if !path.exists() {
            continue;
        }

        let iterator = dotenvy::from_path_iter(&path)?;
        for item in iterator {
            let (key, value) = item?;
            if !original_env_keys.contains(&key) {
                std::env::set_var(key, value);
            }
        }
        loaded_files.push(path);
    }

    Ok(loaded_files)
}

/// 处理 local_env_file_candidates 的具体内部流程。
fn local_env_file_candidates(current_dir: &Path) -> Vec<PathBuf> {
    let project_root = if current_dir
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name == "backend")
    {
        current_dir
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| current_dir.to_path_buf())
    } else {
        current_dir.to_path_buf()
    };

    vec![
        project_root.join(".env"),
        project_root.join(".env.local"),
        project_root.join("backend/.env"),
        project_root.join("backend/.env.local"),
    ]
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::local_env_file_candidates;

    #[test]
    /// 处理 env_file_candidates_include_project_and_backend_files 的具体内部流程。
    fn env_file_candidates_include_project_and_backend_files() {
        let candidates = local_env_file_candidates(Path::new("/workspace/bc"));

        assert_eq!(
            candidates,
            vec![
                Path::new("/workspace/bc/.env").to_path_buf(),
                Path::new("/workspace/bc/.env.local").to_path_buf(),
                Path::new("/workspace/bc/backend/.env").to_path_buf(),
                Path::new("/workspace/bc/backend/.env.local").to_path_buf(),
            ]
        );
    }

    #[test]
    /// 处理 env_file_candidates_work_when_started_from_backend_directory 的具体内部流程。
    fn env_file_candidates_work_when_started_from_backend_directory() {
        let candidates = local_env_file_candidates(Path::new("/workspace/bc/backend"));

        assert_eq!(
            candidates,
            vec![
                Path::new("/workspace/bc/.env").to_path_buf(),
                Path::new("/workspace/bc/.env.local").to_path_buf(),
                Path::new("/workspace/bc/backend/.env").to_path_buf(),
                Path::new("/workspace/bc/backend/.env.local").to_path_buf(),
            ]
        );
    }
}
