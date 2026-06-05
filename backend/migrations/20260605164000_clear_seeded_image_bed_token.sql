UPDATE system_settings
SET value = '',
    description = '图床请求 Authorization Token（不含 Bearer 前缀，必须在后台手动配置）',
    updated_at = now()
WHERE key = 'image_bed_authorization_token'
  AND value <> ''
  AND value <> '未配置';

COMMENT ON COLUMN system_settings.value IS '配置值；敏感配置不得在种子数据或迁移中写入真实密钥';
