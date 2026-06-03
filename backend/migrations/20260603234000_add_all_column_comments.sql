COMMENT ON TABLE lotteries IS '彩种配置表，保存每个彩种的基础参数、玩法与调度能力';
COMMENT ON COLUMN lotteries.id IS '彩种唯一标识符';
COMMENT ON COLUMN lotteries.name IS '彩种展示名称';
COMMENT ON COLUMN lotteries.number_type IS '开奖号码位数类型，支持 threeDigit/fiveDigit';
COMMENT ON COLUMN lotteries.draw_mode IS '开奖模式：platform（平台开奖）、api（采集开奖）、manual（手工开奖）';
COMMENT ON COLUMN lotteries.schedule IS '开奖号码生成与开奖时序配置（JSON）';
COMMENT ON COLUMN lotteries.sale_enabled IS '是否对外开放销售';
COMMENT ON COLUMN lotteries.group_buy IS '合买配置（例如是否允许、相关阈值）';
COMMENT ON COLUMN lotteries.play_categories IS '玩法分类配置（JSON）';
COMMENT ON COLUMN lotteries.play_configs IS '玩法赔率与选号约束的动态配置';
COMMENT ON COLUMN lotteries.created_at IS '记录创建时间';
COMMENT ON COLUMN lotteries.updated_at IS '记录更新时间';
COMMENT ON CONSTRAINT lotteries_number_type_check ON lotteries IS '约束 number_type 只允许 threeDigit 或 fiveDigit';
COMMENT ON CONSTRAINT lotteries_draw_mode_check ON lotteries IS '约束 draw_mode 只允许 platform/api/manual';

COMMENT ON TABLE state_documents IS '历史遗留状态文档表，仅保留兼容用途';
COMMENT ON COLUMN state_documents.namespace IS '状态命名空间主键';
COMMENT ON COLUMN state_documents.payload IS '状态内容 JSON 文档快照';
COMMENT ON COLUMN state_documents.created_at IS '创建时间';
COMMENT ON COLUMN state_documents.updated_at IS '更新时间';

COMMENT ON TABLE users IS '用户基础信息表';
COMMENT ON COLUMN users.id IS '用户唯一 ID';
COMMENT ON COLUMN users.username IS '登录用户名';
COMMENT ON COLUMN users.email IS '绑定邮箱，可用于账号标识与通知';
COMMENT ON COLUMN users.kind IS '用户类型，如 user、agent、admin';
COMMENT ON COLUMN users.status IS '用户状态：active、disabled、locked 等';
COMMENT ON COLUMN users.balance_minor IS '可用余额（分）';
COMMENT ON COLUMN users.agent_id IS '所属代理 ID，仅用于代理关系';
COMMENT ON COLUMN users.invite_code IS '用户邀请码';
COMMENT ON COLUMN users.created_at IS '注册时间';
COMMENT ON COLUMN users.updated_at IS '最后更新时间';

COMMENT ON TABLE admin_roles IS '管理员角色表';
COMMENT ON COLUMN admin_roles.id IS '角色唯一 ID';
COMMENT ON COLUMN admin_roles.name IS '角色名称';
COMMENT ON COLUMN admin_roles.scopes IS '角色权限范围（JSON）';
COMMENT ON COLUMN admin_roles.created_at IS '创建时间';
COMMENT ON COLUMN admin_roles.updated_at IS '更新时间';

COMMENT ON TABLE admins IS '管理员账号表';
COMMENT ON COLUMN admins.id IS '管理员唯一 ID';
COMMENT ON COLUMN admins.username IS '登录名';
COMMENT ON COLUMN admins.role_id IS '关联的角色 ID';
COMMENT ON COLUMN admins.role_name IS '角色名称快照（冗余）';
COMMENT ON COLUMN admins.status IS '状态：active 或 disabled';
COMMENT ON COLUMN admins.created_at IS '创建时间';
COMMENT ON COLUMN admins.updated_at IS '更新时间';

COMMENT ON TABLE admin_password_hashes IS '管理员密码哈希表';
COMMENT ON COLUMN admin_password_hashes.admin_id IS '管理员 ID';
COMMENT ON COLUMN admin_password_hashes.password_hash IS '加密后的密码哈希值';
COMMENT ON COLUMN admin_password_hashes.updated_at IS '密码更新时间';

COMMENT ON TABLE admin_sessions IS '管理员会话表';
COMMENT ON COLUMN admin_sessions.token IS '会话 Token';
COMMENT ON COLUMN admin_sessions.admin_id IS '关联管理员 ID';
COMMENT ON COLUMN admin_sessions.created_at IS '会话创建时间';

COMMENT ON TABLE system_settings IS '系统配置表';
COMMENT ON COLUMN system_settings.key IS '配置键';
COMMENT ON COLUMN system_settings.value IS '配置值';
COMMENT ON COLUMN system_settings.description IS '配置说明';
COMMENT ON COLUMN system_settings.updated_at IS '最后更新时间';

COMMENT ON TABLE registration_config IS '注册策略配置表（单例）';
COMMENT ON COLUMN registration_config.id IS '配置主键，固定为 default';
COMMENT ON COLUMN registration_config.username_enabled IS '是否允许用户名注册';
COMMENT ON COLUMN registration_config.email_enabled IS '是否允许邮箱注册';
COMMENT ON COLUMN registration_config.agent_invite_required IS '代理是否必须填写/使用邀请码';
COMMENT ON COLUMN registration_config.updated_at IS '更新时间';
COMMENT ON CONSTRAINT registration_config_singleton_check ON registration_config IS '约束 id 必须为 default';

COMMENT ON TABLE access_runtime IS '访问运行时计数器表';
COMMENT ON COLUMN access_runtime.key IS '运行时 Key';
COMMENT ON COLUMN access_runtime.value IS 'Key 对应数值';
COMMENT ON COLUMN access_runtime.updated_at IS '更新时间';

COMMENT ON TABLE draw_issues IS '开奖期号表';
COMMENT ON COLUMN draw_issues.id IS '期号唯一 ID';
COMMENT ON COLUMN draw_issues.lottery_id IS '所属彩种 ID';
COMMENT ON COLUMN draw_issues.lottery_name IS '彩种名称快照';
COMMENT ON COLUMN draw_issues.issue IS '期号';
COMMENT ON COLUMN draw_issues.number_type IS '对应号码位数类型';
COMMENT ON COLUMN draw_issues.draw_mode IS '该期开奖模式';
COMMENT ON COLUMN draw_issues.scheduled_at IS '计划开奖时间（字符串）';
COMMENT ON COLUMN draw_issues.sale_closed_at IS '销售截止时间';
COMMENT ON COLUMN draw_issues.status IS '期号状态：open/closed/drawn/cancelled';
COMMENT ON COLUMN draw_issues.draw_number IS '开奖号码（逗号分隔）';
COMMENT ON COLUMN draw_issues.drawn_at IS '开奖时间（字符串）';
COMMENT ON COLUMN draw_issues.created_at IS '创建时间';
COMMENT ON COLUMN draw_issues.updated_at IS '更新时间';

COMMENT ON TABLE draw_controls IS '开奖控制表（手动/测试干预）';
COMMENT ON COLUMN draw_controls.lottery_id IS '关联彩种 ID';
COMMENT ON COLUMN draw_controls.enabled IS '是否开启控制模式';
COMMENT ON COLUMN draw_controls.draw_number IS '手工控制开奖号码（逗号分隔）';
COMMENT ON COLUMN draw_controls.updated_at IS '最近更新时间';

COMMENT ON TABLE draw_sources IS '开奖源配置表';
COMMENT ON COLUMN draw_sources.id IS '开奖源唯一 ID';
COMMENT ON COLUMN draw_sources.name IS '开奖源展示名称';
COMMENT ON COLUMN draw_sources.provider IS '开奖方类型（如 api68/kjApi）';
COMMENT ON COLUMN draw_sources.lot_code IS '开奖源期号参数，如 lotCode/lotKey';
COMMENT ON COLUMN draw_sources.endpoint IS '开奖接口地址';
COMMENT ON COLUMN draw_sources.reusable_for_lottery_ids IS '可复用的彩种 ID 列表（JSON）';
COMMENT ON COLUMN draw_sources.created_at IS '创建时间';
COMMENT ON COLUMN draw_sources.updated_at IS '更新时间';

COMMENT ON TABLE orders IS '投注订单表';
COMMENT ON COLUMN orders.id IS '订单唯一 ID';
COMMENT ON COLUMN orders.user_id IS '下单用户 ID';
COMMENT ON COLUMN orders.lottery_id IS '所属彩种 ID';
COMMENT ON COLUMN orders.lottery_name IS '彩种名称快照';
COMMENT ON COLUMN orders.issue IS '投注期号';
COMMENT ON COLUMN orders.rule_code IS '玩法编码';
COMMENT ON COLUMN orders.number_type IS '号码位数类型';
COMMENT ON COLUMN orders.selection IS '用户投注选择（JSON）';
COMMENT ON COLUMN orders.stake_count IS '注数';
COMMENT ON COLUMN orders.unit_amount_minor IS '单注金额（分）';
COMMENT ON COLUMN orders.amount_minor IS '订单总金额（分）';
COMMENT ON COLUMN orders.odds_basis_points IS '下单时赔率（基点）';
COMMENT ON COLUMN orders.expanded_bets IS '展开后的投注明细（JSON）';
COMMENT ON COLUMN orders.draw_number IS '开奖号码回填';
COMMENT ON COLUMN orders.matched_bets IS '命中注码集合（JSON）';
COMMENT ON COLUMN orders.payout_minor IS '最终派奖金额（分）';
COMMENT ON COLUMN orders.status IS '订单状态';
COMMENT ON COLUMN orders.settled_at IS '结算完成时间';
COMMENT ON COLUMN orders.created_at IS '创建时间';
COMMENT ON COLUMN orders.updated_at IS '更新时间';

COMMENT ON TABLE order_settlement_runs IS '期号结算批次表';
COMMENT ON COLUMN order_settlement_runs.id IS '结算批次 ID';
COMMENT ON COLUMN order_settlement_runs.draw_issue_id IS '关联 draw_issues 主键';
COMMENT ON COLUMN order_settlement_runs.lottery_id IS '所属彩种 ID';
COMMENT ON COLUMN order_settlement_runs.lottery_name IS '彩种名称快照';
COMMENT ON COLUMN order_settlement_runs.issue IS '期号';
COMMENT ON COLUMN order_settlement_runs.draw_number IS '期期开奖号';
COMMENT ON COLUMN order_settlement_runs.settled_order_count IS '本批结算订单数';
COMMENT ON COLUMN order_settlement_runs.winning_order_count IS '中奖订单数';
COMMENT ON COLUMN order_settlement_runs.total_stake_amount_minor IS '本期投注总额（分）';
COMMENT ON COLUMN order_settlement_runs.total_payout_minor IS '本期派奖总额（分）';
COMMENT ON COLUMN order_settlement_runs.created_at IS '创建时间';
COMMENT ON COLUMN order_settlement_runs.updated_at IS '更新时间';

COMMENT ON TABLE order_settlements IS '订单结算明细表';
COMMENT ON COLUMN order_settlements.settlement_id IS '所属结算批次 ID';
COMMENT ON COLUMN order_settlements.order_id IS '订单 ID';
COMMENT ON COLUMN order_settlements.user_id IS '用户 ID';
COMMENT ON COLUMN order_settlements.rule_code IS '玩法编码';
COMMENT ON COLUMN order_settlements.stake_count IS '命中注数';
COMMENT ON COLUMN order_settlements.amount_minor IS '结算金额（分）';
COMMENT ON COLUMN order_settlements.is_winning IS '是否中奖';
COMMENT ON COLUMN order_settlements.matched_bets IS '命中的注码明细（JSON）';
COMMENT ON COLUMN order_settlements.odds_basis_points IS '结算时赔率（基点）';
COMMENT ON COLUMN order_settlements.payout_minor IS '结算派奖金额（分）';
COMMENT ON COLUMN order_settlements.status IS '明细状态';

COMMENT ON TABLE order_runtime IS '订单运行时计数器';
COMMENT ON COLUMN order_runtime.key IS '运行时 Key';
COMMENT ON COLUMN order_runtime.value IS 'Key 对应数值';
COMMENT ON COLUMN order_runtime.updated_at IS '更新时间';

COMMENT ON TABLE financial_accounts IS '用户资金账户表';
COMMENT ON COLUMN financial_accounts.user_id IS '用户 ID';
COMMENT ON COLUMN financial_accounts.available_balance_minor IS '可用余额（分）';
COMMENT ON COLUMN financial_accounts.frozen_balance_minor IS '冻结余额（分）';
COMMENT ON COLUMN financial_accounts.updated_at IS '更新时间';

COMMENT ON TABLE ledger_entries IS '资金流水表';
COMMENT ON COLUMN ledger_entries.id IS '流水 ID';
COMMENT ON COLUMN ledger_entries.user_id IS '归属用户 ID';
COMMENT ON COLUMN ledger_entries.kind IS '流水类型';
COMMENT ON COLUMN ledger_entries.amount_minor IS '金额（分）';
COMMENT ON COLUMN ledger_entries.balance_after_minor IS '流水后余额（分）';
COMMENT ON COLUMN ledger_entries.reference_id IS '业务引用 ID';
COMMENT ON COLUMN ledger_entries.description IS '流水说明';
COMMENT ON COLUMN ledger_entries.created_at IS '创建时间';

COMMENT ON TABLE finance_runtime IS '资金运行时计数器';
COMMENT ON COLUMN finance_runtime.key IS '运行时 Key';
COMMENT ON COLUMN finance_runtime.value IS 'Key 对应数值';
COMMENT ON COLUMN finance_runtime.updated_at IS '更新时间';

COMMENT ON TABLE group_buy_plans IS '合买方案表';
COMMENT ON COLUMN group_buy_plans.id IS '方案 ID';
COMMENT ON COLUMN group_buy_plans.lottery_id IS '关联彩种 ID';
COMMENT ON COLUMN group_buy_plans.lottery_name IS '彩种名称';
COMMENT ON COLUMN group_buy_plans.initiator_user_id IS '发起人用户 ID';
COMMENT ON COLUMN group_buy_plans.initiator_username IS '发起人用户名';
COMMENT ON COLUMN group_buy_plans.total_amount_minor IS '目标金额（分）';
COMMENT ON COLUMN group_buy_plans.filled_amount_minor IS '已募集金额（分）';
COMMENT ON COLUMN group_buy_plans.min_share_amount_minor IS '每份最小金额（分）';
COMMENT ON COLUMN group_buy_plans.participant_min_amount_minor IS '参与者最低金额（分）';
COMMENT ON COLUMN group_buy_plans.share_count IS '方案份数';
COMMENT ON COLUMN group_buy_plans.status IS '方案状态';
COMMENT ON COLUMN group_buy_plans.note IS '方案说明';
COMMENT ON COLUMN group_buy_plans.created_at IS '创建时间';
COMMENT ON COLUMN group_buy_plans.updated_at IS '更新时间';

COMMENT ON TABLE group_buy_participants IS '合买参与记录表';
COMMENT ON COLUMN group_buy_participants.id IS '参与记录 ID';
COMMENT ON COLUMN group_buy_participants.plan_id IS '所属合买方案 ID';
COMMENT ON COLUMN group_buy_participants.user_id IS '参与用户 ID';
COMMENT ON COLUMN group_buy_participants.username IS '参与者用户名';
COMMENT ON COLUMN group_buy_participants.amount_minor IS '参与金额（分）';
COMMENT ON COLUMN group_buy_participants.share_count IS '认购份数';
COMMENT ON COLUMN group_buy_participants.note IS '参与备注';
COMMENT ON COLUMN group_buy_participants.created_at IS '参与时间';

COMMENT ON TABLE invite_records IS '邀请记录表';
COMMENT ON COLUMN invite_records.id IS '记录 ID';
COMMENT ON COLUMN invite_records.inviter_user_id IS '邀请人用户 ID';
COMMENT ON COLUMN invite_records.inviter_username IS '邀请人用户名';
COMMENT ON COLUMN invite_records.invitee_user_id IS '被邀请人用户 ID';
COMMENT ON COLUMN invite_records.invitee_username IS '被邀请人用户名';
COMMENT ON COLUMN invite_records.invite_code IS '使用的邀请码';
COMMENT ON COLUMN invite_records.status IS '邀请关系状态';
COMMENT ON COLUMN invite_records.rebate_enabled IS '是否开启返利';
COMMENT ON COLUMN invite_records.note IS '记录说明';
COMMENT ON COLUMN invite_records.created_at IS '创建时间';
COMMENT ON COLUMN invite_records.updated_at IS '更新时间';
COMMENT ON TABLE rebate_policy IS '返利策略表（单例）';
COMMENT ON COLUMN rebate_policy.id IS '策略主键';
COMMENT ON COLUMN rebate_policy.agents_can_invite IS '是否允许代理邀请';
COMMENT ON COLUMN rebate_policy.regular_users_can_invite IS '是否允许普通用户邀请';
COMMENT ON COLUMN rebate_policy.rebate_mode IS '返利模式';
COMMENT ON COLUMN rebate_policy.default_recharge_rebate_basis_points IS '默认充值返利（基点）';
COMMENT ON COLUMN rebate_policy.updated_at IS '更新时间';
COMMENT ON CONSTRAINT rebate_policy_singleton_check ON rebate_policy IS '约束 id 必须为 default';

COMMENT ON TABLE robot_configs IS '机器人配置表';
COMMENT ON COLUMN robot_configs.id IS '机器人 ID';
COMMENT ON COLUMN robot_configs.name IS '机器人名称';
COMMENT ON COLUMN robot_configs.kind IS '机器人类型';
COMMENT ON COLUMN robot_configs.status IS '机器人状态';
COMMENT ON COLUMN robot_configs.description IS '机器人说明';
COMMENT ON COLUMN robot_configs.updated_at IS '更新时间';

COMMENT ON TABLE robot_lottery_bindings IS '机器人绑定彩种关系表';
COMMENT ON COLUMN robot_lottery_bindings.robot_id IS '机器人 ID';
COMMENT ON COLUMN robot_lottery_bindings.lottery_id IS '绑定的彩种 ID';

COMMENT ON TABLE support_conversations IS '客服会话主表';
COMMENT ON COLUMN support_conversations.id IS '会话 ID';
COMMENT ON COLUMN support_conversations.user_id IS '关联用户 ID';
COMMENT ON COLUMN support_conversations.username IS '用户名称';
COMMENT ON COLUMN support_conversations.subject IS '会话主题';
COMMENT ON COLUMN support_conversations.status IS '会话状态';
COMMENT ON COLUMN support_conversations.priority IS '优先级';
COMMENT ON COLUMN support_conversations.assigned_admin_id IS '分配的管理员 ID';
COMMENT ON COLUMN support_conversations.assigned_admin_name IS '分配的管理员名称';
COMMENT ON COLUMN support_conversations.unread_count IS '未读消息数';
COMMENT ON COLUMN support_conversations.created_at IS '创建时间';
COMMENT ON COLUMN support_conversations.updated_at IS '更新时间';

COMMENT ON TABLE support_messages IS '客服消息明细表';
COMMENT ON COLUMN support_messages.id IS '消息 ID';
COMMENT ON COLUMN support_messages.conversation_id IS '所属会话 ID';
COMMENT ON COLUMN support_messages.author IS '消息发送方角色';
COMMENT ON COLUMN support_messages.author_id IS '发送方 ID';
COMMENT ON COLUMN support_messages.author_name IS '发送方名称';
COMMENT ON COLUMN support_messages.content IS '消息内容';
COMMENT ON COLUMN support_messages.created_at IS '发送时间';

COMMENT ON TABLE draw_scheduler_config IS '调度器参数配置表（单例）';
COMMENT ON COLUMN draw_scheduler_config.id IS '配置主键';
COMMENT ON COLUMN draw_scheduler_config.enabled IS '是否启用常驻调度';
COMMENT ON COLUMN draw_scheduler_config.interval_seconds IS '常驻调度周期（秒）';
COMMENT ON COLUMN draw_scheduler_config.future_issue_count IS '每彩种提前生成期号数量';
COMMENT ON COLUMN draw_scheduler_config.sale_close_lead_seconds IS '封盘提前秒数';
COMMENT ON COLUMN draw_scheduler_config.updated_at IS '配置更新时间';
COMMENT ON CONSTRAINT draw_scheduler_config_singleton_check ON draw_scheduler_config IS '约束 id 必须为 default';

COMMENT ON TABLE draw_scheduler_runs IS '调度执行日志表';
COMMENT ON COLUMN draw_scheduler_runs.id IS '执行记录 ID';
COMMENT ON COLUMN draw_scheduler_runs."trigger" IS '触发来源（手动/manual 或定时/timer）';
COMMENT ON COLUMN draw_scheduler_runs.status IS '执行结果状态';
COMMENT ON COLUMN draw_scheduler_runs.started_at IS '开始时间';
COMMENT ON COLUMN draw_scheduler_runs.finished_at IS '完成时间';
COMMENT ON COLUMN draw_scheduler_runs.now IS '执行时间快照';
COMMENT ON COLUMN draw_scheduler_runs.error IS '错误信息';
COMMENT ON COLUMN draw_scheduler_runs.closed_issue_count IS '本轮关闭期号数';
COMMENT ON COLUMN draw_scheduler_runs.drawn_issue_count IS '本轮开奖期号数';
COMMENT ON COLUMN draw_scheduler_runs.settlement_run_count IS '本轮结算批次数';
COMMENT ON COLUMN draw_scheduler_runs.ledger_entry_count IS '本轮生成流水数';
COMMENT ON COLUMN draw_scheduler_runs.generated_issue_count IS '本轮新生成期号数';
COMMENT ON COLUMN draw_scheduler_runs.skipped_issue_count IS '本轮跳过期开奖号数';
COMMENT ON COLUMN draw_scheduler_runs.skipped_lottery_count IS '本轮跳过彩种数';
COMMENT ON COLUMN draw_scheduler_runs.skipped_issues IS '跳过期号明细（JSON）';
COMMENT ON COLUMN draw_scheduler_runs.skipped_lotteries IS '跳过彩种及原因明细（JSON）';

COMMENT ON TABLE draw_scheduler_runtime IS '调度运行时计数器';
COMMENT ON COLUMN draw_scheduler_runtime.key IS '运行时 Key';
COMMENT ON COLUMN draw_scheduler_runtime.value IS 'Key 对应数值';
COMMENT ON COLUMN draw_scheduler_runtime.updated_at IS '更新时间';
