# 订单与投注基础

## 目标

在玩法规则引擎之上补齐订单与投注的第一阶段能力：后端可以创建投注订单、校验彩种与玩法、计算注数和投注金额、查询订单列表和详情；管理后台可以进入“订单管理”页面创建测试投注单并查看订单。这个阶段让后续开奖、计奖、派奖、财务和机器人购彩有真实订单数据可以接入。

## 已知信息

- 上一阶段已完成玩法规则引擎，后端可通过 `evaluate_play_rule` 计算注数、展开投注和中奖判断。
- 当前 `backend/src/domain/order.rs` 只有 `OrderSummary` 和 `GroupBuyPlanSummary` 静态展示结构。
- 当前 dashboard 的最近订单来自 `backend/src/services/dashboard.rs` 的演示数据，不是仓储数据。
- 当前 `AppState` 只有彩种仓储，订单还没有仓储。
- 管理后台“订单管理”仍是占位页。
- 当前工作区存在用户或 IDE 产生的未提交变更：`admin/vite.config.ts`、`backend/src/main.rs`、`.idea/`。本任务会保留这些变更，不主动提交或覆盖。
- 项目文档必须使用中文输出。

## 本轮范围

### 后端

- 扩展订单领域模型：
  - 订单状态。
  - 投注订单详情。
  - 创建订单请求。
  - 订单投注内容。
  - 订单创建结果。
- 新增内存订单仓储：
  - 创建订单。
  - 列表查询。
  - 详情查询。
  - 取消待开奖订单。
  - 最近订单摘要供 dashboard 使用。
- 创建订单时完成校验：
  - 彩种存在。
  - 彩种销售状态开启。
  - 彩种号码类型与玩法规则一致。
  - 彩种玩法分类包含该玩法对应分类。
  - 期号、用户 ID、单注金额合法。
  - 玩法规则引擎返回注数大于 0。
  - 总金额 = 注数 × 单注金额。
- 新增 API：
  - `GET /api/admin/orders`
  - `GET /api/admin/orders/{id}`
  - `POST /api/admin/orders`
  - `PATCH /api/admin/orders/{id}/cancel`
- dashboard 改为读取订单仓储中的最近订单摘要。

### 管理后台

- 新增“订单管理”真实页面。
- 页面展示订单列表、状态、用户、彩种、期号、玩法、注数和金额。
- 页面提供创建测试投注单表单：
  - 用户 ID。
  - 彩种。
  - 期号。
  - 玩法。
  - 选号参数。
  - 单注金额。
- 页面调用后端创建订单接口，展示后端计算出的注数、金额、展开投注和创建结果。
- 页面支持取消待开奖订单。
- 订单创建成功后刷新 dashboard，使最近订单和订单数与仓储一致。

## 暂不包含

- 不扣减用户余额。
- 不生成资金流水。
- 不开奖、不计奖、不派奖。
- 不实现订单数据库持久化。
- 不实现真实用户选择器、认证鉴权或权限拦截。
- 不实现手机端购彩。
- 不实现合买订单。

## 数据契约草案

创建订单请求：

```json
{
  "userId": "U10001",
  "lotteryId": "fc3d",
  "issue": "2026155",
  "ruleCode": "threeDirect",
  "selection": {
    "positions": [[2], [4], [7]]
  },
  "unitAmountMinor": 200
}
```

订单响应：

```json
{
  "id": "O202606020001",
  "userId": "U10001",
  "lotteryId": "fc3d",
  "lotteryName": "福彩 3D",
  "issue": "2026155",
  "ruleCode": "threeDirect",
  "numberType": "threeDigit",
  "selection": {
    "positions": [[2], [4], [7]]
  },
  "stakeCount": 1,
  "unitAmountMinor": 200,
  "amountMinor": 200,
  "expandedBets": ["247"],
  "status": "pendingDraw",
  "createdAt": "2026-06-02T15:50:00+08:00"
}
```

## 验收标准

- [x] 后端创建订单时复用玩法规则引擎计算注数和展开投注。
- [x] 后端能按彩种号码类型、销售状态和玩法分类拒绝非法订单。
- [x] 后端总金额严格等于 `stakeCount * unitAmountMinor`，不使用浮点数。
- [x] 后端支持订单列表、详情和取消待开奖订单。
- [x] dashboard 的最近订单来自订单仓储，而不是静态订单函数。
- [x] 管理后台新增真实“订单管理”页面，不再只是占位。
- [x] 订单页面可以创建 3 位直选测试订单并看到金额与注数。
- [x] `cargo fmt --check`、`cargo check`、`cargo test`、`npm run build` 通过。

## 完成定义

- 订单创建逻辑集中在服务层，不写进路由处理函数。
- 订单 API 返回统一 API 信封。
- 订单金额使用最小货币单位整数。
- 前端 API client、类型、hook 与后端契约一致。
- 架构、TODO、规格和任务记录同步更新。
- 完成本阶段后提交 Git，并归档任务。

## 技术方案

- 后端扩展 `domain/order.rs`，复用 `PlayRuleCode`、`PlaySelection`、`LotteryNumberType`。
- 后端新增 `services/order.rs`，内部使用 `Arc<RwLock<OrderStore>>` 的内存仓储。
- `AppState` 增加 `orders: OrderRepository`，路由创建订单时同时读取 `lotteries` 仓储。
- `dashboard_summary` 增加订单摘要入参，避免继续使用静态 `recent_orders()`。
- 前端新增 `types/orders.ts`、`useOrders`、`OrderManagementPage`，在 `App.tsx` 中把 `orders` 入口接入真实页面。
- 订单页面复用玩法规则页面的选号表单思路，但本阶段保持紧凑，优先支持管理员测试下单。

## 风险与约束

- 订单创建是资金链路入口，本轮虽然不扣余额，也必须保证注数和金额计算在后端完成。
- 当前订单仓储先使用内存模式，服务重启后订单会丢失；后续订单数据库持久化需要单独实现。
- 当前没有真实用户表和余额表，`userId` 先作为字符串录入并只做非空校验。
- 玩法分类与具体玩法规则的映射必须在服务层统一维护，避免前端绕过。
- 用户已有端口和 IDE 配置变更，本任务提交时需要排除这些未识别变更。

## 技术备注

- 已阅读：`AGENTS.md`、`架构设计.md`、现有订单领域模型、彩种仓储、玩法规则服务、后台页面模式。
- 相关后端文件：`backend/src/domain/order.rs`、`backend/src/services/play_rules.rs`、`backend/src/services/lottery.rs`、`backend/src/routes/admin.rs`、`backend/src/app.rs`。
- 相关前端文件：`admin/src/App.tsx`、`admin/src/api/client.ts`、`admin/src/pages/PlaceholderPage.tsx`、`admin/src/pages/LotteryManagementPage.tsx`、`admin/src/pages/PlayRulesPage.tsx`。
- 任务创建时间：2026-06-02 HKT。

## 完成记录

- 完成时间：2026-06-02 15:54:58 HKT。
- 后端完成：订单领域模型、内存订单仓储、订单创建/列表/详情/取消 API、dashboard 最近订单仓储化。
- 前端完成：订单类型、订单 API client、`useOrders` hook、真实“订单管理”页面、工作台最近订单展示。
- 文档完成：已同步更新 `架构设计.md`、`TODO.md` 和 `.trellis/spec/backend/api-contracts.md`。
- 验证完成：后端测试 24 个通过，`npm run build` 通过，API smoke 和浏览器订单创建验证通过。
