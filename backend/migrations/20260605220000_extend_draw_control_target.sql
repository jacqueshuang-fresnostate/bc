ALTER TABLE draw_controls
    ADD COLUMN IF NOT EXISTS target_scope TEXT NOT NULL DEFAULT 'lottery',
    ADD COLUMN IF NOT EXISTS target_issue TEXT,
    ADD COLUMN IF NOT EXISTS target_order_id TEXT;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'draw_controls_target_scope_check'
          AND conrelid = 'draw_controls'::regclass
    ) THEN
        ALTER TABLE draw_controls
            ADD CONSTRAINT draw_controls_target_scope_check
            CHECK (target_scope IN ('lottery', 'issue', 'order'));
    END IF;
END $$;

COMMENT ON COLUMN draw_controls.target_scope IS '开奖控制范围：lottery 整个彩种，issue 指定期号，order 指定订单所在期号';
COMMENT ON COLUMN draw_controls.target_issue IS '开奖控制目标期号，范围为 issue 或 order 时必填';
COMMENT ON COLUMN draw_controls.target_order_id IS '开奖控制目标订单 ID，范围为 order 时必填，仅用于定位与审计';
COMMENT ON CONSTRAINT draw_controls_target_scope_check ON draw_controls IS '限制开奖控制范围只能为 lottery、issue 或 order';
