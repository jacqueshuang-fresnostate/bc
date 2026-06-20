CREATE TABLE users (
    id TEXT PRIMARY KEY,
    username TEXT NOT NULL,
    email TEXT,
    kind TEXT NOT NULL,
    status TEXT NOT NULL,
    balance_minor BIGINT NOT NULL,
    agent_id TEXT,
    invite_code TEXT NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE admin_roles (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    scopes JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE admins (
    id TEXT PRIMARY KEY,
    username TEXT NOT NULL UNIQUE,
    role_id TEXT NOT NULL,
    role_name TEXT NOT NULL,
    status TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE admin_password_hashes (
    admin_id TEXT PRIMARY KEY,
    password_hash TEXT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE admin_sessions (
    token TEXT PRIMARY KEY,
    admin_id TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE system_settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    description TEXT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE registration_config (
    id TEXT PRIMARY KEY,
    username_enabled BOOLEAN NOT NULL,
    email_enabled BOOLEAN NOT NULL,
    agent_invite_required BOOLEAN NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT registration_config_singleton_check CHECK (id = 'default')
);

CREATE TABLE access_runtime (
    key TEXT PRIMARY KEY,
    value BIGINT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE draw_issues (
    id TEXT PRIMARY KEY,
    lottery_id TEXT NOT NULL,
    lottery_name TEXT NOT NULL,
    issue TEXT NOT NULL,
    number_type TEXT NOT NULL,
    draw_mode TEXT NOT NULL,
    scheduled_at TEXT NOT NULL,
    sale_closed_at TEXT NOT NULL,
    status TEXT NOT NULL,
    draw_number TEXT,
    drawn_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (lottery_id, issue)
);

CREATE TABLE draw_controls (
    lottery_id TEXT PRIMARY KEY,
    enabled BOOLEAN NOT NULL,
    draw_number TEXT,
    updated_at TEXT NOT NULL
);

CREATE TABLE draw_sources (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    provider TEXT NOT NULL,
    lot_code TEXT NOT NULL,
    endpoint TEXT NOT NULL,
    reusable_for_lottery_ids JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE orders (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    lottery_id TEXT NOT NULL,
    lottery_name TEXT NOT NULL,
    issue TEXT NOT NULL,
    rule_code TEXT NOT NULL,
    number_type TEXT NOT NULL,
    selection JSONB NOT NULL,
    stake_count INTEGER NOT NULL,
    unit_amount_minor BIGINT NOT NULL,
    amount_minor BIGINT NOT NULL,
    odds_basis_points BIGINT NOT NULL,
    expanded_bets JSONB NOT NULL,
    draw_number TEXT,
    matched_bets JSONB NOT NULL,
    payout_minor BIGINT NOT NULL,
    status TEXT NOT NULL,
    settled_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE order_settlement_runs (
    id TEXT PRIMARY KEY,
    draw_issue_id TEXT NOT NULL,
    lottery_id TEXT NOT NULL,
    lottery_name TEXT NOT NULL,
    issue TEXT NOT NULL,
    draw_number TEXT NOT NULL,
    settled_order_count INTEGER NOT NULL,
    winning_order_count INTEGER NOT NULL,
    total_stake_amount_minor BIGINT NOT NULL,
    total_payout_minor BIGINT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE order_settlements (
    settlement_id TEXT NOT NULL,
    order_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    rule_code TEXT NOT NULL,
    stake_count INTEGER NOT NULL,
    amount_minor BIGINT NOT NULL,
    is_winning BOOLEAN NOT NULL,
    matched_bets JSONB NOT NULL,
    odds_basis_points BIGINT NOT NULL,
    payout_minor BIGINT NOT NULL,
    status TEXT NOT NULL,
    PRIMARY KEY (settlement_id, order_id)
);

CREATE TABLE order_runtime (
    key TEXT PRIMARY KEY,
    value BIGINT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE financial_accounts (
    user_id TEXT PRIMARY KEY,
    available_balance_minor BIGINT NOT NULL,
    frozen_balance_minor BIGINT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE ledger_entries (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    kind TEXT NOT NULL,
    amount_minor BIGINT NOT NULL,
    balance_after_minor BIGINT NOT NULL,
    reference_id TEXT,
    description TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE TABLE finance_runtime (
    key TEXT PRIMARY KEY,
    value BIGINT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE group_buy_plans (
    id TEXT PRIMARY KEY,
    lottery_id TEXT NOT NULL,
    lottery_name TEXT NOT NULL,
    initiator_user_id TEXT NOT NULL,
    initiator_username TEXT NOT NULL,
    total_amount_minor BIGINT NOT NULL,
    filled_amount_minor BIGINT NOT NULL,
    min_share_amount_minor BIGINT NOT NULL,
    participant_min_amount_minor BIGINT NOT NULL,
    share_count INTEGER NOT NULL,
    status TEXT NOT NULL,
    note TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE group_buy_participants (
    id TEXT PRIMARY KEY,
    plan_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    username TEXT NOT NULL,
    amount_minor BIGINT NOT NULL,
    share_count INTEGER NOT NULL,
    note TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE TABLE invite_records (
    id TEXT PRIMARY KEY,
    inviter_user_id TEXT NOT NULL,
    inviter_username TEXT NOT NULL,
    invitee_user_id TEXT NOT NULL,
    invitee_username TEXT NOT NULL,
    invite_code TEXT NOT NULL,
    status TEXT NOT NULL,
    rebate_enabled BOOLEAN NOT NULL,
    note TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE (inviter_user_id, invitee_user_id)
);

CREATE TABLE rebate_policy (
    id TEXT PRIMARY KEY,
    agents_can_invite BOOLEAN NOT NULL,
    regular_users_can_invite BOOLEAN NOT NULL,
    rebate_mode TEXT NOT NULL,
    default_recharge_rebate_basis_points INTEGER NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT rebate_policy_singleton_check CHECK (id = 'default')
);

CREATE TABLE robot_configs (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    kind TEXT NOT NULL,
    status TEXT NOT NULL,
    description TEXT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE robot_lottery_bindings (
    robot_id TEXT NOT NULL,
    lottery_id TEXT NOT NULL,
    PRIMARY KEY (robot_id, lottery_id)
);

CREATE TABLE support_conversations (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    username TEXT NOT NULL,
    subject TEXT NOT NULL,
    status TEXT NOT NULL,
    priority TEXT NOT NULL,
    assigned_admin_id TEXT,
    assigned_admin_name TEXT,
    unread_count INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE support_messages (
    id TEXT PRIMARY KEY,
    conversation_id TEXT NOT NULL,
    author TEXT NOT NULL,
    author_id TEXT NOT NULL,
    author_name TEXT NOT NULL,
    content TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE TABLE draw_scheduler_config (
    id TEXT PRIMARY KEY,
    enabled BOOLEAN NOT NULL,
    interval_seconds BIGINT NOT NULL,
    future_issue_count INTEGER NOT NULL,
    sale_close_lead_seconds INTEGER NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT draw_scheduler_config_singleton_check CHECK (id = 'default')
);

CREATE TABLE draw_scheduler_runs (
    id TEXT PRIMARY KEY,
    trigger TEXT NOT NULL,
    status TEXT NOT NULL,
    started_at TEXT NOT NULL,
    finished_at TEXT NOT NULL,
    now TEXT NOT NULL,
    error TEXT,
    closed_issue_count INTEGER NOT NULL,
    drawn_issue_count INTEGER NOT NULL,
    settlement_run_count INTEGER NOT NULL,
    ledger_entry_count INTEGER NOT NULL,
    generated_issue_count INTEGER NOT NULL,
    skipped_issue_count INTEGER NOT NULL,
    skipped_lottery_count INTEGER NOT NULL
);

CREATE TABLE draw_scheduler_runtime (
    key TEXT PRIMARY KEY,
    value BIGINT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX draw_issues_lottery_issue_idx ON draw_issues (lottery_id, issue);
CREATE INDEX orders_lottery_issue_idx ON orders (lottery_id, issue);
CREATE INDEX orders_user_id_idx ON orders (user_id);
CREATE INDEX ledger_entries_user_id_idx ON ledger_entries (user_id);
CREATE INDEX group_buy_participants_plan_id_idx ON group_buy_participants (plan_id);
CREATE INDEX support_messages_conversation_id_idx ON support_messages (conversation_id);
