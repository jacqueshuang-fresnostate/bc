INSERT INTO system_settings (key, value, description)
VALUES ('withdrawal_turnover_enabled', 'false', '是否开启提现前充值等额有效投注要求')
ON CONFLICT (key) DO UPDATE
SET description = EXCLUDED.description;

CREATE TABLE IF NOT EXISTS user_withdrawal_turnovers (
    user_id TEXT PRIMARY KEY,
    cumulative_recharge_minor BIGINT NOT NULL DEFAULT 0 CHECK (cumulative_recharge_minor >= 0),
    required_effective_bet_minor BIGINT NOT NULL DEFAULT 0 CHECK (required_effective_bet_minor >= 0),
    completed_effective_bet_minor BIGINT NOT NULL DEFAULT 0 CHECK (completed_effective_bet_minor >= 0),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS user_withdrawal_turnover_events (
    ledger_entry_id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    kind TEXT NOT NULL,
    amount_minor BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

COMMENT ON TABLE user_withdrawal_turnovers IS '用户提现流水要求累计表';
COMMENT ON COLUMN user_withdrawal_turnovers.user_id IS '用户 ID';
COMMENT ON COLUMN user_withdrawal_turnovers.cumulative_recharge_minor IS '用户累计真实充值本金（分）';
COMMENT ON COLUMN user_withdrawal_turnovers.required_effective_bet_minor IS '提现前需要完成的有效投注金额（分），当前规则为真实充值本金等额';
COMMENT ON COLUMN user_withdrawal_turnovers.completed_effective_bet_minor IS '用户已完成有效投注金额（分），投注扣款增加，投注退款扣回';
COMMENT ON COLUMN user_withdrawal_turnovers.created_at IS '累计记录创建时间';
COMMENT ON COLUMN user_withdrawal_turnovers.updated_at IS '累计记录最后更新时间';

COMMENT ON TABLE user_withdrawal_turnover_events IS '提现流水累计已处理资金流水事件表';
COMMENT ON COLUMN user_withdrawal_turnover_events.ledger_entry_id IS '已处理的资金流水 ID，防止资金快照重写时重复累计';
COMMENT ON COLUMN user_withdrawal_turnover_events.user_id IS '资金流水所属用户 ID';
COMMENT ON COLUMN user_withdrawal_turnover_events.kind IS '资金流水类型';
COMMENT ON COLUMN user_withdrawal_turnover_events.amount_minor IS '资金流水金额（分）';
COMMENT ON COLUMN user_withdrawal_turnover_events.created_at IS '累计事件记录创建时间';

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

WITH turnover_totals AS (
    SELECT
        user_id,
        COALESCE(SUM(CASE WHEN kind = 'rechargeCredit' AND amount_minor > 0 THEN amount_minor ELSE 0 END), 0)::BIGINT AS cumulative_recharge_minor,
        GREATEST(
            0,
            COALESCE(SUM(CASE
                WHEN kind IN ('orderDebit', 'groupBuyDebit') AND amount_minor < 0 THEN -amount_minor
                WHEN kind IN ('orderRefund', 'groupBuyRefund') AND amount_minor > 0 THEN -amount_minor
                ELSE 0
            END), 0)
        )::BIGINT AS completed_effective_bet_minor
    FROM ledger_entries
    WHERE kind IN ('rechargeCredit', 'orderDebit', 'orderRefund', 'groupBuyDebit', 'groupBuyRefund')
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
    completed_effective_bet_minor,
    now(),
    now()
FROM turnover_totals
WHERE cumulative_recharge_minor > 0 OR completed_effective_bet_minor > 0
ON CONFLICT (user_id) DO UPDATE SET
    cumulative_recharge_minor = EXCLUDED.cumulative_recharge_minor,
    required_effective_bet_minor = EXCLUDED.required_effective_bet_minor,
    completed_effective_bet_minor = EXCLUDED.completed_effective_bet_minor,
    updated_at = now();

CREATE OR REPLACE FUNCTION apply_user_withdrawal_turnover_from_ledger()
RETURNS trigger AS $$
DECLARE
    recharge_delta BIGINT := 0;
    effective_delta BIGINT := 0;
    processed_count INTEGER := 0;
BEGIN
    IF NEW.kind = 'rechargeCredit' AND NEW.amount_minor > 0 THEN
        recharge_delta := NEW.amount_minor;
    ELSIF NEW.kind IN ('orderDebit', 'groupBuyDebit') AND NEW.amount_minor < 0 THEN
        effective_delta := -NEW.amount_minor;
    ELSIF NEW.kind IN ('orderRefund', 'groupBuyRefund') AND NEW.amount_minor > 0 THEN
        effective_delta := -NEW.amount_minor;
    END IF;

    IF recharge_delta = 0 AND effective_delta = 0 THEN
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
        recharge_delta,
        recharge_delta,
        GREATEST(0, effective_delta),
        now(),
        now()
    )
    ON CONFLICT (user_id) DO UPDATE SET
        cumulative_recharge_minor = user_withdrawal_turnovers.cumulative_recharge_minor + EXCLUDED.cumulative_recharge_minor,
        required_effective_bet_minor = user_withdrawal_turnovers.required_effective_bet_minor + EXCLUDED.required_effective_bet_minor,
        completed_effective_bet_minor = GREATEST(
            0,
            user_withdrawal_turnovers.completed_effective_bet_minor + effective_delta
        ),
        updated_at = now();

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION apply_user_withdrawal_turnover_from_ledger() IS '根据新增资金流水增量维护用户提现流水要求累计数据';

DROP TRIGGER IF EXISTS ledger_entries_withdrawal_turnover_insert ON ledger_entries;

CREATE TRIGGER ledger_entries_withdrawal_turnover_insert
AFTER INSERT ON ledger_entries
FOR EACH ROW
EXECUTE FUNCTION apply_user_withdrawal_turnover_from_ledger();

COMMENT ON TRIGGER ledger_entries_withdrawal_turnover_insert ON ledger_entries IS '资金流水新增后自动更新用户提现流水累计';

CREATE INDEX IF NOT EXISTS user_withdrawal_turnover_events_user_created_idx
    ON user_withdrawal_turnover_events (user_id, created_at DESC);

COMMENT ON INDEX user_withdrawal_turnover_events_user_created_idx IS '按用户查看提现流水累计事件的辅助索引';
