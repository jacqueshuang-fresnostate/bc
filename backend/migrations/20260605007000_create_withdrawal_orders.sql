CREATE TABLE withdrawal_orders (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    username TEXT NOT NULL,
    method_id TEXT NOT NULL,
    method_type TEXT NOT NULL,
    account_holder TEXT NOT NULL,
    account_number TEXT NOT NULL,
    bank_name TEXT,
    amount_minor BIGINT NOT NULL,
    status TEXT NOT NULL,
    created_at TEXT NOT NULL,
    reviewed_at TEXT,
    CONSTRAINT withdrawal_orders_method_type_check CHECK (method_type IN ('alipay', 'wechat', 'bankCard')),
    CONSTRAINT withdrawal_orders_status_check CHECK (status IN ('pending', 'approved', 'rejected', 'cancelled')),
    CONSTRAINT withdrawal_orders_amount_minor_check CHECK (amount_minor > 0)
);

CREATE TABLE withdrawal_runtime (
    key TEXT PRIMARY KEY,
    value BIGINT NOT NULL
);

CREATE INDEX withdrawal_orders_user_id_idx ON withdrawal_orders (user_id, id);
CREATE INDEX withdrawal_orders_status_idx ON withdrawal_orders (status, id);
CREATE INDEX withdrawal_orders_method_id_idx ON withdrawal_orders (method_id);

COMMENT ON TABLE withdrawal_orders IS '用户提现申请表，记录用户提交的提现订单和收款方式快照';
COMMENT ON COLUMN withdrawal_orders.id IS '提现申请 ID，由后端按 W 加序号生成';
COMMENT ON COLUMN withdrawal_orders.user_id IS '提交提现申请的用户 ID';
COMMENT ON COLUMN withdrawal_orders.username IS '提交提现申请时的用户名快照';
COMMENT ON COLUMN withdrawal_orders.method_id IS '用户提现方式 ID';
COMMENT ON COLUMN withdrawal_orders.method_type IS '提现方式类型：alipay 支付宝，wechat 微信，bankCard 银行卡';
COMMENT ON COLUMN withdrawal_orders.account_holder IS '提现收款账户名快照';
COMMENT ON COLUMN withdrawal_orders.account_number IS '提现收款账号或银行卡号快照';
COMMENT ON COLUMN withdrawal_orders.bank_name IS '银行卡所属银行名称，非银行卡提现方式为空';
COMMENT ON COLUMN withdrawal_orders.amount_minor IS '提现金额（分），必须大于 0';
COMMENT ON COLUMN withdrawal_orders.status IS '提现状态：pending 待审核，approved 已通过，rejected 已驳回，cancelled 已取消';
COMMENT ON COLUMN withdrawal_orders.created_at IS '提现申请创建时间，格式为 YYYY-MM-DD HH:MM:SS';
COMMENT ON COLUMN withdrawal_orders.reviewed_at IS '提现审核时间，未审核时为空';
COMMENT ON CONSTRAINT withdrawal_orders_method_type_check ON withdrawal_orders IS '限制提现方式类型枚举值';
COMMENT ON CONSTRAINT withdrawal_orders_status_check ON withdrawal_orders IS '限制提现申请状态枚举值';
COMMENT ON CONSTRAINT withdrawal_orders_amount_minor_check ON withdrawal_orders IS '限制提现金额必须大于 0';

COMMENT ON TABLE withdrawal_runtime IS '提现模块运行时序号表';
COMMENT ON COLUMN withdrawal_runtime.key IS '运行时配置键，例如 next_sequence';
COMMENT ON COLUMN withdrawal_runtime.value IS '运行时配置值，当前用于保存下一个提现申请序号';

COMMENT ON COLUMN ledger_entries.kind IS '流水类型：manualAdjustment 手动调账，orderDebit 投注扣款，orderRefund 取消退款，payoutCredit 派奖入账，rechargeCredit 充值入账，withdrawalFreeze 提现冻结';
