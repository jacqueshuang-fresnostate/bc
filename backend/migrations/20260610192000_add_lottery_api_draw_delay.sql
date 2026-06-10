ALTER TABLE lotteries
ADD COLUMN api_draw_delay_seconds INTEGER NOT NULL DEFAULT 0;

ALTER TABLE lotteries
ADD CONSTRAINT lotteries_api_draw_delay_seconds_check
CHECK (api_draw_delay_seconds >= 0);

COMMENT ON COLUMN lotteries.api_draw_delay_seconds IS 'API开奖源延迟秒数，仅API开奖彩种生效；调度在官方开奖时间后等待该秒数再请求第三方开奖号码';
COMMENT ON CONSTRAINT lotteries_api_draw_delay_seconds_check ON lotteries IS '约束API开奖源延迟秒数不能小于0';
