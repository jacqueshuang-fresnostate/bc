CREATE INDEX IF NOT EXISTS draw_issues_scheduler_active_due_idx
ON draw_issues (scheduled_at, id)
WHERE status IN ('open', 'closed');

COMMENT ON INDEX draw_issues_scheduler_active_due_idx IS
'秒级开奖调度按计划开奖时间判断待处理活跃期号';

CREATE INDEX IF NOT EXISTS draw_issues_scheduler_refund_idx
ON draw_issues (lottery_id, issue, sale_closed_at)
WHERE status IN ('open', 'closed', 'drawn');

COMMENT ON INDEX draw_issues_scheduler_refund_idx IS
'秒级开奖调度按彩种期号和封盘时间检查合买兜底与流单退款';

CREATE INDEX IF NOT EXISTS draw_issues_unsettled_drawn_idx
ON draw_issues (scheduled_at, id)
WHERE status = 'drawn' AND draw_number IS NOT NULL;

COMMENT ON INDEX draw_issues_unsettled_drawn_idx IS
'开奖结算队列按计划时间扫描已有开奖号码但尚未结算的期号';

CREATE INDEX IF NOT EXISTS group_buy_plans_runtime_guard_idx
ON group_buy_plans (lottery_id, issue, status)
WHERE status IN ('draft', 'open', 'filled');

COMMENT ON INDEX group_buy_plans_runtime_guard_idx IS
'开奖和机器人调度按彩种期号查找待补满、待退款或待补建订单的合买计划';

CREATE INDEX IF NOT EXISTS order_settlement_runs_draw_issue_id_idx
ON order_settlement_runs (draw_issue_id);

COMMENT ON INDEX order_settlement_runs_draw_issue_id_idx IS
'开奖调度按期号判断是否已经生成结算批次，避免每秒读取全部历史结算 ID';
