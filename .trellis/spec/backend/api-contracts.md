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
