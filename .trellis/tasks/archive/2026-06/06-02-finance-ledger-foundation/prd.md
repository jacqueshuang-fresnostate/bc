# 用户资金与资金流水基础 PRD

## 背景

当前系统已经具备彩种管理、玩法规则、订单、开奖期号和基础计奖派奖能力，但订单创建不会扣减用户余额，取消订单不会退款，中奖结算也不会真正写入用户资金。管理后台“财务管理”仍然停留在概览入口，无法查看用户账户或资金流水。

本阶段需要建立用户资金基础闭环，让投注、取消和结算结果能够反映到用户余额与流水中，为后续充值、提现、财务审核、异常复核和数据库持久化打基础。

## 目标

1. 后端新增资金账户与资金流水领域模型。
2. 后端新增内存资金仓储，支持账户查看、流水查看和手动调账。
3. 订单创建时校验余额并扣款，扣款失败时拒绝创建订单。
4. 取消待开奖订单时退回投注金额并写入退款流水。
5. 结算中奖订单时把派奖金额入账并写入派奖流水。
6. dashboard 财务概览和资金账户摘要读取同一份资金仓储。
7. 管理后台“财务管理”入口升级为真实页面，展示账户、流水和手动调账表单。
8. 更新 `架构设计.md`、API 规范和 `TODO.md`，所有文档输出保持中文。

## 非目标

1. 不接入真实支付、充值、提现、银行卡或第三方通道。
2. 不实现提现冻结、提现审核、拒绝/打款状态流转。
3. 不新增资金 PostgreSQL 迁移，本阶段继续使用内存仓储。
4. 不实现真实赔率、奖金上限、风控拦截、异常复核和重结算撤销。
5. 不实现用户 CRUD 或完整会员中心。
6. 不实现合买份额资金拆分。

## 后端需求

### 资金账户

- 每个资金账户至少包含：
  - `userId`
  - `availableBalanceMinor`
  - `frozenBalanceMinor`
- 金额全部使用最小货币单位整数。
- 种子账户需要覆盖当前演示用户，保证订单创建默认用户有余额。

### 资金流水

- 每条流水至少包含：
  - `id`
  - `userId`
  - `kind`
  - `amountMinor`
  - `balanceAfterMinor`
  - `referenceId`
  - `description`
  - `createdAt`
- `kind` 至少支持：
  - `manualAdjustment`
  - `orderDebit`
  - `orderRefund`
  - `payoutCredit`
- 流水金额约定：
  - 扣款为负数。
  - 退款、派奖和正向调账为正数。
  - 负向调账为负数。

### API

新增接口继续使用统一 API 信封：

- `GET /api/admin/financial-accounts`
- `GET /api/admin/ledger-entries`
- `POST /api/admin/financial-adjustments`

`POST /api/admin/financial-adjustments` 请求体：

```json
{
  "userId": "U10001",
  "amountMinor": 1000,
  "description": "后台手动补款"
}
```

### 订单资金联动

- 创建订单前先计算订单金额，再检查用户可用余额。
- 可用余额不足时返回业务错误，不创建订单。
- 订单创建成功后写入 `orderDebit` 流水。
- 取消待开奖订单成功后写入 `orderRefund` 流水。
- 对同一订单重复退款必须被阻止或保持幂等，不能重复加钱。

### 结算资金联动

- 执行结算后，对中奖且 `payoutMinor > 0` 的订单写入 `payoutCredit` 流水。
- 未中奖订单不写入派奖流水。
- 结算服务已有重复结算保护，资金服务仍需要避免同一结算订单重复入账。

### Dashboard 联动

- `/api/admin/dashboard` 的 `finance` 和 `financialAccounts` 从资金仓储读取。
- `todayPayoutMinor` 根据当天派奖流水统计。
- `totalBalanceMinor` 根据账户可用余额和冻结余额汇总。

## 前端需求

1. 新增财务管理真实页面。
2. 页面展示资金概览、账户列表和流水列表。
3. 页面支持手动调账，提交后刷新账户和流水。
4. API 调用集中在 `admin/src/api/client.ts`。
5. 新增 `useFinance` hook 管理 loading、error、refresh 和 saving。
6. 类型放入 `admin/src/types/finance.ts`，字段名与后端 `camelCase` 保持一致。
7. 页面要保持后台工作台风格，提供可见加载和错误状态。

## 验收标准

1. `GET /api/admin/financial-accounts` 返回种子资金账户。
2. `POST /api/admin/orders` 创建订单后，对应用户可用余额减少，并出现 `orderDebit` 流水。
3. `PATCH /api/admin/orders/{id}/cancel` 取消订单后，对应用户可用余额恢复，并出现 `orderRefund` 流水。
4. 对中奖订单执行结算后，对应用户可用余额增加派奖金额，并出现 `payoutCredit` 流水。
5. 余额不足创建订单返回错误，且不产生订单。
6. 管理后台“财务管理”页面能展示账户和流水，并能执行手动调账。
7. `cargo fmt --check`、`cargo check`、`cargo test`、`npm run build` 通过。
8. 更新 `架构设计.md`、`.trellis/spec/backend/api-contracts.md` 和 `TODO.md`。

## 风险与约束

1. 当前订单、开奖、结算和资金都使用内存仓储，服务重启后会恢复种子数据。
2. 订单创建和资金扣款是同一请求内的顺序操作，但还不是数据库事务；后续持久化时需要用事务保证一致性。
3. 本阶段的派奖金额仍来自基础结算倍数，不代表生产赔率。
4. 手动调账只是后台基础能力，后续需要增加管理员身份、审批、备注约束和审计字段。
