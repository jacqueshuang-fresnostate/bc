CREATE INDEX IF NOT EXISTS group_buy_plans_initiator_created_idx
    ON group_buy_plans (initiator_user_id, created_at DESC, id DESC);

CREATE INDEX IF NOT EXISTS orders_direct_user_created_idx
    ON orders (user_id, created_at DESC, id DESC)
    WHERE order_source = 'direct';

CREATE INDEX IF NOT EXISTS withdrawal_orders_approved_user_amount_idx
    ON withdrawal_orders (user_id)
    INCLUDE (amount_minor)
    WHERE status = 'approved' AND amount_minor > 0;

CREATE INDEX IF NOT EXISTS invite_records_invitee_created_idx
    ON invite_records (invitee_user_id, created_at DESC, id DESC);

COMMENT ON INDEX group_buy_plans_initiator_created_idx IS
    '用户端按发起人和创建时间分页读取合买计划的加速索引';
COMMENT ON INDEX orders_direct_user_created_idx IS
    '用户端按用户和创建时间分页读取独立注单的加速索引';
COMMENT ON INDEX withdrawal_orders_approved_user_amount_idx IS
    '代理中心按直属用户汇总已通过提现金额的覆盖索引';
COMMENT ON INDEX invite_records_invitee_created_idx IS
    '按被邀请用户反查邀请关系和返利归属的加速索引';
