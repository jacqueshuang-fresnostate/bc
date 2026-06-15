ALTER TABLE robot_configs
    ADD COLUMN group_buy_fill_strategy TEXT NOT NULL DEFAULT 'rhythm',
    ADD COLUMN group_buy_fill_before_draw_seconds INTEGER NOT NULL DEFAULT 15;

ALTER TABLE robot_configs
    ADD CONSTRAINT robot_configs_group_buy_fill_strategy_check
    CHECK (group_buy_fill_strategy IN ('rhythm', 'beforeDraw'));

ALTER TABLE robot_configs
    ADD CONSTRAINT robot_configs_group_buy_fill_before_draw_seconds_check
    CHECK (group_buy_fill_before_draw_seconds BETWEEN 1 AND 86400);

COMMENT ON COLUMN robot_configs.group_buy_fill_strategy IS '合买机器人补满策略：rhythm 为阶段性补单，beforeDraw 为开奖前指定秒数直接补满';
COMMENT ON COLUMN robot_configs.group_buy_fill_before_draw_seconds IS '合买机器人开奖前多少秒触发一次性补满，只有 beforeDraw 策略生效';
COMMENT ON CONSTRAINT robot_configs_group_buy_fill_strategy_check ON robot_configs IS '约束合买机器人补满策略只能是 rhythm 或 beforeDraw';
COMMENT ON CONSTRAINT robot_configs_group_buy_fill_before_draw_seconds_check ON robot_configs IS '约束开奖前补满秒数必须在 1 到 86400 秒之间';
