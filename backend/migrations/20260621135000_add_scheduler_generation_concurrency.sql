ALTER TABLE draw_scheduler_config
ADD COLUMN local_issue_generation_concurrency INTEGER NOT NULL DEFAULT 4,
ADD COLUMN api_issue_generation_concurrency INTEGER NOT NULL DEFAULT 8;

COMMENT ON COLUMN draw_scheduler_config.local_issue_generation_concurrency IS '平台或手动彩种未来期号补期并发上限';
COMMENT ON COLUMN draw_scheduler_config.api_issue_generation_concurrency IS 'API 彩种未来期号计划生成并发上限';
