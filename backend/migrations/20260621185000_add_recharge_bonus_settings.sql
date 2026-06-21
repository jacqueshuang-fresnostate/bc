INSERT INTO system_settings (key, value, description)
VALUES
    ('recharge_bonus_enabled', 'false', '是否开启用户充值赠送活动'),
    ('recharge_bonus_rules', '[]', '用户充值赠送活动档位，保存为 JSON 数组，金额单位为分')
ON CONFLICT (key) DO NOTHING;

COMMENT ON COLUMN ledger_entries.kind IS '流水类型：agentRebateWithdrawal 代理返利提现，manualAdjustment 手动调账，orderDebit 投注扣款，orderRefund 取消退款，payoutCredit 派奖入账，rechargeBonusCredit 充值赠送入账，rechargeCredit 充值入账，rechargeRebateCredit 充值返利入账，withdrawalFreeze 提现冻结，withdrawalPayout 提现打款，withdrawalReject 提现驳回解冻，groupBuyDebit 合买认购扣款，groupBuyRefund 合买退款，redPacketDebit 聊天大厅红包扣款，redPacketCredit 聊天大厅红包入账';
