ALTER TABLE lotteries
    ALTER COLUMN sale_enabled SET DEFAULT false;

ALTER TABLE lotteries
    ALTER COLUMN group_buy SET DEFAULT
        '{"enabled":false,"minShareAmountMinor":100,"initiatorMinPercent":10,"participantMinAmountMinor":1000}'::jsonb;

UPDATE lotteries
SET sale_enabled = false,
    group_buy = jsonb_set(group_buy, '{enabled}', 'false'::jsonb, true),
    updated_at = now();

COMMENT ON COLUMN lotteries.sale_enabled IS '是否对外开放销售，默认 false 表示新增和种子彩种默认停售';
COMMENT ON COLUMN lotteries.group_buy IS '合买配置，默认 enabled=false 表示新增和种子彩种默认关闭合买';
