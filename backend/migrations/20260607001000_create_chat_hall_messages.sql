CREATE TABLE IF NOT EXISTS chat_hall_messages (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    username TEXT NOT NULL,
    content TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS chat_hall_messages_created_at_idx
    ON chat_hall_messages (created_at DESC, id DESC);

COMMENT ON TABLE chat_hall_messages IS '手机端公共聊天大厅消息表';
COMMENT ON COLUMN chat_hall_messages.id IS '聊天大厅消息唯一标识';
COMMENT ON COLUMN chat_hall_messages.user_id IS '发送消息的用户 ID';
COMMENT ON COLUMN chat_hall_messages.username IS '发送消息时展示的用户名';
COMMENT ON COLUMN chat_hall_messages.content IS '聊天消息内容';
COMMENT ON COLUMN chat_hall_messages.created_at IS '消息发送时间文本';
