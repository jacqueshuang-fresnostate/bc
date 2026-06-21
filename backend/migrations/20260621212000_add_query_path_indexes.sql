CREATE INDEX IF NOT EXISTS orders_user_source_status_created_idx
    ON orders (user_id, order_source, status, created_at DESC, id DESC)
    WHERE amount_minor > 0;

CREATE INDEX IF NOT EXISTS group_buy_participants_user_plan_amount_idx
    ON group_buy_participants (user_id, plan_id)
    WHERE amount_minor > 0;

CREATE INDEX IF NOT EXISTS order_settlement_runs_created_id_idx
    ON order_settlement_runs (created_at DESC, id DESC);

CREATE INDEX IF NOT EXISTS recharge_orders_export_filter_idx
    ON recharge_orders (status, user_id, created_at DESC, id DESC);

COMMENT ON INDEX orders_user_source_status_created_idx IS '按用户集合、下单来源和状态读取投注画像订单的加速索引';
COMMENT ON INDEX group_buy_participants_user_plan_amount_idx IS '按参与用户反查合买计划的加速索引';
COMMENT ON INDEX order_settlement_runs_created_id_idx IS '计奖派奖批次按时间倒序分页的加速索引';
COMMENT ON INDEX recharge_orders_export_filter_idx IS '后台充值记录按状态、用户和时间范围导出 CSV 的加速索引';
