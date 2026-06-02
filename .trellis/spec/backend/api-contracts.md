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
  "playCategories": ["direct", "groupThree", "groupSix"]
}
```

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
- 跨层联调需要至少请求列表、创建、销售开关、删除和 dashboard，确认同一仓储数据一致。
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
    "window": "full",
    "description": "按百位、十位、个位顺序完全匹配"
  }
]
```

`POST /api/admin/play-rules/evaluate` 请求体：

```json
{
  "numberType": "threeDigit",
  "ruleCode": "threeDirect",
  "selection": {
    "positions": [[2], [4], [7]]
  },
  "drawNumber": "247"
}
```

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
| 3 位玩法开奖号码不是 3 位数字 | HTTP 400，返回开奖号码长度或数字错误 |
| 5 位玩法开奖号码不是 5 位数字 | HTTP 400，返回开奖号码长度或数字错误 |
| 直选没有 3 个位置选择 | HTTP 400，返回 `direct play requires three position selections` |
| 选号为空 | HTTP 400，返回 `digit selection cannot be empty` |
| 选号数字大于 9 | HTTP 400，返回 `digit selection must be between 0 and 9` |
| 组三胆码数量不是 1 | HTTP 400，返回胆码数量错误 |
| 组六胆码数量不是 1 或 2 | HTTP 400，返回胆码数量错误 |
| 胆码和拖码重复 | HTTP 400，返回 `banker digits and drag digits cannot overlap` |
| 大小单双没有选择属性 | HTTP 400，返回大小单双属性错误 |

### 5. Good / Base / Bad Cases

- Good：`threeDirect` 选择 `[[2], [4], [7]]`，开奖号码 `247`，返回 `stakeCount=1`、`isWinning=true`。
- Good：`fiveBackGroupSix` 选择 `2,4,7,9`，开奖号码 `78942` 的后三为 `942`，属于组六且数字都在选号范围内，应命中。
- Good：`fiveBigSmallOddEven` 当前默认按后两位判断，开奖号码 `78942` 的十位 `4` 为小、个位 `2` 为双。
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

## 场景：订单与投注基础接口

### 1. 范围 / 触发条件

- 触发条件：新增或修改投注订单创建、订单列表、订单详情、订单取消、dashboard 最近订单。
- 范围：后端订单领域模型、内存订单仓储、订单 API、玩法规则引擎复用、彩种配置校验、前端订单页面。

### 2. 签名

- `GET /api/admin/orders`
- `GET /api/admin/orders/{id}`
- `POST /api/admin/orders`
- `PATCH /api/admin/orders/{id}/cancel`
- `GET /api/admin/dashboard` 的 `recentOrders` 和今日订单指标读取订单仓储。

### 3. 契约

所有接口继续使用统一 API 信封。订单金额字段必须使用最小货币单位整数，不使用浮点数。

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
| 单注金额小于等于 0 | HTTP 400，返回 `unit amount must be greater than zero` |
| 彩种停售 | HTTP 400，返回 `lottery is not on sale` |
| 玩法号码类型与彩种号码类型不匹配 | HTTP 400，返回 `rule code does not match lottery number type` |
| 彩种未启用玩法分类 | HTTP 400，返回 `lottery does not enable this play category` |
| 玩法选号无效 | HTTP 400，透传玩法规则引擎的校验错误 |
| 订单金额溢出 | HTTP 400，返回 `order amount is too large` |
| 查询或取消不存在订单 | HTTP 404，返回订单不存在 |
| 取消非待开奖订单 | HTTP 400，返回 `only pending draw orders can be cancelled` |

### 5. Good / Base / Bad Cases

- Good：`fc3d` 创建 `threeDirect` 订单，选号 `247`、单注 `200` 分，后端返回 `stakeCount=1`、`amountMinor=200`、`expandedBets=["247"]`。
- Good：创建订单后重新请求 `/api/admin/dashboard`，`recentOrders` 包含该订单，今日订单指标等于内存订单数量。
- Base：订单仓储当前是内存模式，服务重启后订单清空；这适合当前后台功能验证。
- Bad：前端传 `amountMinor` 给后端并由后端直接保存；订单金额必须由后端根据注数和单注金额计算。
- Bad：机器人购彩绕过订单接口直接写订单；后续机器人必须复用订单创建校验。

### 6. 必要测试

- 后端需要覆盖订单创建时按玩法规则引擎计算注数、金额和展开投注。
- 后端需要覆盖彩种未启用玩法分类时拒绝创建订单。
- 后端需要覆盖取消待开奖订单，以及重复取消被拒绝。
- 后端需要运行 `cargo fmt --check`、`cargo check`、`cargo test`。
- 前端需要运行 `npm run build`。
- 跨层联调需要创建订单、查询列表、取消订单，并在 dashboard 最近订单确认回流。

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
  "drawNumber": "247"
}
```

`platform` 和 `api` 开奖模式可以传空对象 `{}`，后端本地生成号码。`manual` 开奖模式必须传 `drawNumber`。

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
- Good：`manual-test` 创建期号后传 `{"drawNumber":"78942"}`，后端按 `fiveDigit` 校验并保存开奖结果。
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
  drawNumber: issue.drawNumber ?? '000',
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

所有接口继续使用统一 API 信封，金额字段必须使用最小货币单位整数。基础派奖倍数只用于本阶段链路验证，不代表真实生产赔率。

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
  "drawNumber": "023",
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
  "drawNumber": "023",
  "matchedBets": ["023"],
  "payoutMinor": 2000,
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

- Good：`fc3d` 期号开奖 `023`，同期开奖的 `threeDirect` 订单选号 `023`，结算后订单状态为 `won`，`matchedBets=["023"]`，`payoutMinor=2000`。
- Good：同期开奖的未命中订单结算后状态为 `lost`，`matchedBets=[]`，`payoutMinor=0`。
- Good：同一期号存在已取消订单时，取消订单不参与结算且状态保持 `cancelled`。
- Base：结算批次当前保存在内存订单仓储，服务重启后清空；这适合当前后台流程验证。
- Bad：路由函数直接判断中奖或修改订单状态；结算逻辑必须留在服务层/仓储层并复用玩法规则引擎。
- Bad：结算接口直接修改用户余额；本阶段没有真实资金流水，派奖结果不能代表资金已入账。

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

结算服务复用玩法规则引擎，拿 `matchedBets` 决定订单中奖状态和基础派奖结果。
