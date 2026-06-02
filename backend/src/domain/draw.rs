use serde::{Deserialize, Serialize};

use crate::domain::lottery::{DrawMode, LotteryNumberType};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DrawIssueStatus {
    Open,
    Closed,
    Drawn,
    Cancelled,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateDrawIssueRequest {
    pub lottery_id: String,
    pub issue: String,
    pub scheduled_at: String,
    pub sale_closed_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct DrawIssueResultRequest {
    #[serde(default)]
    pub draw_number: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DrawIssue {
    pub id: String,
    pub lottery_id: String,
    pub lottery_name: String,
    pub issue: String,
    pub number_type: LotteryNumberType,
    pub draw_mode: DrawMode,
    pub scheduled_at: String,
    pub sale_closed_at: String,
    pub status: DrawIssueStatus,
    pub draw_number: Option<String>,
    pub drawn_at: Option<String>,
    pub created_at: String,
}
