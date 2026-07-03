-- 修复服务层提现任务补偿中投注增量被本次新增任务要求压成 0 的问题，并按流水顺序补齐历史漏累计。
DROP TRIGGER IF EXISTS ledger_entries_withdrawal_turnover_insert ON ledger_entries;

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
        GREATEST(0, LEAST(completed_effective_delta, required_effective_delta)),
        now(),
        now()
    )
    ON CONFLICT (user_id) DO UPDATE SET
        cumulative_recharge_minor = user_withdrawal_turnovers.cumulative_recharge_minor
            + EXCLUDED.cumulative_recharge_minor,
        required_effective_bet_minor = user_withdrawal_turnovers.required_effective_bet_minor
            + EXCLUDED.required_effective_bet_minor,
        completed_effective_bet_minor = LEAST(
            user_withdrawal_turnovers.required_effective_bet_minor + EXCLUDED.required_effective_bet_minor,
            GREATEST(
                0,
                user_withdrawal_turnovers.completed_effective_bet_minor +
                    CASE
                        WHEN completed_effective_delta > 0 THEN
                            LEAST(
                                completed_effective_delta,
                                GREATEST(
                                    0,
                                    (user_withdrawal_turnovers.required_effective_bet_minor + EXCLUDED.required_effective_bet_minor)
                                        - user_withdrawal_turnovers.completed_effective_bet_minor
                                )
                            )
                        ELSE
                            completed_effective_delta
                    END
            )
        ),
        updated_at = now();

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION apply_user_withdrawal_turnover_from_ledger() IS
    '历史兼容函数：提现任务累计当前由服务层资金流水事务显式维护；投注增量使用原始流水金额，不再被本次新增任务要求压成 0';

CREATE TEMP TABLE withdrawal_turnover_recomputed (
    user_id TEXT PRIMARY KEY,
    cumulative_recharge_minor BIGINT NOT NULL DEFAULT 0,
    required_effective_bet_minor BIGINT NOT NULL DEFAULT 0,
    completed_effective_bet_minor BIGINT NOT NULL DEFAULT 0
) ON COMMIT DROP;

DO $$
DECLARE
    ledger_row RECORD;
    effective_delta BIGINT := 0;
    current_required BIGINT := 0;
    current_completed BIGINT := 0;
BEGIN
    FOR ledger_row IN
        SELECT user_id, kind, amount_minor
        FROM ledger_entries
        WHERE (kind = 'rechargeCredit' AND amount_minor > 0)
           OR (kind = 'rechargeBonusCredit' AND amount_minor > 0)
           OR (kind IN ('orderDebit', 'groupBuyDebit') AND amount_minor < 0)
           OR (kind IN ('orderRefund', 'groupBuyRefund') AND amount_minor > 0)
        ORDER BY user_id, created_at, id
    LOOP
        INSERT INTO withdrawal_turnover_recomputed (user_id)
        VALUES (ledger_row.user_id)
        ON CONFLICT (user_id) DO NOTHING;

        IF ledger_row.kind = 'rechargeCredit' AND ledger_row.amount_minor > 0 THEN
            UPDATE withdrawal_turnover_recomputed
            SET cumulative_recharge_minor = cumulative_recharge_minor + ledger_row.amount_minor,
                required_effective_bet_minor = required_effective_bet_minor + ledger_row.amount_minor
            WHERE user_id = ledger_row.user_id;
        ELSIF ledger_row.kind = 'rechargeBonusCredit' AND ledger_row.amount_minor > 0 THEN
            UPDATE withdrawal_turnover_recomputed
            SET required_effective_bet_minor = required_effective_bet_minor + ledger_row.amount_minor
            WHERE user_id = ledger_row.user_id;
        ELSE
            effective_delta := -ledger_row.amount_minor;

            SELECT required_effective_bet_minor, completed_effective_bet_minor
            INTO current_required, current_completed
            FROM withdrawal_turnover_recomputed
            WHERE user_id = ledger_row.user_id;

            IF effective_delta > 0 THEN
                current_completed := current_completed
                    + LEAST(effective_delta, GREATEST(0, current_required - current_completed));
            ELSE
                current_completed := current_completed + effective_delta;
            END IF;

            current_completed := GREATEST(0, LEAST(current_required, current_completed));

            UPDATE withdrawal_turnover_recomputed
            SET completed_effective_bet_minor = current_completed
            WHERE user_id = ledger_row.user_id;
        END IF;
    END LOOP;
END $$;

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
FROM withdrawal_turnover_recomputed
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
