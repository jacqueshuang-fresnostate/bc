UPDATE lotteries
SET issue_format = '{date}{seq4}'
WHERE issue_format = '{yyyy}{MM}{dd}{HH}{mm}{ss}';

ALTER TABLE lotteries
ALTER COLUMN issue_format SET DEFAULT '{date}{seq4}';

COMMENT ON COLUMN lotteries.issue_format IS '平台开奖期号生成格式，仅平台开奖模式生效；默认 {date}{seq4}，支持 {yyyy}/{yy}/{MM}/{dd}/{HH}/{mm}/{ss}/{date}/{time}/{timestamp}/{seq4} 变量';
