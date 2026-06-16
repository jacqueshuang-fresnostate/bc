COMMENT ON COLUMN lotteries.sale_close_lead_seconds IS '彩种开盘后可售秒数，生成新期号时用本期开奖开盘时间加该秒数得到封盘时间，超过开奖时间时按开奖时间封盘';
COMMENT ON CONSTRAINT lotteries_sale_close_lead_seconds_check ON lotteries IS '约束彩种开盘后可售秒数必须大于0';
COMMENT ON COLUMN draw_scheduler_config.sale_close_lead_seconds IS '调度默认开盘后可售秒数，彩种未单独覆盖时使用';
