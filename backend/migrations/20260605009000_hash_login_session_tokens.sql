DELETE FROM admin_sessions;
DELETE FROM user_sessions;

COMMENT ON COLUMN admin_sessions.token IS '管理员登录会话 token 的 SHA-256 摘要，不保存原始 Bearer token';
COMMENT ON COLUMN user_sessions.token IS '用户登录会话 token 的 SHA-256 摘要，不保存原始 Bearer token';
