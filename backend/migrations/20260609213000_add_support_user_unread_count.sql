ALTER TABLE support_conversations
    ADD COLUMN IF NOT EXISTS user_unread_count INTEGER NOT NULL DEFAULT 0;

COMMENT ON COLUMN support_conversations.user_unread_count IS '用户侧客服未读消息数，只统计客服回复后用户尚未查看的消息';
COMMENT ON COLUMN support_conversations.unread_count IS '后台客服侧未读消息数，只统计用户发来的待处理消息';
