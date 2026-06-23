//! 充值服务，管理彩虹易支付与客服直充订单

use std::{
    collections::{BTreeMap, HashMap},
    sync::{Arc, RwLock},
};

use chrono::Local;
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgRow, PgConnection, Row};
use urlencoding::encode;

use crate::{
    domain::{
        finance::LedgerEntry,
        permission::SystemSetting,
        recharge::{
            ConfirmRechargeOrderRequest, CreateRechargeOrderRequest, CreateRechargeOrderResponse,
            RechargeBonusRule, RechargeChannel, RechargeChannelConfig, RechargeConfigResponse,
            RechargeOrderStatus, RechargeOrderSummary,
        },
        user::UserSummary,
    },
    error::{ApiError, ApiResult},
    services::{
        business_database::BusinessDatabase,
        finance::{
            save_finance_store_incremental_in_transaction, FinanceRepository, LedgerEntryIdRemap,
        },
        pagination::{ListPage, PageRequest},
        rebate::RechargeRebateCredit,
    },
};

use super::business_database::{enum_from_string, enum_to_string};

const DEFAULT_GATEWAY_URL: &str = "https://pay.example.com";
const DEFAULT_NOTIFY_PATH: &str = "/api/user/recharge/epay/notify";
const DEFAULT_RETURN_PATH: &str = "/api/user/recharge/epay/return";
const DEFAULT_MIN_AMOUNT_MINOR: i64 = 100;
const DEFAULT_MAX_AMOUNT_MINOR: i64 = 10_000_000;
const DEFAULT_RECHARGE_BONUS_RULES: &str = "[]";

#[derive(Clone)]
/// 充值订单仓储，负责该模块数据读取、业务变更和持久化协调。
pub struct RechargeRepository {
    /// 充值模块内存快照锁，保存充值订单和运行序号。
    pub(crate) inner: Arc<RwLock<RechargeStore>>,
    /// 可选数据库持久化句柄；内存模式下为空。
    pub(crate) persistence: Option<BusinessDatabase>,
}

#[derive(Debug, Clone)]
/// 充值运行配置，从系统设置读取金额范围、渠道和支付参数。
pub struct RechargeSettings {
    /// rainbow启用字段。
    pub rainbow_enabled: bool,
    /// rainbowgatewayurl字段。
    pub rainbow_gateway_url: String,
    /// rainbowpid字段。
    pub rainbow_pid: String,
    /// rainbowkey字段。
    pub rainbow_key: String,
    /// rainbownotifyurl字段。
    pub rainbow_notify_url: String,
    /// rainbowreturnurl字段。
    pub rainbow_return_url: String,
    /// rainbowpaytypes字段。
    pub rainbow_pay_types: Vec<String>,
    /// customerservice启用字段。
    pub customer_service_enabled: bool,
    /// customerservicemessage字段。
    pub customer_service_message: String,
    /// minamountminor字段。
    pub min_amount_minor: i64,
    /// maxamountminor字段。
    pub max_amount_minor: i64,
    /// 充值赠送活动开关。
    pub bonus_enabled: bool,
    /// 充值赠送活动档位。
    pub bonus_rules: Vec<RechargeBonusRule>,
}

#[derive(Debug, Clone)]
/// 客服直充订单创建后绑定的客服会话信息。
pub struct RechargeSupportTicket {
    /// conversationid字段。
    pub conversation_id: String,
    /// 客服会话主题。
    pub subject: String,
    /// 消息正文内容。
    pub content: String,
}

#[derive(Debug, Clone)]
/// 充值确认后的完整处理结果，包含订单和本次事务内写入的代理返利流水。
pub struct RechargeConfirmResult {
    /// 已确认或幂等返回的充值订单。
    pub order: RechargeOrderSummary,
    /// 本次确认同步写入的代理返利流水；重复确认不会再次返回。
    pub rebate_entry: Option<LedgerEntry>,
}

/// 充值订单仓储，负责该模块数据读取、业务变更和持久化协调。
impl RechargeRepository {
    /// 返回空的内存充值仓储，适配无数据库开发模式。
    pub fn memory() -> Self {
        Self {
            inner: Arc::new(RwLock::new(RechargeStore::default())),
            persistence: None,
        }
    }

    /// 从数据库加载充值订单，并创建持久化仓储。
    pub async fn persistent(persistence: BusinessDatabase) -> ApiResult<Self> {
        let store = load_recharge_store(&persistence).await?;
        Ok(Self {
            inner: Arc::new(RwLock::new(store)),
            persistence: Some(persistence),
        })
    }

    /// 返回全部充值订单，用于后台财务管理查看。
    pub async fn list(&self) -> ApiResult<Vec<RechargeOrderSummary>> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("recharge store lock poisoned".to_string()))
            .map(|store| store.list())
    }

    /// 按充值订单 ID 返回订单快照，供确认入账前计算同事务返利方案。
    pub async fn get_order(&self, order_id: &str) -> ApiResult<RechargeOrderSummary> {
        let order_id = required_trimmed(order_id, "充值订单 ID")?;
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("recharge store lock poisoned".to_string()))?
            .orders
            .get(&order_id)
            .cloned()
            .ok_or_else(|| ApiError::NotFound(format!("充值订单 `{order_id}` 不存在")))
    }

    /// 按导出条件返回充值订单；数据库模式下把过滤下推到 SQL，避免无条件导出时只能靠路由层筛选。
    pub async fn export_orders(
        &self,
        user_id: Option<&str>,
        status: Option<RechargeOrderStatus>,
        created_from: Option<&str>,
        created_to: Option<&str>,
    ) -> ApiResult<Vec<RechargeOrderSummary>> {
        let user_id = normalized_optional_filter(user_id);
        let created_from = normalized_optional_filter(created_from);
        let created_to = normalized_optional_filter(created_to);
        if let Some(persistence) = &self.persistence {
            return query_recharge_orders_for_export(
                persistence,
                user_id.as_deref(),
                status,
                created_from.as_deref(),
                created_to.as_deref(),
            )
            .await;
        }

        let status_filter = status.as_ref();
        let mut orders = self
            .list()
            .await?
            .into_iter()
            .filter(|order| {
                user_id
                    .as_deref()
                    .map_or(true, |target| order.user_id == target)
                    && status_filter.map_or(true, |target| order.status == *target)
                    && created_from
                        .as_deref()
                        .map_or(true, |from| order.created_at.as_str() >= from)
                    && created_to
                        .as_deref()
                        .map_or(true, |to| order.created_at.as_str() <= to)
            })
            .collect::<Vec<_>>();
        orders.sort_by(|left, right| {
            right
                .created_at
                .cmp(&left.created_at)
                .then_with(|| right.id.cmp(&left.id))
        });
        Ok(orders)
    }

    /// 分页返回全部充值订单；数据库模式下直接按时间倒序分页。
    pub async fn list_page(&self, page: PageRequest) -> ApiResult<ListPage<RechargeOrderSummary>> {
        if let Some(persistence) = &self.persistence {
            return query_recharge_order_page(persistence, None, page).await;
        }

        let mut orders = self.list().await?;
        orders.sort_by(|left, right| {
            right
                .created_at
                .cmp(&left.created_at)
                .then_with(|| right.id.cmp(&left.id))
        });
        Ok(ListPage::from_all(orders, page))
    }

    /// 返回已支付充值订单，供代理返利统计避免读取待支付和已取消订单。
    pub async fn paid_orders(&self) -> ApiResult<Vec<RechargeOrderSummary>> {
        if let Some(persistence) = &self.persistence {
            return query_recharge_orders_by_status(persistence, RechargeOrderStatus::Paid).await;
        }

        Ok(self
            .list()
            .await?
            .into_iter()
            .filter(|order| order.status == RechargeOrderStatus::Paid)
            .collect())
    }

    /// 一键清除充值订单历史；仅删除记录，不回滚已入账余额和资金流水。
    pub async fn clear_records(&self) -> ApiResult<usize> {
        let (deleted_count, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("recharge store lock poisoned".to_string()))?;
            let deleted_count = store.clear_records();
            (deleted_count, store.clone())
        };
        self.persist(&snapshot).await?;
        Ok(deleted_count)
    }

    /// 按指定用户已入账充值订单校准累计充值表，供聊天大厅发言资格和提现流水门槛读取。
    ///
    /// 正常确认充值时已经会写入累计表；这里作为读取资格前的兜底校准，专门处理旧版本确认过的充值、
    /// 迁移未及时补齐或服务器中途升级导致累计表低于充值订单总额的情况。内存模式没有数据库累计表，直接返回。
    pub async fn reconcile_paid_recharge_turnover_for_user(&self, user_id: &str) -> ApiResult<()> {
        let user_id = required_trimmed(user_id, "用户 ID")?;
        let Some(database) = &self.persistence else {
            return Ok(());
        };

        let mut tx = database
            .pool()
            .begin()
            .await
            .map_err(|_| ApiError::Internal("用户累计充值资格校准事务开启失败".to_string()))?;
        reconcile_paid_recharge_turnover_for_user_in_transaction(&mut *tx, &user_id).await?;
        tx.commit()
            .await
            .map_err(|_| ApiError::Internal("用户累计充值资格校准事务提交失败".to_string()))
    }

    /// 返回指定用户充值订单。
    pub async fn list_for_user(&self, user_id: &str) -> ApiResult<Vec<RechargeOrderSummary>> {
        let user_id = required_trimmed(user_id, "user id")?;
        Ok(self
            .inner
            .read()
            .map_err(|_| ApiError::Internal("recharge store lock poisoned".to_string()))?
            .list_for_user(&user_id))
    }

    /// 分页返回指定用户充值订单，供手机端避免全量拉取历史充值。
    pub async fn list_for_user_page(
        &self,
        user_id: &str,
        page: PageRequest,
    ) -> ApiResult<ListPage<RechargeOrderSummary>> {
        let user_id = required_trimmed(user_id, "user id")?;
        if let Some(persistence) = &self.persistence {
            return query_recharge_order_page(persistence, Some(&user_id), page).await;
        }

        let mut orders = self.list_for_user(&user_id).await?;
        orders.sort_by(|left, right| {
            right
                .created_at
                .cmp(&left.created_at)
                .then_with(|| right.id.cmp(&left.id))
        });
        Ok(ListPage::from_all(orders, page))
    }

    /// 创建充值订单；彩虹易支付返回跳转 URL，客服直充返回客服会话 ID。
    pub async fn create_order(
        &self,
        user: &UserSummary,
        request: CreateRechargeOrderRequest,
        settings: &RechargeSettings,
    ) -> ApiResult<CreateRechargeOrderResponse> {
        let (result, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("recharge store lock poisoned".to_string()))?;
            let result = store.create_order(user, request, settings)?;
            (result, store.clone())
        };
        self.persist(&snapshot).await?;
        Ok(result)
    }

    /// 为客服直充订单补充客服会话 ID。
    pub async fn attach_support_conversation(
        &self,
        order_id: &str,
        conversation_id: &str,
    ) -> ApiResult<RechargeOrderSummary> {
        let (result, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("recharge store lock poisoned".to_string()))?;
            let result = store.attach_support_conversation(order_id, conversation_id)?;
            (result, store.clone())
        };
        self.persist(&snapshot).await?;
        Ok(result)
    }

    #[cfg(test)]
    #[allow(dead_code)]
    /// 处理彩虹易支付异步通知，验签成功且状态成功时给用户入账。
    pub async fn confirm_rainbow_notify(
        &self,
        params: HashMap<String, String>,
        settings: &RechargeSettings,
        finance: &FinanceRepository,
    ) -> ApiResult<RechargeOrderSummary> {
        self.confirm_rainbow_notify_with_rebate(params, settings, finance, None)
            .await
            .map(|result| result.order)
    }

    /// 处理彩虹易支付异步通知，并在同一个充值资金事务内发放代理返利。
    pub async fn confirm_rainbow_notify_with_rebate(
        &self,
        params: HashMap<String, String>,
        settings: &RechargeSettings,
        finance: &FinanceRepository,
        rebate_credit: Option<&RechargeRebateCredit>,
    ) -> ApiResult<RechargeConfirmResult> {
        verify_rainbow_sign(&params, &settings.rainbow_key)?;
        let status = params.get("trade_status").map(String::as_str).unwrap_or("");
        if status != "TRADE_SUCCESS" {
            return Err(ApiError::BadRequest(
                "彩虹易支付通知状态不是成功".to_string(),
            ));
        }

        let order_id = params
            .get("out_trade_no")
            .map(String::as_str)
            .ok_or_else(|| ApiError::BadRequest("彩虹易支付通知缺少商户订单号".to_string()))?;
        let trade_no = params.get("trade_no").cloned();
        let money_text = params
            .get("money")
            .map(String::as_str)
            .ok_or_else(|| ApiError::BadRequest("彩虹易支付通知缺少金额".to_string()))?;
        let paid_amount_minor = money_to_minor(money_text)?;

        let previous_recharge_store = self
            .inner
            .read()
            .map_err(|_| ApiError::Internal("recharge store lock poisoned".to_string()))?
            .clone();
        let mut recharge_store = previous_recharge_store.clone();
        let previous_finance_store = finance
            .inner
            .read()
            .map_err(|_| ApiError::Internal("finance store lock poisoned".to_string()))?
            .clone();
        let mut finance_store = previous_finance_store.clone();

        let was_paid = recharge_store.is_paid_order(order_id)?;
        let order = recharge_store.mark_paid(order_id, paid_amount_minor, trade_no)?;
        let mut rebate_entry = None;
        if !was_paid {
            finance_store.credit_recharge(&order.user_id, order.amount_minor, &order.id)?;
            credit_recharge_bonus_for_order(&mut finance_store, &order, settings)?;
            rebate_entry =
                credit_recharge_rebate_for_order(&mut finance_store, &order, rebate_credit)?;
        }

        let id_remap = persist_recharge_finance_stores(
            self,
            finance,
            &previous_recharge_store,
            &recharge_store,
            &previous_finance_store,
            &mut finance_store,
            Some(&order),
        )
        .await?;
        id_remap.apply_to_optional_entry(&mut rebate_entry);
        self.replace_store(recharge_store)?;
        finance.replace_store(finance_store)?;
        Ok(RechargeConfirmResult {
            order,
            rebate_entry,
        })
    }

    #[cfg(test)]
    /// 后台确认客服直充已收款，并给用户余额入账。
    pub async fn confirm_customer_service_order(
        &self,
        order_id: &str,
        request: ConfirmRechargeOrderRequest,
        settings: &RechargeSettings,
        finance: &FinanceRepository,
    ) -> ApiResult<RechargeOrderSummary> {
        self.confirm_customer_service_order_with_rebate(order_id, request, settings, finance, None)
            .await
            .map(|result| result.order)
    }

    /// 后台确认客服直充已收款，并在同一个充值资金事务内发放代理返利。
    pub async fn confirm_customer_service_order_with_rebate(
        &self,
        order_id: &str,
        request: ConfirmRechargeOrderRequest,
        settings: &RechargeSettings,
        finance: &FinanceRepository,
        rebate_credit: Option<&RechargeRebateCredit>,
    ) -> ApiResult<RechargeConfirmResult> {
        let previous_recharge_store = self
            .inner
            .read()
            .map_err(|_| ApiError::Internal("recharge store lock poisoned".to_string()))?
            .clone();
        let mut recharge_store = previous_recharge_store.clone();
        let previous_finance_store = finance
            .inner
            .read()
            .map_err(|_| ApiError::Internal("finance store lock poisoned".to_string()))?
            .clone();
        let mut finance_store = previous_finance_store.clone();

        let was_paid = recharge_store.is_paid_order(order_id)?;
        let order = recharge_store.confirm_customer_service_order(order_id, request)?;
        let mut rebate_entry = None;
        if !was_paid {
            finance_store.credit_recharge(&order.user_id, order.amount_minor, &order.id)?;
            credit_recharge_bonus_for_order(&mut finance_store, &order, settings)?;
            rebate_entry =
                credit_recharge_rebate_for_order(&mut finance_store, &order, rebate_credit)?;
        }

        let id_remap = persist_recharge_finance_stores(
            self,
            finance,
            &previous_recharge_store,
            &recharge_store,
            &previous_finance_store,
            &mut finance_store,
            Some(&order),
        )
        .await?;
        id_remap.apply_to_optional_entry(&mut rebate_entry);
        self.replace_store(recharge_store)?;
        finance.replace_store(finance_store)?;
        Ok(RechargeConfirmResult {
            order,
            rebate_entry,
        })
    }
    /// 把当前仓储快照同步保存到持久化存储。
    async fn persist(&self, store: &RechargeStore) -> ApiResult<()> {
        if let Some(persistence) = &self.persistence {
            save_recharge_store(persistence, store).await?;
        }
        Ok(())
    }

    /// 从数据库重新加载充值订单快照，供后台缓存维护使用。
    pub async fn reload_from_database(&self) -> ApiResult<bool> {
        let Some(persistence) = &self.persistence else {
            return Ok(false);
        };
        let store = load_recharge_store(persistence).await?;
        self.replace_store(store)?;
        Ok(true)
    }

    /// 用事务提交后的快照替换当前充值订单内存状态。
    pub(crate) fn replace_store(&self, store: RechargeStore) -> ApiResult<()> {
        *self
            .inner
            .write()
            .map_err(|_| ApiError::Internal("recharge store lock poisoned".to_string()))? = store;
        Ok(())
    }
}

/// 在同一个数据库事务中保存充值和资金快照，确保入账订单与余额流水一致。
async fn persist_recharge_finance_stores(
    recharges: &RechargeRepository,
    finance: &FinanceRepository,
    previous_recharge_store: &RechargeStore,
    recharge_store: &RechargeStore,
    previous_finance_store: &super::finance::FinanceStore,
    finance_store: &mut super::finance::FinanceStore,
    confirmed_order: Option<&RechargeOrderSummary>,
) -> ApiResult<LedgerEntryIdRemap> {
    match (&recharges.persistence, &finance.persistence) {
        (Some(database), Some(_)) => {
            let mut tx = database
                .pool()
                .begin()
                .await
                .map_err(|_| ApiError::Internal("充值资金事务开启失败".to_string()))?;
            save_recharge_store_incremental_in_transaction(
                &mut *tx,
                previous_recharge_store,
                recharge_store,
            )
            .await?;
            let id_remap = save_finance_store_incremental_in_transaction(
                &mut *tx,
                previous_finance_store,
                finance_store,
            )
            .await?;
            if let Some(order) = confirmed_order {
                ensure_paid_recharge_turnover_from_orders_in_transaction(&mut *tx, order).await?;
            }
            tx.commit()
                .await
                .map_err(|_| ApiError::Internal("充值资金事务提交失败".to_string()))?;
            Ok(id_remap)
        }
        (None, None) => Ok(LedgerEntryIdRemap::default()),
        _ => Err(ApiError::Internal("充值和资金持久化配置不一致".to_string())),
    }
}

/// 按用户已入账充值单兜底校准累计充值表，保证聊天大厅发言门槛不依赖资金流水是否仍存在。
///
/// 正常路径由 `ledger_entries` 触发器和资金补偿事件维护累计表；这里在充值确认事务末尾再次按
/// `recharge_orders.status=paid` 聚合当前用户的真实充值本金，用 `GREATEST` 只补高不回退。这样即使
/// 服务器旧库缺少触发器、资金流水被清理、或重复确认已入账订单时没有新流水，也能让发言资格恢复。
async fn ensure_paid_recharge_turnover_from_orders_in_transaction(
    connection: &mut PgConnection,
    order: &RechargeOrderSummary,
) -> ApiResult<()> {
    if order.status != RechargeOrderStatus::Paid {
        return Ok(());
    }
    reconcile_paid_recharge_turnover_for_user_in_transaction(connection, &order.user_id).await
}

/// 在同一个数据库事务内按用户 paid 充值订单重算累计充值，并只补高不回退。
async fn reconcile_paid_recharge_turnover_for_user_in_transaction(
    connection: &mut PgConnection,
    user_id: &str,
) -> ApiResult<()> {
    sqlx::query(
        "WITH paid_recharge_totals AS (
            SELECT
                user_id,
                COALESCE(SUM(amount_minor), 0)::BIGINT AS cumulative_recharge_minor
            FROM recharge_orders
            WHERE user_id = $1
              AND status = 'paid'
              AND amount_minor > 0
            GROUP BY user_id
         )
         INSERT INTO user_withdrawal_turnovers (
            user_id,
            cumulative_recharge_minor,
            required_effective_bet_minor,
            completed_effective_bet_minor,
            created_at,
            updated_at
         )
         SELECT
            user_id,
            cumulative_recharge_minor,
            cumulative_recharge_minor,
            0,
            now(),
            now()
         FROM paid_recharge_totals
         WHERE cumulative_recharge_minor > 0
         ON CONFLICT (user_id) DO UPDATE SET
            cumulative_recharge_minor = GREATEST(
                user_withdrawal_turnovers.cumulative_recharge_minor,
                EXCLUDED.cumulative_recharge_minor
            ),
            required_effective_bet_minor = GREATEST(
                user_withdrawal_turnovers.required_effective_bet_minor,
                EXCLUDED.required_effective_bet_minor
            ),
            updated_at = now()",
    )
    .bind(user_id)
    .execute(&mut *connection)
    .await
    .map_err(|error| {
        tracing::error!(
            %error,
            user_id,
            "用户累计充值资格校准失败"
        );
        ApiError::Internal("用户累计充值资格校准失败".to_string())
    })?;
    Ok(())
}

/// 按充值赠送活动配置给用户补送彩金，未开启或未命中档位时静默跳过。
fn credit_recharge_bonus_for_order(
    finance_store: &mut super::finance::FinanceStore,
    order: &RechargeOrderSummary,
    settings: &RechargeSettings,
) -> ApiResult<Option<crate::domain::finance::LedgerEntry>> {
    let bonus_amount_minor = recharge_bonus_amount_minor(order.amount_minor, settings)?;
    if bonus_amount_minor <= 0 {
        return Ok(None);
    }

    finance_store
        .credit_recharge_bonus(&order.user_id, bonus_amount_minor, &order.id)
        .map(Some)
}

/// 按预先计算的返利方案给代理入账；仅在充值本金同事务入账时调用。
fn credit_recharge_rebate_for_order(
    finance_store: &mut super::finance::FinanceStore,
    order: &RechargeOrderSummary,
    rebate_credit: Option<&RechargeRebateCredit>,
) -> ApiResult<Option<LedgerEntry>> {
    let Some(rebate_credit) = rebate_credit else {
        return Ok(None);
    };
    if rebate_credit.invitee_user_id != order.user_id {
        return Err(ApiError::BadRequest(
            "充值返利下级用户与充值订单不一致".to_string(),
        ));
    }

    finance_store
        .credit_recharge_rebate(
            &rebate_credit.agent_user_id,
            &rebate_credit.invitee_user_id,
            rebate_credit.amount_minor,
            &order.id,
        )
        .map(Some)
}

/// 计算单笔充值可获得的赠送彩金，命中多个档位时取最高充值门槛对应的赠送金额。
fn recharge_bonus_amount_minor(
    recharge_amount_minor: i64,
    settings: &RechargeSettings,
) -> ApiResult<i64> {
    if !settings.bonus_enabled || recharge_amount_minor <= 0 {
        return Ok(0);
    }

    Ok(settings
        .bonus_rules
        .iter()
        .filter(|rule| {
            rule.threshold_amount_minor > 0
                && rule.bonus_amount_minor > 0
                && recharge_amount_minor >= rule.threshold_amount_minor
        })
        .max_by_key(|rule| rule.threshold_amount_minor)
        .map(|rule| rule.bonus_amount_minor)
        .unwrap_or(0))
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
/// 充值订单运行时数据快照，用于内存模式和数据库持久化前的业务校验。
pub(crate) struct RechargeStore {
    orders: BTreeMap<String, RechargeOrderSummary>,
    next_sequence: u64,
}

/// 充值订单运行时数据快照，用于内存模式和数据库持久化前的业务校验。
impl RechargeStore {
    /// 返回按创建顺序倒序排列的充值订单列表。
    fn list(&self) -> Vec<RechargeOrderSummary> {
        self.orders.values().rev().cloned().collect()
    }

    /// 返回某个用户自己的充值订单列表。
    fn list_for_user(&self, user_id: &str) -> Vec<RechargeOrderSummary> {
        self.orders
            .values()
            .filter(|order| order.user_id == user_id)
            .cloned()
            .rev()
            .collect()
    }

    /// 判断订单是否已经入账，用于充值回调和后台确认保持赠送彩金幂等。
    fn is_paid_order(&self, order_id: &str) -> ApiResult<bool> {
        let order = self
            .orders
            .get(order_id)
            .ok_or_else(|| ApiError::NotFound(format!("recharge order `{order_id}` not found")))?;
        Ok(order.status == RechargeOrderStatus::Paid)
    }

    /// 清除所有充值订单记录并保留下一订单流水号，避免清理后生成重复单号。
    fn clear_records(&mut self) -> usize {
        let deleted_count = self.orders.len();
        self.orders.clear();
        deleted_count
    }

    /// 校验配置和金额并创建充值订单。
    fn create_order(
        &mut self,
        user: &UserSummary,
        request: CreateRechargeOrderRequest,
        settings: &RechargeSettings,
    ) -> ApiResult<CreateRechargeOrderResponse> {
        validate_amount(request.amount_minor, settings)?;

        self.next_sequence += 1;
        let order_id = format!("R{:012}", self.next_sequence);
        let now = current_time_label();

        match request.channel {
            RechargeChannel::RainbowEpay => {
                validate_rainbow_settings(settings)?;
                if settings.rainbow_pay_types.is_empty() {
                    return Err(ApiError::BadRequest(
                        "彩虹易支付未开启任何支付方式".to_string(),
                    ));
                }
                let pay_type = request
                    .pay_type
                    .map(|value| value.trim().to_string())
                    .filter(|value| !value.is_empty())
                    .unwrap_or_else(|| {
                        settings
                            .rainbow_pay_types
                            .first()
                            .cloned()
                            .unwrap_or_default()
                    });
                if !settings.rainbow_pay_types.is_empty()
                    && !settings.rainbow_pay_types.contains(&pay_type)
                {
                    return Err(ApiError::BadRequest(
                        "彩虹易支付方式未在后台配置中启用".to_string(),
                    ));
                }

                let payment_url =
                    rainbow_payment_url(settings, &order_id, request.amount_minor, &pay_type)?;
                let order = RechargeOrderSummary {
                    id: order_id.clone(),
                    user_id: user.id.clone(),
                    username: user.username.clone(),
                    channel: RechargeChannel::RainbowEpay,
                    amount_minor: request.amount_minor,
                    status: RechargeOrderStatus::Pending,
                    pay_type: Some(pay_type),
                    provider_trade_no: None,
                    payment_url: Some(payment_url.clone()),
                    support_conversation_id: None,
                    remark: String::new(),
                    created_at: now,
                    paid_at: None,
                };
                self.orders.insert(order_id, order.clone());
                Ok(CreateRechargeOrderResponse {
                    order,
                    payment_url: Some(payment_url),
                    support_conversation_id: None,
                    message: "请跳转到彩虹易支付完成充值".to_string(),
                })
            }
            RechargeChannel::CustomerService => {
                if !settings.customer_service_enabled {
                    return Err(ApiError::BadRequest("客服直充未开启".to_string()));
                }
                let conversation_id = format!("CS-RCH-{order_id}");
                let order = RechargeOrderSummary {
                    id: order_id.clone(),
                    user_id: user.id.clone(),
                    username: user.username.clone(),
                    channel: RechargeChannel::CustomerService,
                    amount_minor: request.amount_minor,
                    status: RechargeOrderStatus::WaitingCustomerService,
                    pay_type: None,
                    provider_trade_no: None,
                    payment_url: None,
                    support_conversation_id: Some(conversation_id.clone()),
                    remark: String::new(),
                    created_at: now,
                    paid_at: None,
                };
                self.orders.insert(order_id, order.clone());
                Ok(CreateRechargeOrderResponse {
                    order,
                    payment_url: None,
                    support_conversation_id: Some(conversation_id),
                    message: settings.customer_service_message.clone(),
                })
            }
        }
    }

    /// 绑定客服会话 ID，重复绑定同一个 ID 时保持幂等。
    fn attach_support_conversation(
        &mut self,
        order_id: &str,
        conversation_id: &str,
    ) -> ApiResult<RechargeOrderSummary> {
        let order = self
            .orders
            .get_mut(order_id)
            .ok_or_else(|| ApiError::NotFound(format!("recharge order `{order_id}` not found")))?;
        order.support_conversation_id = Some(required_trimmed(
            conversation_id,
            "support conversation id",
        )?);
        Ok(order.clone())
    }

    /// 将充值订单标记为已支付，并校验通知金额和订单状态。
    pub(crate) fn mark_paid(
        &mut self,
        order_id: &str,
        amount_minor: i64,
        provider_trade_no: Option<String>,
    ) -> ApiResult<RechargeOrderSummary> {
        let order = self
            .orders
            .get_mut(order_id)
            .ok_or_else(|| ApiError::NotFound(format!("recharge order `{order_id}` not found")))?;
        if order.channel != RechargeChannel::RainbowEpay {
            return Err(ApiError::BadRequest(
                "充值订单不是彩虹易支付订单".to_string(),
            ));
        }
        if amount_minor != order.amount_minor {
            return Err(ApiError::BadRequest(
                "彩虹易支付通知金额与订单不一致".to_string(),
            ));
        }
        if order.status == RechargeOrderStatus::Paid {
            return Ok(order.clone());
        }

        order.status = RechargeOrderStatus::Paid;
        order.provider_trade_no = provider_trade_no;
        order.paid_at = Some(current_time_label());
        Ok(order.clone())
    }

    /// 后台确认客服直充订单收款成功。
    pub(crate) fn confirm_customer_service_order(
        &mut self,
        order_id: &str,
        request: ConfirmRechargeOrderRequest,
    ) -> ApiResult<RechargeOrderSummary> {
        let order = self
            .orders
            .get_mut(order_id)
            .ok_or_else(|| ApiError::NotFound(format!("recharge order `{order_id}` not found")))?;
        if order.channel != RechargeChannel::CustomerService {
            return Err(ApiError::BadRequest("充值订单不是客服直充订单".to_string()));
        }
        if order.status == RechargeOrderStatus::Cancelled {
            return Err(ApiError::BadRequest(
                "充值订单已取消，不能确认入账".to_string(),
            ));
        }
        if order.status == RechargeOrderStatus::Paid {
            return Ok(order.clone());
        }

        order.status = RechargeOrderStatus::Paid;
        order.provider_trade_no = request
            .provider_trade_no
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        order.remark = normalized_confirm_remark(request.remark);
        order.paid_at = Some(current_time_label());
        Ok(order.clone())
    }
}

/// 归一化后台确认入账备注，空白备注保存为空字符串。
fn normalized_confirm_remark(remark: Option<String>) -> String {
    remark
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_default()
}

/// 从系统设置构造用户端充值配置。
pub fn recharge_settings_from_system_settings(settings: &[SystemSetting]) -> RechargeSettings {
    let map = settings
        .iter()
        .map(|setting| (setting.key.as_str(), setting.value.as_str()))
        .collect::<HashMap<_, _>>();

    RechargeSettings {
        rainbow_enabled: bool_setting(&map, "recharge_rainbow_epay_enabled", false),
        rainbow_gateway_url: string_setting(
            &map,
            "recharge_rainbow_epay_gateway_url",
            DEFAULT_GATEWAY_URL,
        ),
        rainbow_pid: string_setting(&map, "recharge_rainbow_epay_pid", ""),
        rainbow_key: string_setting(&map, "recharge_rainbow_epay_key", ""),
        rainbow_notify_url: string_setting(&map, "recharge_rainbow_epay_notify_url", ""),
        rainbow_return_url: string_setting(&map, "recharge_rainbow_epay_return_url", ""),
        rainbow_pay_types: csv_setting(&map, "recharge_rainbow_epay_pay_types"),
        customer_service_enabled: bool_setting(&map, "recharge_customer_service_enabled", true),
        customer_service_message: string_setting(
            &map,
            "recharge_customer_service_message",
            "客服已收到您的直充申请，请在会话中确认付款方式和到账信息。",
        ),
        min_amount_minor: i64_setting(&map, "recharge_min_amount_minor", DEFAULT_MIN_AMOUNT_MINOR),
        max_amount_minor: i64_setting(&map, "recharge_max_amount_minor", DEFAULT_MAX_AMOUNT_MINOR),
        bonus_enabled: bool_setting(&map, "recharge_bonus_enabled", false),
        bonus_rules: recharge_bonus_rules_setting(&map, "recharge_bonus_rules"),
    }
}

/// 将后台充值配置转换成用户端可见的渠道说明。
pub fn recharge_config_response(settings: &RechargeSettings) -> RechargeConfigResponse {
    RechargeConfigResponse {
        channels: vec![
            RechargeChannelConfig {
                channel: RechargeChannel::RainbowEpay,
                name: "彩虹易支付".to_string(),
                enabled: settings.rainbow_enabled && !settings.rainbow_pay_types.is_empty(),
                description: "跳转到彩虹易支付完成在线充值".to_string(),
                pay_types: settings.rainbow_pay_types.clone(),
            },
            RechargeChannelConfig {
                channel: RechargeChannel::CustomerService,
                name: "客服直充".to_string(),
                enabled: settings.customer_service_enabled,
                description: settings.customer_service_message.clone(),
                pay_types: Vec::new(),
            },
        ],
        min_amount_minor: settings.min_amount_minor,
        max_amount_minor: settings.max_amount_minor,
        bonus_enabled: settings.bonus_enabled && !settings.bonus_rules.is_empty(),
        bonus_rules: settings.bonus_rules.clone(),
    }
}

/// 根据充值订单生成客服会话初始化参数。
pub fn support_ticket_for_recharge(order: &RechargeOrderSummary) -> Option<RechargeSupportTicket> {
    let conversation_id = order.support_conversation_id.clone()?;
    Some(RechargeSupportTicket {
        conversation_id,
        subject: format!("客服直充 {}", order.id),
        content: format!(
            "我需要客服直充，充值单号：{}，充值金额：{}。",
            order.id,
            minor_to_money(order.amount_minor)
        ),
    })
}

/// 从数据库加载充值订单运行时快照，空库时按模块规则初始化。
async fn load_recharge_store(database: &BusinessDatabase) -> ApiResult<RechargeStore> {
    let pool = database.pool();
    let mut orders = BTreeMap::new();
    for row in sqlx::query(
        "SELECT id, user_id, username, channel, amount_minor, status, pay_type,
                provider_trade_no, payment_url, support_conversation_id, remark, created_at, paid_at
         FROM recharge_orders
         ORDER BY id ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::Internal("充值订单数据读取失败".to_string()))?
    {
        let order = recharge_order_from_row(row)?;
        orders.insert(order.id.clone(), order);
    }

    let next_sequence = sqlx::query_scalar::<_, i64>(
        "SELECT value FROM recharge_runtime WHERE key = 'next_sequence'",
    )
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::Internal("充值运行数据读取失败".to_string()))?
    .unwrap_or_default();

    Ok(RechargeStore {
        orders,
        next_sequence: u64::try_from(next_sequence).unwrap_or_default(),
    })
}

/// 从数据库行恢复充值订单结构，供启动加载和分页查询复用。
fn recharge_order_from_row(row: PgRow) -> ApiResult<RechargeOrderSummary> {
    Ok(RechargeOrderSummary {
        id: row
            .try_get("id")
            .map_err(|_| ApiError::Internal("充值订单数据读取失败".to_string()))?,
        user_id: row
            .try_get("user_id")
            .map_err(|_| ApiError::Internal("充值订单数据读取失败".to_string()))?,
        username: row
            .try_get("username")
            .map_err(|_| ApiError::Internal("充值订单数据读取失败".to_string()))?,
        channel: enum_from_string(
            row.try_get("channel")
                .map_err(|_| ApiError::Internal("充值订单数据读取失败".to_string()))?,
        )?,
        amount_minor: row
            .try_get("amount_minor")
            .map_err(|_| ApiError::Internal("充值订单数据读取失败".to_string()))?,
        status: enum_from_string(
            row.try_get("status")
                .map_err(|_| ApiError::Internal("充值订单数据读取失败".to_string()))?,
        )?,
        pay_type: row
            .try_get("pay_type")
            .map_err(|_| ApiError::Internal("充值订单数据读取失败".to_string()))?,
        provider_trade_no: row
            .try_get("provider_trade_no")
            .map_err(|_| ApiError::Internal("充值订单数据读取失败".to_string()))?,
        payment_url: row
            .try_get("payment_url")
            .map_err(|_| ApiError::Internal("充值订单数据读取失败".to_string()))?,
        support_conversation_id: row
            .try_get("support_conversation_id")
            .map_err(|_| ApiError::Internal("充值订单数据读取失败".to_string()))?,
        remark: row
            .try_get("remark")
            .map_err(|_| ApiError::Internal("充值订单数据读取失败".to_string()))?,
        created_at: row
            .try_get("created_at")
            .map_err(|_| ApiError::Internal("充值订单数据读取失败".to_string()))?,
        paid_at: row
            .try_get("paid_at")
            .map_err(|_| ApiError::Internal("充值订单数据读取失败".to_string()))?,
    })
}

/// 数据库模式下分页读取充值订单，支持后台列表和用户端本人列表。
async fn query_recharge_order_page(
    database: &BusinessDatabase,
    user_id: Option<&str>,
    page: PageRequest,
) -> ApiResult<ListPage<RechargeOrderSummary>> {
    let total_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*)
         FROM recharge_orders
         WHERE ($1::text IS NULL OR user_id = $1)",
    )
    .bind(user_id)
    .fetch_one(database.pool())
    .await
    .map_err(|_| ApiError::Internal("充值订单分页总数读取失败".to_string()))?;
    let total_count = usize::try_from(total_count)
        .map_err(|_| ApiError::Internal("充值订单分页总数无效".to_string()))?;
    let resolved = page.resolve(total_count);
    let rows = sqlx::query(
        "SELECT id, user_id, username, channel, amount_minor, status, pay_type,
                provider_trade_no, payment_url, support_conversation_id, remark, created_at, paid_at
         FROM recharge_orders
         WHERE ($1::text IS NULL OR user_id = $1)
         ORDER BY created_at DESC, id DESC
         LIMIT $2 OFFSET $3",
    )
    .bind(user_id)
    .bind(resolved.limit_i64()?)
    .bind(resolved.offset_i64()?)
    .fetch_all(database.pool())
    .await
    .map_err(|_| ApiError::Internal("充值订单分页数据读取失败".to_string()))?;
    let items = rows
        .into_iter()
        .map(recharge_order_from_row)
        .collect::<ApiResult<Vec<_>>>()?;

    Ok(ListPage::new(items, resolved))
}

/// 数据库模式下按导出条件读取充值订单，供 CSV 导出避免固定全量扫描。
async fn query_recharge_orders_for_export(
    database: &BusinessDatabase,
    user_id: Option<&str>,
    status: Option<RechargeOrderStatus>,
    created_from: Option<&str>,
    created_to: Option<&str>,
) -> ApiResult<Vec<RechargeOrderSummary>> {
    let status = status.map(|status| enum_to_string(&status)).transpose()?;
    let rows = sqlx::query(
        "SELECT id, user_id, username, channel, amount_minor, status, pay_type,
                provider_trade_no, payment_url, support_conversation_id, remark, created_at, paid_at
         FROM recharge_orders
         WHERE ($1::text IS NULL OR user_id = $1)
           AND ($2::text IS NULL OR status = $2)
           AND ($3::text IS NULL OR created_at >= $3)
           AND ($4::text IS NULL OR created_at <= $4)
         ORDER BY created_at DESC, id DESC",
    )
    .bind(user_id)
    .bind(status.as_deref())
    .bind(created_from)
    .bind(created_to)
    .fetch_all(database.pool())
    .await
    .map_err(|_| ApiError::Internal("充值订单导出数据读取失败".to_string()))?;

    rows.into_iter().map(recharge_order_from_row).collect()
}

/// 数据库模式下按状态读取充值订单，供聚合统计只读取必要业务状态。
async fn query_recharge_orders_by_status(
    database: &BusinessDatabase,
    status: RechargeOrderStatus,
) -> ApiResult<Vec<RechargeOrderSummary>> {
    let status = enum_to_string(&status)?;
    let rows = sqlx::query(
        "SELECT id, user_id, username, channel, amount_minor, status, pay_type,
                provider_trade_no, payment_url, support_conversation_id, remark, created_at, paid_at
         FROM recharge_orders
         WHERE status = $1
         ORDER BY created_at DESC, id DESC",
    )
    .bind(status)
    .fetch_all(database.pool())
    .await
    .map_err(|_| ApiError::Internal("充值订单状态数据读取失败".to_string()))?;

    rows.into_iter().map(recharge_order_from_row).collect()
}

/// 把充值订单运行时快照保存到数据库。
async fn save_recharge_store(database: &BusinessDatabase, store: &RechargeStore) -> ApiResult<()> {
    let mut tx = database
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::Internal("充值事务开启失败".to_string()))?;

    save_recharge_store_in_transaction(&mut *tx, store).await?;

    tx.commit()
        .await
        .map_err(|_| ApiError::Internal("充值事务提交失败".to_string()))
}

/// 在外层事务中保存充值订单运行时快照，供跨仓储事务复用。
pub(crate) async fn save_recharge_store_in_transaction(
    connection: &mut PgConnection,
    store: &RechargeStore,
) -> ApiResult<()> {
    for table in ["recharge_orders", "recharge_runtime"] {
        sqlx::query(&format!("DELETE FROM {table}"))
            .execute(&mut *connection)
            .await
            .map_err(|_| ApiError::Internal("充值数据清理失败".to_string()))?;
    }

    for order in store.orders.values() {
        sqlx::query(
            "INSERT INTO recharge_orders
             (id, user_id, username, channel, amount_minor, status, pay_type,
              provider_trade_no, payment_url, support_conversation_id, remark, created_at, paid_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)",
        )
        .bind(&order.id)
        .bind(&order.user_id)
        .bind(&order.username)
        .bind(enum_to_string(&order.channel)?)
        .bind(order.amount_minor)
        .bind(enum_to_string(&order.status)?)
        .bind(&order.pay_type)
        .bind(&order.provider_trade_no)
        .bind(&order.payment_url)
        .bind(&order.support_conversation_id)
        .bind(&order.remark)
        .bind(&order.created_at)
        .bind(&order.paid_at)
        .execute(&mut *connection)
        .await
        .map_err(|_| ApiError::Internal("充值订单数据保存失败".to_string()))?;
    }

    let next_sequence = i64::try_from(store.next_sequence)
        .map_err(|_| ApiError::Internal("充值序号过大".to_string()))?;
    sqlx::query("INSERT INTO recharge_runtime (key, value) VALUES ('next_sequence', $1)")
        .bind(next_sequence)
        .execute(&mut *connection)
        .await
        .map_err(|_| ApiError::Internal("充值运行数据保存失败".to_string()))?;

    Ok(())
}

/// 在外层事务中按前后快照差异保存充值订单，避免确认入账时重写全部充值历史。
pub(crate) async fn save_recharge_store_incremental_in_transaction(
    connection: &mut PgConnection,
    previous: &RechargeStore,
    store: &RechargeStore,
) -> ApiResult<()> {
    for order_id in previous
        .orders
        .keys()
        .filter(|order_id| !store.orders.contains_key(*order_id))
    {
        sqlx::query("DELETE FROM recharge_orders WHERE id = $1")
            .bind(order_id)
            .execute(&mut *connection)
            .await
            .map_err(|_| ApiError::Internal("充值订单数据删除失败".to_string()))?;
    }

    for (order_id, order) in &store.orders {
        if previous.orders.get(order_id) == Some(order) {
            continue;
        }
        upsert_recharge_order_in_transaction(connection, order).await?;
    }

    let next_sequence = i64::try_from(store.next_sequence)
        .map_err(|_| ApiError::Internal("充值序号过大".to_string()))?;
    sqlx::query(
        "INSERT INTO recharge_runtime (key, value) VALUES ('next_sequence', $1)
         ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value",
    )
    .bind(next_sequence)
    .execute(&mut *connection)
    .await
    .map_err(|_| ApiError::Internal("充值运行数据保存失败".to_string()))?;

    Ok(())
}

/// 在事务中插入或更新单个充值订单。
async fn upsert_recharge_order_in_transaction(
    connection: &mut PgConnection,
    order: &RechargeOrderSummary,
) -> ApiResult<()> {
    sqlx::query(
        "INSERT INTO recharge_orders
         (id, user_id, username, channel, amount_minor, status, pay_type,
          provider_trade_no, payment_url, support_conversation_id, remark, created_at, paid_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
         ON CONFLICT (id) DO UPDATE SET
            user_id = EXCLUDED.user_id,
            username = EXCLUDED.username,
            channel = EXCLUDED.channel,
            amount_minor = EXCLUDED.amount_minor,
            status = EXCLUDED.status,
            pay_type = EXCLUDED.pay_type,
            provider_trade_no = EXCLUDED.provider_trade_no,
            payment_url = EXCLUDED.payment_url,
            support_conversation_id = EXCLUDED.support_conversation_id,
            remark = EXCLUDED.remark,
            created_at = EXCLUDED.created_at,
            paid_at = EXCLUDED.paid_at",
    )
    .bind(&order.id)
    .bind(&order.user_id)
    .bind(&order.username)
    .bind(enum_to_string(&order.channel)?)
    .bind(order.amount_minor)
    .bind(enum_to_string(&order.status)?)
    .bind(&order.pay_type)
    .bind(&order.provider_trade_no)
    .bind(&order.payment_url)
    .bind(&order.support_conversation_id)
    .bind(&order.remark)
    .bind(&order.created_at)
    .bind(&order.paid_at)
    .execute(&mut *connection)
    .await
    .map_err(|_| ApiError::Internal("充值订单数据保存失败".to_string()))?;
    Ok(())
}

/// 归一化可选筛选值，空字符串按未设置处理。
fn normalized_optional_filter(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}
/// 校验充值金额是否在后台配置范围内。
fn validate_amount(amount_minor: i64, settings: &RechargeSettings) -> ApiResult<()> {
    if amount_minor < settings.min_amount_minor {
        return Err(ApiError::BadRequest("充值金额低于后台最小限制".to_string()));
    }
    if amount_minor > settings.max_amount_minor {
        return Err(ApiError::BadRequest("充值金额超过后台最大限制".to_string()));
    }
    Ok(())
}
/// 校验彩虹易支付配置是否完整且已启用。
fn validate_rainbow_settings(settings: &RechargeSettings) -> ApiResult<()> {
    if !settings.rainbow_enabled {
        return Err(ApiError::BadRequest("彩虹易支付未开启".to_string()));
    }
    if is_unconfigured_value(&settings.rainbow_gateway_url)
        || settings.rainbow_gateway_url.contains("example.com")
        || is_unconfigured_value(&settings.rainbow_pid)
        || is_unconfigured_value(&settings.rainbow_key)
    {
        return Err(ApiError::BadRequest(
            "彩虹易支付网关、商户号或密钥未配置".to_string(),
        ));
    }
    Ok(())
}
/// 生成彩虹易支付收银台跳转地址。
fn rainbow_payment_url(
    settings: &RechargeSettings,
    order_id: &str,
    amount_minor: i64,
    pay_type: &str,
) -> ApiResult<String> {
    let notify_url = url_or_default(&settings.rainbow_notify_url, DEFAULT_NOTIFY_PATH);
    let return_url = url_or_default(&settings.rainbow_return_url, DEFAULT_RETURN_PATH);
    let money = minor_to_money(amount_minor);
    let mut params = BTreeMap::new();
    params.insert("money".to_string(), money);
    params.insert("name".to_string(), format!("用户充值 {order_id}"));
    params.insert("notify_url".to_string(), notify_url);
    params.insert("out_trade_no".to_string(), order_id.to_string());
    params.insert("pid".to_string(), settings.rainbow_pid.clone());
    params.insert("return_url".to_string(), return_url);
    params.insert("type".to_string(), pay_type.to_string());
    let sign = rainbow_sign(&params, &settings.rainbow_key);
    params.insert("sign".to_string(), sign);
    params.insert("sign_type".to_string(), "MD5".to_string());

    let base = settings.rainbow_gateway_url.trim().trim_end_matches('/');
    let query = params
        .iter()
        .map(|(key, value)| format!("{}={}", encode(key), encode(value)))
        .collect::<Vec<_>>()
        .join("&");
    Ok(format!("{base}/submit.php?{query}"))
}
/// 校验彩虹易支付回调签名。
fn verify_rainbow_sign(params: &HashMap<String, String>, key: &str) -> ApiResult<()> {
    let provided_sign = params
        .get("sign")
        .map(|value| value.trim().to_ascii_lowercase())
        .ok_or_else(|| ApiError::BadRequest("彩虹易支付通知缺少签名".to_string()))?;
    let mut sorted = BTreeMap::new();
    for (name, value) in params {
        if name == "sign" || name == "sign_type" || value.trim().is_empty() {
            continue;
        }
        sorted.insert(name.clone(), value.clone());
    }
    let expected = rainbow_sign(&sorted, key).to_ascii_lowercase();
    if expected != provided_sign {
        return Err(ApiError::BadRequest("彩虹易支付通知签名无效".to_string()));
    }
    Ok(())
}
/// 按彩虹易支付规则生成 MD5 签名。
fn rainbow_sign(params: &BTreeMap<String, String>, key: &str) -> String {
    let query = params
        .iter()
        .filter(|(name, value)| {
            name.as_str() != "sign" && name.as_str() != "sign_type" && !value.trim().is_empty()
        })
        .map(|(name, value)| format!("{name}={value}"))
        .collect::<Vec<_>>()
        .join("&");
    format!("{:x}", md5::compute(format!("{query}{key}")))
}
/// 把支付平台元金额字符串转换为分。
fn money_to_minor(value: &str) -> ApiResult<i64> {
    let value = value.trim();
    let (yuan, cent) = value.split_once('.').unwrap_or((value, "0"));
    let yuan_minor = yuan
        .parse::<i64>()
        .map_err(|_| ApiError::BadRequest("支付金额格式无效".to_string()))?
        .checked_mul(100)
        .ok_or_else(|| ApiError::BadRequest("支付金额过大".to_string()))?;
    let cent = format!("{:0<2}", cent.chars().take(2).collect::<String>());
    let cent_minor = cent
        .parse::<i64>()
        .map_err(|_| ApiError::BadRequest("支付金额格式无效".to_string()))?;
    yuan_minor
        .checked_add(cent_minor)
        .ok_or_else(|| ApiError::BadRequest("支付金额过大".to_string()))
}
/// 把分金额格式化为支付平台元金额字符串。
fn minor_to_money(amount_minor: i64) -> String {
    format!("{}.{:02}", amount_minor / 100, amount_minor.abs() % 100)
}
/// 读取配置 URL，未配置时使用默认路径。
fn url_or_default(value: &str, fallback: &str) -> String {
    let value = value.trim();
    if value.is_empty() {
        fallback.to_string()
    } else {
        value.to_string()
    }
}
/// 判断配置值是否仍是未配置占位内容。
fn is_unconfigured_value(value: &str) -> bool {
    let value = value.trim();
    value.is_empty() || matches!(value, "未配置" | "请配置" | "please-configure")
}
/// 从系统设置读取布尔配置。
fn bool_setting(map: &HashMap<&str, &str>, key: &str, fallback: bool) -> bool {
    map.get(key)
        .map(|value| matches!(value.trim(), "true" | "1" | "yes" | "on"))
        .unwrap_or(fallback)
}
/// 从系统设置读取字符串配置。
fn string_setting(map: &HashMap<&str, &str>, key: &str, fallback: &str) -> String {
    map.get(key)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| fallback.to_string())
}
/// 从系统设置读取逗号分隔配置。
fn csv_setting(map: &HashMap<&str, &str>, key: &str) -> Vec<String> {
    map.get(key)
        .map(|value| {
            value
                .split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_else(|| vec!["alipay".to_string(), "wxpay".to_string()])
}

/// 从系统设置读取充值赠送档位，并过滤掉无效或重复的金额配置。
fn recharge_bonus_rules_setting(map: &HashMap<&str, &str>, key: &str) -> Vec<RechargeBonusRule> {
    let value = map
        .get(key)
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .unwrap_or(DEFAULT_RECHARGE_BONUS_RULES);
    let mut rules = serde_json::from_str::<Vec<RechargeBonusRule>>(value)
        .unwrap_or_default()
        .into_iter()
        .filter(|rule| rule.threshold_amount_minor > 0 && rule.bonus_amount_minor > 0)
        .collect::<Vec<_>>();
    rules.sort_by(|left, right| {
        left.threshold_amount_minor
            .cmp(&right.threshold_amount_minor)
            .then_with(|| right.bonus_amount_minor.cmp(&left.bonus_amount_minor))
    });
    rules.dedup_by_key(|rule| rule.threshold_amount_minor);
    rules
}

/// 从系统设置读取正整数配置。
fn i64_setting(map: &HashMap<&str, &str>, key: &str, fallback: i64) -> i64 {
    map.get(key)
        .and_then(|value| value.trim().parse::<i64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(fallback)
}
/// 清洗必填字符串，空值时返回接口错误。
fn required_trimmed(value: &str, label: &str) -> ApiResult<String> {
    let value = value.trim().to_string();
    if value.is_empty() {
        return Err(ApiError::BadRequest(format!("{label} is required")));
    }
    Ok(value)
}
/// 生成当前本地时间字符串。
fn current_time_label() -> String {
    Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::finance::LedgerEntryKind,
        domain::user::{UserKind, UserStatus},
        services::finance::FinanceRepository,
    };
    /// 验证彩虹易支付签名只使用排序后的非空参数。
    #[test]
    fn rainbow_sign_uses_sorted_non_empty_params() {
        let mut params = BTreeMap::new();
        params.insert("pid".to_string(), "1001".to_string());
        params.insert("type".to_string(), "alipay".to_string());
        params.insert("out_trade_no".to_string(), "R0001".to_string());
        params.insert("money".to_string(), "10.00".to_string());
        params.insert("name".to_string(), "充值".to_string());

        let sign = rainbow_sign(&params, "secret");

        assert_eq!(sign.len(), 32);
        assert_eq!(sign, rainbow_sign(&params, "secret"));
    }
    /// 验证客服直充订单创建流程。
    #[test]
    fn recharge_store_creates_customer_service_order() {
        let mut store = RechargeStore::default();
        let user = user();
        let settings = RechargeSettings {
            rainbow_enabled: false,
            rainbow_gateway_url: String::new(),
            rainbow_pid: String::new(),
            rainbow_key: String::new(),
            rainbow_notify_url: String::new(),
            rainbow_return_url: String::new(),
            rainbow_pay_types: vec!["alipay".to_string()],
            customer_service_enabled: true,
            customer_service_message: "联系客服充值".to_string(),
            min_amount_minor: 100,
            max_amount_minor: 10_000,
            bonus_enabled: false,
            bonus_rules: Vec::new(),
        };

        let response = store
            .create_order(
                &user,
                CreateRechargeOrderRequest {
                    channel: RechargeChannel::CustomerService,
                    amount_minor: 1000,
                    pay_type: None,
                },
                &settings,
            )
            .expect("customer service recharge order can be created");

        assert_eq!(
            response.order.status,
            RechargeOrderStatus::WaitingCustomerService
        );
        assert!(response.support_conversation_id.is_some());
    }
    /// 验证彩虹易支付订单会生成支付地址。
    #[test]
    fn recharge_store_creates_rainbow_payment_url() {
        let mut store = RechargeStore::default();
        let user = user();
        let settings = RechargeSettings {
            rainbow_enabled: true,
            rainbow_gateway_url: "https://pay.example.test".to_string(),
            rainbow_pid: "1001".to_string(),
            rainbow_key: "secret".to_string(),
            rainbow_notify_url: "https://example.test/notify".to_string(),
            rainbow_return_url: "https://example.test/return".to_string(),
            rainbow_pay_types: vec!["alipay".to_string()],
            customer_service_enabled: false,
            customer_service_message: String::new(),
            min_amount_minor: 100,
            max_amount_minor: 10_000,
            bonus_enabled: false,
            bonus_rules: Vec::new(),
        };

        let response = store
            .create_order(
                &user,
                CreateRechargeOrderRequest {
                    channel: RechargeChannel::RainbowEpay,
                    amount_minor: 1234,
                    pay_type: Some("alipay".to_string()),
                },
                &settings,
            )
            .expect("rainbow recharge order can be created");

        let payment_url = response.payment_url.expect("payment url exists");
        assert!(payment_url.starts_with("https://pay.example.test/submit.php?"));
        assert!(payment_url.contains("money=12.34"));
        assert!(payment_url.contains("sign_type=MD5"));
    }
    /// 验证支付类型为空时自动关闭彩虹易支付。
    #[test]
    fn recharge_config_disables_rainbow_when_pay_types_are_empty() {
        let settings = RechargeSettings {
            rainbow_enabled: true,
            rainbow_gateway_url: "https://pay.example.test".to_string(),
            rainbow_pid: "1001".to_string(),
            rainbow_key: "secret".to_string(),
            rainbow_notify_url: String::new(),
            rainbow_return_url: String::new(),
            rainbow_pay_types: Vec::new(),
            customer_service_enabled: true,
            customer_service_message: "联系客服充值".to_string(),
            min_amount_minor: 100,
            max_amount_minor: 10_000,
            bonus_enabled: false,
            bonus_rules: Vec::new(),
        };

        let response = recharge_config_response(&settings);
        let rainbow = response
            .channels
            .iter()
            .find(|channel| channel.channel == RechargeChannel::RainbowEpay)
            .expect("rainbow channel exists");

        assert!(!rainbow.enabled);
        assert!(rainbow.pay_types.is_empty());
    }
    /// 验证支付类型为空时拒绝彩虹易支付下单。
    #[test]
    fn recharge_store_rejects_rainbow_when_pay_types_are_empty() {
        let mut store = RechargeStore::default();
        let user = user();
        let settings = RechargeSettings {
            rainbow_enabled: true,
            rainbow_gateway_url: "https://pay.example.test".to_string(),
            rainbow_pid: "1001".to_string(),
            rainbow_key: "secret".to_string(),
            rainbow_notify_url: "https://example.test/notify".to_string(),
            rainbow_return_url: "https://example.test/return".to_string(),
            rainbow_pay_types: Vec::new(),
            customer_service_enabled: false,
            customer_service_message: String::new(),
            min_amount_minor: 100,
            max_amount_minor: 10_000,
            bonus_enabled: false,
            bonus_rules: Vec::new(),
        };

        let result = store.create_order(
            &user,
            CreateRechargeOrderRequest {
                channel: RechargeChannel::RainbowEpay,
                amount_minor: 1234,
                pay_type: None,
            },
            &settings,
        );

        assert!(
            matches!(result, Err(ApiError::BadRequest(message)) if message == "彩虹易支付未开启任何支付方式")
        );
    }

    #[test]
    /// 清理充值记录只删除历史订单，并保留流水号避免后续充值单号重复。
    fn recharge_store_clear_records_keeps_next_sequence() {
        let mut store = RechargeStore::default();
        let user = user();
        let settings = RechargeSettings {
            rainbow_enabled: false,
            rainbow_gateway_url: String::new(),
            rainbow_pid: String::new(),
            rainbow_key: String::new(),
            rainbow_notify_url: String::new(),
            rainbow_return_url: String::new(),
            rainbow_pay_types: Vec::new(),
            customer_service_enabled: true,
            customer_service_message: "联系客服充值".to_string(),
            min_amount_minor: 100,
            max_amount_minor: 10_000,
            bonus_enabled: false,
            bonus_rules: Vec::new(),
        };
        let first = store
            .create_order(
                &user,
                CreateRechargeOrderRequest {
                    channel: RechargeChannel::CustomerService,
                    amount_minor: 1000,
                    pay_type: None,
                },
                &settings,
            )
            .expect("first order can be created");

        assert_eq!(first.order.id, "R000000000001");
        assert_eq!(store.clear_records(), 1);
        assert!(store.list().is_empty());

        let second = store
            .create_order(
                &user,
                CreateRechargeOrderRequest {
                    channel: RechargeChannel::CustomerService,
                    amount_minor: 1200,
                    pay_type: None,
                },
                &settings,
            )
            .expect("second order can be created");
        assert_eq!(second.order.id, "R000000000002");
    }
    /// 验证客服直充订单确认入账具备幂等性。
    #[tokio::test]
    async fn recharge_repository_confirms_customer_service_order_once() {
        let repository = RechargeRepository::memory();
        let finance = FinanceRepository::memory_seeded();
        let user = user();
        let settings = RechargeSettings {
            rainbow_enabled: false,
            rainbow_gateway_url: String::new(),
            rainbow_pid: String::new(),
            rainbow_key: String::new(),
            rainbow_notify_url: String::new(),
            rainbow_return_url: String::new(),
            rainbow_pay_types: vec!["alipay".to_string()],
            customer_service_enabled: true,
            customer_service_message: "联系客服充值".to_string(),
            min_amount_minor: 100,
            max_amount_minor: 10_000,
            bonus_enabled: false,
            bonus_rules: Vec::new(),
        };
        let created = repository
            .create_order(
                &user,
                CreateRechargeOrderRequest {
                    channel: RechargeChannel::CustomerService,
                    amount_minor: 1200,
                    pay_type: None,
                },
                &settings,
            )
            .await
            .expect("customer service order can be created");

        let confirmed = repository
            .confirm_customer_service_order(
                &created.order.id,
                ConfirmRechargeOrderRequest {
                    provider_trade_no: Some("客服收款凭证".to_string()),
                    remark: Some("线下已核对收款截图".to_string()),
                },
                &settings,
                &finance,
            )
            .await
            .expect("customer service order can be confirmed");
        let confirmed_again = repository
            .confirm_customer_service_order(
                &created.order.id,
                ConfirmRechargeOrderRequest {
                    provider_trade_no: None,
                    remark: None,
                },
                &settings,
                &finance,
            )
            .await
            .expect("confirm is idempotent");

        let entries = finance
            .user_ledger_entries(&user.id)
            .await
            .expect("ledger entries can load");
        let account = finance
            .account_or_create(&user.id)
            .await
            .expect("account can load");

        assert_eq!(confirmed.status, RechargeOrderStatus::Paid);
        assert_eq!(confirmed.remark, "线下已核对收款截图");
        assert_eq!(confirmed_again.status, RechargeOrderStatus::Paid);
        assert_eq!(entries.len(), 1);
        assert_eq!(account.available_balance_minor, 1200);
    }

    /// 验证首次确认充值时，本金和代理返利在同一个资金事务里一起入账。
    #[tokio::test]
    async fn recharge_repository_confirms_customer_service_order_with_rebate() {
        let repository = RechargeRepository::memory();
        let finance = FinanceRepository::memory_seeded();
        let user = user();
        let settings = RechargeSettings {
            rainbow_enabled: false,
            rainbow_gateway_url: String::new(),
            rainbow_pid: String::new(),
            rainbow_key: String::new(),
            rainbow_notify_url: String::new(),
            rainbow_return_url: String::new(),
            rainbow_pay_types: vec!["alipay".to_string()],
            customer_service_enabled: true,
            customer_service_message: "联系客服充值".to_string(),
            min_amount_minor: 100,
            max_amount_minor: 10_000,
            bonus_enabled: false,
            bonus_rules: Vec::new(),
        };
        let created = repository
            .create_order(
                &user,
                CreateRechargeOrderRequest {
                    channel: RechargeChannel::CustomerService,
                    amount_minor: 1200,
                    pay_type: None,
                },
                &settings,
            )
            .await
            .expect("customer service order can be created");
        let rebate_credit = RechargeRebateCredit {
            agent_user_id: "U90001".to_string(),
            invitee_user_id: user.id.clone(),
            amount_minor: 350,
        };

        let result = repository
            .confirm_customer_service_order_with_rebate(
                &created.order.id,
                ConfirmRechargeOrderRequest {
                    provider_trade_no: Some("客服收款凭证".to_string()),
                    remark: None,
                },
                &settings,
                &finance,
                Some(&rebate_credit),
            )
            .await
            .expect("customer service order can be confirmed with rebate");

        let user_account = finance
            .account_or_create(&user.id)
            .await
            .expect("user account can load");
        let agent_account = finance
            .account_or_create("U90001")
            .await
            .expect("agent account can load");
        let rebate_entry = result.rebate_entry.expect("rebate entry exists");

        assert_eq!(result.order.status, RechargeOrderStatus::Paid);
        assert_eq!(user_account.available_balance_minor, 1200);
        assert_eq!(agent_account.available_balance_minor, 520_350);
        assert_eq!(rebate_entry.kind, LedgerEntryKind::RechargeRebateCredit);
        assert_eq!(rebate_entry.amount_minor, 350);
    }

    /// 验证已入账订单重复确认时不会只给代理返利，避免本金缺失但返利增加。
    #[tokio::test]
    async fn recharge_repository_does_not_rebate_when_paid_order_is_reconfirmed() {
        let repository = RechargeRepository::memory();
        let finance = FinanceRepository::memory_seeded();
        let user = user();
        let settings = RechargeSettings {
            rainbow_enabled: false,
            rainbow_gateway_url: String::new(),
            rainbow_pid: String::new(),
            rainbow_key: String::new(),
            rainbow_notify_url: String::new(),
            rainbow_return_url: String::new(),
            rainbow_pay_types: vec!["alipay".to_string()],
            customer_service_enabled: true,
            customer_service_message: "联系客服充值".to_string(),
            min_amount_minor: 100,
            max_amount_minor: 10_000,
            bonus_enabled: false,
            bonus_rules: Vec::new(),
        };
        let created = repository
            .create_order(
                &user,
                CreateRechargeOrderRequest {
                    channel: RechargeChannel::CustomerService,
                    amount_minor: 1200,
                    pay_type: None,
                },
                &settings,
            )
            .await
            .expect("customer service order can be created");
        {
            let mut store = repository
                .inner
                .write()
                .expect("recharge store lock should be available");
            let order = store
                .orders
                .get_mut(&created.order.id)
                .expect("created order exists");
            order.status = RechargeOrderStatus::Paid;
            order.paid_at = Some("2026-06-05 12:00:00".to_string());
        }
        let rebate_credit = RechargeRebateCredit {
            agent_user_id: "U90001".to_string(),
            invitee_user_id: user.id.clone(),
            amount_minor: 350,
        };

        let result = repository
            .confirm_customer_service_order_with_rebate(
                &created.order.id,
                ConfirmRechargeOrderRequest {
                    provider_trade_no: None,
                    remark: None,
                },
                &settings,
                &finance,
                Some(&rebate_credit),
            )
            .await
            .expect("paid order reconfirm remains idempotent");

        let user_account = finance
            .account_or_create(&user.id)
            .await
            .expect("user account can load");
        let agent_account = finance
            .account_or_create("U90001")
            .await
            .expect("agent account can load");

        assert!(result.rebate_entry.is_none());
        assert_eq!(user_account.available_balance_minor, 0);
        assert_eq!(agent_account.available_balance_minor, 520_000);
    }

    /// 验证充值赠送活动按最高命中档位入账，并且重复确认不会重复赠送。
    #[tokio::test]
    async fn recharge_repository_applies_bonus_rule_once() {
        let repository = RechargeRepository::memory();
        let finance = FinanceRepository::memory_seeded();
        let user = user();
        let settings = RechargeSettings {
            rainbow_enabled: false,
            rainbow_gateway_url: String::new(),
            rainbow_pid: String::new(),
            rainbow_key: String::new(),
            rainbow_notify_url: String::new(),
            rainbow_return_url: String::new(),
            rainbow_pay_types: vec!["alipay".to_string()],
            customer_service_enabled: true,
            customer_service_message: "联系客服充值".to_string(),
            min_amount_minor: 100,
            max_amount_minor: 100_000,
            bonus_enabled: true,
            bonus_rules: vec![
                RechargeBonusRule {
                    threshold_amount_minor: 10_000,
                    bonus_amount_minor: 500,
                },
                RechargeBonusRule {
                    threshold_amount_minor: 50_000,
                    bonus_amount_minor: 4_000,
                },
            ],
        };
        let created = repository
            .create_order(
                &user,
                CreateRechargeOrderRequest {
                    channel: RechargeChannel::CustomerService,
                    amount_minor: 10_000,
                    pay_type: None,
                },
                &settings,
            )
            .await
            .expect("customer service order can be created");

        repository
            .confirm_customer_service_order(
                &created.order.id,
                ConfirmRechargeOrderRequest {
                    provider_trade_no: Some("客服收款凭证".to_string()),
                    remark: None,
                },
                &settings,
                &finance,
            )
            .await
            .expect("first confirm should credit recharge and bonus");
        repository
            .confirm_customer_service_order(
                &created.order.id,
                ConfirmRechargeOrderRequest {
                    provider_trade_no: None,
                    remark: None,
                },
                &settings,
                &finance,
            )
            .await
            .expect("second confirm should stay idempotent");

        let entries = finance
            .user_ledger_entries(&user.id)
            .await
            .expect("ledger entries can load");
        let account = finance
            .account_or_create(&user.id)
            .await
            .expect("account can load");

        assert_eq!(entries.len(), 2);
        assert!(entries
            .iter()
            .any(|entry| entry.kind == LedgerEntryKind::RechargeCredit
                && entry.amount_minor == 10_000));
        assert!(entries
            .iter()
            .any(|entry| entry.kind == LedgerEntryKind::RechargeBonusCredit
                && entry.amount_minor == 500));
        assert_eq!(account.available_balance_minor, 10_500);
    }

    /// 构造充值测试用户。
    fn user() -> UserSummary {
        UserSummary {
            id: "U-RECHARGE".to_string(),
            username: "demo_user".to_string(),
            email: None,
            avatar_url: String::new(),
            contact_qq: String::new(),
            kind: UserKind::Regular,
            status: UserStatus::Active,
            balance_minor: 0,
            agent_id: None,
            invite_code: "ABC12345".to_string(),
            registration_location: crate::domain::user::UserRegistrationLocation::default(),
            created_at: "2026-06-05 10:00:00".to_string(),
        }
    }
}
