ALTER TABLE admin_roles
    ADD COLUMN IF NOT EXISTS permissions JSONB NOT NULL DEFAULT '[]'::jsonb;

COMMENT ON COLUMN admin_roles.permissions IS '角色细粒度操作权限点（JSON）';
