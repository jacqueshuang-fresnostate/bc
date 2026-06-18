//! 广告领域模型，定义后台维护项和手机端轮播公开数据。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 广告投放位置，目前用于区分手机端轮播等展示区域。
pub enum AdvertisementPlacement {
    MobileCarousel,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 广告启停状态，决定手机端是否可以看到该广告。
pub enum AdvertisementStatus {
    Enabled,
    Disabled,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 后台广告维护列表和详情使用的完整广告摘要。
pub struct AdvertisementSummary {
    /// 业务唯一标识。
    pub id: String,
    /// 展示标题。
    pub title: String,
    /// 广告图片地址。
    pub image_url: String,
    /// 点击跳转链接；为空表示不跳转。
    pub link_url: Option<String>,
    /// 广告展示位置。
    pub placement: AdvertisementPlacement,
    /// 业务状态，用于筛选、禁用或流转。
    pub status: AdvertisementStatus,
    /// 展示排序值，数值越小越靠前。
    pub sort_order: i32,
    /// 广告开始展示时间；为空表示立即生效。
    pub start_at: Option<String>,
    /// 广告结束展示时间；为空表示长期有效。
    pub end_at: Option<String>,
    /// 创建时间。
    pub created_at: String,
    /// 最后更新时间。
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 后台新建或编辑广告时提交的表单数据。
pub struct SaveAdvertisementRequest {
    /// 业务唯一标识。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// 展示标题。
    pub title: String,
    /// 广告图片地址。
    pub image_url: String,
    /// 点击跳转链接；为空表示不跳转。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link_url: Option<String>,
    /// 广告展示位置。
    pub placement: AdvertisementPlacement,
    /// 业务状态，用于筛选、禁用或流转。
    pub status: AdvertisementStatus,
    /// 展示排序值，数值越小越靠前。
    pub sort_order: i32,
    /// 广告开始展示时间；为空表示立即生效。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_at: Option<String>,
    /// 广告结束展示时间；为空表示长期有效。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 手机端轮播接口返回的公开广告数据，只保留前台展示需要的字段。
pub struct MobileAdvertisement {
    /// 业务唯一标识。
    pub id: String,
    /// 展示标题。
    pub title: String,
    /// 广告图片地址。
    pub image_url: String,
    /// 点击跳转链接；为空表示不跳转。
    pub link_url: Option<String>,
    /// 展示排序值，数值越小越靠前。
    pub sort_order: i32,
}

/// 广告摘要的展示转换方法。
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
