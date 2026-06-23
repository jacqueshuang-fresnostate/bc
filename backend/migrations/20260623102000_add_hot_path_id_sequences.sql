CREATE SEQUENCE IF NOT EXISTS order_id_sequence;
CREATE SEQUENCE IF NOT EXISTS ledger_entry_id_sequence;

DO $$
DECLARE
    target_order_sequence BIGINT;
    target_ledger_sequence BIGINT;
BEGIN
    SELECT GREATEST(
        COALESCE((SELECT MAX(substring(id FROM 2)::BIGINT) FROM orders WHERE id ~ '^O[0-9]+$'), 0),
        COALESCE((SELECT value FROM order_runtime WHERE key = 'next_sequence'), 0)
    )
    INTO target_order_sequence;

    IF target_order_sequence <= 0 THEN
        PERFORM setval('order_id_sequence', 1, false);
    ELSE
        PERFORM setval('order_id_sequence', target_order_sequence, true);
    END IF;

    SELECT GREATEST(
        COALESCE((SELECT MAX(substring(id FROM 2)::BIGINT) FROM ledger_entries WHERE id ~ '^L[0-9]+$'), 0),
        COALESCE((SELECT value FROM finance_runtime WHERE key = 'next_sequence'), 0)
    )
    INTO target_ledger_sequence;

    IF target_ledger_sequence <= 0 THEN
        PERFORM setval('ledger_entry_id_sequence', 1, false);
    ELSE
        PERFORM setval('ledger_entry_id_sequence', target_ledger_sequence, true);
    END IF;
END $$;

COMMENT ON SEQUENCE order_id_sequence IS '普通下注热路径订单 ID 序列，替代 order_runtime 行锁取号，降低不同用户并发下注阻塞';
COMMENT ON SEQUENCE ledger_entry_id_sequence IS '普通下注热路径资金流水 ID 序列，替代 finance_runtime 行锁取号，降低不同用户并发扣款阻塞';
