//! 广告领域模型，定义后台维护项和手机端轮播公开数据。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AdvertisementPlacement {
    MobileCarousel,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AdvertisementStatus {
    Enabled,
    Disabled,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AdvertisementSummary {
    pub id: String,
    pub title: String,
    pub image_url: String,
    pub link_url: Option<String>,
    pub placement: AdvertisementPlacement,
    pub status: AdvertisementStatus,
    pub sort_order: i32,
    pub start_at: Option<String>,
    pub end_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SaveAdvertisementRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub title: String,
    pub image_url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link_url: Option<String>,
    pub placement: AdvertisementPlacement,
    pub status: AdvertisementStatus,
    pub sort_order: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MobileAdvertisement {
    pub id: String,
    pub title: String,
    pub image_url: String,
    pub link_url: Option<String>,
    pub sort_order: i32,
}

impl AdvertisementSummary {
    /// 转换为手机端可公开读取的轮播广告数据。
    pub fn public_mobile(&self) -> MobileAdvertisement {
        MobileAdvertisement {
            id: self.id.clone(),
            title: self.title.clone(),
            image_url: self.image_url.clone(),
            link_url: self.link_url.clone(),
            sort_order: self.sort_order,
        }
    }
}
