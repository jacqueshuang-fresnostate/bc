ALTER TABLE recharge_orders
ADD COLUMN remark TEXT NOT NULL DEFAULT '';

COMMENT ON COLUMN recharge_orders.remark IS '后台确认充值入账时填写的备注，用于财务核对付款凭证或线下沟通结果';
