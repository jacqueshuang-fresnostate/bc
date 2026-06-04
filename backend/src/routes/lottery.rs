//! 手机端彩票公开接口路由，提供首页彩种分组与开奖摘要。

use axum::{extract::State, routing::get, Json, Router};

use crate::{
    app::AppState, domain::mobile::MobileLotteryHomeResponse, error::ApiResult,
    response::ApiEnvelope, services::mobile_home::build_mobile_lottery_home,
};

/// 组装手机端彩票公开接口路由。
pub fn router() -> Router<AppState> {
    Router::new().route("/home", get(get_lottery_home))
}

/// 返回手机端首页所需的销售中彩种、分类分组和最近开奖号码。
async fn get_lottery_home(
    State(state): State<AppState>,
) -> ApiResult<Json<ApiEnvelope<MobileLotteryHomeResponse>>> {
    let lotteries = state.lotteries.list().await?;
    let categories = state.lotteries.categories().await?;
    let issues = state.draws.list().await?;
    let home = build_mobile_lottery_home(lotteries, categories, issues);

    Ok(Json(ApiEnvelope::success(home)))
}
