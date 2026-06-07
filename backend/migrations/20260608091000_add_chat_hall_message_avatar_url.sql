ALTER TABLE chat_hall_messages
    ADD COLUMN IF NOT EXISTS avatar_url TEXT NOT NULL DEFAULT '';

COMMENT ON COLUMN chat_hall_messages.avatar_url IS '聊天大厅消息发送人头像链接快照';
