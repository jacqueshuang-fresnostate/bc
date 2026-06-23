-- 修复历史库中已支付充值订单没有同步进入用户累计充值表的问题。
-- 聊天大厅发言门槛和提现流水要求都依赖 user_withdrawal_turnovers.cumulative_recharge_minor。

INSERT INTO user_withdrawal_turnover_events (ledger_entry_id, user_id, kind, amount_minor, created_at)
SELECT id, user_id, kind, amount_minor, now()
FROM ledger_entries
WHERE kind IN ('rechargeCredit', 'orderDebit', 'orderRefund', 'groupBuyDebit', 'groupBuyRefund')
  AND (
      (kind = 'rechargeCredit' AND amount_minor > 0)
      OR (kind IN ('orderDebit', 'groupBuyDebit') AND amount_minor < 0)
      OR (kind IN ('orderRefund', 'groupBuyRefund') AND amount_minor > 0)
  )
ON CONFLICT (ledger_entry_id) DO NOTHING;

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
