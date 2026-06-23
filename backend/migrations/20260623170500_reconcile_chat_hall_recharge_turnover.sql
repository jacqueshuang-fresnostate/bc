-- 再次按已入账充值订单校准用户累计真实充值本金。
-- 用于修复服务器在旧逻辑运行期间产生的已支付充值单，避免聊天大厅发言资格继续读取到过低累计值。

WITH paid_recharge_totals AS (
    SELECT
        user_id,
        COALESCE(SUM(amount_minor), 0)::BIGINT AS cumulative_recharge_minor
    FROM recharge_orders
    WHERE status = 'paid'
      AND amount_minor > 0
    GROUP BY user_id
)
INSERT INTO user_withdrawal_turnovers (
    user_id,
    cumulative_recharge_minor,
    required_effective_bet_minor,
    completed_effective_bet_minor,
    created_at,
    updated_at
)
SELECT
    user_id,
    cumulative_recharge_minor,
    cumulative_recharge_minor,
    0,
    now(),
    now()
FROM paid_recharge_totals
WHERE cumulative_recharge_minor > 0
ON CONFLICT (user_id) DO UPDATE SET
    cumulative_recharge_minor = GREATEST(
        user_withdrawal_turnovers.cumulative_recharge_minor,
        EXCLUDED.cumulative_recharge_minor
    ),
    required_effective_bet_minor = GREATEST(
        user_withdrawal_turnovers.required_effective_bet_minor,
        EXCLUDED.required_effective_bet_minor
    ),
    updated_at = now();

COMMENT ON TABLE user_withdrawal_turnovers IS '用户提现流水要求累计表，同时用于聊天大厅发言累计充值资格判断';
