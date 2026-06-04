//! 手机端公开配置领域模型，供后续手机端应用读取基础展示信息。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MobileSiteConfig {
    pub platform_name: String,
    pub logo_image_url: Option<String>,
    pub intro: String,
}
