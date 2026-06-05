ALTER TABLE group_buy_plans
    ADD COLUMN IF NOT EXISTS order_id TEXT;

CREATE INDEX IF NOT EXISTS group_buy_plans_order_id_idx
    ON group_buy_plans (order_id)
    WHERE order_id IS NOT NULL;

COMMENT ON COLUMN group_buy_plans.order_id IS '合买满单后生成的真实投注订单 ID，未满单时为空';
COMMENT ON INDEX group_buy_plans_order_id_idx IS '按真实投注订单反查合买计划，用于开奖结算分账';
