COMMENT ON COLUMN user_withdrawal_turnovers.required_effective_bet_minor IS
    '提现前需要完成的有效投注金额（分），当前规则为真实充值本金加充值赠送金额等额';

WITH inserted_bonus_events AS (
    INSERT INTO user_withdrawal_turnover_events (ledger_entry_id, user_id, kind, amount_minor, created_at)
    SELECT id, user_id, kind, amount_minor, now()
    FROM ledger_entries
    WHERE kind = 'rechargeBonusCredit'
      AND amount_minor > 0
    ON CONFLICT (ledger_entry_id) DO NOTHING
    RETURNING user_id, amount_minor
),
bonus_totals AS (
    SELECT
        user_id,
        COALESCE(SUM(amount_minor), 0)::BIGINT AS bonus_minor
    FROM inserted_bonus_events
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
    0,
    bonus_minor,
    0,
    now(),
    now()
FROM bonus_totals
WHERE bonus_minor > 0
ON CONFLICT (user_id) DO UPDATE SET
    required_effective_bet_minor = user_withdrawal_turnovers.required_effective_bet_minor
        + EXCLUDED.required_effective_bet_minor,
    updated_at = now();

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
        GREATEST(0, completed_effective_delta),
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
            user_withdrawal_turnovers.completed_effective_bet_minor + completed_effective_delta
        ),
        updated_at = now();

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION apply_user_withdrawal_turnover_from_ledger() IS
    '根据新增资金流水增量维护用户提现流水要求累计数据，充值赠送只增加提现投注要求';
