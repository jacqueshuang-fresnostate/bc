INSERT INTO system_settings (key, value, description)
VALUES
    ('mobile_home_featured_enabled', 'false', '手机端首页高频极速模块开关，默认关闭'),
    ('mobile_home_featured_title', '高频极速', '手机端首页高频极速模块标题'),
    ('mobile_home_featured_lottery_codes', '', '手机端首页高频极速展示彩种 ID，多个用英文逗号分隔')
ON CONFLICT (key) DO NOTHING;
