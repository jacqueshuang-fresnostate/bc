# 计奖与派奖基础

## 目标

把订单、玩法规则和开奖结果串起来：管理员在开奖期号已开奖后，可以执行计奖派奖，后端会按订单玩法重新评估是否中奖，更新订单状态，并生成一份基础结算记录。这个阶段只建立开奖后的订单状态流转和派奖结果入口，不写真实用户余额、不生成资金流水。

## 已知信息

- 订单已经支持创建、列表、详情和取消，状态包含 `pendingDraw`、`won`、`lost`、`cancelled`。
- 玩法规则引擎已经能根据 `ruleCode`、选号和 `drawNumber` 返回 `isWinning` 与 `matchedBets`。
- 开奖期号已经支持创建、封盘、开奖和取消，已开奖期号包含 `drawNumber`。
- 当前没有真实赔率配置、奖金表、用户余额仓储、资金流水或合买分账。
- 当前工作区存在用户或 IDE 产生的未提交变更：`admin/vite.config.ts`、`backend/src/main.rs`、`.idea/`。本任务会保留这些变更，不主动提交或覆盖。
- 项目文档必须使用中文输出。

## 本轮范围

### 后端

- 新增结算领域模型：
  - 结算批次。
  - 单笔订单结算结果。
  - 基础派奖倍数。
- 扩展订单详情和订单摘要：
  - `drawNumber`
  - `matchedBets`
  - `payoutMinor`
  - `settledAt`
- 扩展内存订单仓储：
  - 按开奖期号结算同彩种、同一期号的待开奖订单。
  - 中奖订单状态改为 `won`。
  - 未中奖订单状态改为 `lost`。
  - 已取消订单不参与结算。
  - 已结算期号不能重复结算。
  - 保存结算批次列表和详情。
- 新增 API：
  - `GET /api/admin/settlements`
  - `GET /api/admin/settlements/{id}`
  - `POST /api/admin/settlements/draw-issues/{id}`
- 结算规则：
  - 只允许对 `drawn` 状态且有开奖号码的期号结算。
  - 用玩法规则引擎重新评估每笔订单。
  - 基础派奖金额 = 命中投注数 × 单注金额 × 后端基础倍数。
  - 基础倍数只用于本阶段验证链路，不代表真实赔率。
- 暂不写真实财务账户或资金流水。

### 管理后台

- 新增“计奖派奖”真实页面。
- 工作台模块目录新增“计奖派奖”入口。
- 页面展示已开奖期号、结算批次、派奖总额、中奖订单数和单笔订单结算结果。
- 页面支持选择已开奖期号并执行计奖派奖。
- 订单管理页面展示订单的开奖结果、命中投注、派奖金额和结算时间。

## 暂不包含

- 不实现真实赔率配置、奖金表或奖金上限。
- 不扣款、不加款、不更新用户余额。
- 不生成资金流水。
- 不实现合买份额分账。
- 不实现异常复核、撤销结算或重新结算。
- 不实现数据库持久化。
- 不实现自动开奖后自动结算任务。
- 不强制订单创建时必须绑定已存在期号；封盘校验后续单独做。

## 数据契约草案

执行结算：

```http
POST /api/admin/settlements/draw-issues/D000000000001
```

结算批次响应：

```json
{
  "id": "S000000000001",
  "drawIssueId": "D000000000001",
  "lotteryId": "fc3d",
  "lotteryName": "福彩 3D",
  "issue": "2026156",
  "drawNumber": "247",
  "settledOrderCount": 3,
  "winningOrderCount": 1,
  "totalStakeAmountMinor": 600,
  "totalPayoutMinor": 2000,
  "createdAt": "unix:1780389000",
  "orders": [
    {
      "orderId": "O000000000001",
      "userId": "U10001",
      "ruleCode": "threeDirect",
      "stakeCount": 1,
      "amountMinor": 200,
      "isWinning": true,
      "matchedBets": ["247"],
      "payoutMultiplier": 10,
      "payoutMinor": 2000,
      "status": "won"
    }
  ]
}
```

订单响应新增字段：

```json
{
  "drawNumber": "247",
  "matchedBets": ["247"],
  "payoutMinor": 2000,
  "settledAt": "unix:1780389000"
}
```

## 验收标准

- [x] 后端支持按已开奖期号执行结算。
- [x] 后端会把中奖订单更新为 `won`，未中奖订单更新为 `lost`，取消订单不参与结算。
- [x] 后端能生成结算批次列表和详情。
- [x] 后端拒绝未开奖期号结算和重复结算。
- [x] 后端结算逻辑复用玩法规则引擎，不在路由函数里判断中奖。
- [x] 管理后台新增真实“计奖派奖”页面。
- [x] 订单管理页面展示开奖结果、命中投注、派奖金额和结算时间。
- [x] 浏览器可以完成“创建订单 → 创建/开奖期号 → 执行计奖派奖 → 订单状态变更”的链路。
- [x] `cargo fmt --check`、`cargo check`、`cargo test`、`npm run build` 通过。

## 完成定义

- 结算业务逻辑集中在服务层或仓储层，不写进路由处理函数。
- 结算 API 返回统一 API 信封。
- 前端 API client、类型、hook 与后端契约一致。
- 架构、TODO、规格和任务记录同步更新。
- 完成本阶段后提交 Git，并归档任务。

## 技术方案

- 后端新增 `domain/settlement.rs`。
- 扩展 `domain/order.rs`，让订单详情和摘要包含基础结算结果。
- 扩展 `services/order.rs`，在同一把订单仓储写锁内完成订单状态更新和结算批次保存。
- `AppState` 暂不新增独立结算仓储，结算批次跟随订单内存仓储保存，避免订单状态和结算记录不同步。
- `routes/admin.rs` 增加 settlement API，路由只读取开奖期号并调用订单仓储结算方法。
- 前端新增 `types/settlements.ts`、`useSettlements`、`SettlementManagementPage`。
- `services/dashboard.rs` 的模块目录新增 `settlements`，`App.tsx` 将其路由到真实页面。

## 风险与约束

- 基础派奖倍数不是生产赔率，不能作为真实彩票奖金规则上线。
- 当前结算记录是内存模式，服务重启会丢失。
- 当前不操作用户余额，因此“派奖”只表示生成基础派奖结果，不代表资金已入账。
- 订单创建仍未校验期号是否开售或封盘，后续需要单独实现。
- 合买分账和机器人执行必须等资金、份额和风控规则完善后再接入。

## 技术备注

- 已阅读：`AGENTS.md`、`架构设计.md`、订单基础模块、玩法规则引擎、开奖期号模块、dashboard 模块目录。
- 相关后端文件：`backend/src/domain/order.rs`、`backend/src/domain/play.rs`、`backend/src/domain/draw.rs`、`backend/src/services/order.rs`、`backend/src/services/play_rules.rs`、`backend/src/routes/admin.rs`。
- 相关前端文件：`admin/src/types/orders.ts`、`admin/src/api/client.ts`、`admin/src/hooks/useOrders.ts`、`admin/src/pages/OrderManagementPage.tsx`、`admin/src/pages/DrawManagementPage.tsx`。
- 任务创建时间：2026-06-02 HKT。

## 完成记录

- 完成时间：2026-06-02 16:23:42 HKT。
- 后端新增 `SettlementRun` 和 `OrderSettlement`，扩展订单结算字段，并新增 `GET /settlements`、`GET /settlements/{id}`、`POST /settlements/draw-issues/{id}`。
- 后端结算复用玩法规则引擎，按已开奖期号更新待开奖订单状态；中奖订单更新为 `won`，未中奖订单更新为 `lost`，取消订单不参与。
- 前端新增 `SettlementManagementPage`、`useSettlements`、`types/settlements.ts`，并在订单管理页展示结算字段。
- API 验证：期号 `2026200` 开奖 `023`，直选 `023` 订单结算为 `won`，派奖 `2000` 分。
- 浏览器验证：计奖派奖页面展示结算批次 `S000000000001`、中奖订单 `O000000000001` 和 `¥20.00` 派奖。
- 质量验证：`cargo fmt --check`、`cargo check`、`cargo test`、`npm run build` 均通过。
