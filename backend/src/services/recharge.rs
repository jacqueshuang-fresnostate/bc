//! 充值服务，管理彩虹易支付与客服直充订单

use std::{
    collections::{BTreeMap, HashMap},
    sync::{Arc, RwLock},
};

use chrono::Local;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use urlencoding::encode;

use crate::{
    domain::{
        permission::SystemSetting,
        recharge::{
            ConfirmRechargeOrderRequest, CreateRechargeOrderRequest, CreateRechargeOrderResponse,
            RechargeChannel, RechargeChannelConfig, RechargeConfigResponse, RechargeOrderStatus,
            RechargeOrderSummary,
        },
        user::UserSummary,
    },
    error::{ApiError, ApiResult},
    services::{business_database::BusinessDatabase, finance::FinanceRepository},
};

use super::business_database::{enum_from_string, enum_to_string};

const DEFAULT_GATEWAY_URL: &str = "https://pay.example.com";
const DEFAULT_NOTIFY_PATH: &str = "/api/user/recharge/epay/notify";
const DEFAULT_RETURN_PATH: &str = "/api/user/recharge/epay/return";
const DEFAULT_MIN_AMOUNT_MINOR: i64 = 100;
const DEFAULT_MAX_AMOUNT_MINOR: i64 = 10_000_000;

#[derive(Clone)]
pub struct RechargeRepository {
    inner: Arc<RwLock<RechargeStore>>,
    persistence: Option<BusinessDatabase>,
}

#[derive(Debug, Clone)]
pub struct RechargeSettings {
    pub rainbow_enabled: bool,
    pub rainbow_gateway_url: String,
    pub rainbow_pid: String,
    pub rainbow_key: String,
    pub rainbow_notify_url: String,
    pub rainbow_return_url: String,
    pub rainbow_pay_types: Vec<String>,
    pub customer_service_enabled: bool,
    pub customer_service_message: String,
    pub min_amount_minor: i64,
    pub max_amount_minor: i64,
}

#[derive(Debug, Clone)]
pub struct RechargeSupportTicket {
    pub conversation_id: String,
    pub subject: String,
    pub content: String,
}

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

    /// 返回指定用户充值订单。
    pub async fn list_for_user(&self, user_id: &str) -> ApiResult<Vec<RechargeOrderSummary>> {
        let user_id = required_trimmed(user_id, "user id")?;
        Ok(self
            .inner
            .read()
            .map_err(|_| ApiError::Internal("recharge store lock poisoned".to_string()))?
            .list_for_user(&user_id))
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

    /// 处理彩虹易支付异步通知，验签成功且状态成功时给用户入账。
    pub async fn confirm_rainbow_notify(
        &self,
        params: HashMap<String, String>,
        settings: &RechargeSettings,
        finance: &FinanceRepository,
    ) -> ApiResult<RechargeOrderSummary> {
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

        let (order, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("recharge store lock poisoned".to_string()))?;
            let order = store.mark_paid(order_id, paid_amount_minor, trade_no)?;
            (order, store.clone())
        };

        finance
            .credit_recharge(&order.user_id, order.amount_minor, &order.id)
            .await?;
        self.persist(&snapshot).await?;
        Ok(order)
    }

    /// 后台确认客服直充已收款，并给用户余额入账。
    pub async fn confirm_customer_service_order(
        &self,
        order_id: &str,
        request: ConfirmRechargeOrderRequest,
        finance: &FinanceRepository,
    ) -> ApiResult<RechargeOrderSummary> {
        let (order, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("recharge store lock poisoned".to_string()))?;
            let order = store.confirm_customer_service_order(order_id, request)?;
            (order, store.clone())
        };

        finance
            .credit_recharge(&order.user_id, order.amount_minor, &order.id)
            .await?;
        self.persist(&snapshot).await?;
        Ok(order)
    }

    async fn persist(&self, store: &RechargeStore) -> ApiResult<()> {
        if let Some(persistence) = &self.persistence {
            save_recharge_store(persistence, store).await?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct RechargeStore {
    orders: BTreeMap<String, RechargeOrderSummary>,
    next_sequence: u64,
}

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
                let pay_type = request
                    .pay_type
                    .map(|value| value.trim().to_string())
                    .filter(|value| !value.is_empty())
                    .unwrap_or_else(|| {
                        settings
                            .rainbow_pay_types
                            .first()
                            .cloned()
                            .unwrap_or_else(|| "alipay".to_string())
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
    fn mark_paid(
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
    fn confirm_customer_service_order(
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
        order.paid_at = Some(current_time_label());
        Ok(order.clone())
    }
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
    }
}

/// 将后台充值配置转换成用户端可见的渠道说明。
pub fn recharge_config_response(settings: &RechargeSettings) -> RechargeConfigResponse {
    RechargeConfigResponse {
        channels: vec![
            RechargeChannelConfig {
                channel: RechargeChannel::RainbowEpay,
                name: "彩虹易支付".to_string(),
                enabled: settings.rainbow_enabled,
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

async fn load_recharge_store(database: &BusinessDatabase) -> ApiResult<RechargeStore> {
    let pool = database.pool();
    let mut orders = BTreeMap::new();
    for row in sqlx::query(
        "SELECT id, user_id, username, channel, amount_minor, status, pay_type,
                provider_trade_no, payment_url, support_conversation_id, created_at, paid_at
         FROM recharge_orders
         ORDER BY id ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::Internal("充值订单数据读取失败".to_string()))?
    {
        let id: String = row
            .try_get("id")
            .map_err(|_| ApiError::Internal("充值订单数据读取失败".to_string()))?;
        orders.insert(
            id.clone(),
            RechargeOrderSummary {
                id,
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
                created_at: row
                    .try_get("created_at")
                    .map_err(|_| ApiError::Internal("充值订单数据读取失败".to_string()))?,
                paid_at: row
                    .try_get("paid_at")
                    .map_err(|_| ApiError::Internal("充值订单数据读取失败".to_string()))?,
            },
        );
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

async fn save_recharge_store(database: &BusinessDatabase, store: &RechargeStore) -> ApiResult<()> {
    let mut tx = database
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::Internal("充值事务开启失败".to_string()))?;

    for table in ["recharge_orders", "recharge_runtime"] {
        sqlx::query(&format!("DELETE FROM {table}"))
            .execute(&mut *tx)
            .await
            .map_err(|_| ApiError::Internal("充值数据清理失败".to_string()))?;
    }

    for order in store.orders.values() {
        sqlx::query(
            "INSERT INTO recharge_orders
             (id, user_id, username, channel, amount_minor, status, pay_type,
              provider_trade_no, payment_url, support_conversation_id, created_at, paid_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
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
        .bind(&order.created_at)
        .bind(&order.paid_at)
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::Internal("充值订单数据保存失败".to_string()))?;
    }

    let next_sequence = i64::try_from(store.next_sequence)
        .map_err(|_| ApiError::Internal("充值序号过大".to_string()))?;
    sqlx::query("INSERT INTO recharge_runtime (key, value) VALUES ('next_sequence', $1)")
        .bind(next_sequence)
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::Internal("充值运行数据保存失败".to_string()))?;

    tx.commit()
        .await
        .map_err(|_| ApiError::Internal("充值事务提交失败".to_string()))
}

fn validate_amount(amount_minor: i64, settings: &RechargeSettings) -> ApiResult<()> {
    if amount_minor < settings.min_amount_minor {
        return Err(ApiError::BadRequest("充值金额低于后台最小限制".to_string()));
    }
    if amount_minor > settings.max_amount_minor {
        return Err(ApiError::BadRequest("充值金额超过后台最大限制".to_string()));
    }
    Ok(())
}

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

fn minor_to_money(amount_minor: i64) -> String {
    format!("{}.{:02}", amount_minor / 100, amount_minor.abs() % 100)
}

fn url_or_default(value: &str, fallback: &str) -> String {
    let value = value.trim();
    if value.is_empty() {
        fallback.to_string()
    } else {
        value.to_string()
    }
}

fn is_unconfigured_value(value: &str) -> bool {
    let value = value.trim();
    value.is_empty() || matches!(value, "未配置" | "请配置" | "please-configure")
}

fn bool_setting(map: &HashMap<&str, &str>, key: &str, fallback: bool) -> bool {
    map.get(key)
        .map(|value| matches!(value.trim(), "true" | "1" | "yes" | "on"))
        .unwrap_or(fallback)
}

fn string_setting(map: &HashMap<&str, &str>, key: &str, fallback: &str) -> String {
    map.get(key)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| fallback.to_string())
}

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

fn i64_setting(map: &HashMap<&str, &str>, key: &str, fallback: i64) -> i64 {
    map.get(key)
        .and_then(|value| value.trim().parse::<i64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(fallback)
}

fn required_trimmed(value: &str, label: &str) -> ApiResult<String> {
    let value = value.trim().to_string();
    if value.is_empty() {
        return Err(ApiError::BadRequest(format!("{label} is required")));
    }
    Ok(value)
}

fn current_time_label() -> String {
    Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::user::{UserKind, UserStatus},
        services::finance::FinanceRepository,
    };

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
                },
                &finance,
            )
            .await
            .expect("customer service order can be confirmed");
        let confirmed_again = repository
            .confirm_customer_service_order(
                &created.order.id,
                ConfirmRechargeOrderRequest {
                    provider_trade_no: None,
                },
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
        assert_eq!(confirmed_again.status, RechargeOrderStatus::Paid);
        assert_eq!(entries.len(), 1);
        assert_eq!(account.available_balance_minor, 1200);
    }

    fn user() -> UserSummary {
        UserSummary {
            id: "U-RECHARGE".to_string(),
            username: "demo_user".to_string(),
            email: None,
            kind: UserKind::Regular,
            status: UserStatus::Active,
            balance_minor: 0,
            agent_id: None,
            invite_code: "ABC12345".to_string(),
        }
    }
}
