ALTER TABLE users
    ADD COLUMN IF NOT EXISTS contact_qq TEXT NOT NULL DEFAULT '';

COMMENT ON COLUMN users.contact_qq IS '用户注册或维护时填写的 QQ 联系方式，允许为空';
