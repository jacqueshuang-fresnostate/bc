ALTER TABLE support_messages
    ADD COLUMN message_type TEXT NOT NULL DEFAULT 'text',
    ADD COLUMN image_url TEXT;

COMMENT ON COLUMN support_messages.message_type IS '客服消息类型，text 表示文本，image 表示图片';
COMMENT ON COLUMN support_messages.image_url IS '客服图片消息的图片链接，文本消息为空';
