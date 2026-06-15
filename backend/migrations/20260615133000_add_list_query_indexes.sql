CREATE INDEX IF NOT EXISTS ledger_entries_kind_user_created_idx
    ON ledger_entries (kind, user_id, created_at DESC, id DESC);

CREATE INDEX IF NOT EXISTS ledger_entries_user_created_idx
    ON ledger_entries (user_id, created_at DESC, id DESC);

CREATE INDEX IF NOT EXISTS recharge_orders_status_created_idx
    ON recharge_orders (status, created_at DESC, id DESC);

CREATE INDEX IF NOT EXISTS recharge_orders_user_created_idx
    ON recharge_orders (user_id, created_at DESC, id DESC);

CREATE INDEX IF NOT EXISTS withdrawal_orders_status_created_idx
    ON withdrawal_orders (status, created_at DESC, id DESC);

CREATE INDEX IF NOT EXISTS withdrawal_orders_user_created_idx
    ON withdrawal_orders (user_id, created_at DESC, id DESC);

COMMENT ON INDEX ledger_entries_kind_user_created_idx IS '资金流水按类型、用户和创建时间倒序查询索引，用于返利明细和财务流水分页';
COMMENT ON INDEX ledger_entries_user_created_idx IS '资金流水按用户和创建时间倒序查询索引，用于用户资金流水分页';
COMMENT ON INDEX recharge_orders_status_created_idx IS '充值订单按状态和创建时间倒序查询索引，用于已支付充值聚合';
COMMENT ON INDEX recharge_orders_user_created_idx IS '充值订单按用户和创建时间倒序查询索引，用于用户充值记录分页';
COMMENT ON INDEX withdrawal_orders_status_created_idx IS '提现申请按状态和创建时间倒序查询索引，用于已通过提现聚合';
COMMENT ON INDEX withdrawal_orders_user_created_idx IS '提现申请按用户和创建时间倒序查询索引，用于用户提现记录分页';
