CREATE TABLE agent_applications (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    username TEXT NOT NULL,
    invite_code TEXT NOT NULL,
    status TEXT NOT NULL,
    reason TEXT NOT NULL,
    review_note TEXT,
    reviewed_by_admin_id TEXT,
    reviewed_by_admin_username TEXT,
    reviewed_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CONSTRAINT agent_applications_status_check CHECK (status IN ('pending', 'approved', 'rejected'))
);

CREATE INDEX agent_applications_status_created_idx
    ON agent_applications (status, created_at DESC, id DESC);

CREATE INDEX agent_applications_user_id_created_idx
    ON agent_applications (user_id, created_at DESC, id DESC);

COMMENT ON TABLE agent_applications IS '代理申请表，保存普通用户申请成为代理以及后台审核结果';
COMMENT ON COLUMN agent_applications.id IS '代理申请 ID，由后端按 AGAPP 序列生成';
COMMENT ON COLUMN agent_applications.user_id IS '申请用户 ID，对应 users.id';
COMMENT ON COLUMN agent_applications.username IS '申请时的用户名快照，方便后台审核列表直接展示';
COMMENT ON COLUMN agent_applications.invite_code IS '申请用户的邀请码快照，审核通过后该码具备邀请能力';
COMMENT ON COLUMN agent_applications.status IS '审核状态：pending（待审核）、approved（已通过）、rejected（已驳回）';
COMMENT ON COLUMN agent_applications.reason IS '用户填写的申请说明';
COMMENT ON COLUMN agent_applications.review_note IS '后台审核备注，可为空';
COMMENT ON COLUMN agent_applications.reviewed_by_admin_id IS '执行审核的管理员 ID，可为空';
COMMENT ON COLUMN agent_applications.reviewed_by_admin_username IS '执行审核的管理员用户名快照，可为空';
COMMENT ON COLUMN agent_applications.reviewed_at IS '审核完成时间，格式为 YYYY-MM-DD HH:MM:SS，可为空';
COMMENT ON COLUMN agent_applications.created_at IS '申请创建时间，格式为 YYYY-MM-DD HH:MM:SS';
COMMENT ON COLUMN agent_applications.updated_at IS '申请更新时间，格式为 YYYY-MM-DD HH:MM:SS';
COMMENT ON CONSTRAINT agent_applications_status_check ON agent_applications IS '限制代理申请审核状态枚举值';
