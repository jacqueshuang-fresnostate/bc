ALTER TABLE chat_hall_messages
    ADD COLUMN IF NOT EXISTS message_type TEXT NOT NULL DEFAULT 'text',
    ADD COLUMN IF NOT EXISTS payload JSONB;

CREATE TABLE IF NOT EXISTS chat_hall_red_packets (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    username TEXT NOT NULL,
    total_amount_minor BIGINT NOT NULL,
    remaining_amount_minor BIGINT NOT NULL,
    claim_count INTEGER NOT NULL,
    claimed_count INTEGER NOT NULL,
    greeting TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS chat_hall_red_packet_claims (
    id TEXT PRIMARY KEY,
    red_packet_id TEXT NOT NULL REFERENCES chat_hall_red_packets(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL,
    username TEXT NOT NULL,
    amount_minor BIGINT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS chat_hall_red_packet_claims_red_packet_id_idx
    ON chat_hall_red_packet_claims (red_packet_id, id);

COMMENT ON COLUMN chat_hall_messages.message_type IS '聊天大厅消息类型：文本、红包、合买计划分享';
COMMENT ON COLUMN chat_hall_messages.payload IS '聊天大厅消息扩展数据，例如红包进度或合买计划摘要';

COMMENT ON TABLE chat_hall_red_packets IS '聊天大厅红包表';
COMMENT ON COLUMN chat_hall_red_packets.id IS '红包唯一标识';
COMMENT ON COLUMN chat_hall_red_packets.user_id IS '发送红包的用户 ID';
COMMENT ON COLUMN chat_hall_red_packets.username IS '发送红包时展示的用户名';
COMMENT ON COLUMN chat_hall_red_packets.total_amount_minor IS '红包总金额（分）';
COMMENT ON COLUMN chat_hall_red_packets.remaining_amount_minor IS '红包剩余金额（分）';
COMMENT ON COLUMN chat_hall_red_packets.claim_count IS '红包可领取份数';
COMMENT ON COLUMN chat_hall_red_packets.claimed_count IS '红包已领取份数';
COMMENT ON COLUMN chat_hall_red_packets.greeting IS '红包祝福语';
COMMENT ON COLUMN chat_hall_red_packets.created_at IS '红包创建时间文本';

COMMENT ON TABLE chat_hall_red_packet_claims IS '聊天大厅红包领取记录表';
COMMENT ON COLUMN chat_hall_red_packet_claims.id IS '红包领取记录唯一标识';
COMMENT ON COLUMN chat_hall_red_packet_claims.red_packet_id IS '关联红包 ID';
COMMENT ON COLUMN chat_hall_red_packet_claims.user_id IS '领取红包的用户 ID';
COMMENT ON COLUMN chat_hall_red_packet_claims.username IS '领取红包时展示的用户名';
COMMENT ON COLUMN chat_hall_red_packet_claims.amount_minor IS '领取金额（分）';
COMMENT ON COLUMN chat_hall_red_packet_claims.created_at IS '领取时间文本';
COMMENT ON INDEX chat_hall_red_packet_claims_red_packet_id_idx IS '按红包查询领取记录的索引';
