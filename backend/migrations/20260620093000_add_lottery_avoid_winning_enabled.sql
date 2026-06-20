ALTER TABLE lotteries
ADD COLUMN IF NOT EXISTS avoid_winning_enabled BOOLEAN NOT NULL DEFAULT false;

COMMENT ON COLUMN lotteries.avoid_winning_enabled IS '是否开启彩种自动避开中奖策略，开启后开奖前会尝试生成不命中当前期号待开奖注单的开奖号码';
