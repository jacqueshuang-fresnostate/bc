COMMENT ON COLUMN lotteries.sale_close_lead_seconds IS '彩种开奖前封盘提前秒数，生成新期号时用计划开奖时间减去该秒数得到封盘时间，例如300秒周期配置60秒时在周期进行到240秒封盘';
COMMENT ON CONSTRAINT lotteries_sale_close_lead_seconds_check ON lotteries IS '约束彩种封盘提前秒数必须大于0';
COMMENT ON COLUMN draw_scheduler_config.sale_close_lead_seconds IS '调度默认开奖前封盘提前秒数，彩种未单独覆盖时使用';
