DO $$
DECLARE
    target_ledger_sequence BIGINT;
BEGIN
    SELECT GREATEST(
        COALESCE((SELECT MAX(substring(id FROM 2)::BIGINT) FROM ledger_entries WHERE id ~ '^L[0-9]+$'), 0),
        COALESCE((SELECT value FROM finance_runtime WHERE key = 'next_sequence'), 0),
        COALESCE((SELECT last_value FROM ledger_entry_id_sequence), 0)
    )
    INTO target_ledger_sequence;

    IF target_ledger_sequence <= 0 THEN
        PERFORM setval('ledger_entry_id_sequence', 1, false);
    ELSE
        PERFORM setval('ledger_entry_id_sequence', target_ledger_sequence, true);
    END IF;
END $$;

COMMENT ON SEQUENCE ledger_entry_id_sequence IS '资金流水 ID 数据库序列，数据库模式所有新增资金流水统一使用 L + nextval(''ledger_entry_id_sequence'') 生成，避免多取号源冲突';
