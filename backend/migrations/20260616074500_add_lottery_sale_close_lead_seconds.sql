ALTER TABLE lotteries
ADD COLUMN sale_close_lead_seconds INTEGER NOT NULL DEFAULT 1;

ALTER TABLE lotteries
ADD CONSTRAINT lotteries_sale_close_lead_seconds_check
CHECK (sale_close_lead_seconds > 0);

COMMENT ON COLUMN lotteries.sale_close_lead_seconds IS '彩种封盘提前秒数，生成新期号时用计划开奖时间减去该秒数得到封盘时间';
COMMENT ON CONSTRAINT lotteries_sale_close_lead_seconds_check ON lotteries IS '约束彩种封盘提前秒数必须大于0';
COMMENT ON COLUMN lotteries.issue_format IS '平台开奖期号生成格式，仅平台开奖模式生效；默认 {date}{seq4}，支持 {yyyy}/{yy}/{MM}/{dd}/{HH}/{mm}/{ss}/{date}/{time}/{timestamp}/{seq1}/{seq2}/{seq3}/{seq4} 变量';
