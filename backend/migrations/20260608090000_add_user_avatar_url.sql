ALTER TABLE users
    ADD COLUMN IF NOT EXISTS avatar_url TEXT NOT NULL DEFAULT '';

COMMENT ON COLUMN users.avatar_url IS '用户头像图片链接';
