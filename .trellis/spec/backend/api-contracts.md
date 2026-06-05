# API 契约规范

> 后端与管理后台之间的可执行接口契约。

---

## 场景：管理后台首期概览接口

### 1. 范围 / 触发条件

- 触发条件：首期工程新增后端 API、前端 API client、前端 TypeScript 类型和管理后台页面，属于跨层契约。
- 范围：`/api/health`、`/api/admin/dashboard`、统一响应信封、运行端口配置、前端 API base URL。

### 2. 签名

- 后端健康检查：`GET /api/health`
- 后端管理后台概览：`GET /api/admin/dashboard`
- 后端端口环境变量：`PORT`
- 前端 API 地址环境变量：`VITE_API_BASE_URL`
- 可选数据库环境变量：`DATABASE_URL`

### 3. 契约

所有接口返回统一信封：

```json
{
  "success": true,
  "data": {},
  "message": "ok"
}
```

错误响应也使用同一信封：

```json
{
  "success": false,
  "data": null,
  "message": "bad request: ..."
}
```

`GET /api/health` 的 `data` 字段：

```json
{
  "service": "bc-backend",
  "status": "ok",
  "version": "0.1.0"
}
```

`GET /api/admin/dashboard` 的 `data` 字段必须包含：

- `metrics`
- `moduleGroups`
- `lotteries`
- `drawSources`
- `recentOrders`
- `groupBuyPlans`
- `finance`
- `financialAccounts`
- `robots`
- `users`
- `admins`
- `roles`
- `settings`
- `registration`
- `invitePolicy`

字段命名必须使用 `camelCase`，并与 `admin/src/types/dashboard.ts` 保持一致。

金额字段使用最小货币单位，例如 `amountMinor`、`balanceMinor`、`totalBalanceMinor`。返利比例使用 basis points，例如 `350` 表示 `3.5%`。

### 4. 校验与错误矩阵

| 条件 | 预期行为 |
|------|----------|
| 后端接口成功 | HTTP 200，`success=true`，`data` 非空 |
| 后端业务错误 | 对应 HTTP 状态码，`success=false`，`data=null` |
| 前端收到 `success=false` | API client 抛出 `message` |
| 前端收到 `data=null` | API client 抛出错误，不渲染空数据 |
| `PORT` 未设置 | 后端默认监听 `8080` |
| `VITE_API_BASE_URL` 未设置 | 前端使用同源 `/api`，由 Vite proxy 或部署网关转发 |
| `DATABASE_URL` 未设置 | 后端使用内存彩种仓储，接口契约不变 |
| `DATABASE_URL` 已设置但连接或迁移失败 | 后端启动失败，不静默降级 |

### 5. Good / Base / Bad Cases

- Good：本地联调用 `PORT=18080 cargo run` 和 `VITE_API_BASE_URL=http://127.0.0.1:18080 npm run dev -- --port 5174`，不影响已有 `8080` 服务。
- Base：默认开发时后端使用 `8080`，前端 dev server 通过 Vite proxy 转发 `/api`。
- Bad：前端类型使用 `snake_case`，或把 `defaultRechargeRebateBasisPoints` 写成浮点百分比。
- Bad：配置了 `DATABASE_URL` 但数据库不可用时继续以内存模式启动；这会误导用户以为数据已持久化。

### 6. 必要测试

- 后端需要运行 `cargo check`。
- 后端需要运行 `cargo test`，至少确认概览数据包含 `common`、`lottery`、`automation`、`growth` 模块组。
- 前端需要运行 `npm run build`，确认 TypeScript 类型与接口消费代码一致。
- 跨层联调需要请求 `/api/health` 和 `/api/admin/dashboard`，再打开管理后台确认页面无控制台错误。
- 未配置 `DATABASE_URL` 的本地启动需要确认 `/api/admin/lotteries` 和 `/api/admin/dashboard` 仍返回种子彩种。

### 7. Wrong vs Correct

#### 错误

```ts
export interface InvitePolicySummary {
  defaultRechargeRebatePercent: number;
}
```

这个写法暗示前端使用浮点百分比，后续可能导致返利计算精度问题。

#### 正确

```ts
export interface InvitePolicySummary {
  defaultRechargeRebateBasisPoints: number;
}
```

后端返回整数 basis points，前端展示时再除以 `100` 得到百分比。

---

## 场景：手机端实时事件接口

### 1. 范围 / 触发条件

- 触发条件：手机端需要实时刷新开奖、封盘、开盘、用户资金和订单状态。
- 范围：`GET /api/user/realtime` WebSocket、后端实时事件信封、公开事件与用户私有事件过滤、手机端事件归一化。

### 2. 签名

- 手机端实时事件：`GET /api/user/realtime`
- 可选鉴权参数：`token=<用户登录 token>`
- 旧系统路径 `/ws/lottery` 不属于当前系统契约，后续不得继续新增调用。

### 3. 契约

WebSocket 消息统一使用当前系统事件信封：

```json
{
  "event": "lottery.draw_result",
  "scope": "public",
  "occurredAt": "2026-06-05 14:29:00",
  "data": {}
}
```

公开彩种事件：

- `lottery.draw_result`：开奖完成，`data` 包含 `lotteryId`、`lotteryName`、`issue`、`drawNumber`、`resultNumbers`、`drawnAt`。
- `lottery.issue_closed`：期号封盘，`data` 包含 `lotteryId`、`issue`、`scheduledAt`、`saleClosedAt`、`status`。
- `lottery.issue_opened`：新期号开盘，`data` 包含 `lotteryId`、`issue`、`scheduledAt`、`saleClosedAt`、`status`。
- `system.heartbeat`：连接心跳。

用户私有事件：

- `user.balance_changed`：余额变化，必须只发送给 `data.userId` 对应用户连接。
- `user.order_changed`：注单变化，必须只发送给订单所属用户。
- `user.recharge_changed`：充值订单变化，必须只发送给充值订单所属用户。
- `user.withdrawal_changed`：提现订单变化，必须只发送给提现订单所属用户。

### 4. 校验与错误矩阵

| 条件 | 预期行为 |
|------|----------|
| 未携带 token 建立连接 | 允许连接，但只能接收公开事件 |
| 携带合法用户 token | 允许连接，可接收公开事件和本人私有事件 |
| 携带非法用户 token | 握手返回未授权错误 |
| 事件受众为其他用户 | 当前连接不得收到该事件 |
| 客户端消费过慢 | 后端记录中文 warning，跳过过旧事件，不影响主业务 |

### 5. Good / Base / Bad Cases

- Good：自动开奖产生 `lottery.draw_result`，手机端首页和下注页同步刷新。
- Good：用户下注扣款后只给该用户推送 `user.balance_changed` 和 `user.order_changed`。
- Base：匿名用户仍可通过实时连接获取开奖和开盘状态。
- Bad：手机端继续连接 `/ws/lottery`。
- Bad：把 `user.balance_changed` 广播给所有在线连接。

### 6. 必要测试

- 后端需要运行 `cargo check` 和 `cargo test`。
- 手机端需要运行 `npm run build`，确认 WebSocket 事件归一化类型可编译。
- 源码中不得保留 `/ws/lottery` 调用。

### 7. Wrong vs Correct

#### 错误

```ts
new WebSocket(`${API_BASE.replace(/^http/, 'ws')}/ws/lottery`)
```

这个写法继续依赖旧系统残留路径，且无法接收当前系统的用户私有事件。

#### 正确

```ts
const url = new URL('/api/user/realtime', API_BASE || window.location.origin)
url.protocol = url.protocol === 'https:' ? 'wss:' : 'ws:'
```

手机端必须连接当前系统用户侧实时接口，并把后端事件信封先归一化后再交给页面组件。

---

## 场景：彩种管理 CRUD 接口

### 1. 范围 / 触发条件

- 触发条件：新增后端彩种管理 API、前端彩种 API client、`useLotteries` hook 和彩种管理页面。
- 范围：彩种列表、详情、创建、更新、删除、销售开关、dashboard 彩种数据一致性。

### 2. 签名

- `GET /api/admin/lotteries`
- `GET /api/admin/lotteries/{id}`
- `POST /api/admin/lotteries`
- `PUT /api/admin/lotteries/{id}`
- `PATCH /api/admin/lotteries/{id}/sale`
- `DELETE /api/admin/lotteries/{id}`

### 3. 契约

所有接口继续使用统一 API 信封。彩种数据字段必须和 `admin/src/types/dashboard.ts` 的 `LotteryKind` 一致：

```json
{
  "id": "fc3d",
  "name": "福彩 3D",
  "numberType": "threeDigit",
  "drawMode": "api",
  "schedule": {
    "daily": {
      "time": "21:00:15"
    }
  },
  "saleEnabled": true,
  "groupBuy": {
    "enabled": true,
    "minShareAmountMinor": 100,
    "initiatorMinPercent": 10,
    "participantMinAmountMinor": 1000
  },
  "playCategories": ["direct", "groupThree", "groupSix"],
  "playConfigs": [
    {
      "ruleCode": "threeDirect",
      "enabled": true,
      "oddsBasisPoints": 104000
    },
    {
      "ruleCode": "threeGroupThree",
      "enabled": true,
      "oddsBasisPoints": 52000
    }
  ]
}
```

`playConfigs` 是每个彩种的单玩法配置，`oddsBasisPoints` 使用整数基点赔率，`10000` 表示 `1.00 倍`，`104000` 表示 `10.40 倍`。后端保存彩种时会按 `numberType` 补齐该号码类型下所有玩法，并根据启用玩法反推 `playCategories`，避免粗分类和单玩法配置漂移。

`schedule` 是单键枚举对象，只允许以下形状：

- 周期开奖：`{ "periodic": { "intervalSeconds": 60 } }`
- 每日开奖：`{ "daily": { "time": "21:00:15" } }`
- 周开奖：`{ "weekly": { "weekdays": ["Tuesday", "Thursday"], "time": "21:00:00" } }`

注意：Rust 枚举结构体变体字段需要显式使用 `#[serde(rename_all_fields = "camelCase")]`，否则 `intervalSeconds` 会错误地退回为 `interval_seconds`。

`PATCH /api/admin/lotteries/{id}/sale` 请求体：

```json
{
  "saleEnabled": false
}
```

### 4. 校验与错误矩阵

| 条件 | 预期行为 |
|------|----------|
| 彩种 ID 为空 | HTTP 400，返回 `lottery id is required` |
| 彩种名称为空 | HTTP 400，返回 `lottery name is required` |
| 玩法分类为空 | HTTP 400，返回 `at least one play category is required` |
| 单玩法配置号码类型与彩种不匹配 | HTTP 400，返回玩法号码类型错误 |
| 单玩法赔率小于等于 0 | HTTP 400，返回 `play odds basis points must be greater than zero` |
| 周期开奖秒数为 0 | HTTP 400，返回 `periodic interval must be greater than zero` |
| 每日开奖时间为空 | HTTP 400，返回 `daily draw time is required` |
| 周开奖星期或时间为空 | HTTP 400，返回对应 weekly 错误 |
| 合买每份最低金额或参与最低金额小于等于 0 | HTTP 400，返回对应金额错误 |
| 发起人最低比例大于 100 | HTTP 400，返回 `initiator min percent must be between 0 and 100` |
| 创建重复彩种 ID | HTTP 409，返回重复错误 |
| 更新路径 ID 与请求体 ID 不一致 | HTTP 400，返回 `path id must match lottery id` |
| 查询、更新、删除不存在彩种 | HTTP 404，返回 not found |

### 5. Good / Base / Bad Cases

- Good：前端发送 `schedule.periodic.intervalSeconds`，后端成功创建彩种，dashboard 的 `lotteries` 数量同步增加。
- Base：服务重启后内存仓储恢复为种子彩种，适合当前无数据库阶段。
- Bad：前端发送 `intervalSeconds`，后端只接受 `interval_seconds`；这会让创建接口返回反序列化失败，并破坏跨层契约。

### 6. 必要测试

- 后端需要覆盖 `LotteryStore` 的创建、重复 ID、无效开奖周期和销售开关。
- 后端需要覆盖 `DrawSchedule` 对 `intervalSeconds` 的序列化和反序列化。
- 跨层联调需要至少请求列表、创建、销售开关、删除和 dashboard，确认同一仓储数据一致，并确认 `playConfigs` 随彩种列表和 dashboard 一起返回。
- 前端需要运行 `npm run build`，并通过浏览器验证彩种管理页面能新增和删除彩种。

### 7. Wrong vs Correct

#### 错误

```rust
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DrawSchedule {
    Periodic { interval_seconds: u32 },
}
```

这个写法只会重命名枚举变体，不会重命名结构体变体字段，后端仍会期待 `interval_seconds`。

#### 正确

```rust
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum DrawSchedule {
    Periodic { interval_seconds: u32 },
}
```

这样后端会按前端契约接受并返回 `intervalSeconds`。

---

## 场景：玩法规则评估接口

### 1. 范围 / 触发条件

- 触发条件：新增或修改彩票玩法的注数计算、投注展开、中奖判断、管理后台规则验证页面。
- 范围：3 位玩法、5 位前三/中三/后三玩法、5 位大小单双、后端服务层规则校验、前端 API client 和 `usePlayRules` hook。

### 2. 签名

- `GET /api/admin/play-rules`
- `POST /api/admin/play-rules/evaluate`

### 3. 契约

所有接口继续使用统一 API 信封。

`GET /api/admin/play-rules` 的 `data` 字段返回规则目录，字段必须使用 `camelCase`：

```json
[
  {
    "code": "threeDirect",
    "label": "3 位直选",
    "numberType": "threeDigit",
    "category": "direct",
    "window": "full",
    "description": "按百位、十位、个位顺序完全匹配"
  }
]
```

规则目录必须包含 `category`，可选值与彩种 `playCategories` 一致：`direct`、`directCombination`、`groupThree`、`groupSix`、`bigSmallOddEven`。前端展示和赔率配置应读取该字段，不要通过玩法代码字符串自行推断分类。

`POST /api/admin/play-rules/evaluate` 请求体：

```json
{
  "numberType": "threeDigit",
  "ruleCode": "threeDirect",
  "selection": {
    "positions": [[2], [4], [7]]
  },
  "drawNumber": "2,4,7"
}
```

`drawNumber` 为开奖号码展示格式，使用英文逗号分隔每一位数字。投注展开结果 `expandedBets` 和命中投注 `matchedBets` 仍使用紧凑投注编码，例如 `247`。

响应 `data` 字段：

```json
{
  "ruleCode": "threeDirect",
  "stakeCount": 1,
  "expandedBets": ["247"],
  "isWinning": true,
  "matchedBets": ["247"]
}
```

选号结构按玩法使用：

- 直选：`selection.positions`，必须是 3 个位置数组。
- 直选组合、组三复式、组六复式：`selection.numbers`。
- 组三胆拖、组六胆拖：`selection.bankerNumbers` 和 `selection.dragNumbers`。
- 大小单双：`selection.bigSmallOddEven`，每项包含 `position` 和 `attributes`。

### 4. 校验与错误矩阵

| 条件 | 预期行为 |
|------|----------|
| `numberType` 与 `ruleCode` 不匹配 | HTTP 400，返回 `rule code does not match number type` |
| 3 位玩法开奖号码不是 3 个数字 | HTTP 400，返回开奖号码长度或数字错误 |
| 5 位玩法开奖号码不是 5 个数字 | HTTP 400，返回开奖号码长度或数字错误 |
| 直选没有 3 个位置选择 | HTTP 400，返回 `direct play requires three position selections` |
| 选号为空 | HTTP 400，返回 `digit selection cannot be empty` |
| 选号数字大于 9 | HTTP 400，返回 `digit selection must be between 0 and 9` |
| 组三胆码数量不是 1 | HTTP 400，返回胆码数量错误 |
| 组六胆码数量不是 1 或 2 | HTTP 400，返回胆码数量错误 |
| 胆码和拖码重复 | HTTP 400，返回 `banker digits and drag digits cannot overlap` |
| 大小单双没有选择属性 | HTTP 400，返回大小单双属性错误 |

### 5. Good / Base / Bad Cases

- Good：`threeDirect` 选择 `[[2], [4], [7]]`，开奖号码 `2,4,7`，返回 `stakeCount=1`、`isWinning=true`。
- Good：`fiveBackGroupSix` 选择 `2,4,7,9`，开奖号码 `7,8,9,4,2` 的后三为 `942`，属于组六且数字都在选号范围内，应命中。
- Good：`fiveBigSmallOddEven` 当前默认按后两位判断，开奖号码 `7,8,9,4,2` 的十位 `4` 为小、个位 `2` 为双。
- Base：规则评估只计算注数、展开投注和命中，不处理赔率、奖金、订单金额、余额扣减或派奖。
- Bad：前端自行计算玩法结果并只把中奖状态传给后端；后续订单和派奖必须复用后端规则引擎。

### 6. 必要测试

- 后端需要覆盖 3 位直选精确顺序匹配。
- 后端需要覆盖 3 位组三复式、组三胆拖、组六复式、组六胆拖的注数和顺序无关命中。
- 后端需要覆盖 5 位前/中/后窗口选择，例如中三使用第 2-4 位、后三使用第 3-5 位。
- 后端需要覆盖 5 位直选组合排列注数。
- 后端需要覆盖 5 位大小单双默认后两位口径。
- 后端需要覆盖胆码/拖码重复等基础校验。
- 前端需要运行 `npm run build`，确认 `admin/src/types/playRules.ts` 与后端契约一致。
- 跨层联调需要请求规则目录和至少两个评估接口，再在管理后台“玩法规则”页面完成一次计算。

### 7. Wrong vs Correct

#### 错误

```ts
const stakeCount = selectedDigits.length * 2;
```

这个写法把玩法公式写在页面里，后续订单、派奖和手机端很容易与后台展示不一致。

#### 正确

```ts
const result = await evaluatePlayRule(payload);
```

前端只提交选号和开奖号码，注数、展开投注和中奖判断都由后端规则引擎返回。

---

## 场景：玩法与赔率配置接口

### 1. 范围 / 触发条件

- 触发条件：新增或修改彩种玩法配置、赔率维护、订单赔率快照、计奖赔率计算或管理后台“玩法规则与赔率”页面。
- 范围：彩种 `playConfigs`、玩法目录 `category`、订单 `oddsBasisPoints`、结算 `oddsBasisPoints`、前端赔率编辑与展示。

### 2. 签名

- `GET /api/admin/play-rules`
- `GET /api/admin/lotteries`
- `PUT /api/admin/lotteries/{id}`
- `POST /api/admin/orders`
- `POST /api/admin/settlements/draw-issues/{id}`
- `GET /api/admin/dashboard`

### 3. 契约

赔率统一使用整数基点字段 `oddsBasisPoints`，`10000` 表示 `1.00 倍`。前端展示时可以格式化为 `10.40 倍`，但接口和服务层不得使用浮点数保存赔率。

彩种单玩法配置：

```json
{
  "playConfigs": [
    {
      "ruleCode": "threeDirect",
      "enabled": true,
      "oddsBasisPoints": 104000
    },
    {
      "ruleCode": "threeGroupSix",
      "enabled": false,
      "oddsBasisPoints": 50000
    }
  ]
}
```

订单创建时必须从彩种 `playConfigs` 读取对应玩法配置：

- 未配置该玩法：拒绝创建订单。
- 配置存在但 `enabled=false`：拒绝创建订单。
- 配置存在且启用：把当时的 `oddsBasisPoints` 保存进订单。

结算派奖必须使用订单上的赔率快照，不能重新读取当前彩种赔率。派奖公式：

```text
命中投注数 × 单注金额 × oddsBasisPoints / 10000
```

### 4. 校验与错误矩阵

| 条件 | 预期行为 |
|------|----------|
| 彩种提交了不属于当前号码类型的玩法 | HTTP 400，返回玩法号码类型错误 |
| 彩种提交了小于等于 0 的赔率 | HTTP 400，返回 `play odds basis points must be greater than zero` |
| 彩种保存后没有任何启用玩法 | HTTP 400，返回 `at least one play category is required` |
| 订单玩法没有配置 | HTTP 400，返回 `lottery does not configure this play rule` |
| 订单玩法被停用 | HTTP 400，返回 `lottery does not enable this play rule` |
| 结算中奖订单赔率快照小于等于 0 | HTTP 400，返回派奖金额或赔率错误 |

### 5. Good / Base / Bad Cases

- Good：`fc3d.threeDirect` 设置为 `104000`，创建订单后订单响应包含 `oddsBasisPoints=104000`。
- Good：管理员随后把 `fc3d.threeDirect` 改成 `98000`，旧订单结算仍按 `104000` 派奖。
- Good：3 位页面只展示 3 位玩法和 3 位彩种，5 位页面只展示 5 位玩法和 5 位彩种。
- Bad：结算时读取当前彩种赔率；这会导致历史订单派奖被后续调价影响。
- Bad：前端用小数浮点提交赔率，例如 `10.4`；接口必须提交 `104000`。

### 6. 必要测试

- 后端需要覆盖彩种保存后补齐对应号码类型的所有 `playConfigs`。
- 后端需要覆盖订单创建保存赔率快照，并拒绝停用玩法。
- 后端需要覆盖结算按订单赔率快照计算派奖。
- 前端需要运行 `npm run build`，确认玩法、彩种、订单和结算类型一致。
- 跨层联调需要完成“修改彩种玩法赔率 → 创建订单 → 开奖结算 → 核对订单和结算赔率”。

### 7. Wrong vs Correct

#### 错误

```rust
let payout = matched_count * unit_amount * current_lottery_odds(rule_code);
```

这个写法会让后续调价影响历史订单。

#### 正确

```rust
let payout = matched_count * order.unit_amount_minor * order.odds_basis_points / 10_000;
```

订单创建时保存赔率快照，结算时只读取订单自身的赔率。

---

## 场景：订单与投注基础接口

### 1. 范围 / 触发条件

- 触发条件：新增或修改投注订单创建、订单列表、订单详情、订单取消、dashboard 最近订单。
- 范围：后端订单领域模型、内存订单仓储、订单 API、玩法规则引擎复用、彩种配置校验、开奖期号销售状态校验、前端订单页面。

### 2. 签名

- `GET /api/admin/orders`
- `GET /api/admin/orders/{id}`
- `POST /api/admin/orders`
- `PATCH /api/admin/orders/{id}/cancel`
- `GET /api/admin/dashboard` 的 `recentOrders` 和今日订单指标读取订单仓储。

### 3. 契约

所有接口继续使用统一 API 信封。订单金额字段必须使用最小货币单位整数，不使用浮点数。

创建订单前必须存在同彩种、同 `issue` 的开奖期号，并且该期号状态必须为 `open`。`closed`、`drawn`、`cancelled` 期号都不能继续创建订单。

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
  "id": "O000000000001",
  "userId": "U10001",
  "lotteryId": "fc3d",
  "lotteryName": "福彩 3D",
  "issue": "2026155",
  "ruleCode": "threeDirect",
  "numberType": "threeDigit",
  "selection": {
    "positions": [[2], [4], [7]],
    "numbers": [],
    "bankerNumbers": [],
    "dragNumbers": [],
    "bigSmallOddEven": []
  },
  "stakeCount": 1,
  "unitAmountMinor": 200,
  "amountMinor": 200,
  "oddsBasisPoints": 104000,
  "expandedBets": ["247"],
  "status": "pendingDraw",
  "createdAt": "unix:1780386834"
}
```

订单状态当前支持：

- `pendingDraw`
- `won`
- `lost`
- `cancelled`

本阶段只有创建订单和取消订单会真实流转状态，开奖、计奖、派奖后续实现。

### 4. 校验与错误矩阵

| 条件 | 预期行为 |
|------|----------|
| 用户 ID 为空 | HTTP 400，返回 `user id is required` |
| 彩种 ID 为空 | HTTP 400，返回 `lottery id is required` |
| 彩种不存在 | HTTP 404，返回彩种不存在 |
| 请求彩种 ID 与读取的彩种不一致 | HTTP 400，返回 `request lottery id does not match lottery` |
| 期号为空 | HTTP 400，返回 `issue is required` |
| 期号不存在 | HTTP 404，返回 `draw issue ... not found for lottery ...` |
| 期号不是 `open` | HTTP 400，返回 `draw issue is not open for order creation` |
| 单注金额小于等于 0 | HTTP 400，返回 `unit amount must be greater than zero` |
| 彩种停售 | HTTP 400，返回 `lottery is not on sale` |
| 玩法号码类型与彩种号码类型不匹配 | HTTP 400，返回 `rule code does not match lottery number type` |
| 彩种未配置对应玩法 | HTTP 400，返回 `lottery does not configure this play rule` |
| 彩种停用对应玩法 | HTTP 400，返回 `lottery does not enable this play rule` |
| 玩法选号无效 | HTTP 400，透传玩法规则引擎的校验错误 |
| 订单金额溢出 | HTTP 400，返回 `order amount is too large` |
| 查询或取消不存在订单 | HTTP 404，返回订单不存在 |
| 取消非待开奖订单 | HTTP 400，返回 `only pending draw orders can be cancelled` |

### 5. Good / Base / Bad Cases

- Good：`fc3d` 创建 `threeDirect` 订单，选号 `247`、单注 `200` 分，后端返回 `stakeCount=1`、`amountMinor=200`、`oddsBasisPoints` 和 `expandedBets=["247"]`。
- Good：订单创建前先创建 `fc3d` 的 open 期号 `2026155`，订单请求使用同一期号才能成功。
- Good：创建订单后重新请求 `/api/admin/dashboard`，`recentOrders` 包含该订单，今日订单指标等于内存订单数量。
- Base：订单仓储当前是内存模式，服务重启后订单清空；这适合当前后台功能验证。
- Bad：前端手工输入一个不存在的期号仍然提交订单；订单必须从 open 期号中选择，后端也必须再次校验。
- Bad：前端传 `amountMinor` 给后端并由后端直接保存；订单金额必须由后端根据注数和单注金额计算。
- Bad：机器人购彩绕过订单接口直接写订单；后续机器人必须复用订单创建校验。

### 6. 必要测试

- 后端需要覆盖订单创建时按玩法规则引擎计算注数、金额和展开投注。
- 后端需要覆盖 open 期号允许投注，closed/drawn/cancelled 期号拒绝投注。
- 后端需要覆盖彩种未配置或停用单玩法时拒绝创建订单。
- 后端需要覆盖取消待开奖订单，以及重复取消被拒绝。
- 后端需要运行 `cargo fmt --check`、`cargo check`、`cargo test`。
- 前端需要运行 `npm run build`。
- 跨层联调需要创建 open 期号、创建订单、关闭期号后确认同一期号拒绝新订单，并在 dashboard 最近订单确认回流。

### 7. Wrong vs Correct

#### 错误

```ts
await createOrder({
  amountMinor: stakeCount * unitAmountMinor,
  lotteryId,
  ruleCode,
  selection,
});
```

这个写法让前端决定订单金额，后续财务和派奖会失去可信入口。

#### 正确

```ts
await createOrder({
  lotteryId,
  ruleCode,
  selection,
  unitAmountMinor,
  userId,
  issue,
});
```

后端读取彩种配置，调用玩法规则引擎计算 `stakeCount` 和 `amountMinor`，再保存订单。

---

## 场景：用户端下注页接口

### 1. 范围 / 触发条件

- 触发条件：手机端下注页新增或修改玩法配置读取、批量下单、用户注单记录查询。
- 范围：`/api/user/bet/*` 路由、手机端动态下注页、订单仓储、财务扣款、玩法规则引擎和 OpenAPI 文档。

### 2. 签名

- `GET /api/user/bet/page-config/{lottery_id}`：读取当前销售彩种的下注页配置。
- `GET /api/user/bet/orders`：读取当前登录用户自己的投注订单。
- `POST /api/user/bet/orders`：批量创建当前登录用户的投注订单。

### 3. 契约

所有接口必须使用用户 Bearer Token。手机端不能继续请求旧 `/api/bet/page-config/{code}`、`/api/bet/place-batch` 或 `/api/bet/orders`。

下注页配置响应使用 `camelCase`，前端可以兼容旧 `snake_case`，但新后端接口必须输出：

```json
{
  "lottery": {
    "code": "txffc",
    "name": "腾讯分分彩",
    "category": "overseas",
    "drawInterval": 60,
    "groupBuyEnabled": false
  },
  "round": {
    "issue": "202606051234",
    "status": "selling",
    "scheduledDrawAt": "2026-06-05 12:35:00",
    "saleStopAt": "2026-06-05 12:34:30"
  },
  "latestDraw": {
    "issue": "202606051233",
    "resultNumbers": ["1", "2", "3", "4", "5"],
    "openedAt": "2026-06-05 12:34:00"
  },
  "plays": [
    {
      "code": "fiveFrontDirect",
      "ruleCode": "fiveFrontDirect",
      "inputMode": "position-grid",
      "positionGridKind": "direct",
      "positions": [{ "key": "first", "label": "第 1 位" }],
      "odds": "9.50",
      "unitAmount": "2.00"
    }
  ]
}
```

用户端批量下单请求不允许传 `userId`，后端必须从登录会话中取当前用户：

```json
{
  "orders": [
    {
      "lotteryId": "txffc",
      "issue": "202606051234",
      "ruleCode": "fiveFrontDirect",
      "selection": {
        "positions": [[1], [2], [3]]
      },
      "unitAmountMinor": 200
    }
  ]
}
```

手机端倍数没有单独后端字段时，前端可以把 `unitAmountMinor` 折算为“单注金额 × 倍数”；后端仍按玩法展开注数计算 `amountMinor`，不能信任前端提交总金额。

用户注单列表返回的订单详情必须包含 `orderSource`：

- `direct`：用户独立下单。
- `groupBuy`：合买满单后生成的真实投注订单。

手机端注单记录必须按该字段展示“独立下单”或“合买下单”，不能只用订单号、资金流水或旧系统 `source_name` 猜测。

下注页“加入购彩篮”是把当前草稿加入本地待提交购物篮，不是跨彩种组合投注。前端加入和提交时都必须校验购彩篮内所有单据属于同一个彩种和同一期号。

### 4. 校验与错误矩阵

| 条件 | 预期行为 |
|------|----------|
| 未登录读取下注配置或下单 | HTTP 401，返回未授权 |
| 下注页彩种不存在 | HTTP 404，返回彩种不存在 |
| 下注页彩种停售 | HTTP 400，返回 `彩种已停售` |
| 批量下单 `orders` 为空 | HTTP 400，返回 `请先选择投注内容` |
| 批量下单超过 50 笔 | HTTP 400，返回 `一次最多提交 50 笔投注` |
| 购彩篮混入不同彩种 | 手机端拦截，提示 `购彩篮只能提交同一个彩种的投注`，不请求后端 |
| 购彩篮混入旧期号 | 手机端拦截，提示清空购彩篮后重新选择，不请求后端 |
| 请求期号不存在或非 `open` | HTTP 404/400，沿用订单期号校验错误 |
| 玩法、号码类型、选号或赔率无效 | HTTP 400，沿用订单和玩法规则引擎错误 |
| 当前用户余额不足 | HTTP 400/409，沿用财务账户余额校验错误 |
| 扣款失败 | 回滚本次未入账订单，返回财务错误 |

### 5. Good / Base / Bad Cases

- Good：进入销售中的 `txffc` 下注页，读取到 `round.status=selling`、最近开奖和所有已启用玩法赔率。
- Good：前端提交 `positions`、`numbers`、`bankerNumbers/dragNumbers` 或 `bigSmallOddEven`，后端复用订单规则计算注数和扣款。
- Good：直选组合前端使用 `positionGridKind=direct_combination` 多选数字，并按排列数显示注数；后端仍以 `selection.numbers` 展开排列投注。
- Good：用户独立下注后注单记录展示 `orderSource=direct` 和“独立下单”；合买满单成单后注单记录展示 `orderSource=groupBuy` 和“合买下单”。
- Good：用户切换彩种或期号变化后，购彩篮不能继续提交旧彩种或旧期号单据。
- Base：没有 open 期号时，下注页返回 `round.status=opening`，手机端轮询下一期，不允许提交。
- Bad：手机端继续把 `play_code/numbers/amount` 发到旧 `/bet/place-batch`；该接口不是当前系统契约。
- Bad：用户端批量下单允许传 `userId`；这会让用户冒充他人下单。

### 6. 必要测试

- 后端运行 `cargo check --manifest-path backend/Cargo.toml`。
- 后端测试 `cargo test --manifest-path backend/Cargo.toml mobile_bet -- --nocapture`，覆盖当前期、最近开奖、已启用玩法和直选组合配置。
- 后端测试需要覆盖普通订单来源为 `direct`，合买满单生成订单来源为 `groupBuy`。
- OpenAPI 测试必须包含 `/user/bet/page-config/{lottery_id}` 和 `/user/bet/orders`。
- 手机端运行 `cd mobile && npm run build`，确认下注页 API 客户端、动态配置归一化和批量提交类型通过。

### 7. Wrong vs Correct

#### 错误

```ts
await http.post('/bet/place-batch', {
  lottery_code,
  issue,
  items: [{ play_code, numbers, amount }],
})
```

这个写法继续沿用旧接口，并让前端决定订单总额。

#### 正确

```ts
await createUserBetOrders([{
  lotteryId,
  issue,
  ruleCode,
  selection,
  unitAmountMinor,
}])
```

后端从登录态绑定用户，校验 open 期号、玩法赔率和余额，再创建订单并扣款。

---

## 场景：开奖期号与开奖源接口

### 1. 范围 / 触发条件

- 触发条件：新增或修改开奖源列表、开奖期号创建、封盘、开奖、取消，以及管理后台开奖期号页面。
- 范围：后端开奖领域模型、内存开奖仓储、开奖 API、彩种开奖模式复用、前端 draw API client、`useDraws` hook 和“开奖期号与开奖源”页面。

### 2. 签名

- `GET /api/admin/draw-sources`
- `POST /api/admin/draw-sources`
- `PUT /api/admin/draw-sources/{id}`
- `DELETE /api/admin/draw-sources/{id}`
- `GET /api/admin/draw-issues`
- `GET /api/admin/draw-issues/{id}`
- `POST /api/admin/draw-issues`
- `PATCH /api/admin/draw-issues/{id}/close`
- `PATCH /api/admin/draw-issues/{id}/draw`
- `PATCH /api/admin/draw-issues/{id}/cancel`

### 3. 契约

所有接口继续使用统一 API 信封，字段命名必须使用 `camelCase`。

创建期号请求：

```json
{
  "lotteryId": "fc3d",
  "issue": "2026156",
  "scheduledAt": "2026-06-02 21:00:15",
  "saleClosedAt": "2026-06-02 20:59:45"
}
```

执行开奖请求：

```json
{
  "drawNumber": "2,4,7"
}
```

`platform` 开奖模式可以传空对象 `{}`，后端本地生成逗号分隔开奖号码。`api` 开奖模式可以传空对象 `{}`，后端按彩种查找外部开奖源；已配置外部源的彩种不能静默回退到本地生成器。`manual` 开奖模式必须传 `drawNumber`，格式为英文逗号分隔数字，例如 `2,4,7`、`7,8,9,4,2` 或 `1,6,2,4,3,5,7,9,10,8`。后端对 3 位和 5 位号码兼容读取旧的紧凑数字串，其它号码类型必须使用英文逗号分隔；保存和返回统一使用英文逗号分隔格式。

当前已接入的外部开奖源：

- `api68-fc3d`：`fc3d` 福彩 3D 和 `pl3` 排列 3 默认复用 API68 全国彩接口，`lotCode=10041`，响应中按 `result.data[].preDrawIssue` 匹配后台期号，使用 `preDrawCode` 作为开奖号码。
- `api68-au5`：`au5` 澳洲幸运5默认使用 API68 CQShiCai 单彩种接口，`lotCode=10010`，响应中按 `result.data.preDrawIssue` 匹配后台期号，使用 `preDrawCode` 作为英文逗号分隔开奖号码。
- API68 批量接入彩种：`bjpk10`、`tjssc`、`xjssc`、`gd11x5`、`jsk3`、`au10`、`au20`、`bjkl8`、`jx11x5`、`js11x5`、`ah11x5`、`sh11x5`、`ln11x5`、`hb11x5`、`gx11x5`、`jl11x5`、`nmg11x5`、`zj11x5`、`gxk3`、`jlk3`、`hebk3`、`nmgk3`、`ahk3`、`fjk3`、`hubk3`、`bjk3`。这些来源按彩种分别绑定 API68 的 PKS、CQShiCai、ElevenFive、FastThree 或 LuckTwenty endpoint。
- `kj-txffc`：`txffc` 腾讯分分彩默认使用 KJAPI 接口，`lotKey=txffc`，响应中按 `result.data.preDrawIssue` 匹配后台期号，使用 `preDrawCode` 作为英文逗号分隔开奖号码；生成下一期时优先读取 `result.data.drawIssue` 和 `result.data.drawTime`。
- `preDrawCode` 必须继续经过后端开奖号码校验，保存和返回仍统一为英文逗号分隔格式。
- API68 解析器必须兼容 `result.data` 为数组或单对象两种形态；单对象接口还应读取 `drawIssue` 和 `drawTime` 作为下一期锚点。
- 暂未配置外部源的 API 彩种仍保留本地生成器占位能力，仅用于当前内存演示阶段；生产接入时需要显式配置来源。

开奖源响应：

```json
{
  "id": "api68-fc3d",
  "name": "API68 福彩 3D/排列 3",
  "mode": "api",
  "provider": "api68",
  "lotCode": "10041",
  "endpoint": "https://api.api68.com/QuanGuoCai/getLotteryInfoList.do",
  "editable": true,
  "reusableForLotteryIds": ["fc3d", "pl3"]
}
```

保存开奖源请求：

```json
{
  "id": "api68-fc3d",
  "name": "API68 福彩 3D/排列 3",
  "provider": "api68",
  "lotCode": "10041",
  "endpoint": "https://api.api68.com/QuanGuoCai/getLotteryInfoList.do",
  "reusableForLotteryIds": ["fc3d", "pl3"]
}
```

`endpoint` 可为空；为空时后端按供应商写入默认 endpoint。福彩 3D/排列 3 默认来源写入 `draw_sources` 表，endpoint 为 `https://api.api68.com/QuanGuoCai/getLotteryInfoList.do`；澳洲幸运5默认来源写入 `draw_sources` 表，endpoint 为 `https://api.api68.com/CQShiCai/getBaseCQShiCai.do`；腾讯分分彩默认来源写入 `draw_sources` 表，endpoint 为 `https://kjapi.net/hall/hallajax/getLotteryInfo`。后续修改 endpoint 必须通过后台“开奖源配置”或开奖源 API 写入数据库，不通过环境变量覆盖。`platform` 来源也会出现在 `GET /draw-sources` 中，但 `editable=false`，不支持通过 API 源配置接口修改。

KJAPI 来源的 `lotCode` 字段在当前跨层模型中复用为 `lotKey`，例如腾讯分分彩保存 `lotCode="txffc"`；后端请求时会按供应商自动拼接 `lotKey=txffc`，不是 `lotCode=txffc`。后台展示标签可写为 `lotCode / lotKey`。

开奖期号响应：

```json
{
  "id": "D000000000001",
  "lotteryId": "fc3d",
  "lotteryName": "福彩 3D",
  "issue": "2026156",
  "numberType": "threeDigit",
  "drawMode": "api",
  "scheduledAt": "2026-06-02 21:00:15",
  "saleClosedAt": "2026-06-02 20:59:45",
  "status": "open",
  "drawNumber": null,
  "drawnAt": null,
  "createdAt": "unix:1780387757"
}
```

期号状态当前支持：

- `open`
- `closed`
- `drawn`
- `cancelled`

### 4. 校验与错误矩阵

| 条件 | 预期行为 |
|------|----------|
| 彩种 ID 为空 | HTTP 400，返回 `lottery id is required` |
| 彩种不存在 | HTTP 404，返回彩种不存在 |
| 请求彩种 ID 与读取的彩种不一致 | HTTP 400，返回 `request lottery id does not match lottery` |
| 期号为空 | HTTP 400，返回 `issue is required` |
| 开奖时间为空 | HTTP 400，返回 `scheduled time is required` |
| 封盘时间为空 | HTTP 400，返回 `sale close time is required` |
| 同一彩种重复创建同一期号 | HTTP 409，返回期号重复 |
| 开奖源 ID 为空或包含非法字符 | HTTP 400，返回开奖源 ID 错误 |
| 开奖源名称为空 | HTTP 400，返回开奖源名称必填 |
| `lotCode / lotKey` 为空或包含非字母数字、连字符、下划线字符 | HTTP 400，返回 `lot code` 错误 |
| 复用彩种为空 | HTTP 400，返回复用彩种必填 |
| 复用彩种不存在 | HTTP 404，返回彩种不存在 |
| 复用彩种不是 `api` 开奖模式 | HTTP 400，返回彩种不是 API 开奖模式 |
| 同一 API 彩种已绑定其他开奖源 | HTTP 409，返回彩种已绑定其他来源 |
| 关闭非 `open` 期号 | HTTP 400，返回 `only open draw issues can be closed` |
| 手动开奖缺少号码 | HTTP 400，返回 `manual draw requires draw number` |
| 3 位彩种号码不是 3 个数字 | HTTP 400，返回号码长度或数字错误 |
| 5 位彩种号码不是 5 个数字 | HTTP 400，返回号码长度或数字错误 |
| PK10、11 选 5、快 3、快乐 8/幸运 20 号码不符合长度、范围或去重规则 | HTTP 400，返回号码长度、范围或重复错误 |
| 已配置外部源的 API 彩种未匹配到当前期号 | HTTP 404，返回 API 开奖号码未找到 |
| 已配置外部源请求失败或响应结构异常 | HTTP 500，返回外部开奖源错误，不生成假号码 |
| 已开奖或已取消期号再次开奖 | HTTP 400，返回 `draw issue cannot be drawn in current status` |
| 已开奖期号取消 | HTTP 400，返回 `drawn draw issue cannot be cancelled` |
| 已取消期号重复取消 | HTTP 400，返回 `draw issue is already cancelled` |
| 查询或操作不存在期号 | HTTP 404，返回期号不存在 |

### 5. Good / Base / Bad Cases

- Good：`fc3d` 创建期号 `2026143` 后调用 `PATCH /draw` 传 `{}`，后端从 API68 匹配 `preDrawIssue=2026143`，保存 `preDrawCode`，并返回 `status="drawn"`。
- Good：`pl3` 创建同一期号 `2026143` 后调用 `PATCH /draw` 传 `{}`，后端复用同一个 `api68-fc3d` 来源并保存同一条 `preDrawCode`。
- Good：管理员把 `api68-fc3d` 的 `reusableForLotteryIds` 保存为 `["fc3d"]` 后，`pl3` 不再命中该外部源。
- Good：`manual-test` 创建期号后传 `{"drawNumber":"7,8,9,4,2"}`，后端按 `fiveDigit` 校验并保存逗号分隔开奖结果。
- Base：开奖期号仓储当前是内存模式，服务重启后期号清空；这适合当前后台流程验证。没有外部源配置的 API 彩种仍是占位生成，不代表生产能力。
- Bad：前端为 `manual` 期号传空对象执行开奖；后端必须拒绝，不能静默生成号码。
- Bad：两个 API 来源同时绑定 `pl3`；保存时必须拒绝，避免开奖来源歧义。
- Bad：`fc3d` API68 没有当前期号时继续本地生成号码；已配置外部源的 API 彩种必须返回错误，避免伪造开奖结果。
- Bad：开奖后直接改订单状态或资金余额；本阶段还没有计奖、派奖和资金流水，开奖只记录结果事实。

### 6. 必要测试

- 后端需要覆盖期号创建、关闭销售、平台生成号码、API68 期号匹配、多彩种复用、重复彩种绑定拒绝、外部源失败、手动开奖号码必填、号码长度和数字校验。
- 后端需要覆盖已开奖期号不能重复开奖或取消。
- 后端需要运行 `cargo fmt --check`、`cargo check`、`cargo test`。
- 前端需要运行 `npm run build`。
- 跨层联调需要请求开奖源、保存复用彩种、创建 `fc3d/pl3` 期号、封盘、API 开奖、手动开奖，并在管理后台页面确认结果回显。

### 7. Wrong vs Correct

#### 错误

```tsx
await drawIssueResult(issue.id, {
  drawNumber: issue.drawNumber ?? '0,0,0',
});
```

这个写法让前端为平台/API 开奖制造兜底号码，后端无法区分真实开奖结果和前端临时值。

#### 正确

```tsx
await drawIssueResult(issue.id, issue.drawMode === 'manual'
  ? { drawNumber: form.drawNumber.trim() }
  : {}
);
```

后端根据彩种开奖模式决定是校验管理员录入号码、由平台生成器生成号码，还是由已配置 API 开奖源拉取真实号码。

---

## 场景：计奖派奖基础接口

### 1. 范围 / 触发条件

- 触发条件：新增或修改开奖后订单计奖、基础派奖结果、订单状态流转、结算批次列表和管理后台计奖派奖页面。
- 范围：后端结算领域模型、订单结算字段、内存订单仓储结算方法、结算 API、前端 settlement API client、`useSettlements` hook、“计奖派奖”页面和订单列表结算展示。

### 2. 签名

- `GET /api/admin/settlements`
- `GET /api/admin/settlements/{id}`
- `POST /api/admin/settlements/draw-issues/{id}`
- `GET /api/admin/orders` 和 `GET /api/admin/orders/{id}` 的订单响应新增结算字段。
- `GET /api/admin/dashboard` 的 `recentOrders` 新增结算字段。

### 3. 契约

所有接口继续使用统一 API 信封，金额字段必须使用最小货币单位整数。派奖金额使用订单创建时保存的 `oddsBasisPoints` 赔率快照计算，不能在结算时重新读取当前彩种赔率。

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
  "issue": "2026200",
  "drawNumber": "0,2,3",
  "settledOrderCount": 1,
  "winningOrderCount": 1,
  "totalStakeAmountMinor": 200,
  "totalPayoutMinor": 2000,
  "createdAt": "unix:1780388582",
  "orders": [
    {
      "orderId": "O000000000001",
      "userId": "U10001",
      "ruleCode": "threeDirect",
      "stakeCount": 1,
      "amountMinor": 200,
      "isWinning": true,
      "matchedBets": ["023"],
      "oddsBasisPoints": 104000,
      "payoutMinor": 2080,
      "status": "won"
    }
  ]
}
```

订单响应新增字段：

```json
{
  "drawNumber": "0,2,3",
  "matchedBets": ["023"],
  "oddsBasisPoints": 104000,
  "payoutMinor": 2080,
  "settledAt": "unix:1780388582"
}
```

结算后订单状态：

- 命中订单：`won`
- 未命中订单：`lost`
- 已取消订单：保持 `cancelled`，不参与结算。

### 4. 校验与错误矩阵

| 条件 | 预期行为 |
|------|----------|
| 查询不存在结算批次 | HTTP 404，返回结算批次不存在 |
| 查询不存在开奖期号后结算 | HTTP 404，返回期号不存在 |
| 开奖期号不是 `drawn` | HTTP 400，返回 `only drawn issues can be settled` |
| 已开奖期号缺少 `drawNumber` | HTTP 400，返回 `draw issue does not have draw number` |
| 同一开奖期号重复结算 | HTTP 409，返回 `already settled` |
| 待结算订单玩法评估失败 | HTTP 400，透传玩法规则引擎校验错误 |
| 派奖金额溢出 | HTTP 400，返回 `payout amount is too large` |
| 期号没有待结算订单 | HTTP 200，生成 `settledOrderCount=0` 的结算批次 |

### 5. Good / Base / Bad Cases

- Good：`fc3d` 期号开奖 `0,2,3`，同期开奖的 `threeDirect` 订单选号 `023`，订单赔率快照 `oddsBasisPoints=104000`，结算后订单状态为 `won`，`matchedBets=["023"]`，单注 `200` 分派发 `2080` 分。
- Good：同期开奖的未命中订单结算后状态为 `lost`，`matchedBets=[]`，`payoutMinor=0`。
- Good：同一期号存在已取消订单时，取消订单不参与结算且状态保持 `cancelled`。
- Base：结算批次当前保存在内存订单仓储，服务重启后清空；这适合当前后台流程验证。
- Bad：路由函数直接判断中奖或修改订单状态；结算逻辑必须留在服务层/仓储层并复用玩法规则引擎。
- Bad：结算路由绕过资金服务直接修改用户余额；派奖入账必须通过资金仓储生成 `payoutCredit` 流水。

### 6. 必要测试

- 后端需要覆盖中奖订单结算为 `won`，未中奖订单结算为 `lost`。
- 后端需要覆盖已取消订单不参与结算。
- 后端需要覆盖未开奖期号拒绝结算和同一期号重复结算拒绝。
- 后端需要运行 `cargo fmt --check`、`cargo check`、`cargo test`。
- 前端需要运行 `npm run build`。
- 跨层联调需要完成“创建订单 → 创建/开奖期号 → 执行计奖派奖 → 查询订单列表和结算批次”。

### 7. Wrong vs Correct

#### 错误

```rust
async fn settle_draw_issue_orders(...) {
    if order.expanded_bets.contains(&draw_number) {
        order.status = OrderStatus::Won;
    }
}
```

这个写法把中奖判断写进路由，还只适用于直选，无法复用组三、组六、大小单双等规则。

#### 正确

```rust
let evaluation = evaluate_play_rule(PlayRuleEvaluateRequest {
    number_type: order.number_type.clone(),
    rule_code: order.rule_code.clone(),
    selection: order.selection.clone(),
    draw_number: draw_number.clone(),
})?;
```

结算服务复用玩法规则引擎，拿 `matchedBets` 决定订单中奖状态，并使用订单赔率快照计算派奖结果。

---

## 场景：用户资金与资金流水基础接口

### 1. 范围 / 触发条件

- 触发条件：新增或修改用户资金账户、资金流水、手动调账、订单扣款、取消退款、结算派奖入账和管理后台财务页面。
- 范围：后端资金领域模型、内存资金仓储、财务 API、订单创建和取消资金联动、结算派奖资金联动、前端 finance API client、`useFinance` hook 和“财务管理”页面。

### 2. 签名

- `GET /api/admin/financial-accounts`
- `GET /api/admin/ledger-entries`
- `POST /api/admin/financial-adjustments`
- `POST /api/admin/orders` 创建成功后写入投注扣款流水。
- `PATCH /api/admin/orders/{id}/cancel` 取消成功后写入退款流水。
- `POST /api/admin/settlements/draw-issues/{id}` 中奖订单结算后写入派奖流水。
- `GET /api/admin/dashboard` 的 `finance` 和 `financialAccounts` 从资金仓储读取。

### 3. 契约

所有接口继续使用统一 API 信封。金额字段必须使用最小货币单位整数，不能使用浮点数。

资金账户响应：

```json
{
  "userId": "U10001",
  "availableBalanceMinor": 12000,
  "frozenBalanceMinor": 2000
}
```

资金流水响应：

```json
{
  "id": "L000000000001",
  "userId": "U10001",
  "kind": "orderDebit",
  "amountMinor": -200,
  "balanceAfterMinor": 13800,
  "referenceId": "O000000000001",
  "description": "投注扣款：福彩 3D 2026155",
  "createdAt": "unix:1780388582"
}
```

流水类型：

- `manualAdjustment`：后台手动调账。
- `orderDebit`：投注扣款，金额为负数。
- `orderRefund`：取消订单退款，金额为正数。
- `payoutCredit`：中奖派奖入账，金额为正数。

手动调账请求：

```json
{
  "userId": "U10001",
  "amountMinor": 1000,
  "description": "后台手动补款"
}
```

订单创建资金流：

1. 后端按玩法规则计算订单金额。
2. 资金仓储检查 `availableBalanceMinor >= amountMinor`。
3. 订单创建成功后写入 `orderDebit` 流水。
4. 扣款失败时移除刚创建且仍待开奖的未入资订单。

订单取消资金流：

1. 取消前确认订单存在 `orderDebit` 流水。
2. 订单状态成功改为 `cancelled` 后写入 `orderRefund` 流水。
3. 同一订单重复退款必须拒绝或保持幂等，不能重复加钱。

结算派奖资金流：

1. 订单结算服务生成结算批次。
2. 资金仓储只对 `isWinning=true` 且 `payoutMinor > 0` 的订单写入 `payoutCredit`。
3. `payoutCredit` 的 `referenceId` 使用结算批次和订单组合，避免重复入账。

### 4. 校验与错误矩阵

| 条件 | 预期行为 |
|------|----------|
| 查询资金账户 | HTTP 200，返回资金账户列表 |
| 查询资金流水 | HTTP 200，返回最新流水在前的列表 |
| 手动调账 `userId` 为空 | HTTP 400，返回 `user id is required` |
| 手动调账金额为 0 | HTTP 400，返回 `adjustment amount must not be zero` |
| 手动调账说明为空 | HTTP 400，返回 `adjustment description is required` |
| 调账或扣款导致可用余额为负 | HTTP 400，返回余额不足或余额不能为负 |
| 订单创建用户资金账户不存在 | HTTP 404，返回资金账户不存在 |
| 订单创建可用余额不足 | HTTP 400，返回 `insufficient available balance`，不创建订单 |
| 取消订单缺少扣款流水 | HTTP 400，返回 `order debit ledger entry is required before refund` |
| 同一订单重复退款 | HTTP 409 或幂等返回已有退款流水，不能新增第二笔退款 |
| 同一结算订单重复派奖入账 | 返回已有派奖流水或跳过新增，不能重复加钱 |

### 5. Good / Base / Bad Cases

- Good：`U10001` 创建 `200` 分订单后，账户可用余额减少 `200` 分，并出现 `orderDebit` 流水。
- Good：取消同一待开奖订单后，账户可用余额恢复，并出现 `orderRefund` 流水。
- Good：中奖结算产生 `2000` 分派奖后，账户可用余额增加 `2000` 分，并出现 `payoutCredit` 流水。
- Good：dashboard 和财务管理页读取同一份资金仓储，平台余额和账户列表保持一致。
- Base：资金账户和流水当前是内存模式，服务重启后恢复种子账户；这适合当前后台流程验证。
- Bad：前端直接计算扣款或派奖金额后提交给资金接口；金额变更必须由后端订单、结算和资金服务协同产生。
- Bad：路由直接修改账户字段而不生成流水；所有资金变更必须有可查询的 `LedgerEntry`。

### 6. 必要测试

- 后端需要覆盖订单扣款、余额不足拒绝、取消退款、派奖入账和手动调账。
- 后端需要运行 `cargo fmt --check`、`cargo check`、`cargo test`。
- 前端需要运行 `npm run build`。
- 跨层联调需要完成“查询账户 → 创建订单扣款 → 取消订单退款 → 创建中奖订单并结算入账 → 查询流水”。

### 7. Wrong vs Correct

#### 错误

```rust
order.status = OrderStatus::Won;
account.available_balance_minor += order.payout_minor;
```

这个写法绕过资金服务，没有流水、没有幂等保护，也无法审计派奖来源。

#### 正确

```rust
let settlement = state.orders.settle_draw_issue(&draw_issue).await?;
state.finance.credit_settlement(&settlement).await?;
```

结算服务负责订单状态和派奖结果，资金服务负责余额入账和 `payoutCredit` 流水。

---

## 场景：自动封盘开奖结算接口

### 1. 范围 / 触发条件

- 触发条件：新增或修改开奖期号自动封盘、自动开奖、开奖后自动结算、派奖入账和管理后台自动任务入口。
- 范围：后端自动任务领域模型、自动任务服务、开奖仓储、订单结算、资金入账、前端 draw API client、`useDraws` hook 和“开奖期号与开奖源”页面。

### 2. 签名

- `POST /api/admin/draw-automation/run`

### 3. 契约

所有接口继续使用统一 API 信封，字段命名必须使用 `camelCase`。

请求体：

```json
{
  "now": "2026-06-02 22:00:00"
}
```

`now` 使用当前期号字段相同的固定格式 `YYYY-MM-DD HH:mm:ss`。本阶段以传入时间做字符串比较，要求期号 `scheduledAt` 和 `saleClosedAt` 保持同一格式。

响应 `data` 字段：

```json
{
  "now": "2026-06-02 22:00:00",
  "closedIssues": [],
  "drawnIssues": [],
  "settlementRuns": [],
  "ledgerEntries": [],
  "skippedIssues": [
    {
      "drawIssueId": "D000000000001",
      "lotteryId": "manual-test",
      "issue": "20260602001",
      "reason": "manual draw requires administrator draw number"
    }
  ]
}
```

执行顺序：

1. 扫描现有期号，`open` 且 `saleClosedAt <= now` 的期号自动封盘。
2. 再次扫描现有期号，`open/closed` 且 `scheduledAt <= now` 的期号进入开奖判断。
3. `platform` 期号由后端生成逗号分隔开奖号码；已配置外部源的 `api` 期号由后端拉取真实开奖号码，并把成功开奖的期号状态改为 `drawn`。
4. `manual` 期号不自动开奖，写入 `skippedIssues`。
5. 本次自动开奖成功的期号会立即执行结算，并把中奖派奖写入资金流水。

### 4. 校验与错误矩阵

| 条件 | 预期行为 |
|------|----------|
| `now` 为空 | HTTP 400，返回 `automation time is required` |
| 没有到期可处理期号 | HTTP 200，返回空数组 |
| 到期 `open` 期号 | 自动关闭销售并出现在 `closedIssues` |
| 到期 `platform` 期号 | 自动开奖、结算和派奖入账 |
| 到期且外部源可命中的 `api` 期号 | 自动开奖、结算和派奖入账 |
| 到期但外部源未命中或请求失败的 `api` 期号 | 不开奖，出现在 `skippedIssues`，继续处理其他期号 |
| 到期 `manual` 期号 | 不自动开奖，出现在 `skippedIssues` |
| 自动结算重复 | 只处理本次新开奖成功期号，重复结算仍由结算服务拒绝 |
| 中奖用户资金账户不存在 | 资金服务返回错误；后续需要事务化避免部分状态落地 |

### 5. Good / Base / Bad Cases

- Good：`fc3d` 的 open 期号封盘时间和开奖时间都早于 `now`，执行后期号先封盘再开奖，生成结算批次和 `payoutCredit` 流水。
- Good：手动开奖期号到期后只封盘，不自动伪造开奖号码，结果中包含跳过原因。
- Good：API68 没有返回 `fc3d` 当前期号时，自动任务结果中包含跳过原因，其他到期期号继续执行。
- Base：本阶段是后台触发式一次性执行器，适合内存仓储阶段验证状态链路。
- Bad：自动任务直接修改订单状态或用户余额；必须复用订单结算服务和资金服务。
- Bad：为 `manual` 期号静默生成号码；手动开奖必须由管理员录入号码。

### 6. 必要测试

- 后端需要覆盖到期自动封盘、自动开奖、自动结算和派奖入账。
- 后端需要覆盖手动开奖缺少号码时被跳过。
- 后端需要覆盖外部 API 开奖源未命中期号时被跳过，且整轮自动任务不失败。
- 后端需要运行 `cargo fmt --check`、`cargo check`、`cargo test`。
- 前端需要运行 `npm run build`。
- 跨层联调需要创建到期期号、创建订单、运行自动任务，再核对期号状态、结算批次和资金流水。

### 7. Wrong vs Correct

#### 错误

```rust
issue.status = DrawIssueStatus::Drawn;
order.status = OrderStatus::Won;
account.available_balance_minor += payout_minor;
```

这个写法绕过开奖、结算和资金服务，缺少号码校验、赔率快照、结算批次和资金流水。

#### 正确

```rust
let drawn = draws.draw(&issue.id, DrawIssueResultRequest::default()).await?;
let settlement = orders.settle_draw_issue(&drawn).await?;
let entries = finance.credit_settlement(&settlement).await?;
```

自动任务只负责调度现有服务，业务规则仍由各服务层统一执行。

---

## 场景：自动创建下一期号接口

### 1. 范围 / 触发条件

- 触发条件：新增或修改按彩种开奖计划生成下一期号、批量预生成、期号计划预览、期号计划计算、前端生成入口。
- 范围：后端生成期号请求模型、期号生成服务、开奖期号创建、前端 draw API client、`useDraws` hook 和“开奖期号与开奖源”页面。

### 2. 签名

- `POST /api/admin/draw-issues/generate-next`
- `POST /api/admin/draw-issues/preview-generation`
- `POST /api/admin/draw-issues/generate-batch`

### 3. 契约

所有接口继续使用统一 API 信封，字段命名必须使用 `camelCase`。

`generate-next` 请求体：

```json
{
  "lotteryId": "fc3d",
  "now": "2026-06-02 20:00:00",
  "saleCloseLeadSeconds": 30
}
```

`now` 使用 `YYYY-MM-DD HH:mm:ss`。`saleCloseLeadSeconds` 可省略，默认 `30`，表示封盘时间为开奖前 30 秒。

`preview-generation` 和 `generate-batch` 请求体：

```json
{
  "lotteryId": "fc3d",
  "now": "2026-06-02 20:00:00",
  "count": 5,
  "saleCloseLeadSeconds": 30
}
```

`count` 必须在 `1..=50` 之间。`preview-generation` 只返回计划，不写入开奖期号仓储；`generate-batch` 会按计划创建多期。

`generate-next` 和 `generate-batch` 创建后的响应 `data` 字段为标准 `DrawIssue` 或 `DrawIssue[]`：

```json
{
  "id": "D000000000001",
  "lotteryId": "fc3d",
  "lotteryName": "福彩 3D",
  "issue": "2026144",
  "numberType": "threeDigit",
  "drawMode": "api",
  "scheduledAt": "2026-06-03 21:00:15",
  "saleClosedAt": "2026-06-03 20:59:45",
  "status": "open",
  "drawNumber": null,
  "drawnAt": null,
  "createdAt": "unix:1780394800"
}
```

`preview-generation` 响应 `data` 字段为 `DrawIssueGenerationPreview[]`：

```json
[
  {
    "lotteryId": "fc3d",
    "lotteryName": "福彩 3D",
    "issue": "2026144",
    "numberType": "threeDigit",
    "drawMode": "api",
    "scheduledAt": "2026-06-03 21:00:15",
    "saleClosedAt": "2026-06-03 20:59:45"
  }
]
```

生成规则：

1. 后端读取彩种 `DrawSchedule`，不由前端计算开奖时间。
2. 如果同彩种已有期号，默认使用该彩种最新 `scheduledAt` 和传入 `now` 中较晚的时间作为基线。
3. 周期开奖：`baseline + intervalSeconds`。
4. 每日固定开奖：选择严格晚于基线的当天或次日配置时间。
5. 周开奖：选择严格晚于基线的下一个配置星期和时间。
6. 默认期号编码使用开奖时间格式化为 `YYYYMMDDHHMMSS`。
7. 如果彩种绑定了外部 API 开奖源，后端必须先读取外部源最新 `preDrawIssue`，并用该数字期号递增生成未来期号；例如 API68 最新 `2026143` 时，福彩 3D 和复用同源的排列 3 下一期为 `2026144`。
8. 如果外部 API 周期彩种返回 `preDrawTime`，期号生成必须使用 `preDrawTime + 周期间隔 * 期号偏移` 对齐外部开奖节奏，不能用服务器当前秒数直接推导开奖时间。
9. 如果外部 API 周期彩种返回下一期 `drawIssue` 和 `drawTime`，例如 KJAPI 的腾讯分分彩，后端应优先使用这两个字段作为下一期锚点；当 `drawIssue` 已过封盘时间时，应继续递增到后续可销售期号。
10. 生成计划必须跳过 `saleClosedAt <= now` 的候选期号，避免创建已经封盘却显示为 `open` 的期号；期号递增也要同步跳过这些候选期。
11. API 来源已配置但无法解析最新数字期号时，生成和预览接口必须返回错误，不能静默回退为时间戳期号；期号可能超过 32 位整数范围，后端必须按 64 位整数处理。
12. 创建仍复用开奖期号仓储，保持重复期号、彩种匹配、开奖时间和封盘时间校验一致。
13. 批量预览和批量生成必须在同一次计划中跳过已存在的同彩种同 `issue`，并继续寻找后续可用期号。

### 4. 校验与错误矩阵

| 条件 | 预期行为 |
|------|----------|
| 彩种不存在 | HTTP 404，返回彩种不存在 |
| `lotteryId` 为空 | HTTP 400，返回 `lottery id is required` |
| 请求彩种 ID 与读取彩种不一致 | HTTP 400，返回 `request lottery id does not match lottery` |
| `now` 为空 | HTTP 400，返回 `now time is required` |
| `now` 格式错误 | HTTP 400，返回 `now must use YYYY-MM-DD HH:mm:ss format` |
| `saleCloseLeadSeconds=0` | HTTP 400，返回 `sale close lead seconds must be greater than zero` |
| `count` 小于 1 或大于 50 | HTTP 400，返回 `draw issue generation count must be between 1 and 50` |
| 周期开奖秒数为 0 | HTTP 400，返回 `periodic interval must be greater than zero` |
| 每日或周开奖时间格式错误 | HTTP 400，返回 `... must use HH:mm:ss format` |
| 周开奖星期为空或不支持 | HTTP 400，返回 weekday 错误 |
| 已配置 API 来源但最新期号为空或不是数字 | HTTP 500，返回 API 来源最新期号错误，不生成错误期号 |
| 计划尝试次数耗尽仍无法生成足量唯一期号 | HTTP 409，返回唯一期号生成失败 |

### 5. Good / Base / Bad Cases

- Good：`ssc60` 配置 `periodic.intervalSeconds=60`，`now=2026-06-02 20:00:00`，生成 `scheduledAt=2026-06-02 20:01:00`。
- Good：`fc3d` 配置每日 `21:00:15`，API68 最新 `preDrawIssue=2026143`，`now=2026-06-02 22:00:00`，生成 `issue=2026144`、`scheduledAt=2026-06-03 21:00:15`。
- Good：`pl3` 复用 `api68-fc3d` 来源时，生成下一期同样使用 `issue=2026144`。
- Good：`au5` 配置 `periodic.intervalSeconds=300`，API68 最新 `preDrawIssue=51320849`、`preDrawTime=2026-06-03 11:18:40`，`now=2026-06-03 11:20:00`，生成 `issue=51320850`、`scheduledAt=2026-06-03 11:23:40`。
- Good：同样的 `au5` 在 `now=2026-06-03 11:23:30` 且 `51320850` 已过封盘时间时，生成应跳到 `issue=51320851`、`scheduledAt=2026-06-03 11:28:40`，不能生成已封盘的 open 期。
- Good：`txffc` 配置 `periodic.intervalSeconds=60`，KJAPI 返回 `drawIssue=202606031179`、`drawTime=2026-06-03 19:39:00`，`now=2026-06-03 19:38:20`，生成 `issue=202606031179`、`scheduledAt=2026-06-03 19:39:00`。
- Good：同样的 `txffc` 在 `now=2026-06-03 19:38:40` 且 `202606031179` 已过封盘时间时，生成应跳到 `issue=202606031180`、`scheduledAt=2026-06-03 19:40:00`。
- Good：本地已有 `fc3d/2026144` 时，再次生成应得到 `2026145`。
- Good：周二、周四 `21:00:00` 的彩种，在周二 22:00 后生成周四 21:00。
- Good：`preview-generation` 请求 `count=3` 返回未来 3 期计划，但随后请求期号列表不会多出新期号。
- Good：`generate-batch` 请求 `count=3` 创建 3 个 open 期号，并返回标准 `DrawIssue[]`。
- Base：本阶段是后台触发式生成单期或多期，适合内存仓储阶段验证计划计算。
- Bad：前端自己根据彩种 schedule 计算开奖时间；计划计算必须由后端统一负责。

### 6. 必要测试

- 后端需要覆盖周期、每日、周开奖三种计划。
- 后端需要覆盖已有期号时从最新期号继续生成。
- 后端需要覆盖 API68 最新 `preDrawIssue` 驱动福彩 3D/排列 3 真实期号生成。
- 后端需要覆盖 API68 周期彩种使用 `preDrawTime` 对齐开奖时间，并跳过已过封盘时间的候选期。
- 后端需要覆盖 KJAPI 的 `preDrawIssue/preDrawCode/preDrawTime/drawIssue/drawTime` 解析、12 位期号生成和已封盘候选期跳过。
- 后端需要覆盖计划预览不写入仓储。
- 后端需要覆盖批量生成和 `count` 边界。
- 后端需要运行 `cargo fmt --check`、`cargo check`、`cargo test`。
- 前端需要运行 `npm run build`。
- 跨层联调需要请求生成下一期、计划预览和批量生成接口，并在管理后台开奖期号页面确认预览展示和新期号显示。

### 7. Wrong vs Correct

#### 错误

```tsx
const scheduledAt = addSeconds(form.now, lottery.schedule.periodic.intervalSeconds);
await createDrawIssue({ lotteryId, issue, scheduledAt, saleClosedAt });
```

这个写法把开奖计划计算放在前端，后续手机端、机器人和后台调度容易漂移。

#### 正确

```ts
await generateNextDrawIssue({
  lotteryId,
  now,
});
```

前端只提交彩种和基准时间，后端根据彩种计划生成标准期号并复用创建校验。

```ts
await previewDrawIssueGeneration({
  count: 5,
  lotteryId,
  now,
});
```

批量场景也只提交彩种、基准时间和数量，计划仍由后端统一返回。

---

## 场景：系统级常驻调度运行契约

### 1. 范围 / 触发条件

- 触发条件：新增或修改服务启动后的常驻调度、自动补齐未来期号、自动封盘开奖结算循环、调度状态接口或管理后台调度可视化。
- 范围：`app::router_from_env` 启动流程、调度服务、自动任务服务、期号生成服务、合买机器人执行服务、调度配置/运行历史数据库仓储、管理后台状态接口和结构化日志。

### 2. 签名

调度后台任务随服务启动自动创建；调度配置保存在数据库 `draw_scheduler_config` 表中，空库或内存模式使用后端默认值初始化。

管理后台状态接口：

- `GET /api/admin/draw-scheduler/status`

后端内部入口：

- `DrawSchedulerRepository::new(config)`
- `DrawSchedulerRepository::status()`
- `DrawSchedulerRepository::update_config(config)`
- `DrawSchedulerRepository::record_success(trigger, started_at, finished_at, run)`
- `DrawSchedulerRepository::record_failure(trigger, started_at, finished_at, now, error)`
- `spawn_draw_scheduler(access, draws, lotteries, orders, finance, group_buys, robots, realtime, config, scheduler)`
- `run_draw_scheduler_once(draws, lotteries, orders, finance, group_buys, robots, access, config, now)`

### 3. 契约

空库默认配置：

```json
{
  "enabled": false,
  "intervalSeconds": 60,
  "futureIssueCount": 1,
  "saleCloseLeadSeconds": 30
}
```

`GET /api/admin/draw-scheduler/status` 继续返回统一 API 信封，`data` 字段形状如下：

```json
{
  "enabled": true,
  "config": {
    "enabled": true,
    "intervalSeconds": 60,
    "futureIssueCount": 1,
    "saleCloseLeadSeconds": 30
  },
  "runCount": 1,
  "lastRun": {
    "id": "SCH000000000001",
    "trigger": "automatic",
    "status": "success",
    "startedAt": "2026-06-02 21:00:00",
    "finishedAt": "2026-06-02 21:00:00",
    "now": "2026-06-02 21:00:00",
    "error": null,
    "closedIssueCount": 0,
    "drawnIssueCount": 0,
    "settlementRunCount": 0,
    "ledgerEntryCount": 0,
    "generatedIssueCount": 3,
    "skippedIssueCount": 0,
    "skippedLotteryCount": 1
  },
  "recentRuns": []
}
```

字段契约：

1. 所有字段使用 `camelCase`，并与 `admin/src/types/scheduler.ts` 保持一致。
2. `trigger` 当前只支持 `automatic`，用于区分常驻后台循环；手动点击 `POST /api/admin/draw-automation/run` 不写入常驻调度历史。
3. `status` 只允许 `success` 或 `failed`。
4. 成功记录的 `error` 必须为 `null`，失败记录的 `error` 必须包含错误摘要。
5. `recentRuns` 按最新在前排序，内存仓储最多保留最近 20 条。
6. `runCount` 表示当前内存仓储保留的运行记录数量，不是持久化后的全量审计总数。
7. 调度未启用时，状态接口仍返回配置、`enabled=false`、`lastRun=null` 和空 `recentRuns`。

行为契约：

1. 服务启动后必须创建 Tokio 后台任务，即使数据库调度配置为 `enabled=false`。
2. `DATABASE_URL` 已配置时，调度配置必须从 `draw_scheduler_config` 恢复；该表为空时才写入空库默认配置。
3. 后台保存 `enabled=true` 后，已启动的后台任务必须在不重启服务的情况下读取新配置并开始执行。
4. 每轮调度使用服务器当前本地时间，格式为 `YYYY-MM-DD HH:mm:ss`。
5. 每轮先调用既有 `run_draw_automation`，处理到期封盘、开奖、结算和派奖入账。
6. 自动任务执行后，再扫描 `saleEnabled=true` 的彩种，确保每个彩种至少有 `config.futureIssueCount` 个未来可投注 `open` 期号。
7. 未来期号判断只统计同彩种、状态为 `open`，并且 `scheduledAt > now` 的期号；`closed` 期号已经封盘，不能当作下一期缓冲，否则封盘后不会立即开盘下一期。
8. 补期继续调用 `generate_draw_issue_batch`，不在调度服务里重新实现开奖计划算法。
9. 补期完成后必须调用 `run_group_buy_robots` 执行已启用合买机器人；机器人执行不能放在补期前，否则刚补出的 open 期号无法被机器人使用。
10. 机器人执行产生的 `ledgerEntries` 要计入本轮调度 `ledgerEntryCount`，并通过实时事件推送用户余额变化；机器人产生的订单要推送用户订单变化。
11. `saleEnabled=false` 彩种不会自动补期，也不会被合买机器人发起计划，会记录为跳过彩种或机器人跳过项。
12. 调度周期成功或失败都要写入调度运行历史，页面通过状态接口读取历史，而不是解析日志。
13. 调度周期成功或失败都使用 `tracing` 结构化日志记录，不暴露原始请求体或敏感信息；成功日志中的统计字段必须使用中文键名，包括机器人新增合买、机器人满单、机器人生成订单和机器人跳过项。

### 4. 校验与错误矩阵

| 条件 | 预期行为 |
|------|----------|
| `draw_scheduler_config` 为空 | 写入空库默认配置，后台任务常驻等待后台启用 |
| `draw_scheduler_config.enabled=true` | 后台任务启动后按数据库配置执行 |
| `draw_scheduler_config.enabled=false` | 后台任务启动但跳过执行，等待后台启用 |
| `draw_scheduler_config.interval_seconds=0` | 启动时配置校验失败，返回执行周期错误 |
| `draw_scheduler_config.future_issue_count` 小于 1 或大于 50 | 启动时配置校验失败，返回未来期号数量范围错误 |
| `draw_scheduler_config.sale_close_lead_seconds=0` | 启动时配置校验失败，返回封盘提前秒数错误 |
| 单轮调度 `now` 为空 | 返回 `draw scheduler time is required` |
| 自动开奖或补期过程中发生业务错误 | 当前轮记录错误日志，后台任务继续下一轮 |
| 调度未启用时查询状态 | HTTP 200，`enabled=false`，历史为空 |
| 最近运行超过 20 条 | 只保留最新 20 条，旧记录从内存仓储移除 |
| 状态仓储锁异常 | HTTP 500，返回统一错误信封 |

### 5. Good / Base / Bad Cases

- Good：服务启动后即使初始配置禁用，管理后台保存 `enabled=true` 后也会在不重启服务的情况下开始自动调度。
- Good：后台保存 `enabled=true` 和 `intervalSeconds=1` 后，本地启动服务会从数据库恢复配置，并自动为销售开启彩种补齐未来期号。
- Good：调度跑过一轮后，`GET /api/admin/draw-scheduler/status` 返回最新 `SCH...` 记录，管理后台“常驻调度”显示成功状态和运行摘要。
- Good：已有到期开奖期号时，单轮调度先执行封盘/开奖/结算，再补齐下一期期号。
- Good：已有期号刚到封盘时间但未到开奖时间时，单轮调度先把当前期转为 `closed`，再生成下一期 `open`，保证销售链路继续有可投注期号。
- Good：销售中且开启合买的彩种在补出 open 期号后，同轮调度可以执行合买机器人并创建本期机器人合买。
- Base：默认关闭适合本地开发和测试，不会让后台循环干扰手动 API 冒烟。
- Bad：把 `closed` 期号算作未来缓冲；这会让当前期封盘后没有新的 `open` 期号可投注。
- Bad：在调度服务里复制一套封盘、开奖、结算或开奖计划计算逻辑；这些必须继续复用 `run_draw_automation` 和 `generate_draw_issue_batch`。
- Bad：调度器直接手写合买机器人发单逻辑；机器人执行必须在 `group_buy_robot` 服务中复用合买和订单服务。
- Bad：管理后台为了显示调度状态去解析服务日志；页面必须调用 `GET /api/admin/draw-scheduler/status`。

### 6. 必要测试

- 后端需要覆盖调度默认关闭时仍创建后台任务，但不执行开奖调度。
- 后端需要覆盖后台保存 `enabled=true` 后，已启动后台任务无需重启即可执行。
- 后端需要覆盖默认配置写入数据库、数据库配置恢复和无效配置拒绝。
- 后端需要覆盖销售开启彩种自动补齐未来期号，销售关闭彩种跳过。
- 后端需要覆盖未来期号缓冲已满足时不重复生成。
- 后端需要覆盖到期期号先自动开奖，再补齐未来期号。
- 后端需要覆盖当前期到封盘时间后会生成下一期 `open` 期号，不能因为当前期 `closed` 仍未开奖就跳过补期。
- 后端需要覆盖调度历史成功记录、失败记录和最近 20 条保留上限。
- 后端需要覆盖调度单轮会执行合买机器人，并把机器人资金流水计入调度成功记录。
- 后端需要运行 `cargo fmt --check`、`cargo check`、`cargo test`。
- 本地冒烟需要用短周期启用调度，确认 `/api/admin/draw-issues` 能看到自动补齐的 open 期号。
- 本地冒烟需要请求 `/api/admin/draw-scheduler/status`，确认启用状态、配置和最近运行记录。
- 前端需要运行 `npm run build`，并用浏览器确认“开奖期号与开奖源”页面显示“常驻调度”和“最近运行”。

### 7. Wrong vs Correct

#### 错误

```rust
tokio::spawn(async move {
    // 后台任务直接复制封盘、开奖和派奖逻辑
});
```

这个写法会让手动自动任务接口和常驻调度逐渐分叉。

```ts
const status = parseSchedulerLogs(rawLogText);
```

这个写法会让管理后台依赖日志格式，后续日志脱敏、截断或采集方式变化时页面会失真。

```rust
// 调度器里直接拼合买计划、参与记录和订单
state.group_buys.create(request, lotteries, users).await?;
```

这个写法会把机器人发单逻辑散落到调度器，后续手动执行和定时执行容易不一致。

#### 正确

```rust
run_draw_automation(draws, orders, finance, DrawAutomationRunRequest { now }).await?;
generate_draw_issue_batch(draws, lottery, payload).await?;
```

常驻调度只负责编排时机和缓冲数量，业务动作继续复用已有服务。

```ts
await fetchDrawSchedulerStatus();
```

管理后台通过明确的状态接口读取调度配置、最近运行和失败摘要。

```rust
let robot_run = run_group_buy_robots(
    robots, draws, lotteries, orders, finance, group_buys, access, now,
).await?;
```

调度器只负责编排执行顺序，合买机器人自身继续复用独立服务。

---

## 场景：用户权限与系统设置管理接口

### 1. 范围 / 触发条件

- 触发条件：新增或修改用户管理、管理员管理、角色权限、系统设置、注册配置接口，或让 dashboard 读取用户权限仓储。
- 范围：后端用户权限领域模型、内存用户权限仓储、管理后台 API、前端 API client、`useAccessManagement` hook 和“用户权限管理”页面。

### 2. 签名

用户接口：

- `GET /api/admin/users`
- `GET /api/admin/users/{id}`
- `POST /api/admin/users`
- `PUT /api/admin/users/{id}`
- `PATCH /api/admin/users/{id}/status`

管理员接口：

- `GET /api/admin/admins`
- `GET /api/admin/admins/{id}`
- `POST /api/admin/admins`
- `PUT /api/admin/admins/{id}`
- `PATCH /api/admin/admins/{id}/status`

角色与设置接口：

- `GET /api/admin/roles`
- `GET /api/admin/roles/{id}`
- `POST /api/admin/roles`
- `PUT /api/admin/roles/{id}`
- `DELETE /api/admin/roles/{id}`
- `GET /api/admin/system-settings`
- `PATCH /api/admin/system-settings/{key}`
- `GET /api/admin/registration`
- `PUT /api/admin/registration`

### 3. 契约

所有接口继续使用统一 API 信封。用户字段：

```json
{
  "id": "U10001",
  "username": "demo_user",
  "email": "demo@example.com",
  "kind": "regular",
  "status": "active",
  "balanceMinor": 12000,
  "agentId": "U90001",
  "inviteCode": "USER10001"
}
```

管理员字段：

```json
{
  "id": "A10001",
  "username": "admin",
  "roleId": "role-super",
  "roleName": "超级管理员",
  "status": "active"
}
```

角色字段：

```json
{
  "id": "role-ops",
  "name": "运营管理员",
  "scopes": ["users", "orders", "lotteries"]
}
```

系统设置字段：

```json
{
  "key": "email_registration_enabled",
  "value": "true",
  "description": "是否开启邮箱注册"
}
```

注册配置字段：

```json
{
  "usernameEnabled": true,
  "emailEnabled": false,
  "agentInviteRequired": false
}
```

行为契约：

1. `DashboardSummary.users/admins/roles/settings/registration` 必须从同一个用户权限仓储读取，不允许继续使用 dashboard 内部静态函数。
2. 管理员保存时前端提交 `roleId`；后端根据 `roleId` 查找角色并回填 `roleName`，前端不能靠中文角色名反查。
3. 角色权限范围使用后端枚举的 `camelCase` 值：`users`、`orders`、`finance`、`customerService`、`admins`、`roles`、`systemSettings`、`lotteries`、`robots`、`rebates`。
4. 用户余额字段仍是 `balanceMinor` 最小货币单位。本阶段用户摘要余额不强制和财务账户仓储同步。
5. 每个用户摘要都有单个 `inviteCode`。代理用户的邀请码可用于创建邀请关系，普通用户的邀请码只展示，使用时返回“邀请码无效”。
6. 用户管理接口只需要 `users` 权限，不允许让用户管理页额外依赖需要 `rebates` 权限的邀请管理接口。
7. 本阶段不保存管理员密码，不提供真实登录、JWT、菜单拦截或权限鉴权。

### 4. 校验与错误矩阵

| 条件 | 预期行为 |
|------|----------|
| 用户 ID 为空 | HTTP 400，返回 `user id is required` |
| 用户名为空 | HTTP 400，返回 `username is required` |
| 用户余额小于 0 | HTTP 400，返回 `user balance must not be negative` |
| 创建重复用户 ID | HTTP 409，返回重复用户错误 |
| 更新路径 ID 与用户 ID 不一致 | HTTP 400，返回 `path id must match user id` |
| 管理员 ID 或用户名为空 | HTTP 400，返回对应必填错误 |
| 管理员 `roleId` 不存在 | HTTP 404，返回角色不存在 |
| 创建重复管理员 ID | HTTP 409，返回重复管理员错误 |
| 角色 ID 或名称为空 | HTTP 400，返回对应必填错误 |
| 角色权限范围为空 | HTTP 400，返回 `at least one permission scope is required` |
| 删除已分配给管理员的角色 | HTTP 409，返回角色已被管理员使用 |
| 设置 key 不存在 | HTTP 404，返回设置不存在 |
| 设置值为空 | HTTP 400，返回 `setting value is required` |
| 用户名注册和邮箱注册同时关闭 | HTTP 400，返回至少开启一种注册方式 |

### 5. Good / Base / Bad Cases

- Good：创建 `role-audit` 后再创建管理员并传入 `roleId=role-audit`，响应自动带上正确 `roleName`。
- Good：修改角色名称后，已绑定该角色的管理员摘要同步更新 `roleName`。
- Base：无数据库环境下使用内存仓储，服务重启后恢复种子用户、管理员、角色和设置。
- Bad：前端提交 `roleName` 并假设后端按中文名称匹配角色；这会在改名和多语言时失效。
- Bad：dashboard 继续调用独立静态 `users()`、`admins()` 函数；这会让页面保存后首页摘要不同步。

### 6. 必要测试

- 后端需要覆盖创建/更新用户和用户状态变更。
- 后端需要覆盖邀请码按邀请人聚合，并随用户管理接口返回。
- 后端需要覆盖角色权限范围为空拒绝保存。
- 后端需要覆盖已分配角色拒绝删除。
- 后端需要覆盖角色改名后管理员 `roleName` 同步。
- 后端需要覆盖注册方式不能全部关闭。
- 前端需要运行 `npm run build`，确认 `AdminSummary.roleId`、`PermissionScope` 和接口函数类型一致。
- API 冒烟需要创建用户、创建角色、更新注册配置，并确认 `/api/admin/dashboard` 同步返回。
- 浏览器验证需要进入用户、角色、系统设置入口，确认真实页面显示且控制台无错误。

### 7. Wrong vs Correct

#### 错误

```rust
pub fn dashboard_summary_with_orders(...) -> DashboardSummary {
    DashboardSummary {
        users: users(),
        admins: admins(),
        roles: roles(),
        settings: settings(),
        // ...
    }
}
```

这个写法会让 dashboard 和管理页面使用两份数据。

```ts
await createAdmin({
  roleName: '运营管理员',
});
```

这个写法把角色匹配建立在展示文本上，角色改名后会失效。

#### 正确

```rust
let access = state.access.snapshot().await?;
dashboard_summary_with_orders(lotteries, recent_orders, finance, accounts, access)
```

dashboard 和管理页面共用同一个仓储快照。

```ts
await createAdmin({
  roleId: 'role-ops',
  roleName: '运营管理员',
});
```

前端提交稳定 `roleId`，后端根据角色仓储回填可信 `roleName`。

---

## 场景：机器人配置管理接口

### 1. 范围 / 触发条件

- 触发条件：新增或修改合买机器人、购彩机器人配置管理接口，或新增机器人执行能力、调度器机器人编排、dashboard 机器人摘要。
- 范围：后端机器人领域模型、机器人仓储、彩种绑定校验、合买机器人执行服务、管理后台 API、前端 API client、`useRobots` hook 和“机器人配置”页面。

### 2. 签名

- `GET /api/admin/robots`
- `GET /api/admin/robots/{id}`
- `POST /api/admin/robots`
- `PUT /api/admin/robots/{id}`
- `DELETE /api/admin/robots/{id}`
- `PATCH /api/admin/robots/{id}/status`
- `POST /api/admin/robots/run`

### 3. 契约

所有接口继续使用统一 API 信封。机器人配置字段：

```json
{
  "id": "R-BUY-001",
  "name": "购彩模拟机器人",
  "kind": "purchase",
  "lotteryIds": ["ssc60"],
  "status": "paused",
  "description": "按彩种开盘时间模拟普通用户购彩"
}
```

字段契约：

1. `kind` 只允许 `groupBuy` 或 `purchase`。
2. `status` 只允许 `enabled`、`paused`、`disabled`。
3. `lotteryIds` 必须至少包含一个有效彩种 ID；后端保存时会去重并按稳定顺序返回。
4. `DashboardSummary.robots` 必须从 `RobotRepository` 读取，不允许 dashboard 使用独立静态机器人函数。
5. `POST /api/admin/robots/run` 只执行已启用的 `groupBuy` 机器人，不执行 `purchase` 机器人。
6. 合买机器人执行结果字段：
   - `now`：本轮执行时间，格式为 `YYYY-MM-DD HH:mm:ss`。
   - `createdPlans`：本轮新创建的合买计划。
   - `filledPlans`：本轮补满并关联订单的合买计划。
   - `createdOrders`：本轮由满单合买生成的真实投注订单。
   - `ledgerEntries`：本轮机器人合买认购产生的资金流水。
   - `skippedItems`：本轮跳过项，每项包含 `robotId`、`robotName`、`lotteryId`、`issue` 和 `reason`。
7. 合买机器人计划 ID 必须按“机器人 ID + 彩种 ID + 期号”确定性生成，同一期重复执行不能重复创建计划。
8. 合买机器人必须使用当前合买链路：校验彩种开售、合买开启、open 期号、封盘时间、启用玩法、注数报价、余额，再创建计划，并在临近封盘补单窗口内按阶段补单；满单后成单并写入 `groupBuyDebit`。
9. 合买机器人使用系统账户 `U90001` 出资；余额不足时本轮返回跳过原因或业务错误，不允许透支。
10. 合买机器人必须扫描同彩种当前期非机器人发起的 `draft/open` 未满单计划，并按临近封盘阶段目标追加机器人参与记录；`G-ROBOT-` 开头的机器人计划不得被其他机器人交叉补单。
11. 合买机器人补单节奏固定使用封盘前 90 秒窗口：距离封盘 90-61 秒目标进度 40%，60-31 秒目标进度 60%，30-16 秒目标进度 80%，最后 15 秒才允许补到 100% 并触发满单成单。
12. 合买机器人每次补单都必须生成新的参与记录 ID，不允许复用同一个机器人参与记录一次性覆盖剩余金额。

### 4. 校验与错误矩阵

| 条件 | 预期行为 |
|------|----------|
| 机器人 ID 为空 | HTTP 400，返回 `robot id is required` |
| 机器人名称为空 | HTTP 400，返回 `robot name is required` |
| 机器人说明为空 | HTTP 400，返回 `robot description is required` |
| `lotteryIds` 为空 | HTTP 400，返回 `at least one robot lottery is required` |
| `lotteryIds` 包含不存在彩种 | HTTP 404，返回 `lottery ... not found for robot` |
| 创建重复机器人 ID | HTTP 409，返回重复机器人错误 |
| 更新路径 ID 与机器人 ID 不一致 | HTTP 400，返回 `path id must match robot id` |
| 查询、更新、删除不存在机器人 | HTTP 404，返回机器人不存在 |
| 手动执行时系统机器人资金账户不存在 | HTTP 404，返回合买机器人资金账号不存在 |
| 手动执行时彩种停售或未开启合买 | HTTP 200，进入 `skippedItems`，不创建计划 |
| 手动执行时没有可销售 open 期号 | HTTP 200，进入 `skippedItems`，不创建计划 |
| 已创建合买但未进入封盘前 90 秒补单窗口 | HTTP 200，进入 `skippedItems`，不追加机器人参与记录、不成单 |
| 当前合买进度已达到本阶段目标 | HTTP 200，进入 `skippedItems`，等待下一阶段 |
| 手动执行时机器人余额不足 | HTTP 200 或业务错误明细记录为跳过/错误，不允许创建未扣款计划 |

### 5. Good / Base / Bad Cases

- Good：创建 `purchase` 机器人并绑定 `fc3d`、`ssc60`，响应按标准字段返回，dashboard 同步显示该机器人。
- Good：通过 `PATCH /api/admin/robots/{id}/status` 把机器人从 `paused` 改为 `enabled`，列表立即显示启用状态。
- Good：`ssc60` 开售且开启合买、存在未封盘 open 期号时，`POST /api/admin/robots/run` 可以先创建确定性机器人合买计划；未进入封盘前 90 秒补单窗口时只记录跳过原因，不立即补满。
- Good：进入补单窗口后，合买机器人按 40%、60%、80%、100% 的阶段目标追加机器人参与记录，最后 15 秒才补满计划并生成真实投注订单。
- Good：用户或后台已经发起同彩种当前期未满单合买时，合买机器人同样按临近封盘阶段目标追加机器人参与记录，不一次性补足剩余金额。
- Good：同一期重复执行 `POST /api/admin/robots/run` 时返回“本期机器人合买计划已处理”等跳过原因，不重复创建计划。
- Base：无数据库环境下使用内存机器人仓储，服务重启后恢复种子机器人配置。
- Bad：机器人页面直接读取 dashboard 静态 `robots`，保存后列表与首页摘要会漂移。
- Bad：机器人配置保存时不校验彩种存在，后续真实执行会对不存在彩种下单或发起合买。
- Bad：机器人执行绕过 `group_buy_flow` 或订单报价服务，手写投注内容展开、注数和单注金额。

### 6. 必要测试

- 后端需要覆盖机器人创建、状态变更和绑定彩种去重。
- 后端需要覆盖无彩种拒绝保存。
- 后端需要覆盖绑定不存在彩种拒绝保存。
- 后端需要覆盖合买机器人创建计划后不会在补单窗口外立即补满。
- 后端需要覆盖合买机器人在封盘前 90 秒窗口内按 40%、60%、80%、100% 节奏补单，并在最终阶段生成真实订单。
- 后端需要覆盖合买机器人可以按相同节奏补满非机器人发起的当前期未满单计划。
- 后端需要覆盖同一期重复执行不会重复创建机器人合买计划。
- 后端需要运行 `cargo fmt --check`、`cargo check`、`cargo test`。
- 前端需要运行 `npm run build`，确认机器人类型、状态和 API client 类型一致。
- API 冒烟需要创建机器人、切换状态、验证不存在彩种错误，手动执行合买机器人，并确认 dashboard 同步返回。
- 浏览器验证需要进入“合买机器人”和“购彩机器人”入口，确认真实页面显示且控制台无错误。

### 7. Wrong vs Correct

#### 错误

```rust
DashboardSummary {
    robots: robots(),
    // ...
}
```

这个写法让 dashboard 和机器人管理页面使用两份数据。

```rust
robot_store.create(robot)?;
```

如果保存时不传入彩种列表校验，后续机器人执行会绑定不存在彩种。

```rust
let plan_id = next_group_buy_plan_id();
state.group_buys.create(request, lotteries, users).await?;
```

机器人执行如果使用随机计划 ID，常驻调度重复跑同一期会重复发起合买。

#### 正确

```rust
let robots = state.robots.list().await?;
dashboard_summary_with_orders(..., robots)
```

dashboard 读取同一个机器人仓储。

```rust
let lotteries = state.lotteries.list().await?;
state.robots.create(payload, &lotteries).await?;
```

保存机器人前先校验绑定彩种存在。

```rust
let plan_id = robot_plan_id(robot, lottery, issue);
run_group_buy_robots(...).await?;
```

机器人计划 ID 由机器人、彩种和期号确定，并且执行继续复用合买服务、订单报价和资金服务。

---

## 场景：邀请返利配置接口

### 1. 范围 / 触发条件

- 触发条件：新增或修改代理邀请、普通用户邀请、充值返利模式、默认返利比例配置接口，或让 dashboard 读取返利配置仓储。
- 范围：后端返利领域模型、内存返利仓储、管理后台 API、前端 API client、`useRebatePolicy` hook 和“返利配置”页面。

### 2. 签名

- `GET /api/admin/invite-policy`
- `PUT /api/admin/invite-policy`

### 3. 契约

所有接口继续使用统一 API 信封。查询响应字段：

```json
{
  "agentsCanInvite": true,
  "regularUsersCanInvite": false,
  "rebateMode": "immediate",
  "supportedRebateModes": ["immediate", "rechargeTiered"],
  "defaultRechargeRebateBasisPoints": 350
}
```

更新请求字段：

```json
{
  "agentsCanInvite": true,
  "regularUsersCanInvite": true,
  "rebateMode": "rechargeTiered",
  "defaultRechargeRebateBasisPoints": 520
}
```

字段契约：

1. `rebateMode` 只允许 `immediate` 或 `rechargeTiered`。
2. `defaultRechargeRebateBasisPoints` 使用 basis points，`350` 表示 `3.50%`。
3. `supportedRebateModes` 是后端只读能力列表，前端更新时不能提交该字段。
4. `DashboardSummary.invitePolicy` 必须从 `RebateRepository` 读取，不允许 dashboard 内部返回静态返利摘要。
5. 本阶段只维护配置，不执行真实充值返利发放，不写返利流水或财务入账。

### 4. 校验与错误矩阵

| 条件 | 预期行为 |
|------|----------|
| `agentsCanInvite=false` 且 `regularUsersCanInvite=false` | HTTP 400，返回 `agents or regular users must be able to invite` |
| `defaultRechargeRebateBasisPoints > 10000` | HTTP 400，返回 `default recharge rebate basis points must not exceed 10000` |
| `rebateMode` 不是允许枚举值 | HTTP 400，由 JSON 反序列化返回业务错误信封 |
| 请求缺少必填字段 | HTTP 400，由 JSON 反序列化返回业务错误信封 |

### 5. Good / Base / Bad Cases

- Good：把返利模式更新为 `rechargeTiered`，默认比例更新为 `520`，响应和 dashboard 都同步显示 `5.20%`。
- Good：只开启代理邀请，关闭普通用户邀请，符合当前业务默认策略。
- Base：无数据库环境下使用内存返利仓储，服务重启后恢复默认策略。
- Bad：dashboard 继续直接构造静态 `InvitePolicySummary`，保存后首页摘要会与配置页漂移。
- Bad：前端把 `supportedRebateModes` 当作可编辑字段提交，可能让能力列表被错误覆盖。

### 6. 必要测试

- 后端需要覆盖返利策略更新成功。
- 后端需要覆盖代理邀请和普通用户邀请不能同时关闭。
- 后端需要覆盖默认返利比例不能超过 `10000` basis points。
- 后端需要运行 `cargo fmt --check`、`cargo check`、`cargo test`。
- 前端需要运行 `npm run build`，确认返利模式、更新请求和 API client 类型一致。
- API 冒烟需要查询、更新、验证关闭全部邀请入口错误，并确认 dashboard 同步返回。
- 浏览器验证需要进入“返利配置”入口，确认真实页面显示、保存无接口错误且控制台无错误。

### 7. Wrong vs Correct

#### 错误

```rust
invite_policy: InvitePolicySummary {
    agents_can_invite: true,
    regular_users_can_invite: false,
    // ...
}
```

这个写法会让 dashboard 使用静态策略，配置页保存后不会同步。

```ts
await updateInvitePolicy({
  ...policy,
  supportedRebateModes: ['immediate'],
});
```

这个写法把后端能力列表当成可编辑字段，容易误伤系统契约。

#### 正确

```rust
let invite_policy = state.rebates.get().await?;
dashboard_summary_with_orders(..., invite_policy, robots)
```

dashboard 从返利仓储读取配置。

```ts
await updateInvitePolicy({
  agentsCanInvite: form.agentsCanInvite,
  regularUsersCanInvite: form.regularUsersCanInvite,
  rebateMode: form.rebateMode,
  defaultRechargeRebateBasisPoints,
});
```

前端只提交可编辑字段。

---

## 场景：在线客服会话管理接口

### 1. 范围 / 触发条件

- 触发条件：新增或修改在线客服会话、工单状态、客服分配或后台回复接口。
- 范围：后端客服领域模型、内存客服仓储、用户/管理员绑定校验、管理后台 API、前端 API client、`useSupportConversations` hook 和“在线客服”页面。

### 2. 签名

- `GET /api/admin/support/conversations`
- `GET /api/admin/support/conversations/{id}`
- `POST /api/admin/support/conversations`
- `PUT /api/admin/support/conversations/{id}`
- `POST /api/admin/support/conversations/{id}/messages`

### 3. 契约

所有接口继续使用统一 API 信封。客服会话字段：

```json
{
  "id": "CS-10001",
  "userId": "U10001",
  "username": "demo_user",
  "subject": "订单派奖咨询",
  "status": "open",
  "priority": "normal",
  "assignedAdminId": "A10002",
  "assignedAdminName": "locked_admin",
  "unreadCount": 1,
  "createdAt": "2026-06-02 09:20:00",
  "updatedAt": "2026-06-02 09:22:00",
  "messages": []
}
```

创建请求字段：

```json
{
  "id": "CS-NEW",
  "userId": "U10001",
  "subject": "充值咨询",
  "priority": "urgent",
  "content": "充值多久到账？"
}
```

更新请求字段：

```json
{
  "status": "pending",
  "priority": "normal",
  "assignedAdminId": "A10001"
}
```

后台回复字段：

```json
{
  "adminId": "A10001",
  "content": "已为您核对订单。"
}
```

字段契约：

1. `status` 只允许 `open`、`pending`、`resolved`、`closed`。
2. `priority` 只允许 `normal`、`urgent`。
3. 创建会话时 `userId` 必须引用用户仓储中的已有用户，后端根据用户仓储回填 `username`。
4. 更新会话时 `assignedAdminId` 可以为空；非空时必须引用管理员仓储中的已有管理员，后端回填 `assignedAdminName`。
5. 后台回复时 `adminId` 必须引用已有管理员，消息作者为 `admin`。
6. 本阶段只做后台会话/工单记录，不实现实时聊天、WebSocket、文件上传、站内推送或手机端客服入口。

### 4. 校验与错误矩阵

| 条件 | 预期行为 |
|------|----------|
| 会话 ID 为空 | HTTP 400，返回 `support conversation id is required` |
| 创建重复会话 ID | HTTP 409，返回重复会话错误 |
| 创建时用户 ID 为空 | HTTP 400，返回 `support user id is required` |
| 创建时用户不存在 | HTTP 404，返回用户不存在 |
| 主题为空 | HTTP 400，返回 `support subject is required` |
| 首条消息为空 | HTTP 400，返回 `support message content is required` |
| 更新时分配管理员不存在 | HTTP 404，返回管理员不存在 |
| 回复时管理员 ID 为空 | HTTP 400，返回 `support reply admin id is required` |
| 回复时管理员不存在 | HTTP 404，返回管理员不存在 |
| 回复内容为空 | HTTP 400，返回 `support reply content is required` |
| 查询、更新、回复不存在会话 | HTTP 404，返回会话不存在 |

### 5. Good / Base / Bad Cases

- Good：创建 `CS-API-001` 绑定 `U10001`，响应自动带上 `username=demo_user` 和首条用户消息。
- Good：把会话分配给 `A10001`，响应自动带上 `assignedAdminName=admin`。
- Good：客服回复后消息列表新增 `admin` 消息，`unreadCount` 清零。
- Base：无数据库环境下使用内存客服仓储，服务重启后恢复种子会话。
- Bad：前端直接提交 `username` 或 `assignedAdminName` 并让后端信任，会导致用户/管理员改名后数据漂移。
- Bad：把在线客服基础阶段扩展成实时 IM 或 WebSocket，会把本阶段配置管理和复杂消息系统混在一起。

### 6. 必要测试

- 后端需要覆盖创建、更新分配和后台回复。
- 后端需要覆盖创建时用户不存在拒绝。
- 后端需要覆盖分配管理员不存在拒绝。
- 后端需要覆盖空回复拒绝。
- 后端需要运行 `cargo fmt --check`、`cargo check`、`cargo test`。
- 前端需要运行 `npm run build`，确认客服状态、优先级和 API client 类型一致。
- API 冒烟需要创建会话、更新分配、追加回复、验证未知用户和未知管理员错误。
- 浏览器验证需要进入“在线客服”入口，确认真实页面显示、保存状态无接口错误且控制台无错误。

### 7. Wrong vs Correct

#### 错误

```rust
SupportConversation {
    user_id: payload.user_id,
    username: payload.username,
    // ...
}
```

这个写法信任前端提交的用户展示名，用户改名或伪造请求时会产生错误数据。

```ts
await createSupportConversation({
  userId,
  username,
  subject,
  content,
});
```

这个写法把只读展示字段带入创建请求。

#### 正确

```rust
let access = state.access.snapshot().await?;
state.support.create(payload, &access.users).await?;
```

创建会话前从用户仓储校验并回填用户名。

```ts
await createSupportConversation({
  id,
  userId,
  subject,
  priority,
  content,
});
```

前端只提交可编辑和绑定字段，展示名由后端生成。

---

## 场景：邀请管理接口

### 1. 范围 / 触发条件

- 触发条件：新增或修改代理邀请关系、邀请码、邀请状态或返利资格管理接口。
- 范围：后端邀请领域模型、内存邀请仓储、邀请策略校验、管理后台 API、前端 API client、`useInvitations` hook 和“邀请管理”页面。

### 2. 签名

- `GET /api/admin/invitations`
- `GET /api/admin/invitations/{id}`
- `POST /api/admin/invitations`
- `PUT /api/admin/invitations/{id}`

### 3. 契约

所有接口继续使用统一 API 信封。邀请关系字段：

```json
{
  "id": "INV-10001",
  "inviterUserId": "U90001",
  "inviterUsername": "agent_alpha",
  "inviteeUserId": "U10001",
  "inviteeUsername": "demo_user",
  "inviteCode": "AGENT10001",
  "status": "active",
  "rebateEnabled": true,
  "note": "默认代理邀请关系",
  "createdAt": "2026-06-02 08:30:00",
  "updatedAt": "2026-06-02 08:30:00"
}
```

创建请求字段：

```json
{
  "id": "INV-NEW",
  "inviterUserId": "U90001",
  "inviteeUserId": "U10004",
  "inviteCode": "AGENT-NEW",
  "rebateEnabled": true,
  "note": "运营备注"
}
```

更新请求字段：

```json
{
  "status": "disabled",
  "rebateEnabled": false,
  "note": "暂停返利"
}
```

字段契约：

1. `status` 只允许 `pending`、`active`、`disabled`。
2. 创建邀请关系时后端必须读取 `InvitePolicySummary` 判断邀请人是否有邀请权限。
3. 邀请码所有人必须是 `agent` 用户；普通用户自己的邀请码会返回“邀请码无效”。
4. `inviterUsername` 和 `inviteeUsername` 由后端根据用户仓储回填，前端不能提交或覆盖。
5. `rebateEnabled` 只表示该邀请关系有返利资格，不代表已经发放返利。
6. 同一个代理邀请码可以创建多条不同被邀请人的邀请关系；重复关系仍按邀请人和被邀请人组合拒绝。
7. 本阶段只做邀请关系配置，不执行真实充值返利发放，不写返利流水或财务入账。

### 4. 校验与错误矩阵

| 条件 | 预期行为 |
|------|----------|
| 邀请关系 ID 为空 | HTTP 400，返回 `invite record id is required` |
| 创建重复邀请关系 ID | HTTP 409，返回重复关系 ID 错误 |
| 邀请人 ID 为空 | HTTP 400，返回 `inviter user id is required` |
| 被邀请人 ID 为空 | HTTP 400，返回 `invitee user id is required` |
| 邀请人不存在 | HTTP 404，返回邀请人用户不存在 |
| 被邀请人不存在 | HTTP 404，返回被邀请人用户不存在 |
| 邀请人和被邀请人相同 | HTTP 400，返回 `inviter and invitee must be different users` |
| 普通用户邀请码被使用 | HTTP 400，返回 `邀请码无效` |
| 代理邀请但策略未开启 | HTTP 403，返回 `agent invite entry is disabled` |
| 同一邀请人和被邀请人关系重复 | HTTP 409，返回重复邀请关系错误 |
| 邀请码为空 | HTTP 400，返回 `invite code is required` |
| 邀请码与邀请人不匹配 | HTTP 400，返回 `邀请码与邀请人不匹配` |
| 查询或更新不存在邀请关系 | HTTP 404，返回邀请关系不存在 |

### 5. Good / Base / Bad Cases

- Good：`U90001` 代理邀请 `U20092`，响应自动回填 `agent_alpha` 和 `api_invitee`。
- Good：把邀请关系状态改为 `disabled` 并关闭 `rebateEnabled`，后续真实返利发放应跳过该关系。
- Base：无数据库环境下使用内存邀请仓储，服务重启后恢复种子邀请关系。
- Bad：前端提交 `inviterUsername` 或 `inviteeUsername` 并让后端信任，会导致用户改名或伪造请求时数据漂移。
- Bad：邀请管理页面允许普通用户邀请码创建邀请关系，会违反“普通用户码仅展示、不可邀请”的业务规则。

### 6. 必要测试

- 后端需要覆盖代理创建邀请关系和更新状态。
- 后端需要覆盖普通用户邀请码返回“邀请码无效”。
- 后端需要覆盖被邀请人不存在拒绝。
- 后端需要覆盖同一个代理邀请码可用于多个不同被邀请人。
- 后端需要运行 `cargo fmt --check`、`cargo check`、`cargo test`。
- 前端需要运行 `npm run build`，确认邀请状态、请求字段和 API client 类型一致。
- API 冒烟需要创建用户后创建邀请关系、更新状态、验证普通用户邀请被拒绝，并确认 dashboard 中 `invite` 为 `scaffolded`。
- 浏览器验证需要进入“邀请管理”入口，确认真实页面显示、保存状态无接口错误且控制台无错误。

### 7. Wrong vs Correct

#### 错误

```rust
InviteRecord {
    inviter_username: payload.inviter_username,
    invitee_username: payload.invitee_username,
    // ...
}
```

这个写法信任前端提交的展示名，无法保证邀请关系与用户仓储一致。

```ts
await createInvitation({
  inviterUserId,
  inviterUsername,
  inviteeUserId,
  inviteeUsername,
});
```

这个写法把只读展示字段带入创建请求。

#### 正确

```rust
let access = state.access.snapshot().await?;
let policy = state.rebates.get().await?;
state.invites.create(payload, &access.users, &policy).await?;
```

创建邀请关系前从用户仓储和返利配置仓储校验。

```ts
await createInvitation({
  id,
  inviterUserId,
  inviteeUserId,
  inviteCode,
  rebateEnabled,
  note,
});
```

前端只提交可编辑字段和绑定 ID。

---

## 场景：合买管理接口

### 1. 范围 / 触发条件

- 触发条件：新增或修改后端合买领域模型、合买计划仓储、管理后台 API、用户端合买 API、前端 API client、`useGroupBuyPlans` hook 和手机端合买大厅。
- 范围：合买计划列表、详情、创建、状态维护、参与记录添加、dashboard `groupBuyPlans` 真实数据来源、手机端发起合买、参与合买、我的合买、发起选项、满单真实成单、封盘流单退款和开奖中奖分账。
- 当前阶段创建和参与合买都会扣减用户可用余额并写入 `groupBuyDebit` 资金流水；计划满单后会创建一张真实投注订单并回写 `orderId`，封盘未满员会取消计划并写入 `groupBuyRefund`，开奖结算中奖时按参与金额比例写入参与人的 `payoutCredit`。

### 2. 签名

- `GET /api/admin/group-buy/plans`
- `GET /api/admin/group-buy/plans/{id}`
- `POST /api/admin/group-buy/plans`
- `PUT /api/admin/group-buy/plans/{id}`
- `POST /api/admin/group-buy/plans/{id}/participants`
- `GET /api/user/group-buy/plans`
- `POST /api/user/group-buy/plans`
- `GET /api/user/group-buy/plans/{id}`
- `POST /api/user/group-buy/plans/{id}/participants`
- `GET /api/user/group-buy/my`
- `GET /api/user/group-buy/create-options`
- dashboard：`GET /api/admin/dashboard` 的 `groupBuyPlans` 来自 `GroupBuyRepository::list()`。

### 3. 契约

计划详情响应字段：

```json
{
  "id": "G202606020001",
  "lotteryId": "fc3d",
  "lotteryName": "福彩 3D",
  "orderId": null,
  "issue": "20260605001",
  "ruleCode": "threeDirect",
  "title": "福彩 3D 第20260605001期合买",
  "numbers": "1,2,3",
  "initiatorUserId": "U90001",
  "initiatorUsername": "agent_alpha",
  "totalAmountMinor": 100000,
  "filledAmountMinor": 72000,
  "minShareAmountMinor": 100,
  "participantMinAmountMinor": 1000,
  "shareCount": 1000,
  "status": "open",
  "participants": [
    {
      "id": "G202606020001-P001",
      "userId": "U90001",
      "username": "agent_alpha",
      "amountMinor": 10000,
      "shareCount": 100,
      "note": "发起人认购",
      "createdAt": "2026-06-02 09:00:00"
    }
  ],
  "note": "默认合买计划示例",
  "createdAt": "2026-06-02 09:00:00",
  "updatedAt": "2026-06-02 09:30:00"
}
```

创建计划请求字段：

```json
{
  "id": "G-NEW-001",
  "lotteryId": "fc3d",
  "issue": "20260605001",
  "ruleCode": "threeDirect",
  "title": "福彩 3D 第20260605001期合买",
  "numbers": "1,2,3",
  "initiatorUserId": "U90001",
  "totalAmountMinor": 100000,
  "initiatorAmountMinor": 10000,
  "note": "后台创建合买计划"
}
```

用户端发起合买请求字段：

```json
{
  "lotteryId": "fc3d",
  "issue": "20260605001",
  "ruleCode": "threeDirect",
  "title": "用户发起合买",
  "numbers": "1,2,3",
  "totalAmountMinor": 100000,
  "selfAmountMinor": 10000
}
```

用户端参与合买请求字段：

```json
{
  "amountMinor": 1000
}
```

更新计划请求字段：

```json
{
  "status": "cancelled",
  "note": "运营取消"
}
```

新增参与记录请求字段：

```json
{
  "id": "G-NEW-001-P002",
  "userId": "U10001",
  "amountMinor": 1000,
  "note": "后台添加参与记录"
}
```

字段契约：

1. `status` 只允许 `draft`、`open`、`filled`、`cancelled`、`settled`。
2. 金额字段统一使用最小货币单位，前端不传浮点金额。
3. `lotteryName`、`initiatorUsername` 和参与记录 `username` 均由后端根据仓储回填，前端不能提交或覆盖。
4. `shareCount = amountMinor / minShareAmountMinor`，计划总金额和参与金额都必须能按最小份额金额整除。
5. 创建计划会自动写入一条发起人参与记录，参与记录 ID 使用 `{planId}-P001`。
6. 参与金额达到计划总金额时，计划状态自动变为 `filled`。
7. 用户端发起和参与合买时，用户 ID 始终来自登录态；前端不能提交或覆盖发起人、参与人 ID。
8. `issue` 必须是当前彩种处于 `open` 状态的期号，`ruleCode` 必须对应当前彩种已启用玩法。
9. `numbers` 是当前合买投注内容，后端必须通过 `group_buy_flow` 转换为当前订单引擎的 `PlaySelection`；支持直选位置 `1|2|3`、单注逗号 `1,2,3`、组合 `1,2,3`、胆拖 `1|2,3,4` 和大小单双 `tens:big|ones:odd` / 中文“大、小、单、双”。
10. 发起或参与合买扣款成功后必须写入 `ledger_entries.kind=groupBuyDebit`，并通过实时事件推送余额变化。
11. 当 `filledAmountMinor == totalAmountMinor` 时，后端必须立即创建一张真实投注订单，并通过 `group_buy_plans.order_id` 关联；该真实订单不能再次执行普通订单扣款，合买参与扣款才是资金来源。
12. 封盘时仍未满员的 `draft/open` 计划必须自动取消，并按参与记录写入幂等的 `groupBuyRefund` 资金流水。
13. 开奖结算时，如果中奖订单属于合买计划，派奖必须按参与金额占计划总金额的比例拆给参与人；除最后一名参与人外向下取整，最后一名承接余数，随后把计划标记为 `settled`。

### 4. 校验与错误矩阵

| 条件 | 预期行为 |
|------|----------|
| 计划 ID 为空 | HTTP 400，返回 `group buy plan id is required` |
| 创建重复计划 ID | HTTP 409，返回重复计划 ID 错误 |
| 彩种 ID 为空 | HTTP 400，返回 `lottery id is required` |
| 彩种不存在 | HTTP 404，返回彩种不存在 |
| 彩种未开启合买 | HTTP 400，返回 `lottery ... group buy is disabled` |
| 彩种未开售 | HTTP 400，返回彩种未开售 |
| 期号为空或不是当前开放期 | HTTP 400，返回期号错误或期号已停止销售 |
| 玩法为空或未开启 | HTTP 400，返回玩法错误或玩法未开启 |
| 投注内容为空 | HTTP 400，返回请输入合买投注内容 |
| 发起人不存在 | HTTP 404，返回用户不存在 |
| 总金额或发起人认购金额小于等于 0 | HTTP 400，返回金额必须大于 0 |
| 总金额或参与金额不能按 `minShareAmountMinor` 整除 | HTTP 400，返回必须按最小份额金额整除 |
| 总金额不能按投注注数平均分配单注金额 | HTTP 400，返回合买总金额必须能按注数平均分配 |
| 发起人认购低于彩种 `initiatorMinPercent` | HTTP 400，返回发起人最低认购金额 |
| 发起人认购超过计划总金额 | HTTP 400，返回发起人认购不能超过总金额 |
| 发起人或参与人余额不足 | HTTP 400，返回余额不足 |
| 查询或更新不存在计划 | HTTP 404，返回计划不存在 |
| 计划未满额却更新为 `filled` 或 `settled` | HTTP 400，返回必须满额后才能进入已满单或已结算 |
| 参与记录 ID 为空 | HTTP 400，返回参与记录 ID 必填 |
| 同一计划参与记录 ID 重复 | HTTP 409，返回重复参与记录 ID |
| 参与用户不存在 | HTTP 404，返回用户不存在 |
| 参与金额低于 `participantMinAmountMinor` | HTTP 400，返回参与金额最低要求 |
| 参与金额超过剩余可认购金额 | HTTP 400，返回超额参与错误 |
| 计划不是 `draft` 或 `open` | HTTP 400，返回计划不可参与 |
| 满员计划关联真实订单失败 | 回滚新建计划或新增参与记录，已创建的未入账订单必须移除 |
| 后台取消已开奖或已取消的合买真实订单 | HTTP 400，返回已开奖或已取消的合买订单不能取消 |

### 5. Good / Base / Bad Cases

- Good：`fc3d` 开启合买，`U90001` 发起 `100000` 分计划并认购 `10000` 分，后端自动生成发起人参与记录和 `1000` 份总份额。
- Good：手机端当前用户发起或参与合买后，后端扣减可用余额，写入 `groupBuyDebit` 资金流水，并返回最新合买计划和余额。
- Good：追加 `U10001` 参与记录后，如果 `filledAmountMinor == totalAmountMinor`，后端自动把计划状态改为 `filled`，创建真实投注订单并在响应里返回 `orderId`。
- Good：自动化封盘时取消未满员计划，按每条参与记录退回认购金额，重复执行不会重复退款。
- Good：开奖结算时识别合买订单，中奖金额按参与金额比例拆给参与用户，普通订单仍按订单用户派奖。
- Base：无数据库环境下使用内存合买仓储，服务重启后恢复种子合买计划；数据库模式下使用 `group_buy_plans`、`group_buy_participants` 和 `ledger_entries` 持久化。
- Bad：前端自行计算 `shareCount` 并提交给后端，后续会与彩种合买配置漂移。
- Bad：直接把 dashboard 的 `groupBuyPlans` 写成静态函数，页面创建计划后首页摘要不会同步。
- Bad：用户端继续请求旧 `/group-buys/*` 路径，当前后端没有该旧系统接口。
- Bad：满单后创建普通订单并再次调用 `debit_order`；这会导致用户既被合买认购扣款，又被订单扣款。

### 6. 必要测试

- 后端需要覆盖创建合买计划并自动写入发起人参与记录。
- 后端需要覆盖未开启合买彩种被拒绝。
- 后端需要覆盖发起人认购低于最低比例被拒绝。
- 后端需要覆盖添加参与记录后自动满单。
- 后端需要覆盖超额参与被拒绝。
- 后端需要覆盖合买扣款写入 `groupBuyDebit` 且相同参与记录不会重复扣款。
- 后端需要覆盖满单创建真实订单并回写 `orderId`。
- 后端需要覆盖封盘流单退款写入 `groupBuyRefund` 且幂等。
- 后端需要覆盖合买中奖按参与人份额拆分派奖。
- 后端需要运行 `cargo fmt --check`、`cargo check`、`cargo test`。
- 前端需要运行 `npm run build`，确认计划状态、请求字段和 API client 类型一致。
- API 冒烟需要创建计划、更新状态、添加参与记录到满单、确认 `orderId` 已生成，并验证 dashboard `groupBuyPlans` 来自真实仓储。
- 浏览器验证需要进入“合买配置”入口，确认真实页面显示、保存状态和添加参与记录无接口错误且控制台无错误。

### 7. Wrong vs Correct

#### 错误

```rust
fn group_buy_plans() -> Vec<GroupBuyPlanSummary> {
    vec![GroupBuyPlanSummary {
        id: "G202606020001".to_string(),
        status: "open".to_string(),
        // ...
    }]
}
```

这个写法让 dashboard 永远显示静态示例，创建和更新计划后不会同步。

```ts
await addGroupBuyParticipant(id, {
  userId,
  amountMinor,
  shareCount,
});
```

这个写法把份额计算放到前端，后续彩种配置调整时容易不一致。

#### 正确

```rust
let group_buy_plans = state.group_buys.list().await?;
dashboard_summary_with_orders(
    lotteries,
    recent_orders,
    group_buy_plans,
    finance,
    financial_accounts,
    access,
    invite_policy,
    robots,
)
```

dashboard 从同一个合买仓储读取 summary。

```ts
await addGroupBuyParticipant(id, {
  id: participantId,
  userId,
  amountMinor,
  note,
});
```

前端只提交参与金额和绑定 ID，份额数量由后端按彩种合买配置计算。

```ts
await http.post('/user/group-buy/plans', {
  lotteryId,
  issue,
  ruleCode,
  numbers,
  totalAmountMinor,
  selfAmountMinor,
});
```

手机端只提交当前用户选择的彩种、期号、玩法、投注内容和金额，发起人身份、扣款和流水由后端完成。

---

## 场景：常驻调度配置编辑接口

### 1. 范围 / 触发条件

- 触发条件：新增调度配置更新 API、前端 API client、`useDrawScheduler` 保存能力和“常驻调度”配置表单。
- 范围：调度配置查看、更新、状态回显，以及服务启动时创建的后台循环每轮读取最新配置。
- 调度配置已经接入业务表持久化；后端默认值只作为空库或无持久化模式下的种子配置，不再通过环境变量配置调度参数。

### 2. 签名

- `GET /api/admin/draw-scheduler/status`
- `PUT /api/admin/draw-scheduler/config`

### 3. 契约

配置请求体与状态响应中的 `config` 字段一致：

```json
{
  "enabled": true,
  "intervalSeconds": 30,
  "futureIssueCount": 3,
  "saleCloseLeadSeconds": 20
}
```

`PUT /api/admin/draw-scheduler/config` 成功后返回完整 `DrawSchedulerStatus`：

```json
{
  "enabled": true,
  "config": {
    "enabled": true,
    "intervalSeconds": 30,
    "futureIssueCount": 3,
    "saleCloseLeadSeconds": 20
  },
  "runCount": 0,
  "lastRun": null,
  "recentRuns": []
}
```

字段契约：

1. `enabled` 表示仓储配置是否启用；状态顶层 `enabled` 必须与 `config.enabled` 保持一致。
2. `intervalSeconds` 控制后台循环下一轮等待间隔；已启动循环会在下一轮读取新配置。
3. `futureIssueCount` 控制每个销售开启彩种需要保留的未来期号数量。
4. `saleCloseLeadSeconds` 控制自动生成期号时开奖前多少秒封盘。
5. 如果服务启动时调度为关闭，保存 `enabled=true` 后无需重启；常驻后台循环会在短周期轮询中读取新配置并开始执行。
6. 如果保存 `enabled=false`，下一轮循环会跳过自动任务且不写运行历史。
7. 管理后台必须在“自动任务与调度”页面提供“启动调度”和“关闭调度”直接操作按钮；按钮仍调用 `PUT /api/admin/draw-scheduler/config`，只切换 `enabled`，其它配置保持当前状态值。

### 4. 校验与错误矩阵

| 条件 | 预期行为 |
|------|----------|
| 有效配置 | HTTP 200，返回最新 `DrawSchedulerStatus` |
| `intervalSeconds=0` | HTTP 400，返回执行周期必须大于 0 |
| `futureIssueCount=0` | HTTP 400，返回未来期号数量必须在 1 到 50 |
| `futureIssueCount>50` | HTTP 400，返回未来期号数量必须在 1 到 50 |
| `saleCloseLeadSeconds=0` | HTTP 400，返回封盘提前秒数必须大于 0 |
| 调度仓储锁异常 | HTTP 500，返回内部错误，不暴露运行时细节 |

### 5. Good / Base / Bad Cases

- Good：后台保存 `enabled=true, intervalSeconds=5, futureIssueCount=3, saleCloseLeadSeconds=20`，状态接口立即回显新配置。
- Good：点击“启动调度”后，页面提交当前配置加 `enabled=true`，状态接口回显已启用；点击“关闭调度”后提交当前配置加 `enabled=false`，状态接口回显未启用。
- Good：服务启动时配置为禁用，后台保存 `enabled=true` 后无需重启即可开始自动补期。
- Good：已启动调度循环每轮读取 `DrawSchedulerRepository::config()`，所以 `futureIssueCount` 和 `saleCloseLeadSeconds` 不需要重启进程即可生效。
- Base：数据库中没有调度配置时，服务写入后端默认配置并等待后台启用。
- Bad：前端只改本地表单状态，不调用 `PUT /draw-scheduler/config`，刷新后状态丢失且后端不生效。
- Bad：后台循环继续使用启动时捕获的 `DrawSchedulerConfig`，配置页面虽然保存成功，但自动补期仍按旧配置运行。

### 6. 必要测试

- 后端需要覆盖有效配置更新成功并回显。
- 后端需要覆盖服务启动时禁用、后台保存启用后常驻循环无需重启即可执行。
- 后端需要覆盖无效 `intervalSeconds` 被拒绝。
- 后端需要覆盖数据库配置恢复和空库默认配置写入。
- 后端需要运行 `cargo fmt --check`、`cargo check`、`cargo test`。
- 前端需要运行 `npm run build`，确认配置字段和 API client 类型一致。
- API 冒烟需要保存有效配置、查询状态回显，并验证无效配置返回业务错误。
- 浏览器验证需要进入“开奖期号与开奖源”的“常驻调度”卡片，保存配置无接口错误且控制台无错误。

### 7. Wrong vs Correct

#### 错误

```rust
spawn_draw_scheduler(..., config.clone(), scheduler);
// loop 内一直使用启动时的 config
```

这个写法会让后台页面保存配置后看起来成功，但实际自动任务仍按旧配置运行。

```ts
setForm(nextConfig);
refreshScheduler();
```

这个写法只改前端状态，没有把配置提交到后端。

#### 正确

```rust
let current_config = scheduler.config()?;
if !current_config.enabled {
    continue;
}
run_draw_scheduler_once(..., &current_config, now).await?;
```

后台循环每轮读取最新配置，关闭时跳过执行。

```ts
const nextStatus = await updateDrawSchedulerConfig(payload);
setStatus(nextStatus);
```

前端保存后以服务端返回状态作为事实来源。

---

## 场景：管理后台登录鉴权接口

### 1. 范围 / 触发条件

- 触发条件：新增登录、当前管理员、登出接口，并让 `/api/admin/**` 管理接口开始要求 Bearer Token。
- 范围：后端 auth DTO、统一 API 信封、前端 API client token 处理、菜单权限过滤、HTTP 401/403 错误契约。

### 2. 签名

- `POST /api/admin/auth/login`
- `GET /api/admin/auth/me`
- `POST /api/admin/auth/logout`
- 认证头：`Authorization: Bearer <token>`

### 3. 契约

登录请求：

```json
{
  "username": "admin",
  "password": "admin123"
}
```

登录响应 `data`：

```json
{
  "token": "bcst_8f4f0f6d4c0b8d6a0b1f0c8a5e4d3c2b1a0f9e8d7c6b5a4938271605f4e3d2c1",
  "admin": {
    "id": "A10001",
    "username": "admin",
    "roleId": "role-super",
    "roleName": "超级管理员",
    "status": "active"
  },
  "role": {
    "id": "role-super",
    "name": "超级管理员",
    "scopes": ["users", "orders", "finance"]
  },
  "scopes": ["users", "orders", "finance"]
}
```

`GET /api/admin/auth/me` 返回当前管理员资料，不返回新 token：

```json
{
  "admin": {},
  "role": {},
  "scopes": []
}
```

`POST /api/admin/auth/logout` 返回：

```json
{
  "loggedOut": true
}
```

前端 API client 必须把 token 放入 `localStorage` 的 `bc.admin.authToken`，并在所有管理接口请求中附加 `Authorization` 头。登录页不应提前请求 dashboard。

登录 token 必须是不可预测的 opaque Bearer token，不能包含管理员 ID、用户名、时间戳或计数器。后端只在登录响应中返回原始 token；内存会话索引和数据库 `admin_sessions.token` 必须保存 `sha256:` 摘要，不能保存原始 token。上线迁移清理旧明文会话后，历史登录态需要重新登录。

### 4. 校验与错误矩阵

| 条件 | 预期行为 |
|------|----------|
| 登录账号或密码为空 | HTTP 400，统一错误信封 |
| 账号或密码错误 | HTTP 401，`invalid admin credentials` |
| 管理员状态不是 `active` | HTTP 403，`admin account is not active` |
| `/api/admin/**` 缺少 Authorization | HTTP 401，`authorization token is required` |
| Authorization 不是 Bearer 格式 | HTTP 401，`authorization bearer token is required` |
| token 无效或已登出 | HTTP 401，`invalid admin session` |
| token 有效但缺少路径所需权限 | HTTP 403，提示缺少权限 |
| 登出成功 | HTTP 200，`loggedOut=true`，后续同 token 请求返回 401 |

### 5. Good / Base / Bad Cases

- Good：未登录打开前端只显示登录页；登录后请求 dashboard 带 token，菜单按 scopes 过滤。
- Good：管理员登录返回 `bcst_` 前缀随机 token，数据库只保存 `sha256:` 摘要。
- Base：无数据库阶段使用内存 token；服务重启后 token 失效，前端清理本地 token 并回到登录页。
- Bad：前端在未登录时先请求 `/api/admin/dashboard`，造成无意义 401 和页面闪烁。
- Bad：后端只隐藏菜单但不拦截 API，低权限管理员仍可直接请求无权限接口。
- Bad：后端把 token 做成 `adm-A10001-时间戳-序号`，或把原始 Bearer token 直接保存进数据库。

### 6. 必要测试

- 后端测试需要覆盖活跃管理员登录、锁定管理员拒绝、登出后 token 失效和路径权限映射。
- 后端测试需要覆盖管理员登录 token 不包含账号 ID，且仓储或数据库只保存摘要。
- API 冒烟需要确认无 token 为 401、有效 token 可访问、低权限 token 访问高权限接口为 403。
- 前端需要运行 `npm run build`。
- 浏览器验证需要覆盖未登录登录页、登录成功进入后台、刷新保持登录态、登出回到登录页。

### 7. Wrong vs Correct

#### 错误

```ts
const { data } = useDashboard();
if (!session) return <LoginPage />;
```

这个写法在未登录时仍会先请求 dashboard，制造 401 噪声。

```rust
Router::new().route("/admins", get(list_admins))
// 只在前端隐藏菜单，后端不校验 scopes
```

这个写法不能阻止绕过前端直接调用接口。

#### 正确

```ts
const { data } = useDashboard(Boolean(session));
if (!session) return <LoginPage />;
```

未登录时不发管理接口请求。

```rust
if !session.scopes.contains(&required_scope) {
    return Err(ApiError::Forbidden(...));
}
```

后端按 token 的角色权限做最终拦截。

---

## 场景：用户端登录鉴权接口

### 1. 范围 / 触发条件

- 触发条件：新增或修改用户注册、用户登录、当前用户、登出、绑定邮箱、修改密码、余额、资金流水、充值、提现方式和提现申请接口。
- 范围：后端用户 auth DTO、统一 API 信封、手机端 API client token 处理、HTTP 401 错误契约、用户会话表持久化。

### 2. 签名

- `POST /api/user/register`
- `POST /api/user/login`
- `GET /api/user/me`
- `POST /api/user/logout`
- 用户登录后接口认证头：`Authorization: Bearer <token>`

### 3. 契约

用户登录请求：

```json
{
  "loginKey": "demo_user",
  "password": "12345678"
}
```

用户登录响应 `data`：

```json
{
  "token": "bcst_8f4f0f6d4c0b8d6a0b1f0c8a5e4d3c2b1a0f9e8d7c6b5a4938271605f4e3d2c1",
  "user": {
    "id": "U10001",
    "username": "demo_user",
    "email": "demo@example.com",
    "kind": "regular",
    "status": "active",
    "balanceMinor": 0,
    "agentId": null,
    "inviteCode": "A1B2C3"
  }
}
```

用户登录 token 必须和管理员登录 token 使用同一安全策略：返回给客户端的是 `bcst_` 前缀 opaque Bearer token，不能包含用户 ID、用户名、邮箱、时间戳或计数器；内存会话索引和数据库 `user_sessions.token` 只能保存 `sha256:` 摘要。上线迁移清理旧明文会话后，用户端历史登录态需要重新登录。

### 4. 校验与错误矩阵

| 条件 | 预期行为 |
|------|----------|
| `loginKey` 或 `password` 为空 | HTTP 400，统一错误信封 |
| 用户名/邮箱或密码错误 | HTTP 401，`用户名/密码错误` |
| 用户状态不是 `active` | HTTP 403，`用户状态不可登录` |
| 用户登录后接口缺少 Authorization | HTTP 401，`authorization token is required` |
| Authorization 不是 Bearer 格式 | HTTP 401，`authorization bearer token is required` |
| token 无效或已登出 | HTTP 401，`invalid user session` |
| 登出成功 | HTTP 200，`loggedOut=true`，后续同 token 请求返回 401 |

### 5. Good / Base / Bad Cases

- Good：用户登录返回 `bcst_` 前缀随机 token，数据库只保存 `sha256:` 摘要，手机端带 Bearer token 可访问余额和提现接口。
- Base：无数据库阶段使用内存摘要索引；服务重启后 token 失效，手机端清理本地 token 并回到登录页。
- Bad：后端把 token 做成 `user-U10001-时间戳-序号`，或把原始 Bearer token 直接保存进 `user_sessions`。

### 6. 必要测试

- 后端测试需要覆盖用户登录 token 不包含用户 ID，且仓储或数据库只保存摘要。
- 后端测试需要覆盖用户登出后同 token 失效。
- API 冒烟需要确认无 token 为 401、有效 token 可访问用户资料、余额、资金流水、充值和提现接口。

---

## 场景：管理员密码哈希与重置接口

### 1. 范围 / 触发条件

- 触发条件：新增或修改管理员创建、管理员更新、管理员密码重置、后台登录校验或管理员摘要返回结构。
- 范围：后端 `AccessRepository` 密码哈希、管理员写入 DTO、登录校验、重置密码接口、前端 API client、`useAccessManagement` 和“账号维护” SideSheet。

### 2. 签名

- `POST /api/admin/admins`
- `PUT /api/admin/admins/{id}`
- `PATCH /api/admin/admins/{id}/password`
- `POST /api/admin/auth/login`
- 认证头：`Authorization: Bearer <token>`；管理员写入和重置密码接口需要 `admins` 权限。

### 3. 契约

管理员保存请求 `AdminSaveRequest`：

```json
{
  "id": "A20001",
  "username": "ops_admin",
  "roleId": "role-ops",
  "roleName": "",
  "status": "active",
  "password": "PassOps123"
}
```

`password` 为可选字段：

- 创建管理员时必须传入初始密码；不传、空白或小于 8 位会返回 HTTP 400。
- 更新管理员资料时不传表示保留原密码；传入时更新密码哈希。
- 后端保存前必须按 `roleId` 回填可信 `roleName`，不能信任前端提交的展示名。

重置密码请求 `AdminPasswordResetRequest`：

```json
{
  "password": "NewPass123"
}
```

所有管理员读取接口和认证响应都不返回 `password`、`passwordHash` 或任何哈希内容：

- `GET /api/admin/admins`
- `GET /api/admin/admins/{id}`
- `GET /api/admin/dashboard` 的 `admins`
- `POST /api/admin/auth/login`
- `GET /api/admin/auth/me`

管理员摘要仍为：

```json
{
  "id": "A20001",
  "username": "ops_admin",
  "roleId": "role-ops",
  "roleName": "运营管理员",
  "status": "active"
}
```

密码哈希使用 Argon2id PHC 字符串格式和随机盐。当前无数据库阶段，密码哈希只保存在内存仓储中；种子管理员 `admin` 继续可用 `admin123` 登录。

### 4. 校验与错误矩阵

| 条件 | 预期行为 |
|------|----------|
| 创建或更新管理员时 `roleId` 不存在 | HTTP 404，返回角色不存在 |
| 创建重复管理员 ID | HTTP 409，返回管理员已存在 |
| 创建管理员未传 `password` | HTTP 400，返回 `admin password is required` |
| 更新路径 ID 与请求体 ID 不一致 | HTTP 400，返回 `path id must match admin id` |
| 重置不存在管理员密码 | HTTP 404，返回管理员不存在 |
| 密码为空白 | HTTP 400，返回 `admin password is required` |
| 密码长度小于 8 | HTTP 400，返回最小长度错误 |
| 登录密码错误 | HTTP 401，`invalid admin credentials` |
| 管理员状态不是 `active` | HTTP 403，`admin account is not active` |

### 5. Good / Base / Bad Cases

- Good：新建管理员时传入 `password`，随后该管理员可以用自己的密码登录。
- Good：编辑管理员资料时留空密码字段，只更新角色或状态，不改变原密码。
- Good：调用 `PATCH /admins/{id}/password` 后旧密码登录返回 401，新密码登录成功。
- Base：无数据库阶段服务重启后恢复种子管理员和种子密码哈希。
- Bad：在 `AdminSummary`、dashboard 或 auth/me 中返回 `passwordHash`；这会把敏感凭据暴露给前端。
- Bad：继续使用单个全局 `demo_password` 校验所有管理员；这会让新建账号无法拥有独立凭据。

### 6. 必要测试

- 后端测试需要覆盖种子管理员 `admin/admin123` 可登录。
- 后端测试需要覆盖错误密码返回 `Unauthorized`。
- 后端测试需要覆盖锁定管理员即使密码正确也返回 `Forbidden`。
- 后端测试需要覆盖新建管理员独立密码登录。
- 后端测试需要覆盖重置密码后旧密码失效、新密码生效。
- 后端测试需要覆盖短密码或空白密码返回 `BadRequest`。
- API 冒烟需要创建管理员、登录、重置密码、再次登录，并检查所有读响应不包含密码字段。
- 前端需要运行 `npm run build`，确认 `AdminSaveRequest`、`AdminPasswordResetRequest` 与页面消费一致。

### 7. Wrong vs Correct

#### 错误

```rust
struct AccessStore {
    admins: BTreeMap<String, AdminSummary>,
    demo_password: String,
}
```

这个写法会让所有管理员共享同一个密码，账号维护无法真正设置密码。

```ts
export interface AdminSummary {
  id: string;
  username: string;
  passwordHash: string;
}
```

这个写法把敏感凭据带到前端读取模型。

#### 正确

```rust
struct AccessStore {
    admins: BTreeMap<String, AdminSummary>,
    admin_password_hashes: BTreeMap<String, String>,
}
```

管理员摘要和密码哈希分开存储，读取接口只返回摘要。

```ts
export interface AdminSaveRequest extends AdminSummary {
  password?: string;
}
```

写接口可以携带密码，读接口继续使用不含密码的 `AdminSummary`。

---

## 场景：dashboard 数据按角色权限裁剪

### 1. 范围 / 触发条件

- 触发条件：新增或修改 `/api/admin/dashboard` 摘要字段、模块入口、权限 scopes 或登录会话处理。
- 范围：后端 `DashboardSummary` 构造、当前管理员 `AdminAuthSession.scopes`、dashboard 指标/模块过滤、无权限摘要脱敏、前端 dashboard 与侧边栏菜单二次过滤。

### 2. 签名

- `GET /api/admin/dashboard`
- 认证头：`Authorization: Bearer <token>`
- 后端裁剪函数：`dashboard_summary_for_scopes(summary: DashboardSummary, scopes: &[PermissionScope]) -> DashboardSummary`

### 3. 契约

`GET /api/admin/dashboard` 仍返回完整顶层字段结构，字段名必须保持 `camelCase`：

```json
{
  "metrics": [],
  "moduleGroups": [],
  "lotteries": [],
  "drawSources": [],
  "recentOrders": [],
  "groupBuyPlans": [],
  "finance": {},
  "financialAccounts": [],
  "robots": [],
  "users": [],
  "admins": [],
  "roles": [],
  "settings": [],
  "registration": {},
  "invitePolicy": {}
}
```

dashboard 路由需要登录，但不要求单一业务 scope。响应数据必须按当前 token 对应的 `scopes` 做后端裁剪：

- `users`：保留 `users`、`registration`、用户管理模块、用户注册模块和“用户总数”指标；无权限时 `users=[]`，`registration` 返回关闭状态。
- `orders`：保留 `recentOrders`、订单管理模块、计奖派奖模块和“今日订单”指标；无权限时 `recentOrders=[]`。
- `finance`：保留 `finance`、`financialAccounts`、财务管理模块和“平台余额”指标；无权限时所有金额置 `0`，账户列表清空。
- `admins`：保留 `admins` 和管理员管理模块；无权限时 `admins=[]`。
- `roles`：保留 `roles` 和角色权限模块；无权限时 `roles=[]`。
- `systemSettings`：保留 `settings` 和系统设置模块；无权限时 `settings=[]`。
- `lotteries`：保留 `lotteries`、`drawSources`、`groupBuyPlans`、彩种控制台、彩种管理、开奖模式、开奖时间、合买配置、玩法配置模块和“已配置彩种”指标；无权限时这些数组清空。
- `customerService`：保留在线客服模块；dashboard 当前没有客服摘要字段。
- `robots`：保留机器人模块和 `robots`；无权限时 `robots=[]`。
- `rebates`：保留邀请管理、返利配置模块和 `invitePolicy`；无权限时邀请开关关闭，默认返利比例为 `0`。

前端菜单过滤只是二次防护，不能替代后端裁剪。

### 4. 校验与错误矩阵

| 条件 | 预期行为 |
|------|----------|
| 缺少 Authorization | HTTP 401，统一错误信封 |
| token 无效或已登出 | HTTP 401，统一错误信封 |
| token 有效但 scopes 较少 | HTTP 200，顶层字段结构不变，只返回允许领域摘要 |
| 无某领域权限的数组字段 | 返回空数组，不返回真实明细 |
| 无 `finance` 权限 | `finance` 所有金额为 `0`，`financialAccounts=[]` |
| 无 `users` 权限 | `registration` 返回关闭状态，`users=[]` |
| 无 `rebates` 权限 | `invitePolicy` 返回关闭邀请和 `0` 返利比例 |

### 5. Good / Base / Bad Cases

- Good：`role-super` 请求 dashboard 时看到完整模块、管理员、角色、财务、机器人和返利摘要。
- Good：只拥有 `users`、`orders`、`lotteries` 的运营角色请求 dashboard 时，只看到用户、订单、彩票相关指标和模块，财务/管理员/角色/机器人/返利摘要为空或置零。
- Base：低权限管理员仍可以打开系统概览，但只能看到自己有权限的摘要。
- Bad：只在前端隐藏菜单，后端 dashboard 仍返回完整 `admins`、`roles`、`finance` 等数据。
- Bad：无权限时删除顶层字段；这会破坏 `admin/src/types/dashboard.ts` 和页面消费契约。

### 6. 必要测试

- 后端需要覆盖超级管理员 scopes 下 dashboard 全量保留。
- 后端需要覆盖运营 scopes 下敏感数组清空、财务置零、返利关闭和模块/指标过滤。
- API 冒烟需要用低权限 token 请求 `/api/admin/dashboard`，确认不返回管理员、角色、系统设置、财务、机器人、邀请返利模块和摘要。
- 前端需要运行 `npm run build`，确认顶层字段结构保持兼容。

### 7. Wrong vs Correct

#### 错误

```rust
async fn get_dashboard_summary(State(state): State<AppState>) -> ApiResult<_> {
    Ok(Json(ApiEnvelope::success(dashboard_summary_with_orders(...))))
}
```

这个写法没有使用当前登录会话的 scopes，低权限管理员仍能读取完整概览摘要。

```rust
if !has_scope(scopes, &PermissionScope::Finance) {
    summary.finance = None;
}
```

这个写法改变了顶层字段类型，会破坏前端契约。

#### 正确

```rust
async fn get_dashboard_summary(
    State(state): State<AppState>,
    Extension(session): Extension<AdminAuthSession>,
) -> ApiResult<_> {
    let summary = dashboard_summary_with_orders(...);
    let summary = dashboard_summary_for_scopes(summary, &session.scopes);
    Ok(Json(ApiEnvelope::success(summary)))
}
```

路由从登录中间件读取当前会话，服务层负责统一裁剪。

```rust
if !has_scope(scopes, &PermissionScope::Finance) {
    summary.finance = redacted_finance_overview();
    summary.financial_accounts.clear();
}
```

无权限时保持顶层字段结构，但不暴露真实财务数据。

---

## 场景：彩种控制台控制开奖号码接口

### 1. 范围 / 触发条件

- 触发条件：新增或修改彩种控制台的开奖号码控制、开奖服务控制优先级、自动开奖控制逻辑。
- 范围：后端开奖控制配置 API、开奖服务读取控制号码、自动开奖手动彩种跳过规则、前端控制台类型和 SideSheet 表单。
- 本阶段控制配置为内存仓储；后续接入 PostgreSQL 时接口契约不应变化。

### 2. 签名

- `GET /api/admin/draw-controls`
- `GET /api/admin/draw-controls/{lotteryId}`
- `PUT /api/admin/draw-controls/{lotteryId}`

所有接口需要 `lotteries` 权限，并继续使用统一 API 信封。

### 3. 契约

`GET /api/admin/draw-controls` 返回每个彩种的控制状态；未配置过控制的彩种返回默认关闭状态：

```json
[
  {
    "lotteryId": "fc3d",
    "lotteryName": "福彩 3D",
    "numberType": "threeDigit",
    "enabled": false,
    "drawNumber": null,
    "updatedAt": null
  }
]
```

`PUT /api/admin/draw-controls/{lotteryId}` 请求体：

```json
{
  "enabled": true,
  "drawNumber": "2,4,7"
}
```

保存成功后返回完整 `LotteryDrawControl`：

```json
{
  "lotteryId": "fc3d",
  "lotteryName": "福彩 3D",
  "numberType": "threeDigit",
  "enabled": true,
  "drawNumber": "2,4,7",
  "updatedAt": "unix:1780475520"
}
```

开奖号码继续使用英文逗号分隔格式。后端保存时会规范化号码，例如 `247` 保存为 `2,4,7`；中文逗号输入也会被规范化为英文逗号。

开奖优先级：

1. 如果彩种控制配置 `enabled=true` 且有合法 `drawNumber`，手动触发开奖和自动开奖都优先使用控制号码。
2. 如果控制关闭，`platform` 彩种使用平台生成器。
3. 如果控制关闭，`api` 彩种使用已绑定的 API 开奖源。
4. 如果控制关闭，`manual` 彩种自动任务缺少管理员号码时继续跳过。

### 4. 校验与错误矩阵

| 条件 | 预期行为 |
|------|----------|
| `lotteryId` 不存在 | HTTP 404，返回彩种不存在 |
| `enabled=true` 且 `drawNumber` 为空 | HTTP 400，返回控制开奖号码必填 |
| 3 位彩种号码不是 3 个数字 | HTTP 400，返回号码长度错误 |
| 5 位彩种号码不是 5 个数字 | HTTP 400，返回号码长度错误 |
| PK10、11 选 5、快 3、快乐 8/幸运 20 控制号码不符合长度、范围或去重规则 | HTTP 400，返回号码校验错误 |
| 号码包含非数字内容 | HTTP 400，返回号码格式错误 |
| `enabled=false` 且 `drawNumber` 为空 | HTTP 200，保存关闭状态 |
| 手动彩种启用控制号码后到期自动任务 | 自动开奖成功，不写入跳过期号 |
| API 彩种启用控制号码后开奖 | 不请求或不采用 API 结果，使用控制号码 |

### 5. Good / Base / Bad Cases

- Good：`fc3d` 开启控制号码 `2,4,7`，期号到期后自动开奖保存 `drawNumber=2,4,7`，随后结算使用该号码。
- Good：`au5` 开启控制号码 `7,8,9,4,2`，即使 API68 返回其它结果，本次开奖仍使用控制号码。
- Base：控制关闭时，现有平台生成器和 API68 来源行为保持不变。
- Bad：只在前端保存控制状态，不让后端开奖服务读取；这会导致页面看起来控制成功但自动开奖仍按原来源开奖。
- Bad：前端直接绕过后端校验写入不符合彩种号码类型的号码。

### 6. 必要测试

- 后端需要覆盖平台开奖使用控制号码。
- 后端需要覆盖 API 开奖源被控制号码覆盖。
- 后端需要覆盖控制号码按号码类型校验。
- 后端需要覆盖手动彩种自动任务在控制号码启用时不跳过并完成开奖。
- 前端需要运行 `npm run build`，确认 `LotteryDrawControl` 和保存请求类型与接口字段一致。
- 跨层联调需要在“彩种控制台”保存控制号码，再执行或等待对应期号开奖，确认期号列表和控制台回显的开奖号码一致。

### 7. Wrong vs Correct

#### 错误

```ts
setLocalControl(lotteryId, drawNumber);
```

这个写法只改变页面状态，自动开奖服务不会使用该号码。

#### 正确

```ts
await saveLotteryDrawControl(lotteryId, {
  enabled: true,
  drawNumber: "2,4,7",
});
```

控制配置必须提交后端，由开奖服务作为唯一事实来源读取。

#### 错误

```rust
let draw_number = api_source.draw_number_for(issue).await?;
```

这个写法会让 API 开奖源优先于后台控制号码。

#### 正确

```rust
if let Some(draw_number) = active_draw_control_number(&issue.lottery_id)? {
    return Ok(DrawIssueResultRequest { draw_number: Some(draw_number) });
}
```

开奖服务必须先检查控制号码，再按原开奖模式读取 API 或平台生成器。

---

## 场景：用户端充值与客服直充接口

### 1. 范围 / 触发条件

- 触发条件：新增用户端充值下单、彩虹易支付回调、客服直充会话和后台充值订单查询，属于后端 API、服务层、数据库、前端管理页和 OpenAPI 的跨层契约。
- 范围：用户端充值接口、用户端客服接口、后台充值订单接口、财务流水充值入账类型、系统设置充值配置。

### 2. 签名

- `GET /api/user/recharge/config`
- `GET /api/user/recharge/orders`
- `POST /api/user/recharge/orders`
- `GET /api/user/recharge/epay/notify`
- `POST /api/user/recharge/epay/notify`
- `GET /api/user/recharge/epay/return`
- `GET /api/user/support/conversations`
- `GET /api/user/support/conversations/{id}`
- `POST /api/user/support/conversations/{id}/messages`
- `GET /api/admin/recharge-orders`
- `POST /api/admin/recharge-orders/{id}/confirm`

### 3. 契约

充值配置返回：

```json
{
  "channels": [
    {
      "channel": "rainbowEpay",
      "name": "彩虹易支付",
      "enabled": false,
      "description": "跳转到彩虹易支付完成在线充值",
      "payTypes": ["alipay", "wxpay"]
    },
    {
      "channel": "customerService",
      "name": "客服直充",
      "enabled": true,
      "description": "客服已收到您的直充申请，请在会话中确认付款方式和到账信息。",
      "payTypes": []
    }
  ],
  "minAmountMinor": 100,
  "maxAmountMinor": 10000000
}
```

创建充值订单请求：

```json
{
  "channel": "customerService",
  "amountMinor": 1200,
  "payType": "alipay"
}
```

`channel` 只能是 `rainbowEpay` 或 `customerService`。金额继续使用最小货币单位。彩虹易支付返回 `paymentUrl`，客服直充返回 `supportConversationId` 并同步创建客服会话。

彩虹易支付通知接口不返回统一信封；验签和入账成功后必须返回裸文本 `success`，以便第三方支付平台确认通知已处理。用户端客服消息请求体为：

```json
{
  "content": "我已提交客服直充，请协助确认。"
}
```

后台确认客服直充请求体为：

```json
{
  "providerTradeNo": "客服收款凭证"
}
```

确认成功后订单状态变为 `paid`，并写入 `rechargeCredit` 资金流水。

### 4. 校验与错误矩阵

| 条件 | 预期行为 |
|------|----------|
| 未登录调用用户端充值或客服接口 | HTTP 401 |
| 充值金额低于 `recharge_min_amount_minor` | HTTP 400 |
| 充值金额高于 `recharge_max_amount_minor` | HTTP 400 |
| 彩虹易支付未开启 | HTTP 400 |
| 彩虹易支付网关、商户号或密钥仍为占位值 | HTTP 400 |
| `payType` 不在后台配置列表中 | HTTP 400 |
| 客服直充未开启 | HTTP 400 |
| 用户访问他人的客服会话 | HTTP 404 |
| 客服消息内容为空 | HTTP 400 |
| 支付通知签名无效或金额不匹配 | HTTP 400 |
| 重复支付通知 | 保持幂等，不重复生成 `rechargeCredit` 流水 |
| 后台确认非客服直充订单 | HTTP 400 |
| 后台重复确认已入账客服直充订单 | 保持幂等，不重复生成 `rechargeCredit` 流水 |

### 5. Good / Base / Bad Cases

- Good：客服直充创建订单后返回 `waitingCustomerService`，后台在线客服可看到会话，用户端可继续发消息。
- Good：客服确认收到客服直充款项后，后台调用确认接口，订单变为 `paid`，用户余额增加。
- Good：彩虹易支付通知验签成功后，充值订单变为 `paid`，资金流水新增 `rechargeCredit`，用户余额增加。
- Base：彩虹易支付默认关闭，客服直充默认开启，方便未接入支付网关时先通过客服处理充值。
- Bad：把彩虹易支付网关、商户号或密钥写入环境变量而不写入 `system_settings`；后台无法维护配置。
- Bad：支付通知成功后只改订单状态不调用财务仓储；用户余额不会增加。

### 6. 必要测试

- 后端需要覆盖客服直充订单创建和会话 ID 返回。
- 后端需要覆盖彩虹易支付签名排序和支付 URL 生成。
- 后端需要覆盖充值入账流水幂等。
- 后端需要覆盖用户只能查看和回复自己的客服会话。
- 后端需要覆盖客服直充后台确认入账及重复确认幂等。
- OpenAPI 测试需要覆盖后台充值订单、用户端充值和用户端客服路径。
- 前端需要运行 `npm run build`，确认充值订单类型、资金流水类型和财务页面展示与后端枚举一致。
- PostgreSQL 冒烟需要覆盖用户注册/登录、充值配置读取、客服直充下单、用户客服消息和后台充值订单查询。

### 7. Wrong vs Correct

#### 错误

```rust
let order = mark_recharge_paid(order_id)?;
```

这个写法只改变充值状态，没有给用户余额入账。

#### 正确

```rust
let order = store.mark_paid(order_id, paid_amount_minor, trade_no)?;
finance.credit_recharge(&order.user_id, order.amount_minor, &order.id).await?;
```

充值成功必须同时更新充值订单和资金流水，财务仓储负责保持同一充值单的入账幂等。

#### 错误

```rust
support.get(conversation_id).await
```

用户端直接按会话 ID 查询会泄露他人客服消息。

#### 正确

```rust
support.get_for_user(conversation_id, &session.user.id).await
```

用户端客服接口必须按当前登录用户校验会话归属，归属不匹配时返回不存在。

---

## 场景：用户提现申请接口与用户维护余额边界

### 1. 范围 / 触发条件

- 触发条件：新增用户端提现申请接口，并收紧后台用户维护中余额、用户 ID、邀请码的编辑权限。
- 范围：`/api/user/withdrawals`、`/api/user/withdrawal-methods`、`FinanceRepository::freeze_withdrawal`、后台 `/api/admin/users` 返回余额展示。

### 2. 签名

- `GET /api/user/withdrawals`
- `POST /api/user/withdrawals`
- `GET/POST/PUT/DELETE /api/user/withdrawal-methods`
- 后台用户列表：`GET /api/admin/users`
- 资金流水类型：`ledger_entries.kind = withdrawalFreeze`

### 3. 契约

提现申请请求：

```json
{
  "methodId": "WM000001",
  "amountMinor": 10000
}
```

提现申请响应：

```json
{
  "id": "W000000000001",
  "userId": "U10001",
  "username": "demo_user",
  "methodId": "WM000001",
  "methodType": "bankCard",
  "accountHolder": "张三",
  "accountNumber": "6222000000000000",
  "bankName": "招商银行",
  "amountMinor": 10000,
  "status": "pending",
  "createdAt": "2026-06-04 22:28:00",
  "reviewedAt": null
}
```

提交提现申请必须同步执行财务冻结：

- `financial_accounts.available_balance_minor -= amountMinor`
- `financial_accounts.frozen_balance_minor += amountMinor`
- 新增 `ledger_entries.kind=withdrawalFreeze`
- 流水 `referenceId` 必须是提现申请 ID

后台用户维护接口不得作为余额或邀请码编辑入口：

- `PUT /api/admin/users/{id}` 必须保留原 `balanceMinor` 和 `inviteCode`。
- `GET /api/admin/users` 返回的 `balanceMinor` 应以 `financial_accounts.available_balance_minor` 为准。
- 用户 ID 仍作为资源 ID 使用，更新时路径 ID 必须与请求体 ID 一致。

### 4. 校验与错误矩阵

| 条件 | 预期行为 |
|------|----------|
| `methodId` 为空 | HTTP 400 |
| 提现方式不存在 | HTTP 404 |
| 提现方式不属于当前用户 | HTTP 400 |
| `amountMinor <= 0` | HTTP 400 |
| 可用余额不足 | HTTP 400，返回余额不足业务错误 |
| 重复冻结同一个提现申请 ID | 保持幂等，返回既有 `withdrawalFreeze` 流水 |
| 用户维护请求修改 `inviteCode` | 后端忽略，保留原邀请码 |
| 用户维护请求修改 `balanceMinor` | 后端忽略，余额仅由财务账户决定 |

### 5. Good / Base / Bad Cases

- Good：用户先绑定银行卡提现方式，再提交 `POST /api/user/withdrawals`，返回 `pending` 提现申请，资金账户可用余额减少、冻结余额增加。
- Good：后台用户维护列表中余额来自财务账户；财务手动调账后刷新用户维护页能看到新余额。
- Base：提现审核、打款、驳回和解冻流程后续再接入；当前接口只负责申请和冻结。
- Bad：手机端提交 `method_id` 或 `amount` 字段；后端当前契约只接受 `methodId` 和 `amountMinor`。
- Bad：通过用户维护直接改 `balanceMinor`；这会绕过财务流水，破坏审计链路。

### 6. 必要测试

- 后端需要覆盖用户更新时保留余额和邀请码。
- 后端需要覆盖提现冻结会减少可用余额、增加冻结余额，并生成 `withdrawalFreeze` 流水。
- 后端需要覆盖提现方式归属校验。
- OpenAPI 测试需要覆盖 `/user/withdrawals` 路径。
- 管理后台需要运行 `npm run build`，确认新增流水枚举能展示。
- 手机端需要运行 `npm run build`，确认提现提交字段与后端契约一致。

### 7. Wrong vs Correct

#### 错误

```rust
user.balance_minor = payload.balance_minor;
```

用户维护直接写余额会绕过资金流水。

#### 正确

```rust
user.balance_minor = existing.balance_minor;
```

用户维护只改基础资料；余额通过 `FinanceRepository` 和资金流水变化。

#### 错误

```ts
await http.post('/user/withdrawals', { method_id, amount });
```

这个请求字段不符合后端 `camelCase` 契约，也没有使用分作为金额单位。

#### 正确

```ts
await createWithdrawalOrder({ methodId, amountMinor });
```

用户端提现申请统一使用 `methodId` 和 `amountMinor`。

---

## 场景：手机端首页销售中彩种接口

### 1. 范围 / 触发条件

- 触发条件：手机端首页需要展示彩种入口、分类分组、当前期号倒计时或最近开奖号码。
- 范围：`/api/lottery/home`、`LotteryRepository`、`DrawRepository`、`mobile/src/api/lottery.ts`、`HomeView.vue`、首页开奖更新组合函数。

### 2. 签名

- `GET /api/lottery/home`
- `GET /api/lottery/groups`
- `GET /api/lottery/history/latest?group_code=all&lottery_code=fc3d`
- `GET /api/lottery/history?lottery_code=fc3d&page=1&page_size=50`

### 3. 契约

接口返回统一 API 信封，`data` 字段为首页聚合数据。所有字段必须使用 `camelCase`：

```json
{
  "serverTime": "2026-06-05 00:33:00",
  "settings": {
    "bannersEnabled": true,
    "tickerEnabled": true,
    "featuredEnabled": true,
    "groupsEnabled": true,
    "statsEnabled": false
  },
  "ticker": {
    "enabled": true,
    "items": [{ "id": "fc3d-20260605001", "text": "福彩 3D 第20260605001期 开奖号码 1,2,3" }]
  },
  "featuredSection": {
    "enabled": true,
    "title": "高频极速",
    "lotteries": []
  },
  "groups": [
    {
      "code": "welfare",
      "name": "福利彩种",
      "lotteries": [
        {
          "code": "fc3d",
          "name": "福彩 3D",
          "category": "welfare",
          "logoUrl": null,
          "issue": "20260605002",
          "status": "selling",
          "nextDrawTime": "2026-06-05 21:00:15",
          "saleStopTime": "2026-06-05 20:59:45",
          "drawInterval": null,
          "drawTimeText": "每日 21:00:15",
          "scheduleText": "每日 21:00:15",
          "latestResult": ["1", "2", "3"],
          "resultStyle": "balls",
          "resultCount": 3,
          "groupBuyEnabled": false,
          "latestDraw": {
            "issue": "20260605001",
            "resultNumbers": ["1", "2", "3"],
            "openedAt": "2026-06-05 21:00:15"
          }
        }
      ]
    }
  ],
  "stats": {
    "todayWinnerCount": 0,
    "totalPayoutDisplay": "¥0"
  }
}
```

`groups` 必须只包含 `saleEnabled=true` 的彩种；停售彩种不能出现在首页推荐区、分组区或跑马灯中。分组顺序跟随后台彩种分类配置；分类下没有销售中彩种时不返回空组。若彩种引用了不存在的分类，必须进入以分类代码命名的兜底组。

`settings.statsEnabled` 默认必须为 `false`，手机端首页默认不展示统计卡片；后续如果增加后台开关，也必须由后台显式开启后才返回 `true`。

每个彩种卡片必须优先返回当前可售或封盘待开奖期号作为 `issue/nextDrawTime/saleStopTime/status`，并单独返回最近已开奖期号的 `latestResult/latestDraw`。首页倒计时由前端基于 `saleStopTime` 或 `nextDrawTime` 本地计算，不要后端返回剩余秒数。

### 4. 校验与错误矩阵

| 条件 | 预期行为 |
|------|----------|
| 没有销售中彩种 | `groups=[]`、`featuredSection.lotteries=[]`，接口仍返回成功 |
| 销售中彩种没有期号 | 彩种仍返回，`status=waiting`、`latestResult=[]` |
| 销售中彩种没有开奖历史 | 彩种仍返回，`latestDraw=null`、`latestResult=[]` |
| 彩种已停售 | 不出现在任何首页彩种集合中 |
| 开奖号以英文逗号保存 | 前端收到 `latestResult` 数组，展示时不再二次猜分隔符 |

### 5. Good / Base / Bad Cases

- Good：首页接口一次返回所有销售中彩种分组，手机端按 `groups` 动态渲染全部分类。
- Good：首页推荐区可以复用销售中高频彩种，但完整彩种入口仍以 `groups` 为准。
- Bad：前端只渲染 `groups[0]` 和 `groups[1]`；分类增多后会漏掉彩种。
- Bad：接口返回 `snake_case` 字段，例如 `latest_result`；这会和当前用户端 API 类型冲突。
- Bad：停售彩种仍出现在首页，只依赖投注页再拦截；这会误导用户进入不可售彩种。

### 6. 必要测试

- 后端需要覆盖只返回销售中彩种、按分类分组、带最近开奖号码。
- OpenAPI 测试需要覆盖 `/lottery/home` 路径。
- 手机端需要运行 `npm run build`，确认 `camelCase` 首页字段和组件消费一致。

### 7. 补充分组与历史接口契约

`/api/lottery/groups`、`/api/lottery/history/latest`、`/api/lottery/history` 当前服务于手机端全部彩种、开奖历史、合买创建入口。它们也必须返回统一 API 信封；为了兼容这些既有手机端页面，数据字段沿用当前页面消费的 `snake_case` 形状，例如 `lottery_code`、`result_numbers`、`opened_at`、`page_size`。

`/api/lottery/groups` 返回销售中彩种分组：

```json
[
  {
    "code": "welfare",
    "name": "福利彩种",
    "lotteries": [
      {
        "code": "fc3d",
        "name": "福彩 3D",
        "category": "welfare",
        "logo_url": null,
        "draw_interval": null,
        "daily_draw_time": "21:00:15",
        "group_sort_order": 0,
        "is_recommended": false
      }
    ]
  }
]
```

`/api/lottery/history/latest` 返回每个销售中彩种最近一期已开奖数据；`/api/lottery/history` 返回分页历史：

```json
{
  "items": [
    {
      "id": "D000000000001",
      "lottery_code": "fc3d",
      "lottery_name": "福彩 3D",
      "category": "welfare",
      "logo_url": null,
      "issue": "20260605001",
      "result": "1,2,3",
      "result_numbers": ["1", "2", "3"],
      "opened_at": "2026-06-05 21:00:15",
      "status": "drawn"
    }
  ],
  "total_count": 1,
  "page": 1,
  "page_size": 50,
  "total_pages": 1
}
```

这三个接口必须过滤停售彩种；开奖历史只返回 `status=drawn` 且有开奖号码的期号。`latest` 接口不分页，每个彩种最多返回一条最新开奖记录；`history` 接口需要按 `page/page_size` 分页，并把 `page_size` 限制在安全上限内。

---

## 场景：后台财务分页与提现审核接口

### 1. 范围 / 触发条件

- 触发条件：财务管理页展示资金账户、充值订单、资金流水或提现申请，或者后台审核提现申请。
- 范围：`/api/admin/finance-overview`、`/api/admin/financial-accounts`、`/api/admin/recharge-orders`、`/api/admin/ledger-entries`、`/api/admin/withdrawal-orders`、`FinanceRepository`、`WithdrawalRepository`、管理后台财务页面。

### 2. 签名

- `GET /api/admin/finance-overview`
- `GET /api/admin/financial-accounts?page=1&pageSize=20`
- `GET /api/admin/recharge-orders?page=1&pageSize=20`
- `GET /api/admin/ledger-entries?page=1&pageSize=20`
- `GET /api/admin/withdrawal-orders?page=1&pageSize=20`
- `POST /api/admin/withdrawal-orders/{id}/approve`
- `POST /api/admin/withdrawal-orders/{id}/reject`

### 3. 契约

分页列表响应统一返回：

```json
{
  "items": [],
  "totalCount": 0,
  "page": 1,
  "pageSize": 20,
  "totalPages": 0
}
```

不传 `page/pageSize` 时允许返回全量列表，用于兼容内部调试；管理后台页面必须显式传入分页参数。

资金账户列表项必须包含用户名：

```json
{
  "userId": "U10001",
  "username": "demo_user",
  "availableBalanceMinor": 12000,
  "frozenBalanceMinor": 2000
}
```

提现审核接口不需要请求体。通过提现申请时，后端必须扣减冻结余额、写入 `withdrawalPayout` 流水，并把申请状态改为 `approved`。驳回提现申请时，后端必须把冻结余额退回可用余额、写入 `withdrawalReject` 流水，并把申请状态改为 `rejected`。

### 4. 校验与错误矩阵

| 条件 | 预期行为 |
|------|----------|
| `page <= 0` 或缺失 | 使用第 1 页 |
| `pageSize <= 0` 或缺失 | 使用默认每页 20 条 |
| 请求页码超过最大页 | 返回最后一页 |
| 资金账户没有匹配用户 | `username=null`，不阻塞账户展示 |
| 提现申请不存在 | HTTP 404 |
| 提现申请已通过再驳回 | HTTP 400 |
| 提现申请已驳回再通过 | HTTP 400 |
| 冻结余额不足 | HTTP 400，不改变提现状态 |

### 5. Good / Base / Bad Cases

- Good：财务管理页按分页请求资金账户，表格同时展示用户名和用户 ID。
- Good：充值订单、资金流水、提现申请都有分页控件，翻页不会一次性拉取所有历史记录。
- Good：待审核提现点击“通过”后状态变为已通过，冻结余额减少，资金流水出现提现打款。
- Good：待审核提现点击“驳回”后状态变为已驳回，冻结余额退回可用余额，资金流水出现提现驳回解冻。
- Bad：前端拿全量数组后自行分页；数据量增大后会拖慢财务页面。
- Bad：后台只改提现申请状态，不写资金流水；这会破坏财务审计链路。

### 6. 必要测试

- 后端需要覆盖提现通过、提现驳回和反向审核拒绝。
- OpenAPI 测试需要覆盖财务总览、提现申请列表和提现审核路径。
- 管理后台需要运行 `npm run build`，确认分页响应、用户名字段和提现状态枚举类型一致。
