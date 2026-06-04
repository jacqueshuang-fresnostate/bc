CREATE TABLE recharge_orders (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    username TEXT NOT NULL,
    channel TEXT NOT NULL,
    amount_minor BIGINT NOT NULL,
    status TEXT NOT NULL,
    pay_type TEXT,
    provider_trade_no TEXT,
    payment_url TEXT,
    support_conversation_id TEXT,
    created_at TEXT NOT NULL,
    paid_at TEXT,
    CONSTRAINT recharge_orders_channel_check CHECK (channel IN ('rainbowEpay', 'customerService')),
    CONSTRAINT recharge_orders_status_check CHECK (status IN ('pending', 'waitingCustomerService', 'paid', 'cancelled')),
    CONSTRAINT recharge_orders_amount_minor_check CHECK (amount_minor > 0)
);

CREATE TABLE recharge_runtime (
    key TEXT PRIMARY KEY,
    value BIGINT NOT NULL
);

CREATE INDEX recharge_orders_user_id_idx ON recharge_orders (user_id, id);
CREATE INDEX recharge_orders_status_idx ON recharge_orders (status, id);
CREATE INDEX recharge_orders_support_conversation_id_idx
    ON recharge_orders (support_conversation_id)
    WHERE support_conversation_id IS NOT NULL;

COMMENT ON TABLE recharge_orders IS '用户充值订单表，记录彩虹易支付和客服直充的充值申请';
COMMENT ON COLUMN recharge_orders.id IS '充值订单 ID，由后端按 R 加序号生成';
COMMENT ON COLUMN recharge_orders.user_id IS '提交充值申请的用户 ID';
COMMENT ON COLUMN recharge_orders.username IS '提交充值申请时的用户名快照';
COMMENT ON COLUMN recharge_orders.channel IS '充值渠道：rainbowEpay 表示彩虹易支付，customerService 表示客服直充';
COMMENT ON COLUMN recharge_orders.amount_minor IS '充值金额（分），必须大于 0';
COMMENT ON COLUMN recharge_orders.status IS '充值状态：pending 待支付，waitingCustomerService 等待客服处理，paid 已入账，cancelled 已取消';
COMMENT ON COLUMN recharge_orders.pay_type IS '彩虹易支付方式，例如 alipay 或 wxpay；客服直充为空';
COMMENT ON COLUMN recharge_orders.provider_trade_no IS '第三方支付平台返回的交易号';
COMMENT ON COLUMN recharge_orders.payment_url IS '彩虹易支付跳转地址，客服直充为空';
COMMENT ON COLUMN recharge_orders.support_conversation_id IS '客服直充绑定的客服会话 ID';
COMMENT ON COLUMN recharge_orders.created_at IS '充值订单创建时间，格式为 YYYY-MM-DD HH:MM:SS';
COMMENT ON COLUMN recharge_orders.paid_at IS '充值入账时间，未入账时为空';
COMMENT ON CONSTRAINT recharge_orders_channel_check ON recharge_orders IS '限制充值渠道枚举值';
COMMENT ON CONSTRAINT recharge_orders_status_check ON recharge_orders IS '限制充值订单状态枚举值';
COMMENT ON CONSTRAINT recharge_orders_amount_minor_check ON recharge_orders IS '限制充值金额必须大于 0';

COMMENT ON TABLE recharge_runtime IS '充值模块运行时序号表';
COMMENT ON COLUMN recharge_runtime.key IS '运行时配置键，例如 next_sequence';
COMMENT ON COLUMN recharge_runtime.value IS '运行时配置值，当前用于保存下一个充值订单序号';

COMMENT ON COLUMN ledger_entries.kind IS '流水类型：manualAdjustment 手动调账，orderDebit 投注扣款，orderRefund 取消退款，payoutCredit 派奖入账，rechargeCredit 充值入账';

INSERT INTO system_settings (key, value, description)
VALUES
    ('recharge_min_amount_minor', '100', '用户单笔充值最小金额（分）'),
    ('recharge_max_amount_minor', '10000000', '用户单笔充值最大金额（分）'),
    ('recharge_rainbow_epay_enabled', 'false', '是否开启彩虹易支付在线充值'),
    ('recharge_rainbow_epay_gateway_url', 'https://pay.example.com', '彩虹易支付网关域名，不需要填写 submit.php'),
    ('recharge_rainbow_epay_pid', '未配置', '彩虹易支付商户号'),
    ('recharge_rainbow_epay_key', '未配置', '彩虹易支付商户密钥'),
    ('recharge_rainbow_epay_notify_url', '/api/user/recharge/epay/notify', '彩虹易支付异步通知地址，生产环境建议填写完整外网 URL'),
    ('recharge_rainbow_epay_return_url', '/api/user/recharge/epay/return', '彩虹易支付同步返回地址，生产环境建议填写完整外网 URL'),
    ('recharge_rainbow_epay_pay_types', 'alipay,wxpay', '彩虹易支付允许的支付方式，多个值用英文逗号分隔'),
    ('recharge_customer_service_enabled', 'true', '是否开启客服直充'),
    ('recharge_customer_service_message', '客服已收到您的直充申请，请在会话中确认付款方式和到账信息。', '客服直充创建订单后返回给用户的提示文案')
ON CONFLICT (key) DO NOTHING;
