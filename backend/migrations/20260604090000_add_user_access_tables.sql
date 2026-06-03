CREATE TABLE IF NOT EXISTS user_password_hashes (
    user_id TEXT PRIMARY KEY,
    password_hash TEXT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS user_sessions (
    token TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT to_char(now(), 'YYYY-MM-DD HH24:MI:SS')
);

CREATE TABLE IF NOT EXISTS user_password_reset_tokens (
    token TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    expires_at_unix BIGINT NOT NULL,
    created_at TEXT NOT NULL DEFAULT to_char(now(), 'YYYY-MM-DD HH24:MI:SS')
);

CREATE TABLE IF NOT EXISTS user_withdrawal_methods (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    method_type TEXT NOT NULL,
    account_holder TEXT NOT NULL,
    account_number TEXT NOT NULL,
    bank_name TEXT,
    is_default BOOLEAN NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS user_sessions_user_id_idx ON user_sessions (user_id);
CREATE INDEX IF NOT EXISTS user_withdrawal_methods_user_id_idx ON user_withdrawal_methods (user_id);

COMMENT ON TABLE user_password_hashes IS '用户密码哈希表';
COMMENT ON COLUMN user_password_hashes.user_id IS '用户唯一标识；对应 users.id';
COMMENT ON COLUMN user_password_hashes.password_hash IS '基于 Argon2 的密码哈希值';
COMMENT ON COLUMN user_password_hashes.updated_at IS '密码更新时间';

COMMENT ON TABLE user_sessions IS '用户会话表';
COMMENT ON COLUMN user_sessions.token IS '用户登录会话 token';
COMMENT ON COLUMN user_sessions.user_id IS '归属用户 ID；对应 users.id';
COMMENT ON COLUMN user_sessions.created_at IS '会话创建时间（文本）';

COMMENT ON TABLE user_password_reset_tokens IS '用户忘记密码令牌表';
COMMENT ON COLUMN user_password_reset_tokens.token IS '重置码 token，长度与随机码策略一致';
COMMENT ON COLUMN user_password_reset_tokens.user_id IS '归属用户 ID；对应 users.id';
COMMENT ON COLUMN user_password_reset_tokens.expires_at_unix IS '过期时间（Unix 秒时间戳）';
COMMENT ON COLUMN user_password_reset_tokens.created_at IS '重置码创建时间（文本）';

COMMENT ON TABLE user_withdrawal_methods IS '用户提现方式表';
COMMENT ON COLUMN user_withdrawal_methods.id IS '提现方式主键';
COMMENT ON COLUMN user_withdrawal_methods.user_id IS '归属用户 ID；对应 users.id';
COMMENT ON COLUMN user_withdrawal_methods.method_type IS '提现类型：Alipay / Wechat / BankCard';
COMMENT ON COLUMN user_withdrawal_methods.account_holder IS '持卡/账号人姓名';
COMMENT ON COLUMN user_withdrawal_methods.account_number IS '到账账号（微信号/支付宝账号/银行卡号）';
COMMENT ON COLUMN user_withdrawal_methods.bank_name IS '银行卡名称（仅银行卡类型需要）';
COMMENT ON COLUMN user_withdrawal_methods.is_default IS '是否为默认提现方式';
COMMENT ON COLUMN user_withdrawal_methods.created_at IS '创建时间（文本）';
COMMENT ON COLUMN user_withdrawal_methods.updated_at IS '更新时间（文本）';
