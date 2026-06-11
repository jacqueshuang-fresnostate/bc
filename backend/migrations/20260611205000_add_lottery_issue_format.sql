ALTER TABLE lotteries
ADD COLUMN issue_format TEXT NOT NULL DEFAULT '{yyyy}{MM}{dd}{HH}{mm}{ss}';

COMMENT ON COLUMN lotteries.issue_format IS '平台开奖期号生成格式，仅平台开奖模式生效；支持 {yyyy}/{yy}/{MM}/{dd}/{HH}/{mm}/{ss}/{date}/{time}/{timestamp} 变量';
