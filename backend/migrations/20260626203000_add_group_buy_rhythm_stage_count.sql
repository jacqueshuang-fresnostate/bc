ALTER TABLE robot_configs
    ADD COLUMN group_buy_rhythm_stage_count INTEGER NOT NULL DEFAULT 3;

ALTER TABLE robot_configs
    ADD CONSTRAINT robot_configs_group_buy_rhythm_stage_count_check
    CHECK (group_buy_rhythm_stage_count BETWEEN 1 AND 20);

COMMENT ON COLUMN robot_configs.group_buy_fill_before_draw_seconds IS '补单机器人开奖前多少秒触发最终补满，rhythm 和 beforeDraw 策略都使用；字段名沿用历史接口';
COMMENT ON COLUMN robot_configs.group_buy_rhythm_stage_count IS '补单机器人阶段性补单动态切分阶段数量，不包含开奖前最终补满阶段；只有 rhythm 策略生效';
COMMENT ON CONSTRAINT robot_configs_group_buy_rhythm_stage_count_check ON robot_configs IS '约束补单机器人阶段性补单阶段数量必须在 1 到 20 之间';
