-- 提现任务累计改为由后端服务层在资金流水事务内显式维护，避免旧库 trigger 异常时只写事件不更新累计。
DROP TRIGGER IF EXISTS ledger_entries_withdrawal_turnover_insert ON ledger_entries;

COMMENT ON FUNCTION apply_user_withdrawal_turnover_from_ledger() IS
    '历史兼容函数：提现任务累计当前由服务层资金流水事务显式维护，不再通过 ledger_entries 触发器自动执行';

WITH ledger_totals AS (
    SELECT
        user_id,
        COALESCE(SUM(
            CASE
                WHEN kind = 'rechargeCredit' AND amount_minor > 0 THEN amount_minor
                ELSE 0
            END
        ), 0)::BIGINT AS cumulative_recharge_minor,
        COALESCE(SUM(
            CASE
                WHEN kind IN ('rechargeCredit', 'rechargeBonusCredit') AND amount_minor > 0 THEN amount_minor
                ELSE 0
            END
        ), 0)::BIGINT AS required_effective_bet_minor,
        COALESCE(SUM(
            CASE
                WHEN kind IN ('orderDebit', 'groupBuyDebit') AND amount_minor < 0 THEN -amount_minor
                WHEN kind IN ('orderRefund', 'groupBuyRefund') AND amount_minor > 0 THEN -amount_minor
                ELSE 0
            END
        ), 0)::BIGINT AS completed_effective_bet_minor
    FROM ledger_entries
    WHERE (kind = 'rechargeCredit' AND amount_minor > 0)
       OR (kind = 'rechargeBonusCredit' AND amount_minor > 0)
       OR (kind IN ('orderDebit', 'groupBuyDebit') AND amount_minor < 0)
       OR (kind IN ('orderRefund', 'groupBuyRefund') AND amount_minor > 0)
    GROUP BY user_id
),
clamped_totals AS (
    SELECT
        user_id,
        cumulative_recharge_minor,
        required_effective_bet_minor,
        GREATEST(
            0::BIGINT,
            LEAST(required_effective_bet_minor, completed_effective_bet_minor)
        ) AS completed_effective_bet_minor
    FROM ledger_totals
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
    required_effective_bet_minor,
    completed_effective_bet_minor,
    now(),
    now()
FROM clamped_totals
WHERE cumulative_recharge_minor > 0
   OR required_effective_bet_minor > 0
   OR completed_effective_bet_minor > 0
ON CONFLICT (user_id) DO UPDATE SET
    cumulative_recharge_minor = GREATEST(
        user_withdrawal_turnovers.cumulative_recharge_minor,
        EXCLUDED.cumulative_recharge_minor
    ),
    required_effective_bet_minor = GREATEST(
        user_withdrawal_turnovers.required_effective_bet_minor,
        EXCLUDED.required_effective_bet_minor
    ),
    completed_effective_bet_minor = LEAST(
        GREATEST(
            user_withdrawal_turnovers.required_effective_bet_minor,
            EXCLUDED.required_effective_bet_minor
        ),
        GREATEST(
            user_withdrawal_turnovers.completed_effective_bet_minor,
            EXCLUDED.completed_effective_bet_minor
        )
    ),
    updated_at = now();
