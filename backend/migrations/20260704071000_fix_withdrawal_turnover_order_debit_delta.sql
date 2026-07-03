-- 修复服务层提现任务补偿中投注增量被本次新增任务要求压成 0 的问题，并轻量补齐上一版服务层窗口内的漏累计。
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

WITH service_layer_start AS (
    SELECT COALESCE(
        (
            SELECT installed_on
            FROM _sqlx_migrations
            WHERE version = 20260704064000
              AND success = true
            ORDER BY installed_on DESC
            LIMIT 1
        ),
        now()
    ) AS installed_on
),
recent_completed_events AS (
    SELECT
        events.user_id,
        COALESCE(SUM(
            CASE
                WHEN events.kind IN ('orderDebit', 'groupBuyDebit') AND events.amount_minor < 0 THEN -events.amount_minor
                WHEN events.kind IN ('orderRefund', 'groupBuyRefund') AND events.amount_minor > 0 THEN -events.amount_minor
                ELSE 0
            END
        ), 0)::BIGINT AS completed_effective_delta
    FROM user_withdrawal_turnover_events events
    CROSS JOIN service_layer_start
    WHERE events.created_at >= service_layer_start.installed_on
      AND (
          (events.kind IN ('orderDebit', 'groupBuyDebit') AND events.amount_minor < 0)
          OR (events.kind IN ('orderRefund', 'groupBuyRefund') AND events.amount_minor > 0)
      )
    GROUP BY events.user_id
)
UPDATE user_withdrawal_turnovers turnovers
SET completed_effective_bet_minor = LEAST(
        turnovers.required_effective_bet_minor,
        GREATEST(
            0::BIGINT,
            turnovers.completed_effective_bet_minor + recent_completed_events.completed_effective_delta
        )
    ),
    updated_at = now()
FROM recent_completed_events
WHERE turnovers.user_id = recent_completed_events.user_id
  AND recent_completed_events.completed_effective_delta <> 0
  AND turnovers.completed_effective_bet_minor <> LEAST(
        turnovers.required_effective_bet_minor,
        GREATEST(
            0::BIGINT,
            turnovers.completed_effective_bet_minor + recent_completed_events.completed_effective_delta
        )
    );
