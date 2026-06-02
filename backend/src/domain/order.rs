use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum OrderStatus {
    PendingDraw,
    Won,
    Lost,
    Cancelled,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderSummary {
    pub id: String,
    pub user_id: String,
    pub lottery_id: String,
    pub issue: String,
    pub amount_minor: i64,
    pub status: OrderStatus,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupBuyPlanSummary {
    pub id: String,
    pub lottery_id: String,
    pub initiator_user_id: String,
    pub total_amount_minor: i64,
    pub filled_amount_minor: i64,
    pub share_count: u32,
    pub status: String,
}
