-- 补偿旧库中已经写入资金流水、但因触发器缺失或旧迁移异常未进入提现任务累计表的记录。
WITH inserted_events AS (
    INSERT INTO user_withdrawal_turnover_events (ledger_entry_id, user_id, kind, amount_minor, created_at)
    SELECT id, user_id, kind, amount_minor, now()
    FROM ledger_entries
    WHERE (kind = 'rechargeCredit' AND amount_minor > 0)
       OR (kind = 'rechargeBonusCredit' AND amount_minor > 0)
       OR (kind IN ('orderDebit', 'groupBuyDebit') AND amount_minor < 0)
       OR (kind IN ('orderRefund', 'groupBuyRefund') AND amount_minor > 0)
    ON CONFLICT (ledger_entry_id) DO NOTHING
    RETURNING user_id, kind, amount_minor
),
event_deltas AS (
    SELECT
        user_id,
        COALESCE(SUM(
            CASE
                WHEN kind = 'rechargeCredit' AND amount_minor > 0 THEN amount_minor
                ELSE 0
            END
        ), 0)::BIGINT AS cumulative_recharge_delta,
        COALESCE(SUM(
            CASE
                WHEN kind IN ('rechargeCredit', 'rechargeBonusCredit') AND amount_minor > 0 THEN amount_minor
                ELSE 0
            END
        ), 0)::BIGINT AS required_effective_delta,
        COALESCE(SUM(
            CASE
                WHEN kind IN ('orderDebit', 'groupBuyDebit') AND amount_minor < 0 THEN -amount_minor
                WHEN kind IN ('orderRefund', 'groupBuyRefund') AND amount_minor > 0 THEN -amount_minor
                ELSE 0
            END
        ), 0)::BIGINT AS completed_effective_delta
    FROM inserted_events
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
    cumulative_recharge_delta,
    required_effective_delta,
    GREATEST(0::BIGINT, LEAST(required_effective_delta, completed_effective_delta)),
    now(),
    now()
FROM event_deltas
WHERE cumulative_recharge_delta <> 0
   OR required_effective_delta <> 0
   OR completed_effective_delta <> 0
ON CONFLICT (user_id) DO UPDATE SET
    cumulative_recharge_minor = user_withdrawal_turnovers.cumulative_recharge_minor
        + EXCLUDED.cumulative_recharge_minor,
    required_effective_bet_minor = user_withdrawal_turnovers.required_effective_bet_minor
        + EXCLUDED.required_effective_bet_minor,
    completed_effective_bet_minor = GREATEST(
        0::BIGINT,
        LEAST(
            user_withdrawal_turnovers.required_effective_bet_minor
                + EXCLUDED.required_effective_bet_minor,
            user_withdrawal_turnovers.completed_effective_bet_minor +
                CASE
                    WHEN EXCLUDED.completed_effective_bet_minor > 0 THEN
                        LEAST(
                            EXCLUDED.completed_effective_bet_minor,
                            GREATEST(
                                0::BIGINT,
                                (user_withdrawal_turnovers.required_effective_bet_minor
                                    + EXCLUDED.required_effective_bet_minor)
                                    - user_withdrawal_turnovers.completed_effective_bet_minor
                            )
                        )
                    ELSE
                        EXCLUDED.completed_effective_bet_minor
                END
        )
    ),
    updated_at = now();
