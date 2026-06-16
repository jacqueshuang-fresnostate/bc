ALTER TABLE users
    ADD COLUMN IF NOT EXISTS registered_ip text NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS register_country text NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS register_region text NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS register_city text NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS register_geo_source text NOT NULL DEFAULT 'unknown';

COMMENT ON COLUMN users.registered_ip IS '用户注册时后端从请求头识别到的客户端 IP，允许为空';
COMMENT ON COLUMN users.register_country IS '用户注册地国家或地区，来自客户端定位或后续 IP 解析，允许为空';
COMMENT ON COLUMN users.register_region IS '用户注册地省份、州或时区等粗粒度区域，允许为空';
COMMENT ON COLUMN users.register_city IS '用户注册地城市，允许为空';
COMMENT ON COLUMN users.register_geo_source IS '用户注册地来源：gps 表示客户端定位，ip 表示请求 IP，client 表示客户端上报，unknown 表示未知';
