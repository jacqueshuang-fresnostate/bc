CREATE TABLE IF NOT EXISTS robot_scheduler_config (
    id TEXT PRIMARY KEY,
    enabled BOOLEAN NOT NULL,
    interval_seconds BIGINT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT robot_scheduler_config_singleton_check CHECK (id = 'default'),
    CONSTRAINT robot_scheduler_config_interval_check CHECK (interval_seconds > 0)
);

CREATE TABLE IF NOT EXISTS robot_scheduler_runs (
    id TEXT PRIMARY KEY,
    "trigger" TEXT NOT NULL,
    status TEXT NOT NULL,
    started_at TEXT NOT NULL,
    finished_at TEXT NOT NULL,
    now TEXT NOT NULL,
    error TEXT,
    created_plan_count INTEGER NOT NULL,
    filled_plan_count INTEGER NOT NULL,
    created_order_count INTEGER NOT NULL,
    ledger_entry_count INTEGER NOT NULL,
    skipped_item_count INTEGER NOT NULL,
    skipped_items JSONB NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS robot_scheduler_runtime (
    key TEXT PRIMARY KEY,
    value BIGINT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

COMMENT ON TABLE robot_scheduler_config IS '机器人独立调度器配置表（单例）';
COMMENT ON COLUMN robot_scheduler_config.id IS '配置主键，固定为 default';
COMMENT ON COLUMN robot_scheduler_config.enabled IS '是否启用机器人独立调度';
COMMENT ON COLUMN robot_scheduler_config.interval_seconds IS '机器人独立调度执行周期（秒）';
COMMENT ON COLUMN robot_scheduler_config.updated_at IS '配置更新时间';
COMMENT ON CONSTRAINT robot_scheduler_config_singleton_check ON robot_scheduler_config IS '约束机器人调度配置只能保留 default 单例';
COMMENT ON CONSTRAINT robot_scheduler_config_interval_check ON robot_scheduler_config IS '约束机器人调度执行周期必须大于 0 秒';

COMMENT ON TABLE robot_scheduler_runs IS '机器人独立调度执行历史表';
COMMENT ON COLUMN robot_scheduler_runs.id IS '机器人调度执行记录 ID';
COMMENT ON COLUMN robot_scheduler_runs."trigger" IS '触发来源，automatic 表示后台常驻任务自动触发';
COMMENT ON COLUMN robot_scheduler_runs.status IS '执行结果状态，success 表示成功，failed 表示失败';
COMMENT ON COLUMN robot_scheduler_runs.started_at IS '本轮执行开始时间';
COMMENT ON COLUMN robot_scheduler_runs.finished_at IS '本轮执行完成时间';
COMMENT ON COLUMN robot_scheduler_runs.now IS '本轮业务时间快照';
COMMENT ON COLUMN robot_scheduler_runs.error IS '失败原因，成功时为空';
COMMENT ON COLUMN robot_scheduler_runs.created_plan_count IS '本轮机器人创建合买计划数量';
COMMENT ON COLUMN robot_scheduler_runs.filled_plan_count IS '本轮机器人补满合买计划数量';
COMMENT ON COLUMN robot_scheduler_runs.created_order_count IS '本轮满单后生成投注订单数量';
COMMENT ON COLUMN robot_scheduler_runs.ledger_entry_count IS '本轮机器人产生资金流水数量';
COMMENT ON COLUMN robot_scheduler_runs.skipped_item_count IS '本轮机器人跳过明细数量';
COMMENT ON COLUMN robot_scheduler_runs.skipped_items IS '本轮机器人跳过明细 JSON';
COMMENT ON COLUMN robot_scheduler_runs.updated_at IS '记录更新时间';

COMMENT ON TABLE robot_scheduler_runtime IS '机器人独立调度运行时计数器';
COMMENT ON COLUMN robot_scheduler_runtime.key IS '运行时 Key';
COMMENT ON COLUMN robot_scheduler_runtime.value IS 'Key 对应数值';
COMMENT ON COLUMN robot_scheduler_runtime.updated_at IS '更新时间';
