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

## 场景：开奖期号与开奖源接口

### 1. 范围 / 触发条件

- 触发条件：新增或修改开奖源列表、开奖期号创建、封盘、开奖、取消，以及管理后台开奖期号页面。
- 范围：后端开奖领域模型、内存开奖仓储、开奖 API、彩种开奖模式复用、前端 draw API client、`useDraws` hook 和“开奖期号与开奖源”页面。

### 2. 签名

- `GET /api/admin/draw-sources`
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

`platform` 和 `api` 开奖模式可以传空对象 `{}`，后端本地生成逗号分隔开奖号码。`manual` 开奖模式必须传 `drawNumber`，格式为英文逗号分隔数字，例如 `2,4,7` 或 `7,8,9,4,2`。后端兼容读取旧的紧凑数字串，但保存和返回统一使用逗号分隔格式。

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
| 关闭非 `open` 期号 | HTTP 400，返回 `only open draw issues can be closed` |
| 手动开奖缺少号码 | HTTP 400，返回 `manual draw requires draw number` |
| 3 位彩种号码不是 3 位数字 | HTTP 400，返回号码长度或数字错误 |
| 5 位彩种号码不是 5 位数字 | HTTP 400，返回号码长度或数字错误 |
| 已开奖或已取消期号再次开奖 | HTTP 400，返回 `draw issue cannot be drawn in current status` |
| 已开奖期号取消 | HTTP 400，返回 `drawn draw issue cannot be cancelled` |
| 已取消期号重复取消 | HTTP 400，返回 `draw issue is already cancelled` |
| 查询或操作不存在期号 | HTTP 404，返回期号不存在 |

### 5. Good / Base / Bad Cases

- Good：`fc3d` 创建期号后调用 `PATCH /draw` 传 `{}`，后端按 `threeDigit` 生成 3 位数字并返回 `status="drawn"`。
- Good：`manual-test` 创建期号后传 `{"drawNumber":"7,8,9,4,2"}`，后端按 `fiveDigit` 校验并保存逗号分隔开奖结果。
- Base：开奖期号仓储当前是内存模式，服务重启后期号清空；这适合当前后台流程验证。
- Bad：前端为 `manual` 期号传空对象执行开奖；后端必须拒绝，不能静默生成号码。
- Bad：开奖后直接改订单状态或资金余额；本阶段还没有计奖、派奖和资金流水，开奖只记录结果事实。

### 6. 必要测试

- 后端需要覆盖期号创建、关闭销售、平台/API 生成号码、手动开奖号码必填、号码长度和数字校验。
- 后端需要覆盖已开奖期号不能重复开奖或取消。
- 后端需要运行 `cargo fmt --check`、`cargo check`、`cargo test`。
- 前端需要运行 `npm run build`。
- 跨层联调需要请求开奖源、创建期号、封盘、API 开奖、手动开奖，并在管理后台页面确认结果回显。

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

后端根据彩种开奖模式决定是校验管理员录入号码，还是由平台/API 本地生成器生成号码。

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
3. `platform` 和 `api` 期号由后端生成逗号分隔开奖号码，并把期号状态改为 `drawn`。
4. `manual` 期号不自动开奖，写入 `skippedIssues`。
5. 本次自动开奖成功的期号会立即执行结算，并把中奖派奖写入资金流水。

### 4. 校验与错误矩阵

| 条件 | 预期行为 |
|------|----------|
| `now` 为空 | HTTP 400，返回 `automation time is required` |
| 没有到期可处理期号 | HTTP 200，返回空数组 |
| 到期 `open` 期号 | 自动关闭销售并出现在 `closedIssues` |
| 到期 `platform/api` 期号 | 自动开奖、结算和派奖入账 |
| 到期 `manual` 期号 | 不自动开奖，出现在 `skippedIssues` |
| 自动结算重复 | 只处理本次新开奖成功期号，重复结算仍由结算服务拒绝 |
| 中奖用户资金账户不存在 | 资金服务返回错误；后续需要事务化避免部分状态落地 |

### 5. Good / Base / Bad Cases

- Good：`fc3d` 的 open 期号封盘时间和开奖时间都早于 `now`，执行后期号先封盘再开奖，生成结算批次和 `payoutCredit` 流水。
- Good：手动开奖期号到期后只封盘，不自动伪造开奖号码，结果中包含跳过原因。
- Base：本阶段是后台触发式一次性执行器，适合内存仓储阶段验证状态链路。
- Bad：自动任务直接修改订单状态或用户余额；必须复用订单结算服务和资金服务。
- Bad：为 `manual` 期号静默生成号码；手动开奖必须由管理员录入号码。

### 6. 必要测试

- 后端需要覆盖到期自动封盘、自动开奖、自动结算和派奖入账。
- 后端需要覆盖手动开奖缺少号码时被跳过。
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
  "issue": "20260602210015",
  "numberType": "threeDigit",
  "drawMode": "api",
  "scheduledAt": "2026-06-02 21:00:15",
  "saleClosedAt": "2026-06-02 20:59:45",
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
    "issue": "20260602210015",
    "numberType": "threeDigit",
    "drawMode": "api",
    "scheduledAt": "2026-06-02 21:00:15",
    "saleClosedAt": "2026-06-02 20:59:45"
  }
]
```

生成规则：

1. 后端读取彩种 `DrawSchedule`，不由前端计算开奖时间。
2. 如果同彩种已有期号，使用该彩种最新 `scheduledAt` 和传入 `now` 中较晚的时间作为基线。
3. 周期开奖：`baseline + intervalSeconds`。
4. 每日固定开奖：选择严格晚于基线的当天或次日配置时间。
5. 周开奖：选择严格晚于基线的下一个配置星期和时间。
6. 期号编码使用开奖时间格式化为 `YYYYMMDDHHMMSS`。
7. 创建仍复用开奖期号仓储，保持重复期号、彩种匹配、开奖时间和封盘时间校验一致。
8. 批量预览和批量生成必须在同一次计划中跳过已存在的同彩种同 `issue`，并继续寻找后续可用期号。

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
| 计划尝试次数耗尽仍无法生成足量唯一期号 | HTTP 409，返回唯一期号生成失败 |

### 5. Good / Base / Bad Cases

- Good：`ssc60` 配置 `periodic.intervalSeconds=60`，`now=2026-06-02 20:00:00`，生成 `scheduledAt=2026-06-02 20:01:00`。
- Good：`fc3d` 配置每日 `21:00:15`，`now=2026-06-02 22:00:00`，生成次日 `2026-06-03 21:00:15`。
- Good：周二、周四 `21:00:00` 的彩种，在周二 22:00 后生成周四 21:00。
- Good：`preview-generation` 请求 `count=3` 返回未来 3 期计划，但随后请求期号列表不会多出新期号。
- Good：`generate-batch` 请求 `count=3` 创建 3 个 open 期号，并返回标准 `DrawIssue[]`。
- Base：本阶段是后台触发式生成单期或多期，适合内存仓储阶段验证计划计算。
- Bad：前端自己根据彩种 schedule 计算开奖时间；计划计算必须由后端统一负责。

### 6. 必要测试

- 后端需要覆盖周期、每日、周开奖三种计划。
- 后端需要覆盖已有期号时从最新期号继续生成。
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
- 范围：后端运行环境变量、`app::router_from_env` 启动流程、调度服务、自动任务服务、期号生成服务、调度运行历史仓储、管理后台状态接口和结构化日志。

### 2. 签名

调度通过服务启动时读取环境变量启用：

- `DRAW_SCHEDULER_ENABLED`
- `DRAW_SCHEDULER_INTERVAL_SECONDS`
- `DRAW_SCHEDULER_FUTURE_ISSUE_COUNT`
- `DRAW_SCHEDULER_SALE_CLOSE_LEAD_SECONDS`

管理后台状态接口：

- `GET /api/admin/draw-scheduler/status`

后端内部入口：

- `DrawSchedulerConfig::from_env()`
- `DrawSchedulerRepository::new(config)`
- `DrawSchedulerRepository::status()`
- `DrawSchedulerRepository::record_success(trigger, started_at, finished_at, run)`
- `DrawSchedulerRepository::record_failure(trigger, started_at, finished_at, now, error)`
- `spawn_draw_scheduler(draws, lotteries, orders, finance, config, scheduler)`
- `run_draw_scheduler_once(draws, lotteries, orders, finance, config, now)`

### 3. 契约

默认配置：

```text
DRAW_SCHEDULER_ENABLED=false
DRAW_SCHEDULER_INTERVAL_SECONDS=60
DRAW_SCHEDULER_FUTURE_ISSUE_COUNT=1
DRAW_SCHEDULER_SALE_CLOSE_LEAD_SECONDS=30
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

1. 未设置 `DRAW_SCHEDULER_ENABLED` 时，调度默认关闭，不会后台改写期号、订单或资金数据。
2. `DRAW_SCHEDULER_ENABLED=true` 时，服务启动后创建 Tokio 后台任务。
3. 每轮调度使用服务器当前本地时间，格式为 `YYYY-MM-DD HH:mm:ss`。
4. 每轮先调用既有 `run_draw_automation`，处理到期封盘、开奖、结算和派奖入账。
5. 自动任务执行后，再扫描 `saleEnabled=true` 的彩种，确保每个彩种至少有 `DRAW_SCHEDULER_FUTURE_ISSUE_COUNT` 个未来 `open/closed` 期号。
6. 未来期号判断只统计同彩种、状态为 `open` 或 `closed`，并且 `scheduledAt >= now` 的期号。
7. 补期继续调用 `generate_draw_issue_batch`，不在调度服务里重新实现开奖计划算法。
8. `saleEnabled=false` 彩种不会自动补期，会记录为跳过彩种。
9. 调度周期成功或失败都要写入调度运行历史，页面通过状态接口读取历史，而不是解析日志。
10. 调度周期成功或失败都使用 `tracing` 结构化日志记录，不暴露原始请求体或敏感信息。

### 4. 校验与错误矩阵

| 条件 | 预期行为 |
|------|----------|
| `DRAW_SCHEDULER_ENABLED` 未设置 | 使用默认 `false`，记录调度关闭日志 |
| `DRAW_SCHEDULER_ENABLED=true/1/yes/on` | 启动后台调度 |
| `DRAW_SCHEDULER_ENABLED=false/0/no/off` | 不启动后台调度 |
| `DRAW_SCHEDULER_ENABLED` 为其他值 | 启动失败，返回清晰配置错误 |
| `DRAW_SCHEDULER_INTERVAL_SECONDS=0` | 启动失败，返回 `draw scheduler interval seconds must be greater than zero` |
| `DRAW_SCHEDULER_FUTURE_ISSUE_COUNT` 小于 1 或大于 50 | 启动失败，返回未来期号数量范围错误 |
| `DRAW_SCHEDULER_SALE_CLOSE_LEAD_SECONDS=0` | 启动失败，返回封盘提前秒数错误 |
| 单轮调度 `now` 为空 | 返回 `draw scheduler time is required` |
| 自动开奖或补期过程中发生业务错误 | 当前轮记录错误日志，后台任务继续下一轮 |
| 调度未启用时查询状态 | HTTP 200，`enabled=false`，历史为空 |
| 最近运行超过 20 条 | 只保留最新 20 条，旧记录从内存仓储移除 |
| 状态仓储锁异常 | HTTP 500，返回统一错误信封 |

### 5. Good / Base / Bad Cases

- Good：设置 `DRAW_SCHEDULER_ENABLED=true` 和 `DRAW_SCHEDULER_INTERVAL_SECONDS=1` 后，本地启动服务会自动为销售开启彩种补齐未来期号。
- Good：调度跑过一轮后，`GET /api/admin/draw-scheduler/status` 返回最新 `SCH...` 记录，管理后台“常驻调度”显示成功状态和运行摘要。
- Good：已有到期开奖期号时，单轮调度先执行封盘/开奖/结算，再补齐下一期期号。
- Base：默认关闭适合本地开发和测试，不会让后台循环干扰手动 API 冒烟。
- Bad：在调度服务里复制一套封盘、开奖、结算或开奖计划计算逻辑；这些必须继续复用 `run_draw_automation` 和 `generate_draw_issue_batch`。
- Bad：管理后台为了显示调度状态去解析服务日志；页面必须调用 `GET /api/admin/draw-scheduler/status`。

### 6. 必要测试

- 后端需要覆盖调度默认关闭时不启动后台任务。
- 后端需要覆盖环境变量解析和无效值拒绝。
- 后端需要覆盖销售开启彩种自动补齐未来期号，销售关闭彩种跳过。
- 后端需要覆盖未来期号缓冲已满足时不重复生成。
- 后端需要覆盖到期期号先自动开奖，再补齐未来期号。
- 后端需要覆盖调度历史成功记录、失败记录和最近 20 条保留上限。
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
