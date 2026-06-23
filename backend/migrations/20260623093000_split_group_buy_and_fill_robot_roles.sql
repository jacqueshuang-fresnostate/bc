UPDATE robot_configs
SET
    name = '合买发单机器人',
    kind = 'groupBuy',
    description = '只负责发起合买计划到合买大厅',
    group_buy_fill_strategy = 'rhythm',
    group_buy_fill_before_draw_seconds = 15,
    updated_at = now()
WHERE id = 'R-GROUP-001';

UPDATE robot_configs
SET
    name = '合买补单机器人',
    kind = 'purchase',
    status = CASE WHEN status = 'paused' THEN 'enabled' ELSE status END,
    description = '只负责认购合买大厅未满单计划',
    updated_at = now()
WHERE id = 'R-BUY-001';

COMMENT ON COLUMN robot_configs.group_buy_fill_strategy IS '补单机器人补满策略：rhythm 为阶段性补单，beforeDraw 为开奖前指定秒数直接补满；字段名沿用历史接口';
COMMENT ON COLUMN robot_configs.group_buy_fill_before_draw_seconds IS '补单机器人开奖前多少秒触发一次性补满，只有 beforeDraw 策略生效；字段名沿用历史接口';
COMMENT ON CONSTRAINT robot_configs_group_buy_fill_strategy_check ON robot_configs IS '约束补单机器人补满策略只能是 rhythm 或 beforeDraw；字段名沿用历史接口';
COMMENT ON CONSTRAINT robot_configs_group_buy_fill_before_draw_seconds_check ON robot_configs IS '约束补单机器人开奖前补满秒数必须在 1 到 86400 秒之间；字段名沿用历史接口';
