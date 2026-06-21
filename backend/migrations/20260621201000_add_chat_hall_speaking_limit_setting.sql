INSERT INTO system_settings (key, value, description)
VALUES
    ('chat_hall_speaking_min_recharge_minor', '0', '聊天大厅发言最低累计充值金额（分），0 表示不限制')
ON CONFLICT (key) DO NOTHING;
