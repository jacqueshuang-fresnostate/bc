UPDATE system_settings
SET description = '用户充值赠送活动档位，后台按元维护，系统保存为分'
WHERE key = 'recharge_bonus_rules';

UPDATE system_settings
SET description = '聊天大厅发言最低累计充值金额（元），0 表示不限制'
WHERE key = 'chat_hall_speaking_min_recharge_minor';
