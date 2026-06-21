//! 手机端公共聊天大厅仓储，负责消息校验、历史列表和数据库持久化。

use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    sync::{Arc, RwLock},
};

use chrono::Local;
use serde_json::Value;
use sqlx::{PgConnection, Row};
use tokio::sync::Mutex;

use crate::{
    domain::{
        chat_hall::{
            ChatHallGroupBuyPlanPayload, ChatHallMessage, ChatHallMessageType, ChatHallRedPacket,
            ChatHallRedPacketClaim, ChatHallRedPacketClaimsResponse, ChatHallRedPacketPayload,
            ClaimChatHallRedPacketResponse, CreateChatHallMessageRequest,
            CreateChatHallRedPacketRequest,
        },
        user::UserSummary,
    },
    error::{ApiError, ApiResult},
};

use super::{
    business_database::{enum_from_string, enum_to_string, to_json, BusinessDatabase},
    finance::{save_finance_store_incremental_in_transaction, FinanceRepository},
};

const CHAT_HALL_HISTORY_LIMIT: usize = 200;
const CHAT_HALL_LIST_LIMIT: usize = 100;
const CHAT_HALL_MESSAGE_MAX_LENGTH: usize = 500;
const CHAT_HALL_RED_PACKET_GREETING_MAX_LENGTH: usize = 60;
const CHAT_HALL_RED_PACKET_MAX_CLAIM_COUNT: u32 = 100;

#[derive(Clone)]
/// 手机端公共聊天大厅仓储，负责该模块数据读取、业务变更和持久化协调。
pub struct ChatHallRepository {
    inner: Arc<RwLock<ChatHallStore>>,
    mutation_lock: Arc<Mutex<()>>,
    persistence: Option<BusinessDatabase>,
}

/// 手机端公共聊天大厅仓储，负责该模块数据读取、业务变更和持久化协调。
impl ChatHallRepository {
    /// 创建空的内存聊天大厅仓储，供未配置数据库的本地开发使用。
    pub fn memory() -> Self {
        Self {
            inner: Arc::new(RwLock::new(ChatHallStore::default())),
            mutation_lock: Arc::new(Mutex::new(())),
            persistence: None,
        }
    }

    /// 从数据库加载聊天大厅历史消息，并启用持久化保存。
    pub async fn persistent(persistence: BusinessDatabase) -> ApiResult<Self> {
        let store = load_chat_hall_store(&persistence).await?;
        Ok(Self {
            inner: Arc::new(RwLock::new(store)),
            mutation_lock: Arc::new(Mutex::new(())),
            persistence: Some(persistence),
        })
    }

    /// 返回最近的聊天大厅消息，按发送时间正序展示。
    pub async fn list(&self) -> ApiResult<Vec<ChatHallMessage>> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("聊天大厅数据锁读取失败".to_string()))
            .map(|store| store.list())
    }

    /// 一键清除聊天大厅历史消息和对应红包展示记录；资金流水不做回滚。
    pub async fn clear_messages(&self) -> ApiResult<usize> {
        let _mutation_guard = self.mutation_lock.lock().await;
        let mut snapshot = self.current_store_snapshot()?;
        let deleted_count = snapshot.clear_messages();
        if deleted_count > 0 {
            self.persist(&snapshot).await?;
            self.replace_store(snapshot)?;
        }

        Ok(deleted_count)
    }

    /// 发送一条聊天大厅消息，保存后由路由层负责广播实时事件。
    pub async fn send(
        &self,
        user: &UserSummary,
        request: CreateChatHallMessageRequest,
    ) -> ApiResult<ChatHallMessage> {
        let _mutation_guard = self.mutation_lock.lock().await;
        let mut snapshot = self.current_store_snapshot()?;
        let message = snapshot.send(user, request)?;
        self.persist(&snapshot).await?;
        self.replace_store(snapshot)?;

        Ok(message)
    }

    /// 发送聊天大厅红包：先扣减发送人余额，再保存红包消息和红包记录。
    pub async fn send_red_packet(
        &self,
        finance: &FinanceRepository,
        user: &UserSummary,
        request: CreateChatHallRedPacketRequest,
    ) -> ApiResult<ChatHallMessage> {
        let _mutation_guard = self.mutation_lock.lock().await;
        let mut chat_snapshot = self.current_store_snapshot()?;
        let previous_finance_store = finance
            .inner
            .read()
            .map_err(|_| ApiError::Internal("资金数据锁读取失败".to_string()))?
            .clone();
        let mut finance_snapshot = previous_finance_store.clone();

        let prepared = chat_snapshot.prepare_red_packet(user, request)?;
        finance_snapshot.debit_chat_red_packet(
            &user.id,
            prepared.red_packet.total_amount_minor,
            &prepared.red_packet.id,
        )?;

        let message = chat_snapshot.insert_prepared_red_packet(prepared)?;
        self.persist_with_finance(
            finance,
            &chat_snapshot,
            &previous_finance_store,
            &finance_snapshot,
        )
        .await?;
        self.replace_store(chat_snapshot)?;
        finance.replace_store(finance_snapshot)?;

        Ok(message)
    }

    /// 领取聊天大厅红包：每个用户只能领取一次，入账成功后更新红包消息进度。
    pub async fn claim_red_packet(
        &self,
        finance: &FinanceRepository,
        user: &UserSummary,
        red_packet_id: &str,
    ) -> ApiResult<ClaimChatHallRedPacketResponse> {
        let _mutation_guard = self.mutation_lock.lock().await;
        let mut chat_snapshot = self.current_store_snapshot()?;
        let previous_finance_store = finance
            .inner
            .read()
            .map_err(|_| ApiError::Internal("资金数据锁读取失败".to_string()))?
            .clone();
        let mut finance_snapshot = previous_finance_store.clone();

        let prepared = chat_snapshot.prepare_red_packet_claim(user, red_packet_id)?;
        finance_snapshot.credit_chat_red_packet(
            &user.id,
            prepared.claim.amount_minor,
            &prepared.claim.id,
            &prepared.claim.red_packet_id,
        )?;

        let (message, claim) = chat_snapshot.apply_prepared_red_packet_claim(prepared)?;
        let account = finance_snapshot.account_or_create(&user.id)?;
        self.persist_with_finance(
            finance,
            &chat_snapshot,
            &previous_finance_store,
            &finance_snapshot,
        )
        .await?;
        self.replace_store(chat_snapshot)?;
        finance.replace_store(finance_snapshot)?;

        Ok(ClaimChatHallRedPacketResponse {
            message,
            claim,
            available_balance_minor: account.available_balance_minor,
        })
    }

    /// 返回指定聊天大厅红包的领取记录，供手机端查看谁抢到了红包。
    pub async fn red_packet_claims(
        &self,
        red_packet_id: &str,
    ) -> ApiResult<ChatHallRedPacketClaimsResponse> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("聊天大厅数据锁读取失败".to_string()))?
            .red_packet_claims(red_packet_id)
    }

    /// 分享一条当前用户自己的合买计划摘要到聊天大厅。
    pub async fn share_group_buy_plan(
        &self,
        user: &UserSummary,
        payload: ChatHallGroupBuyPlanPayload,
    ) -> ApiResult<ChatHallMessage> {
        let _mutation_guard = self.mutation_lock.lock().await;
        let mut snapshot = self.current_store_snapshot()?;
        let message = snapshot.share_group_buy_plan(user, payload)?;
        self.persist(&snapshot).await?;
        self.replace_store(snapshot)?;

        Ok(message)
    }

    /// 用户头像变更后同步刷新大厅内该用户历史消息的头像快照。
    pub async fn update_user_avatar(&self, user_id: &str, avatar_url: &str) -> ApiResult<()> {
        let _mutation_guard = self.mutation_lock.lock().await;
        let mut snapshot = self.current_store_snapshot()?;
        if !snapshot.update_user_avatar(user_id, avatar_url) {
            return Ok(());
        }
        self.persist(&snapshot).await?;
        self.replace_store(snapshot)
    }

    /// 从数据库重新加载聊天大厅消息和红包快照，供后台缓存维护使用。
    pub async fn reload_from_database(&self) -> ApiResult<bool> {
        let Some(persistence) = &self.persistence else {
            return Ok(false);
        };
        let store = load_chat_hall_store(persistence).await?;
        *self
            .inner
            .write()
            .map_err(|_| ApiError::Internal("聊天大厅缓存刷新失败".to_string()))? = store;
        Ok(true)
    }

    /// PostgreSQL 模式下把当前最近消息快照写入数据库。
    async fn persist(&self, store: &ChatHallStore) -> ApiResult<()> {
        if let Some(persistence) = &self.persistence {
            save_chat_hall_store(persistence, store).await?;
        }

        Ok(())
    }

    /// PostgreSQL 模式下把聊天大厅和资金快照放入同一个事务，避免红包扣款和消息状态只落一边。
    async fn persist_with_finance(
        &self,
        finance: &FinanceRepository,
        chat_store: &ChatHallStore,
        previous_finance_store: &crate::services::finance::FinanceStore,
        finance_store: &crate::services::finance::FinanceStore,
    ) -> ApiResult<()> {
        match (&self.persistence, &finance.persistence) {
            (Some(persistence), Some(_)) => {
                let mut tx = persistence
                    .pool()
                    .begin()
                    .await
                    .map_err(|_| ApiError::Internal("聊天红包资金事务开启失败".to_string()))?;
                save_chat_hall_store_in_transaction(&mut *tx, chat_store).await?;
                save_finance_store_incremental_in_transaction(
                    &mut *tx,
                    previous_finance_store,
                    finance_store,
                )
                .await?;
                tx.commit()
                    .await
                    .map_err(|_| ApiError::Internal("聊天红包资金事务提交失败".to_string()))?;
            }
            (None, None) => {}
            _ => {
                return Err(ApiError::Internal(
                    "聊天大厅和资金持久化配置不一致".to_string(),
                ))
            }
        }

        Ok(())
    }

    /// 克隆当前聊天大厅快照，供写操作先在临时快照中完成校验和变更。
    fn current_store_snapshot(&self) -> ApiResult<ChatHallStore> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("聊天大厅数据锁读取失败".to_string()))
            .map(|store| store.clone())
    }

    /// 用事务提交后的快照替换聊天大厅内存状态。
    fn replace_store(&self, store: ChatHallStore) -> ApiResult<()> {
        *self
            .inner
            .write()
            .map_err(|_| ApiError::Internal("聊天大厅数据锁写入失败".to_string()))? = store;
        Ok(())
    }
}

#[derive(Clone, Debug, Default)]
/// 手机端公共聊天大厅运行时数据快照，用于内存模式和数据库持久化前的业务校验。
pub(crate) struct ChatHallStore {
    messages: VecDeque<ChatHallMessage>,
    red_packets: BTreeMap<String, ChatHallRedPacket>,
    next_sequence: u64,
    next_red_packet_sequence: u64,
    next_red_packet_claim_sequence: u64,
}

#[derive(Clone, Debug)]
struct PreparedRedPacket {
    message_id: String,
    avatar_url: String,
    red_packet: ChatHallRedPacket,
}

#[derive(Clone, Debug)]
struct PreparedRedPacketClaim {
    claim: ChatHallRedPacketClaim,
}

/// 手机端公共聊天大厅运行时数据快照，用于内存模式和数据库持久化前的业务校验。
impl ChatHallStore {
    /// 返回最近消息，超过展示上限时仅返回末尾一段历史。
    fn list(&self) -> Vec<ChatHallMessage> {
        let mut messages = self
            .messages
            .iter()
            .rev()
            .take(CHAT_HALL_LIST_LIMIT)
            .cloned()
            .collect::<Vec<_>>();
        messages.reverse();
        messages
    }

    /// 返回指定红包的领取进度和领取人列表，不修改红包或资金状态。
    fn red_packet_claims(&self, red_packet_id: &str) -> ApiResult<ChatHallRedPacketClaimsResponse> {
        let red_packet_id = red_packet_id.trim();
        if red_packet_id.is_empty() {
            return Err(ApiError::BadRequest("红包编号不能为空".to_string()));
        }
        let red_packet = self
            .red_packets
            .get(red_packet_id)
            .ok_or_else(|| ApiError::NotFound("红包不存在".to_string()))?;

        Ok(ChatHallRedPacketClaimsResponse {
            red_packet_id: red_packet.id.clone(),
            greeting: red_packet.greeting.clone(),
            total_amount_minor: red_packet.total_amount_minor,
            remaining_amount_minor: red_packet.remaining_amount_minor,
            claim_count: red_packet.claim_count,
            claimed_count: red_packet.claimed_count,
            claims: red_packet.claims.clone(),
        })
    }

    /// 清空全部大厅展示消息，保留下一条消息序号，避免后续消息编号复用。
    fn clear_messages(&mut self) -> usize {
        let deleted_count = self.messages.len();
        self.messages.clear();
        self.red_packets.clear();
        deleted_count
    }

    /// 校验用户输入并追加一条新的聊天消息。
    fn send(
        &mut self,
        user: &UserSummary,
        request: CreateChatHallMessageRequest,
    ) -> ApiResult<ChatHallMessage> {
        let content = normalize_chat_hall_content(request.content)?;
        self.next_sequence = self.next_sequence.saturating_add(1);
        let message = ChatHallMessage {
            id: chat_hall_message_id(self.next_sequence),
            user_id: user.id.clone(),
            username: user.username.clone(),
            avatar_url: user.avatar_url.clone(),
            content,
            message_type: ChatHallMessageType::Text,
            payload: None,
            created_at: current_time_label(),
        };
        self.messages.push_back(message.clone());
        self.trim_history();

        Ok(message)
    }

    /// 校验红包请求并生成待写入的红包和消息编号，不提前修改仓储状态。
    fn prepare_red_packet(
        &self,
        user: &UserSummary,
        request: CreateChatHallRedPacketRequest,
    ) -> ApiResult<PreparedRedPacket> {
        let greeting = normalize_red_packet_greeting(request.greeting)?;
        let claim_count = request.claim_count;
        if claim_count == 0 {
            return Err(ApiError::BadRequest("红包个数必须大于 0".to_string()));
        }
        if claim_count > CHAT_HALL_RED_PACKET_MAX_CLAIM_COUNT {
            return Err(ApiError::BadRequest(format!(
                "红包个数不能超过 {CHAT_HALL_RED_PACKET_MAX_CLAIM_COUNT} 个"
            )));
        }
        if request.amount_minor <= 0 {
            return Err(ApiError::BadRequest("红包金额必须大于 0".to_string()));
        }
        if request.amount_minor < i64::from(claim_count) {
            return Err(ApiError::BadRequest(
                "红包金额不能小于红包个数对应的最小金额".to_string(),
            ));
        }

        let red_packet_id = chat_hall_red_packet_id(self.next_red_packet_sequence + 1);
        let red_packet = ChatHallRedPacket {
            id: red_packet_id,
            user_id: user.id.clone(),
            username: user.username.clone(),
            total_amount_minor: request.amount_minor,
            remaining_amount_minor: request.amount_minor,
            claim_count,
            claimed_count: 0,
            greeting,
            claims: Vec::new(),
            created_at: current_time_label(),
        };

        Ok(PreparedRedPacket {
            message_id: chat_hall_message_id(self.next_sequence + 1),
            avatar_url: user.avatar_url.clone(),
            red_packet,
        })
    }

    /// 在资金扣款成功后写入红包和对应聊天消息。
    fn insert_prepared_red_packet(
        &mut self,
        prepared: PreparedRedPacket,
    ) -> ApiResult<ChatHallMessage> {
        self.next_sequence = self.next_sequence.saturating_add(1);
        self.next_red_packet_sequence = self.next_red_packet_sequence.saturating_add(1);
        let payload = red_packet_payload(&prepared.red_packet)?;
        let message = ChatHallMessage {
            id: prepared.message_id,
            user_id: prepared.red_packet.user_id.clone(),
            username: prepared.red_packet.username.clone(),
            avatar_url: prepared.avatar_url,
            content: prepared.red_packet.greeting.clone(),
            message_type: ChatHallMessageType::RedPacket,
            payload: Some(payload),
            created_at: prepared.red_packet.created_at.clone(),
        };
        self.red_packets
            .insert(prepared.red_packet.id.clone(), prepared.red_packet);
        self.messages.push_back(message.clone());
        self.trim_history();

        Ok(message)
    }

    /// 校验红包领取请求并生成待写入领取记录。
    fn prepare_red_packet_claim(
        &self,
        user: &UserSummary,
        red_packet_id: &str,
    ) -> ApiResult<PreparedRedPacketClaim> {
        let red_packet_id = red_packet_id.trim();
        if red_packet_id.is_empty() {
            return Err(ApiError::BadRequest("红包编号不能为空".to_string()));
        }
        let red_packet = self
            .red_packets
            .get(red_packet_id)
            .ok_or_else(|| ApiError::NotFound("红包不存在".to_string()))?;
        if red_packet.user_id == user.id {
            return Err(ApiError::BadRequest("不能领取自己发送的红包".to_string()));
        }
        if red_packet
            .claims
            .iter()
            .any(|claim| claim.user_id == user.id)
        {
            return Err(ApiError::Conflict("你已经领取过这个红包".to_string()));
        }
        if red_packet.claimed_count >= red_packet.claim_count
            || red_packet.remaining_amount_minor <= 0
        {
            return Err(ApiError::Conflict("红包已经被抢完".to_string()));
        }
        let remaining_claims = red_packet
            .claim_count
            .saturating_sub(red_packet.claimed_count)
            .max(1);
        let amount_minor = if remaining_claims == 1 {
            red_packet.remaining_amount_minor
        } else {
            (red_packet.remaining_amount_minor / i64::from(remaining_claims)).max(1)
        };

        Ok(PreparedRedPacketClaim {
            claim: ChatHallRedPacketClaim {
                id: chat_hall_red_packet_claim_id(self.next_red_packet_claim_sequence + 1),
                red_packet_id: red_packet_id.to_string(),
                user_id: user.id.clone(),
                username: user.username.clone(),
                amount_minor,
                created_at: current_time_label(),
            },
        })
    }

    /// 资金入账成功后落地领取记录，并刷新红包消息的 payload。
    fn apply_prepared_red_packet_claim(
        &mut self,
        prepared: PreparedRedPacketClaim,
    ) -> ApiResult<(ChatHallMessage, ChatHallRedPacketClaim)> {
        self.next_red_packet_claim_sequence = self.next_red_packet_claim_sequence.saturating_add(1);
        let red_packet = self
            .red_packets
            .get_mut(&prepared.claim.red_packet_id)
            .ok_or_else(|| ApiError::NotFound("红包不存在".to_string()))?;
        if red_packet.remaining_amount_minor < prepared.claim.amount_minor {
            return Err(ApiError::Conflict("红包余额不足".to_string()));
        }
        red_packet.remaining_amount_minor -= prepared.claim.amount_minor;
        red_packet.claimed_count = red_packet.claimed_count.saturating_add(1);
        red_packet.claims.push(prepared.claim.clone());
        let payload = red_packet_payload(red_packet)?;

        let message = self
            .messages
            .iter_mut()
            .find(|message| {
                message.message_type == ChatHallMessageType::RedPacket
                    && message
                        .payload
                        .as_ref()
                        .and_then(|payload| payload.get("redPacketId"))
                        .and_then(Value::as_str)
                        == Some(&prepared.claim.red_packet_id)
            })
            .ok_or_else(|| ApiError::NotFound("红包消息不存在".to_string()))?;
        message.payload = Some(payload);

        Ok((message.clone(), prepared.claim))
    }

    /// 追加一条合买计划分享消息。
    fn share_group_buy_plan(
        &mut self,
        user: &UserSummary,
        payload: ChatHallGroupBuyPlanPayload,
    ) -> ApiResult<ChatHallMessage> {
        if payload.plan_id.trim().is_empty() {
            return Err(ApiError::BadRequest("合买计划编号不能为空".to_string()));
        }
        self.next_sequence = self.next_sequence.saturating_add(1);
        let content = format!("分享合买计划：{}", payload.title);
        let message = ChatHallMessage {
            id: chat_hall_message_id(self.next_sequence),
            user_id: user.id.clone(),
            username: user.username.clone(),
            avatar_url: user.avatar_url.clone(),
            content,
            message_type: ChatHallMessageType::GroupBuyPlan,
            payload: Some(to_json(&payload)?),
            created_at: current_time_label(),
        };
        self.messages.push_back(message.clone());
        self.trim_history();

        Ok(message)
    }

    /// 批量刷新指定用户在聊天大厅历史消息里的头像链接。
    fn update_user_avatar(&mut self, user_id: &str, avatar_url: &str) -> bool {
        let user_id = user_id.trim();
        let avatar_url = avatar_url.trim().to_string();
        let mut changed = false;
        for message in &mut self.messages {
            if message.user_id == user_id && message.avatar_url != avatar_url {
                message.avatar_url = avatar_url.clone();
                changed = true;
            }
        }
        changed
    }

    /// 限制内存和数据库中保留的大厅历史消息数量，避免无限增长。
    fn trim_history(&mut self) {
        while self.messages.len() > CHAT_HALL_HISTORY_LIMIT {
            self.messages.pop_front();
        }
        let active_red_packet_ids = self
            .messages
            .iter()
            .filter_map(|message| {
                if message.message_type != ChatHallMessageType::RedPacket {
                    return None;
                }
                message
                    .payload
                    .as_ref()
                    .and_then(|payload| payload.get("redPacketId"))
                    .and_then(Value::as_str)
                    .map(str::to_string)
            })
            .collect::<std::collections::BTreeSet<_>>();
        self.red_packets
            .retain(|id, _| active_red_packet_ids.contains(id));
    }
}

/// 从数据库读取聊天大厅消息，并恢复下一条消息的序号。
async fn load_chat_hall_store(database: &BusinessDatabase) -> ApiResult<ChatHallStore> {
    let rows = sqlx::query(
        "WITH recent_messages AS (
            SELECT m.id, m.user_id, m.username,
                   COALESCE(NULLIF(m.avatar_url, ''), u.avatar_url, '') AS avatar_url,
                   m.content, m.message_type, m.payload, m.created_at
            FROM chat_hall_messages m
            LEFT JOIN users u ON u.id = m.user_id
            ORDER BY m.created_at DESC, m.id DESC
            LIMIT $1
         )
         SELECT id, user_id, username, avatar_url, content, message_type, payload, created_at
         FROM recent_messages
         ORDER BY created_at ASC, id ASC",
    )
    .bind(i64::try_from(CHAT_HALL_HISTORY_LIMIT).unwrap_or(200))
    .fetch_all(database.pool())
    .await
    .map_err(|_| ApiError::Internal("聊天大厅消息数据读取失败".to_string()))?;

    let mut store = ChatHallStore::default();
    for row in rows {
        let id: String = row
            .try_get("id")
            .map_err(|_| ApiError::Internal("聊天大厅消息数据读取失败".to_string()))?;
        if let Some(sequence) = sequence_from_chat_hall_message_id(&id) {
            store.next_sequence = store.next_sequence.max(sequence);
        }
        store.messages.push_back(ChatHallMessage {
            id,
            user_id: row
                .try_get("user_id")
                .map_err(|_| ApiError::Internal("聊天大厅消息数据读取失败".to_string()))?,
            username: row
                .try_get("username")
                .map_err(|_| ApiError::Internal("聊天大厅消息数据读取失败".to_string()))?,
            avatar_url: row
                .try_get("avatar_url")
                .map_err(|_| ApiError::Internal("聊天大厅消息数据读取失败".to_string()))?,
            content: row
                .try_get("content")
                .map_err(|_| ApiError::Internal("聊天大厅消息数据读取失败".to_string()))?,
            message_type: enum_from_string(
                row.try_get("message_type")
                    .map_err(|_| ApiError::Internal("聊天大厅消息数据读取失败".to_string()))?,
            )?,
            payload: row
                .try_get("payload")
                .map_err(|_| ApiError::Internal("聊天大厅消息数据读取失败".to_string()))?,
            created_at: row
                .try_get("created_at")
                .map_err(|_| ApiError::Internal("聊天大厅消息数据读取失败".to_string()))?,
        });
    }

    store.next_sequence = store.next_sequence.max(
        max_chat_hall_sequence(
            database,
            "SELECT id FROM chat_hall_messages ORDER BY id DESC LIMIT 1",
            sequence_from_chat_hall_message_id,
            "聊天大厅消息序号读取失败",
        )
        .await?,
    );
    store.next_red_packet_sequence = store.next_red_packet_sequence.max(
        max_chat_hall_sequence(
            database,
            "SELECT id FROM chat_hall_red_packets ORDER BY id DESC LIMIT 1",
            sequence_from_chat_hall_red_packet_id,
            "聊天大厅红包序号读取失败",
        )
        .await?,
    );
    store.next_red_packet_claim_sequence = store.next_red_packet_claim_sequence.max(
        max_chat_hall_sequence(
            database,
            "SELECT id FROM chat_hall_red_packet_claims ORDER BY id DESC LIMIT 1",
            sequence_from_chat_hall_red_packet_claim_id,
            "聊天大厅红包领取序号读取失败",
        )
        .await?,
    );

    let active_red_packet_ids = store
        .messages
        .iter()
        .filter_map(|message| {
            message
                .payload
                .as_ref()
                .and_then(|payload| payload.get("redPacketId"))
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .collect::<BTreeSet<_>>();
    if active_red_packet_ids.is_empty() {
        return Ok(store);
    }
    let active_red_packet_ids = active_red_packet_ids.into_iter().collect::<Vec<_>>();

    for row in sqlx::query(
        "SELECT id, user_id, username, total_amount_minor, remaining_amount_minor,
                claim_count, claimed_count, greeting, created_at
         FROM chat_hall_red_packets
         WHERE id = ANY($1::text[])
         ORDER BY id ASC",
    )
    .bind(&active_red_packet_ids)
    .fetch_all(database.pool())
    .await
    .map_err(|_| ApiError::Internal("聊天大厅红包数据读取失败".to_string()))?
    {
        let id: String = row
            .try_get("id")
            .map_err(|_| ApiError::Internal("聊天大厅红包数据读取失败".to_string()))?;
        if let Some(sequence) = sequence_from_chat_hall_red_packet_id(&id) {
            store.next_red_packet_sequence = store.next_red_packet_sequence.max(sequence);
        }
        let claim_count: i32 = row
            .try_get("claim_count")
            .map_err(|_| ApiError::Internal("聊天大厅红包数据读取失败".to_string()))?;
        let claimed_count: i32 = row
            .try_get("claimed_count")
            .map_err(|_| ApiError::Internal("聊天大厅红包数据读取失败".to_string()))?;
        store.red_packets.insert(
            id.clone(),
            ChatHallRedPacket {
                id,
                user_id: row
                    .try_get("user_id")
                    .map_err(|_| ApiError::Internal("聊天大厅红包数据读取失败".to_string()))?,
                username: row
                    .try_get("username")
                    .map_err(|_| ApiError::Internal("聊天大厅红包数据读取失败".to_string()))?,
                total_amount_minor: row
                    .try_get("total_amount_minor")
                    .map_err(|_| ApiError::Internal("聊天大厅红包数据读取失败".to_string()))?,
                remaining_amount_minor: row
                    .try_get("remaining_amount_minor")
                    .map_err(|_| ApiError::Internal("聊天大厅红包数据读取失败".to_string()))?,
                claim_count: u32::try_from(claim_count)
                    .map_err(|_| ApiError::Internal("聊天大厅红包个数无效".to_string()))?,
                claimed_count: u32::try_from(claimed_count)
                    .map_err(|_| ApiError::Internal("聊天大厅红包领取数无效".to_string()))?,
                greeting: row
                    .try_get("greeting")
                    .map_err(|_| ApiError::Internal("聊天大厅红包数据读取失败".to_string()))?,
                claims: Vec::new(),
                created_at: row
                    .try_get("created_at")
                    .map_err(|_| ApiError::Internal("聊天大厅红包数据读取失败".to_string()))?,
            },
        );
    }

    for row in sqlx::query(
        "SELECT id, red_packet_id, user_id, username, amount_minor, created_at
         FROM chat_hall_red_packet_claims
         WHERE red_packet_id = ANY($1::text[])
         ORDER BY id ASC",
    )
    .bind(&active_red_packet_ids)
    .fetch_all(database.pool())
    .await
    .map_err(|_| ApiError::Internal("聊天大厅红包领取数据读取失败".to_string()))?
    {
        let id: String = row
            .try_get("id")
            .map_err(|_| ApiError::Internal("聊天大厅红包领取数据读取失败".to_string()))?;
        if let Some(sequence) = sequence_from_chat_hall_red_packet_claim_id(&id) {
            store.next_red_packet_claim_sequence =
                store.next_red_packet_claim_sequence.max(sequence);
        }
        let red_packet_id: String = row
            .try_get("red_packet_id")
            .map_err(|_| ApiError::Internal("聊天大厅红包领取数据读取失败".to_string()))?;
        if let Some(red_packet) = store.red_packets.get_mut(&red_packet_id) {
            red_packet.claims.push(ChatHallRedPacketClaim {
                id,
                red_packet_id,
                user_id: row
                    .try_get("user_id")
                    .map_err(|_| ApiError::Internal("聊天大厅红包领取数据读取失败".to_string()))?,
                username: row
                    .try_get("username")
                    .map_err(|_| ApiError::Internal("聊天大厅红包领取数据读取失败".to_string()))?,
                amount_minor: row
                    .try_get("amount_minor")
                    .map_err(|_| ApiError::Internal("聊天大厅红包领取数据读取失败".to_string()))?,
                created_at: row
                    .try_get("created_at")
                    .map_err(|_| ApiError::Internal("聊天大厅红包领取数据读取失败".to_string()))?,
            });
        }
    }

    store.trim_history();

    Ok(store)
}

/// 读取聊天大厅相关表的最大业务 ID 序号，避免启动时只加载近期数据后重复生成编号。
async fn max_chat_hall_sequence(
    database: &BusinessDatabase,
    sql: &str,
    parser: fn(&str) -> Option<u64>,
    error_message: &'static str,
) -> ApiResult<u64> {
    let latest_id = sqlx::query_scalar::<_, String>(sql)
        .fetch_optional(database.pool())
        .await
        .map_err(|_| ApiError::Internal(error_message.to_string()))?;
    Ok(latest_id.as_deref().and_then(parser).unwrap_or_default())
}

/// 保存聊天大厅最近消息快照；大厅消息只保留近 200 条。
async fn save_chat_hall_store(database: &BusinessDatabase, store: &ChatHallStore) -> ApiResult<()> {
    let mut tx = database
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::Internal("聊天大厅事务开启失败".to_string()))?;

    save_chat_hall_store_in_transaction(&mut *tx, store).await?;

    tx.commit()
        .await
        .map_err(|_| ApiError::Internal("聊天大厅事务提交失败".to_string()))
}

/// 在外层事务中保存手机端公共聊天大厅运行时快照，供跨仓储事务复用。
pub(crate) async fn save_chat_hall_store_in_transaction(
    connection: &mut PgConnection,
    store: &ChatHallStore,
) -> ApiResult<()> {
    for table in [
        "chat_hall_red_packet_claims",
        "chat_hall_red_packets",
        "chat_hall_messages",
    ] {
        sqlx::query(&format!("DELETE FROM {table}"))
            .execute(&mut *connection)
            .await
            .map_err(|_| ApiError::Internal("聊天大厅历史数据清理失败".to_string()))?;
    }

    for message in &store.messages {
        sqlx::query(
            "INSERT INTO chat_hall_messages
             (id, user_id, username, avatar_url, content, message_type, payload, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(&message.id)
        .bind(&message.user_id)
        .bind(&message.username)
        .bind(&message.avatar_url)
        .bind(&message.content)
        .bind(enum_to_string(&message.message_type)?)
        .bind(&message.payload)
        .bind(&message.created_at)
        .execute(&mut *connection)
        .await
        .map_err(|_| ApiError::Internal("聊天大厅消息保存失败".to_string()))?;
    }

    for red_packet in store.red_packets.values() {
        sqlx::query(
            "INSERT INTO chat_hall_red_packets
             (id, user_id, username, total_amount_minor, remaining_amount_minor,
              claim_count, claimed_count, greeting, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(&red_packet.id)
        .bind(&red_packet.user_id)
        .bind(&red_packet.username)
        .bind(red_packet.total_amount_minor)
        .bind(red_packet.remaining_amount_minor)
        .bind(
            i32::try_from(red_packet.claim_count)
                .map_err(|_| ApiError::Internal("红包个数过大".to_string()))?,
        )
        .bind(
            i32::try_from(red_packet.claimed_count)
                .map_err(|_| ApiError::Internal("红包领取数过大".to_string()))?,
        )
        .bind(&red_packet.greeting)
        .bind(&red_packet.created_at)
        .execute(&mut *connection)
        .await
        .map_err(|_| ApiError::Internal("聊天大厅红包保存失败".to_string()))?;

        for claim in &red_packet.claims {
            sqlx::query(
                "INSERT INTO chat_hall_red_packet_claims
                 (id, red_packet_id, user_id, username, amount_minor, created_at)
                 VALUES ($1, $2, $3, $4, $5, $6)",
            )
            .bind(&claim.id)
            .bind(&claim.red_packet_id)
            .bind(&claim.user_id)
            .bind(&claim.username)
            .bind(claim.amount_minor)
            .bind(&claim.created_at)
            .execute(&mut *connection)
            .await
            .map_err(|_| ApiError::Internal("聊天大厅红包领取记录保存失败".to_string()))?;
        }
    }

    Ok(())
}

/// 去除首尾空白并限制大厅消息长度。
fn normalize_chat_hall_content(content: String) -> ApiResult<String> {
    let content = content.trim().to_string();
    if content.is_empty() {
        return Err(ApiError::BadRequest("聊天内容不能为空".to_string()));
    }
    if content.chars().count() > CHAT_HALL_MESSAGE_MAX_LENGTH {
        return Err(ApiError::BadRequest(format!(
            "聊天内容不能超过 {CHAT_HALL_MESSAGE_MAX_LENGTH} 个字符"
        )));
    }

    Ok(content)
}

/// 清洗红包祝福语，允许为空时使用默认文案。
fn normalize_red_packet_greeting(greeting: String) -> ApiResult<String> {
    let greeting = greeting.trim();
    let greeting = if greeting.is_empty() {
        "恭喜发财，大吉大利"
    } else {
        greeting
    };
    if greeting.chars().count() > CHAT_HALL_RED_PACKET_GREETING_MAX_LENGTH {
        return Err(ApiError::BadRequest(format!(
            "红包祝福语不能超过 {CHAT_HALL_RED_PACKET_GREETING_MAX_LENGTH} 个字符"
        )));
    }

    Ok(greeting.to_string())
}

/// 把红包实体转换为聊天消息 payload，供手机端直接渲染红包卡片。
fn red_packet_payload(red_packet: &ChatHallRedPacket) -> ApiResult<Value> {
    to_json(&ChatHallRedPacketPayload {
        red_packet_id: red_packet.id.clone(),
        greeting: red_packet.greeting.clone(),
        total_amount_minor: red_packet.total_amount_minor,
        remaining_amount_minor: red_packet.remaining_amount_minor,
        claim_count: red_packet.claim_count,
        claimed_count: red_packet.claimed_count,
    })
}

/// 生成聊天大厅消息编号，便于数据库恢复时解析最大序号。
fn chat_hall_message_id(sequence: u64) -> String {
    format!("CHM-{sequence:012}")
}

/// 生成聊天大厅红包编号。
fn chat_hall_red_packet_id(sequence: u64) -> String {
    format!("CHRP-{sequence:012}")
}

/// 生成聊天大厅红包领取记录编号。
fn chat_hall_red_packet_claim_id(sequence: u64) -> String {
    format!("CHRPC-{sequence:012}")
}

/// 从聊天大厅消息编号中解析序号。
fn sequence_from_chat_hall_message_id(id: &str) -> Option<u64> {
    id.strip_prefix("CHM-")?.parse().ok()
}

/// 从红包编号中解析序号。
fn sequence_from_chat_hall_red_packet_id(id: &str) -> Option<u64> {
    id.strip_prefix("CHRP-")?.parse().ok()
}

/// 从红包领取记录编号中解析序号。
fn sequence_from_chat_hall_red_packet_claim_id(id: &str) -> Option<u64> {
    id.strip_prefix("CHRPC-")?.parse().ok()
}

/// 返回与系统其他业务字段一致的本地时间文本。
fn current_time_label() -> String {
    Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::user::{UserKind, UserStatus};

    #[test]
    /// 验证聊天大厅发送消息会修剪内容并按最近消息正序返回。
    fn chat_hall_store_sends_and_lists_messages() {
        let mut store = ChatHallStore::default();
        let user = test_user_with_avatar("U90001", "alice", "https://cdn.example.com/a.png");

        let message = store
            .send(
                &user,
                CreateChatHallMessageRequest {
                    content: "  大家好  ".to_string(),
                },
            )
            .unwrap();

        assert_eq!(message.id, "CHM-000000000001");
        assert_eq!(message.user_id, "U90001");
        assert_eq!(message.username, "alice");
        assert_eq!(message.avatar_url, "https://cdn.example.com/a.png");
        assert_eq!(message.content, "大家好");
        assert_eq!(message.message_type, ChatHallMessageType::Text);
        assert!(message.payload.is_none());
        assert_eq!(store.list(), vec![message]);
    }

    #[test]
    /// 验证聊天大厅拒绝空消息和超长消息。
    fn chat_hall_store_rejects_invalid_content() {
        let mut store = ChatHallStore::default();
        let user = test_user("U90001", "alice");

        assert!(matches!(
            store.send(
                &user,
                CreateChatHallMessageRequest {
                    content: "  ".to_string(),
                },
            ),
            Err(ApiError::BadRequest(_))
        ));
        assert!(matches!(
            store.send(
                &user,
                CreateChatHallMessageRequest {
                    content: "字".repeat(CHAT_HALL_MESSAGE_MAX_LENGTH + 1),
                },
            ),
            Err(ApiError::BadRequest(_))
        ));
    }

    #[test]
    /// 验证聊天大厅只保留最近 200 条历史消息。
    fn chat_hall_store_keeps_recent_history() {
        let mut store = ChatHallStore::default();
        let user = test_user("U90001", "alice");

        for index in 0..=CHAT_HALL_HISTORY_LIMIT {
            store
                .send(
                    &user,
                    CreateChatHallMessageRequest {
                        content: format!("消息 {index}"),
                    },
                )
                .unwrap();
        }

        assert_eq!(store.messages.len(), CHAT_HALL_HISTORY_LIMIT);
        assert_eq!(store.messages.front().unwrap().content, "消息 1");
        assert_eq!(
            store.messages.back().unwrap().id,
            chat_hall_message_id((CHAT_HALL_HISTORY_LIMIT + 1) as u64)
        );
    }

    #[test]
    /// 验证后台清空聊天大厅消息会清除展示记录，但不会重置后续消息序号。
    fn chat_hall_store_clears_messages_without_resetting_sequence() {
        let mut store = ChatHallStore::default();
        let user = test_user("U90001", "alice");
        store
            .send(
                &user,
                CreateChatHallMessageRequest {
                    content: "第一条".to_string(),
                },
            )
            .expect("message can be sent");
        store
            .send(
                &user,
                CreateChatHallMessageRequest {
                    content: "第二条".to_string(),
                },
            )
            .expect("message can be sent");

        assert_eq!(store.clear_messages(), 2);
        assert!(store.list().is_empty());

        let next_message = store
            .send(
                &user,
                CreateChatHallMessageRequest {
                    content: "清空后新消息".to_string(),
                },
            )
            .expect("message can be sent after clear");
        assert_eq!(next_message.id, "CHM-000000000003");
    }

    #[test]
    /// 验证用户更新头像后，聊天大厅历史消息会同步刷新头像链接。
    fn chat_hall_store_updates_user_avatar_snapshot() {
        let mut store = ChatHallStore::default();
        let user = test_user("U90001", "alice");
        store
            .send(
                &user,
                CreateChatHallMessageRequest {
                    content: "先发一条".to_string(),
                },
            )
            .unwrap();

        assert!(store.update_user_avatar("U90001", "https://cdn.example.com/new.png"));
        assert_eq!(
            store.messages.front().unwrap().avatar_url,
            "https://cdn.example.com/new.png"
        );
        assert!(!store.update_user_avatar("U404", "https://cdn.example.com/new.png"));
    }

    #[test]
    /// 验证红包消息会生成红包 payload，并在领取后更新已领数量和剩余金额。
    fn chat_hall_store_sends_and_claims_red_packet() {
        let mut store = ChatHallStore::default();
        let sender = test_user("U90001", "alice");
        let receiver = test_user("U90002", "bob");

        let prepared = store
            .prepare_red_packet(
                &sender,
                CreateChatHallRedPacketRequest {
                    amount_minor: 300,
                    claim_count: 3,
                    greeting: "好运常在".to_string(),
                },
            )
            .expect("red packet can be prepared");
        let message = store
            .insert_prepared_red_packet(prepared)
            .expect("red packet can be inserted");

        assert_eq!(message.message_type, ChatHallMessageType::RedPacket);
        assert_eq!(
            message
                .payload
                .as_ref()
                .and_then(|payload| payload.get("claimedCount"))
                .and_then(Value::as_u64),
            Some(0)
        );

        let prepared_claim = store
            .prepare_red_packet_claim(&receiver, "CHRP-000000000001")
            .expect("red packet can be claimed");
        assert_eq!(prepared_claim.claim.amount_minor, 100);
        let (updated, claim) = store
            .apply_prepared_red_packet_claim(prepared_claim)
            .expect("claim can be applied");
        assert_eq!(claim.user_id, receiver.id);
        assert_eq!(
            updated
                .payload
                .as_ref()
                .and_then(|payload| payload.get("claimedCount"))
                .and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(
            updated
                .payload
                .as_ref()
                .and_then(|payload| payload.get("remainingAmountMinor"))
                .and_then(Value::as_i64),
            Some(200)
        );
    }

    #[test]
    /// 验证红包领取记录可以查询到领取人、金额和领取进度。
    fn chat_hall_store_lists_red_packet_claims() {
        let mut store = ChatHallStore::default();
        let sender = test_user("U90001", "alice");
        let receiver = test_user("U90002", "bob");
        let prepared = store
            .prepare_red_packet(
                &sender,
                CreateChatHallRedPacketRequest {
                    amount_minor: 200,
                    claim_count: 2,
                    greeting: "手气不错".to_string(),
                },
            )
            .unwrap();
        store.insert_prepared_red_packet(prepared).unwrap();
        let prepared_claim = store
            .prepare_red_packet_claim(&receiver, "CHRP-000000000001")
            .unwrap();
        store
            .apply_prepared_red_packet_claim(prepared_claim)
            .unwrap();

        let response = store.red_packet_claims("CHRP-000000000001").unwrap();
        assert_eq!(response.greeting, "手气不错");
        assert_eq!(response.claimed_count, 1);
        assert_eq!(response.remaining_amount_minor, 100);
        assert_eq!(response.claims.len(), 1);
        assert_eq!(response.claims[0].username, "bob");
        assert_eq!(response.claims[0].amount_minor, 100);
    }

    #[test]
    /// 验证红包不能被发送人领取，也不能被同一用户重复领取。
    fn chat_hall_store_rejects_invalid_red_packet_claim() {
        let mut store = ChatHallStore::default();
        let sender = test_user("U90001", "alice");
        let receiver = test_user("U90002", "bob");
        let prepared = store
            .prepare_red_packet(
                &sender,
                CreateChatHallRedPacketRequest {
                    amount_minor: 200,
                    claim_count: 2,
                    greeting: "".to_string(),
                },
            )
            .unwrap();
        store.insert_prepared_red_packet(prepared).unwrap();

        assert!(matches!(
            store.prepare_red_packet_claim(&sender, "CHRP-000000000001"),
            Err(ApiError::BadRequest(_))
        ));
        let prepared_claim = store
            .prepare_red_packet_claim(&receiver, "CHRP-000000000001")
            .unwrap();
        store
            .apply_prepared_red_packet_claim(prepared_claim)
            .unwrap();
        assert!(matches!(
            store.prepare_red_packet_claim(&receiver, "CHRP-000000000001"),
            Err(ApiError::Conflict(_))
        ));
    }
    /// 构造测试用用户摘要。
    fn test_user(id: &str, username: &str) -> UserSummary {
        test_user_with_avatar(id, username, "")
    }
    /// 构造用户带头像测试数据。
    fn test_user_with_avatar(id: &str, username: &str, avatar_url: &str) -> UserSummary {
        UserSummary {
            id: id.to_string(),
            username: username.to_string(),
            email: None,
            avatar_url: avatar_url.to_string(),
            contact_qq: String::new(),
            kind: UserKind::Regular,
            status: UserStatus::Active,
            balance_minor: 0,
            agent_id: None,
            invite_code: "INVITE1".to_string(),
            registration_location: crate::domain::user::UserRegistrationLocation::default(),
            created_at: "2026-06-05 10:00:00".to_string(),
        }
    }
}
