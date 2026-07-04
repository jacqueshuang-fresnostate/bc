INSERT INTO system_settings (key, value, description)
VALUES (
    'mobile_default_avatar_url',
    '未配置',
    '用户和机器人未配置头像时使用的默认头像图片链接'
)
ON CONFLICT (key) DO UPDATE
SET description = EXCLUDED.description;
