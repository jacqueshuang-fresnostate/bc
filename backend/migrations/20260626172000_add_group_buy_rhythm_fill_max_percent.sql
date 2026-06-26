ALTER TABLE robot_configs
    ADD COLUMN group_buy_rhythm_fill_max_percent INTEGER NOT NULL DEFAULT 20;

ALTER TABLE robot_configs
    ADD CONSTRAINT robot_configs_group_buy_rhythm_fill_max_percent_check
    CHECK (group_buy_rhythm_fill_max_percent BETWEEN 1 AND 100);

COMMENT ON COLUMN robot_configs.group_buy_rhythm_fill_max_percent IS '补单机器人阶段性补单单阶段最高百分比，按合买总金额计算；只有 rhythm 策略生效';
COMMENT ON CONSTRAINT robot_configs_group_buy_rhythm_fill_max_percent_check ON robot_configs IS '约束补单机器人阶段性补单单阶段最高百分比必须在 1 到 100 之间';
