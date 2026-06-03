ALTER TABLE lotteries
ADD COLUMN category TEXT NOT NULL DEFAULT 'regional';

UPDATE lotteries
SET category = 'regional';

UPDATE lotteries
SET category = 'overseas'
WHERE id IN ('au5', 'txffc');

ALTER TABLE lotteries
ADD CONSTRAINT lotteries_category_check
CHECK (category IN ('regional', 'overseas', 'welfare', 'other'));

COMMENT ON COLUMN lotteries.category IS '彩种分类：regional（地方彩种）、overseas（海外彩种）、welfare（福利彩种）、other（其他）';

COMMENT ON CONSTRAINT lotteries_category_check ON lotteries IS '约束 category 只允许 regional/overseas/welfare/other';
