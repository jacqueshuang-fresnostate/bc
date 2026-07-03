UPDATE user_withdrawal_turnovers
SET completed_effective_bet_minor = LEAST(
    required_effective_bet_minor,
    GREATEST(completed_effective_bet_minor, 0)
)
WHERE completed_effective_bet_minor <> LEAST(
    required_effective_bet_minor,
    GREATEST(completed_effective_bet_minor, 0)
);

CREATE OR REPLACE FUNCTION apply_user_withdrawal_turnover_from_ledger()
RETURNS trigger AS $$
DECLARE
    cumulative_recharge_delta BIGINT := 0;
    required_effective_delta BIGINT := 0;
    completed_effective_delta BIGINT := 0;
    processed_count INTEGER := 0;
BEGIN
    IF NEW.kind = 'rechargeCredit' AND NEW.amount_minor > 0 THEN
        cumulative_recharge_delta := NEW.amount_minor;
        required_effective_delta := NEW.amount_minor;
    ELSIF NEW.kind = 'rechargeBonusCredit' AND NEW.amount_minor > 0 THEN
        required_effective_delta := NEW.amount_minor;
    ELSIF NEW.kind IN ('orderDebit', 'groupBuyDebit') AND NEW.amount_minor < 0 THEN
        completed_effective_delta := -NEW.amount_minor;
    ELSIF NEW.kind IN ('orderRefund', 'groupBuyRefund') AND NEW.amount_minor > 0 THEN
        completed_effective_delta := -NEW.amount_minor;
    END IF;

    IF cumulative_recharge_delta = 0
        AND required_effective_delta = 0
        AND completed_effective_delta = 0 THEN
        RETURN NEW;
    END IF;

    INSERT INTO user_withdrawal_turnover_events (ledger_entry_id, user_id, kind, amount_minor, created_at)
    VALUES (NEW.id, NEW.user_id, NEW.kind, NEW.amount_minor, now())
    ON CONFLICT (ledger_entry_id) DO NOTHING;

    GET DIAGNOSTICS processed_count = ROW_COUNT;
    IF processed_count = 0 THEN
        RETURN NEW;
    END IF;

    INSERT INTO user_withdrawal_turnovers (
        user_id,
        cumulative_recharge_minor,
        required_effective_bet_minor,
        completed_effective_bet_minor,
        created_at,
        updated_at
    )
    VALUES (
        NEW.user_id,
        cumulative_recharge_delta,
        required_effective_delta,
        GREATEST(
            0,
            LEAST(required_effective_delta, completed_effective_delta)
        ),
        now(),
        now()
    )
    ON CONFLICT (user_id) DO UPDATE SET
        cumulative_recharge_minor = user_withdrawal_turnovers.cumulative_recharge_minor
            + EXCLUDED.cumulative_recharge_minor,
        required_effective_bet_minor = user_withdrawal_turnovers.required_effective_bet_minor
            + EXCLUDED.required_effective_bet_minor,
        completed_effective_bet_minor = GREATEST(
            0,
            LEAST(
                user_withdrawal_turnovers.required_effective_bet_minor + EXCLUDED.required_effective_bet_minor,
                user_withdrawal_turnovers.completed_effective_bet_minor +
                    CASE
                        WHEN EXCLUDED.completed_effective_bet_minor > 0 THEN
                            LEAST(
                                EXCLUDED.completed_effective_bet_minor,
                                GREATEST(
                                    0,
                                    (user_withdrawal_turnovers.required_effective_bet_minor + EXCLUDED.required_effective_bet_minor)
                                        - user_withdrawal_turnovers.completed_effective_bet_minor
                                )
                            )
                        ELSE
                            EXCLUDED.completed_effective_bet_minor
                    END
            )
        ),
        updated_at = now();

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION apply_user_withdrawal_turnover_from_ledger() IS
    '根据新增资金流水增量维护用户提现流水要求累计数据，已完成有效投注不会超过所需上限';
