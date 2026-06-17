CREATE TABLE IF NOT EXISTS api_draw_source_snapshots (
    id text PRIMARY KEY,
    source_id text NOT NULL,
    source_name text NOT NULL,
    provider text NOT NULL,
    lottery_id text NOT NULL,
    request_kind text NOT NULL,
    requested_issue text,
    latest_issue text,
    latest_draw_time text,
    next_issue text,
    next_draw_time text,
    draw_number text,
    endpoint text NOT NULL,
    lot_code text NOT NULL DEFAULT '',
    http_status integer,
    success boolean NOT NULL DEFAULT false,
    error_message text,
    raw_response jsonb,
    raw_response_text text NOT NULL DEFAULT '',
    crawled_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_api_draw_source_snapshots_lottery_time
    ON api_draw_source_snapshots (lottery_id, crawled_at DESC);

CREATE INDEX IF NOT EXISTS idx_api_draw_source_snapshots_source_time
    ON api_draw_source_snapshots (source_id, crawled_at DESC);

CREATE INDEX IF NOT EXISTS idx_api_draw_source_snapshots_requested_issue
    ON api_draw_source_snapshots (lottery_id, requested_issue, crawled_at DESC)
    WHERE requested_issue IS NOT NULL;

COMMENT ON TABLE api_draw_source_snapshots IS 'API 开奖源每次抓取的快照记录，用于对比第三方期号、开奖号码和本系统处理结果';
COMMENT ON COLUMN api_draw_source_snapshots.id IS '采集快照编号，由后端生成，保证每次抓取唯一';
COMMENT ON COLUMN api_draw_source_snapshots.source_id IS '开奖源配置编号，对应 draw_sources.id';
COMMENT ON COLUMN api_draw_source_snapshots.source_name IS '开奖源配置名称，保存抓取当时的展示名称';
COMMENT ON COLUMN api_draw_source_snapshots.provider IS '开奖源供应商类型，如 api68、kjApi、bbKaijiang、indonesiaLottery';
COMMENT ON COLUMN api_draw_source_snapshots.lottery_id IS '本系统彩种编号';
COMMENT ON COLUMN api_draw_source_snapshots.request_kind IS '采集用途：latestIssue 表示同步最新期号，drawNumber 表示按期号获取开奖号码';
COMMENT ON COLUMN api_draw_source_snapshots.requested_issue IS '按期号获取开奖号码时请求的本系统期号，最新期号同步时为空';
COMMENT ON COLUMN api_draw_source_snapshots.latest_issue IS '接口解析出的最新已开奖期号';
COMMENT ON COLUMN api_draw_source_snapshots.latest_draw_time IS '接口解析出的最新已开奖时间，按第三方原始时间字符串保存';
COMMENT ON COLUMN api_draw_source_snapshots.next_issue IS '接口解析出的下一期期号';
COMMENT ON COLUMN api_draw_source_snapshots.next_draw_time IS '接口解析出的下一期开奖时间，按第三方原始时间字符串保存';
COMMENT ON COLUMN api_draw_source_snapshots.draw_number IS '接口解析出的开奖号码，统一保存为逗号分隔格式';
COMMENT ON COLUMN api_draw_source_snapshots.endpoint IS '实际请求的开奖源完整地址，包含 lotCode、lotKey 或 gameCodeList 等查询参数';
COMMENT ON COLUMN api_draw_source_snapshots.lot_code IS '抓取时使用的开奖源编码，如 lotCode、lotKey 或 gameCodeList';
COMMENT ON COLUMN api_draw_source_snapshots.http_status IS '第三方接口 HTTP 状态码，静态测试或请求未建立时为空';
COMMENT ON COLUMN api_draw_source_snapshots.success IS '本次采集和解析是否成功';
COMMENT ON COLUMN api_draw_source_snapshots.error_message IS '本次采集失败或解析失败的错误信息，成功时为空';
COMMENT ON COLUMN api_draw_source_snapshots.raw_response IS '第三方接口原始响应的 JSON 结构，非 JSON 响应时为空';
COMMENT ON COLUMN api_draw_source_snapshots.raw_response_text IS '第三方接口原始响应文本，便于排查 JSON 解析失败或字段变化';
COMMENT ON COLUMN api_draw_source_snapshots.crawled_at IS '本次抓取快照写入数据库的时间';
