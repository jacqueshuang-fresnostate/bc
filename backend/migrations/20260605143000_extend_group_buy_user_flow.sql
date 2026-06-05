ALTER TABLE group_buy_plans
    ADD COLUMN IF NOT EXISTS issue TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS rule_code TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS title TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS numbers TEXT NOT NULL DEFAULT '';

UPDATE group_buy_plans
SET title = CASE
        WHEN title = '' AND issue <> '' THEN lottery_name || ' 第' || issue || '期合买'
        WHEN title = '' THEN lottery_name || ' 合买计划'
        ELSE title
    END;

COMMENT ON COLUMN group_buy_plans.issue IS '合买对应期号';
COMMENT ON COLUMN group_buy_plans.rule_code IS '合买玩法代码';
COMMENT ON COLUMN group_buy_plans.title IS '合买计划标题';
COMMENT ON COLUMN group_buy_plans.numbers IS '合买投注内容展示文本';

COMMENT ON COLUMN ledger_entries.kind IS '流水类型：manualAdjustment 手动调账，orderDebit 投注扣款，orderRefund 取消退款，payoutCredit 派奖入账，rechargeCredit 充值入账，withdrawalFreeze 提现冻结，withdrawalPayout 提现打款，withdrawalReject 提现驳回解冻，groupBuyDebit 合买认购扣款，groupBuyRefund 合买退款';
