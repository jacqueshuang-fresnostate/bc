CREATE INDEX IF NOT EXISTS idx_users_agent_id
    ON users (agent_id)
    WHERE agent_id IS NOT NULL;

COMMENT ON INDEX idx_users_agent_id IS '支持后台用户管理按上级代理筛选直属下级的分页查询';
