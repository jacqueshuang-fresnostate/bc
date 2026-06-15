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

## 场景：彩种开售即时补齐期号

### 1. 范围 / 触发条件

- 触发条件：管理后台切换彩种销售状态，且彩种从 `saleEnabled=false` 变为 `saleEnabled=true`。
- 范围：`PATCH /api/admin/lotteries/{id}/sale`、开奖期号生成、调度配置读取、实时开盘事件。
- 适用开奖模式：`api` 和 `platform`。
- 不适用开奖模式：`manual`，手动开奖彩种的期号仍由运营显式维护。

### 2. 签名

- 彩种销售状态接口：`PATCH /api/admin/lotteries/{id}/sale`
- 请求体：

```json
{
  "saleEnabled": true
}
```

- 统一响应信封仍返回更新后的彩种对象。
- 内部配置来源：`DrawSchedulerRepository.config()`，使用 `futureIssueCount` 和 `saleCloseLeadSeconds`。
- 实时事件：`lottery.issue_opened`。

### 3. 契约

当彩种从停售切换为开售时：

- 如果 `drawMode=api`，后端必须按当前绑定开奖源的最新期号和开奖时间生成缺失的未来 `open` 期号。
- 如果 `drawMode=platform`，后端必须按本地开奖计划生成缺失的未来 `open` 期号。
- 如果 `drawMode=manual`，后端不得自动生成期号，避免跳过运营指定开奖号码流程。
- 未来期号缓冲只统计同彩种、`status=open` 且 `scheduledAt > now` 的期号。
- 需要补期时，补齐数量为 `futureIssueCount - existingFutureOpenIssueCount`。
- 生成的每个新期号都必须发布 `lottery.issue_opened`，让手机端首页、下注页和后台彩种控制台能够刷新当前期号。
- 如果补期失败，销售状态切换结果仍然保留；后端记录中文 warning，并在日志中保留具体错误信息。

### 4. 校验与错误矩阵

| 条件 | 预期行为 |
|------|----------|
| 彩种不存在 | 返回业务错误，不修改状态 |
| `saleEnabled=false -> true` 且 `drawMode=api` | 保存开售状态并按外部开奖源补齐未来期号 |
| `saleEnabled=false -> true` 且 `drawMode=platform` | 保存开售状态并按本地开奖计划补齐未来期号 |
| `saleEnabled=false -> true` 且 `drawMode=manual` | 只保存开售状态，不自动补齐期号 |
| 未来 `open` 期号数量已达到 `futureIssueCount` | 不重复生成期号 |
| 补期成功 | 每个新期号发布 `lottery.issue_opened` |
| 补期失败 | 保留销售状态，记录中文 warning 和具体错误 |

### 5. Good / Base / Bad Cases

- Good：运营把平台开奖彩种 `ssc60` 从停售改为开售后，接口立即生成下一期 `open` 期号，手机端首页收到 `issue_opened` 后刷新倒计时。
- Good：运营把 API 彩种 `fc3d` 从停售改为开售后，接口按 API68 最新期号生成下一期，不生成内部时间戳期号。
- Base：彩种已经开售，再次提交 `saleEnabled=true` 只保存状态，不额外触发补期。
- Bad：把 `closed` 期号计入未来缓冲，导致当前期封盘后没有下一期可投注。
- Bad：手动开奖彩种开售时自动生成期号，导致运营还没有指定开奖号码就开放投注。

### 6. 必要测试

- 后端需要覆盖 `api` 和 `platform` 返回需要补期，`manual` 不补期。
- 后端需要覆盖平台开奖彩种开售后生成未来 `open` 期号。
- 后端需要运行 `cargo fmt --check`、`cargo check` 和 `cargo test`。
- 修改实时事件发布时，需要补充或保留 `lottery.issue_opened` 的消费契约。

### 7. Wrong vs Correct

#### 错误

```rust
matches!(draw_mode, DrawMode::Api)
```

这个写法只覆盖 API 彩种，平台开奖彩种开售后必须等待调度下一轮，调度未启用时会表现为不会自动更新期号。

#### 正确

```rust
matches!(draw_mode, DrawMode::Api | DrawMode::Platform)
```

API 彩种和平台开奖彩种都能在开售时立即补齐可销售期号，手动开奖彩种仍保留人工维护流程。

---

## 场景：平台开奖期号格式配置

### 1. 范围 / 触发条件

- 触发条件：后台创建或更新平台开奖彩种，或开奖调度为平台开奖彩种生成下一期。
- 范围：`LotteryKind.issueFormat`、`lotteries.issue_format`、开奖期号生成器、后台彩种管理表单。
- 适用开奖模式：`platform`。
- 不适用开奖模式：`api`，API 彩种必须继续按开奖源最新期号顺延。

### 2. 契约

- 彩种对象必须返回 `issueFormat`，默认 `{date}{seq4}`，实际期号形如 `202606130001`。
- 平台开奖生成期号时按 `issueFormat` 渲染计划开奖时间；`{seq4}` 按开奖日期每日从 `0001` 递增。
- API 开奖有开奖源锚点时必须使用外部最新期号顺延，不能使用 `issueFormat`。
- 支持变量：`{yyyy}`、`{yy}`、`{MM}`、`{dd}`、`{HH}`、`{mm}`、`{ss}`、`{date}`、`{time}`、`{timestamp}`、`{seq4}`。
- 保存彩种时需要校验模板至少包含一个日期或时间变量，生成结果只能包含字母、数字、短横线或下划线，且不能超过 64 个字符。

### 3. 必要测试

- 后端需要覆盖平台开奖使用自定义 `issueFormat` 生成期号。
- 后端需要保留 API 开奖按开奖源期号顺延的测试，防止被平台模板影响。
- 后台构建需要确认彩种新增/编辑表单提交 `issueFormat`。

---

## 场景：手机端实时事件接口

### 1. 范围 / 触发条件

- 触发条件：手机端需要实时刷新开奖、封盘、开盘、用户资金、订单状态和公共聊天大厅。
- 范围：`GET /api/user/realtime` WebSocket、后端实时事件信封、公开事件与用户私有事件过滤、聊天大厅公开事件、手机端事件归一化。

### 2. 签名

- 手机端实时事件：`GET /api/user/realtime`
- 后台实时事件：`GET /api/admin/realtime?token=<管理员登录 token>`
- 聊天大厅消息列表：`GET /api/user/chat-hall/messages`
- 发送聊天大厅消息：`POST /api/user/chat-hall/messages`
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
- `chat_hall.message_created`：公共聊天大厅新增消息，`data.message` 包含 `id`、`userId`、`username`、`avatarUrl`、`content`、`messageType`、`payload`、`createdAt`，所有在线手机端连接都可收到。
- `system.heartbeat`：连接心跳。

用户私有事件：

- `user.balance_changed`：余额变化，必须只发送给 `data.userId` 对应用户连接。
- `user.order_changed`：注单变化，必须只发送给订单所属用户。
- `user.recharge_changed`：充值订单变化，必须只发送给充值订单所属用户。
- `user.withdrawal_changed`：提现订单变化，必须只发送给提现订单所属用户。

客服实时事件：

- `support.message_created`：客服会话新增消息，`data` 包含 `conversationId`、`userId`、`conversation` 和 `message`；`message` 需要携带 `messageType`、`content` 和可选 `imageUrl`，方便后台与手机端按文本或图片渲染。该事件必须同时发布给会话所属用户和后台客服连接，不允许发给匿名用户或其他普通用户。
- `support.conversation_updated`：客服会话状态、优先级、分配客服或用户侧已读状态变化，`data` 包含 `conversationId`、`userId` 和 `conversation`。后台修改状态、优先级或分配客服时必须同时发布给会话所属用户和后台客服连接，手机端客服页需要据此刷新“客服已接入”和会话状态；用户调用已读接口只发布给会话所属用户，避免后台客服列表出现无意义刷新。
- `support.conversation_deleted`：后台删除已解决客服会话，`data` 只包含 `conversationId` 和 `userId`。该事件必须同时发布给会话所属用户和后台客服连接，客户端收到后移除本地会话并刷新未读状态。
- 后台连接使用 `/api/admin/realtime?token=...`，因为浏览器 WebSocket 不能设置 `Authorization` 头；后端必须用查询参数 token 校验管理员会话，并要求具备 `customerService` 权限。

聊天大厅接口：

- `GET /api/user/chat-hall/messages` 返回最近大厅消息，消息字段使用 `camelCase`，按发送时间正序展示，并携带发送人的 `avatarUrl`；历史消息头像为空时后端需要用用户表当前头像兜底。
- `POST /api/user/chat-hall/messages` 请求体为 `{ "content": "..." }`，后端修剪首尾空白，空内容返回业务错误，超过 500 个字符返回业务错误。
- 聊天大厅消息必须写入 `chat_hall_messages` 表；`avatar_url` 保存发送人头像快照，用户更新头像后需要同步刷新该用户历史消息的头像快照。运行期只保留最近 200 条历史，接口返回最近 100 条。
- 聊天大厅是所有登录用户可进入的公共大厅，不使用客服会话表，不允许把公共大厅消息写入 `support_conversations` 或 `support_messages`。

### 4. 校验与错误矩阵

| 条件 | 预期行为 |
|------|----------|
| 未携带 token 建立连接 | 允许连接，但只能接收公开事件 |
| 携带合法用户 token | 允许连接，可接收公开事件和本人私有事件 |
| 携带非法用户 token | 握手返回未授权错误 |
| 事件受众为其他用户 | 当前连接不得收到该事件 |
| 客服消息新增 | 当前用户和后台客服连接收到 `support.message_created`，其他用户或匿名连接收不到 |
| 客服状态或分配变更 | 当前用户和后台客服连接收到 `support.conversation_updated`，其他用户或匿名连接收不到 |
| 客服已解决会话被删除 | 当前用户和后台客服连接收到 `support.conversation_deleted`，其他用户或匿名连接收不到 |
| 用户标记客服会话已读 | 当前用户收到 `support.conversation_updated`，后台客服连接和其他用户收不到 |
| 聊天大厅消息新增 | 所有在线手机端连接收到 `chat_hall.message_created` |
| 聊天大厅发送空内容 | 返回业务错误，不保存、不广播 |
| 聊天大厅发送超过 500 字 | 返回业务错误，不保存、不广播 |
| 后台实时连接缺少 token 或 token 无效 | 握手返回未授权错误 |
| 后台实时连接缺少客服权限 | 握手返回权限不足 |
| 客户端消费过慢 | 后端记录中文 warning，跳过过旧事件，不影响主业务 |

### 5. Good / Base / Bad Cases

- Good：自动开奖产生 `lottery.draw_result`，手机端首页和下注页同步刷新。
- Good：用户下注扣款后只给该用户推送 `user.balance_changed` 和 `user.order_changed`。
- Good：客服直充创建会话、用户继续发消息或后台回复后，用户客服页和后台客服页都能通过 `support.message_created` 实时刷新。
- Good：后台分配客服或保存会话状态后，用户客服页通过 `support.conversation_updated` 实时刷新接入客服和状态。
- Good：后台删除已解决会话后，后台会话列表和用户端客服未读缓存通过 `support.conversation_deleted` 刷新。
- Good：客服确认充值后，充值页通过 `user.recharge_changed` 刷新充值记录，通过 `user.balance_changed` 刷新余额。
- Good：用户在聊天大厅发送“大家好”，后端保存到 `chat_hall_messages` 并发布 `chat_hall.message_created`，其他在线用户无需刷新即可看到。
- Base：匿名用户仍可通过实时连接获取开奖和开盘状态。
- Bad：手机端继续连接 `/ws/lottery`。
- Bad：把 `user.balance_changed` 广播给所有在线连接。
- Bad：把客服消息作为公开事件广播，导致匿名连接或其他用户收到会话内容。
- Bad：把聊天大厅消息写入客服会话，导致公共聊天和客服工单混在一起。

### 6. 必要测试

- 后端需要运行 `cargo check` 和 `cargo test`。
- 后端需要覆盖客服消息事件结构和管理员实时受众过滤。
- 后端需要覆盖客服会话更新事件结构，以及用户在 `pending` 会话中回复后会话重新变为 `open`。
- 后端需要覆盖聊天大厅消息事件结构、空内容校验和最近历史上限。
- 手机端需要运行 `npm run build`，确认 WebSocket 事件归一化类型可编译。
- 管理后台需要运行 `npm run build`，确认后台实时事件归一化和客服页消费类型可编译。
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

## 场景：客服新消息 Telegram 提醒

### 1. 范围 / 触发条件

- 触发条件：用户创建客服直充会话，或在用户端继续回复自己的客服会话。
- 范围：`support_messages` 落库、`support.message_created` 实时事件、后台系统设置、Telegram Bot API 外部提醒。
- 不触发条件：后台客服回复用户、后台修改会话状态、系统消息。

### 2. 签名

后台系统设置键：

- `support_telegram_notification_enabled`：是否开启 Telegram 提醒，默认 `false`。
- `support_telegram_bot_token`：Telegram Bot Token，默认 `未配置`。
- `support_telegram_chat_id`：Telegram 接收提醒的 Chat ID、群组 ID 或频道用户名，默认 `未配置`。

Telegram 请求：

```json
{
  "chat_id": "-1001234567890",
  "text": "新的客服消息提醒\n会话：CS-10001\n用户：demo（U10001）\n主题：充值咨询\n时间：2026-06-09 18:00:00\n内容：请帮我确认充值凭证",
  "disable_web_page_preview": true
}
```

### 3. 契约

- 客服消息必须先通过当前 `SupportRepository` 保存成功，再发布 WebSocket 事件，再异步尝试发送 Telegram 提醒。
- Telegram 提醒只针对 `SupportMessageAuthor::User`，避免后台回复再次触发外部提醒。
- 配置未开启时不得请求 Telegram。
- 配置开启但 Bot Token 或 Chat ID 为空、`未配置`、`请配置`、`please-configure` 时，只记录中文 warning，不影响用户接口响应。
- Telegram 请求失败、超时或返回非 2xx 时，只记录中文 warning，不回滚客服消息、不影响 WebSocket 推送。
- 日志 message 必须为中文，结构化字段可包含 `conversation_id`、`user_id`、`message_id` 和原始第三方错误详情，但不能输出 Bot Token。

### 4. 校验与错误矩阵

| 条件 | 预期行为 |
|------|----------|
| 用户发送客服消息且开关关闭 | 保存消息并发布实时事件，不请求 Telegram |
| 用户发送客服消息且配置完整 | 保存消息、发布实时事件、异步发送 Telegram 文本提醒 |
| 用户发送客服消息但 Token 未配置 | 保存消息并发布实时事件，记录中文 warning |
| Telegram 返回失败 | 保存消息并发布实时事件，记录中文 warning 和第三方错误详情 |
| 后台客服回复消息 | 保存消息并发布实时事件，不触发 Telegram |

### 5. Good / Base / Bad Cases

- Good：客服直充用户上传凭证后，后台 WebSocket 收到 `support.message_created`，Telegram 群也收到包含会话、用户、主题和内容摘要的提醒。
- Base：未配置 Telegram 时，客服流程保持原有 HTTP + WebSocket 行为。
- Bad：把 Telegram 请求放在消息落库前；第三方失败会导致用户消息无法保存。
- Bad：把 Bot Token 写入日志或提交到代码。

### 6. 必要测试

- 后端需要覆盖 Telegram 配置解析、只提醒用户消息、提醒文本包含会话上下文。
- 后端需要运行 `cargo fmt --check`、`cargo check` 和定向 `support_telegram` 测试。
- 后台需要运行 `npm run build`，确认系统设置分组和开关下拉类型可编译。

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
      "oddsBasisPoints": 104000,
      "positionSelectLimits": [
        { "positionKey": "hundreds", "maxSelectCount": 7 }
      ]
    },
    {
      "ruleCode": "threeGroupThree",
      "enabled": true,
      "oddsBasisPoints": 52000
    }
  ]
}
```

`playConfigs` 是每个彩种的单玩法配置，`oddsBasisPoints` 使用整数基点赔率，`10000` 表示 `1.00 倍`，`104000` 表示 `10.40 倍`。`positionSelectLimits` 是单玩法位置选号上限，数组为空或缺失表示不限制；每项使用玩法位置 key，例如 `fiveFrontDirect` 支持 `first/second/third`，`threeDirect` 支持 `hundreds/tens/ones`，复式玩法支持 `numbers`，胆拖玩法支持 `banker/drag`，大小单双支持 `tens/ones`。后端保存彩种时会按 `numberType` 补齐该号码类型下所有玩法，并根据启用玩法反推 `playCategories`，避免粗分类和单玩法配置漂移。

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
| 单玩法位置上限 key 不属于当前玩法 | HTTP 400，返回 `play position select limit key is not allowed for this rule` |
| 单玩法位置上限小于等于 0 | HTTP 400，返回 `play position select limit must be greater than zero` |
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

## 场景：后台订单列表分页

### 1. 范围 / 触发条件

- 触发条件：后台订单管理展示投注订单列表，或彩种控制台需要读取订单用于控单信息展示。
- 范围：`GET /api/admin/orders`、后台订单 API client、`useOrders` 和订单管理页面。

### 2. 签名

- `GET /api/admin/orders?page=1&pageSize=20`
- 可选参数：`includeRobotData=true`
- 统一响应信封中的 `data` 字段为分页结构：

```json
{
  "items": [],
  "totalCount": 0,
  "page": 1,
  "pageSize": 20,
  "totalPages": 0
}
```

### 3. 契约

- 后台订单管理必须显式传入 `page` 和 `pageSize`。
- 后端默认过滤合买机器人账户订单；只有 `includeRobotData=true` 时才返回机器人订单。
- 不传 `page/pageSize` 时，后端仍返回分页结构，但 `items` 包含过滤后的完整订单列表，供彩种控制台继续一次读取控单所需订单。
- 分页字段使用 `camelCase`，前端类型必须与后端 `FinancePage<OrderDetail>` 结构一致。

### 4. 校验与错误矩阵

| 条件 | 预期行为 |
|------|----------|
| 未传分页参数 | 返回完整订单页，`page=1`，`pageSize` 为当前总数或 1 |
| `pageSize` 小于 1 | 按 1 处理 |
| `page` 超过最大页 | 返回最后一页 |
| 无订单 | 返回空 `items`，`totalCount=0`，`totalPages=0` |
| 未传 `includeRobotData` | 过滤机器人订单 |
| `includeRobotData=true` | 纳入机器人订单 |

### 5. Good / Base / Bad Cases

- Good：订单管理页请求 `GET /api/admin/orders?page=1&pageSize=20`，表格展示第一页订单，总数来自 `totalCount`。
- Good：运营打开“显示机器人数据”后，请求带 `includeRobotData=true`，并把页码重置为第 1 页。
- Base：彩种控制台请求 `GET /api/admin/orders` 不传分页参数，仍能通过 `data.items` 读取全部可见订单。
- Bad：前端继续把 `GET /api/admin/orders` 当成 `OrderDetail[]` 消费，会导致订单管理和彩种控制台拿不到列表。

### 6. 必要测试

- 后端需要运行 `cargo check`。
- 管理后台需要运行 `npm run build`。
- 修改分页控件时，确认财务管理、合买管理和订单管理均能编译。

### 7. Wrong vs Correct

#### 错误

```ts
const orders = await fetchOrders()
setOrders(orders)
```

这个写法把后端分页响应当成数组，会丢失 `totalCount`、`page` 和 `totalPages`，也会破坏彩种控制台读取 `items` 的路径。

#### 正确

```ts
const orderPage = await fetchOrders(signal, { page: 1, pageSize: 20 })
setOrders(orderPage.items)
```

前端必须消费分页信封，并从 `items` 中读取当前页订单。

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
      "oddsBasisPoints": 104000,
      "positionSelectLimits": [
        { "positionKey": "hundreds", "maxSelectCount": 7 }
      ]
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
- 配置了 `positionSelectLimits`：下单时后端必须按位置 key 校验对应选号数量，超过上限时拒绝订单；未配置的位置不限制。

结算派奖必须使用订单上的赔率快照，不能重新读取当前彩种赔率。派奖公式：

```text
命中投注数 × 单注金额 × oddsBasisPoints / 10000
```

### 4. 校验与错误矩阵

| 条件 | 预期行为 |
|------|----------|
| 彩种提交了不属于当前号码类型的玩法 | HTTP 400，返回玩法号码类型错误 |
| 彩种提交了小于等于 0 的赔率 | HTTP 400，返回 `play odds basis points must be greater than zero` |
| 彩种提交了不属于玩法的位置上限 key | HTTP 400，返回 `play position select limit key is not allowed for this rule` |
| 彩种提交了小于等于 0 的位置上限 | HTTP 400，返回 `play position select limit must be greater than zero` |
| 彩种保存后没有任何启用玩法 | HTTP 400，返回 `at least one play category is required` |
| 订单玩法没有配置 | HTTP 400，返回 `lottery does not configure this play rule` |
| 订单玩法被停用 | HTTP 400，返回 `lottery does not enable this play rule` |
| 订单某位置选号超过上限 | HTTP 400，返回 `{位置}最多选择 N 个号码` |
| 结算中奖订单赔率快照小于等于 0 | HTTP 400，返回派奖金额或赔率错误 |

### 5. Good / Base / Bad Cases

- Good：`fc3d.threeDirect` 设置为 `104000`，创建订单后订单响应包含 `oddsBasisPoints=104000`。
- Good：管理员随后把 `fc3d.threeDirect` 改成 `98000`，旧订单结算仍按 `104000` 派奖。
- Good：`fiveFrontDirect` 的 `first` 配置 `maxSelectCount=7` 后，手机端第一位最多选择 7 个数字，第二位和第三位未配置时不限制。
- Good：绕过手机端直接提交第一位 8 个数字时，后端订单报价返回 `{位置}最多选择 7 个号码`。
- Good：3 位页面只展示 3 位玩法和 3 位彩种，5 位页面只展示 5 位玩法和 5 位彩种。
- Bad：结算时读取当前彩种赔率；这会导致历史订单派奖被后续调价影响。
- Bad：前端用小数浮点提交赔率，例如 `10.4`；接口必须提交 `104000`。
- Bad：只在手机端限制选号数量，后端订单报价不校验；这会被直接调用 API 绕过。

### 6. 必要测试

- 后端需要覆盖彩种保存后补齐对应号码类型的所有 `playConfigs`，并保留合法 `positionSelectLimits`。
- 后端需要覆盖订单创建保存赔率快照，并拒绝停用玩法。
- 后端需要覆盖订单某位置选号超过 `positionSelectLimits` 时被拒绝。
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

- `GET /api/admin/orders?includeRobotData=false`
- `GET /api/admin/orders/{id}`
- `POST /api/admin/orders`
- `PATCH /api/admin/orders/{id}/cancel`
- `GET /api/admin/dashboard` 的 `recentOrders` 和今日订单指标读取订单仓储。

### 3. 契约

所有接口继续使用统一 API 信封。订单金额字段必须使用最小货币单位整数，不使用浮点数。

后台订单列表不传 `includeRobotData` 时等同于 `false`，默认排除系统机器人账户订单；管理后台打开“显示机器人数据”开关后传入 `includeRobotData=true`，才返回机器人发起或机器人账户关联的订单。

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
| 订单列表未传 `includeRobotData` | 默认过滤系统机器人账户订单 |
| 订单列表传 `includeRobotData=true` | 返回真实用户订单和系统机器人账户订单 |

### 5. Good / Base / Bad Cases

- Good：`fc3d` 创建 `threeDirect` 订单，选号 `247`、单注 `200` 分，后端返回 `stakeCount=1`、`amountMinor=200`、`oddsBasisPoints` 和 `expandedBets=["247"]`。
- Good：订单创建前先创建 `fc3d` 的 open 期号 `2026155`，订单请求使用同一期号才能成功。
- Good：创建订单后重新请求 `/api/admin/dashboard`，`recentOrders` 包含该订单，今日订单指标等于内存订单数量。
- Good：后台订单管理默认不展示机器人账户订单，打开“显示机器人数据”后才展示。
- Base：订单仓储当前是内存模式，服务重启后订单清空；这适合当前后台功能验证。
- Bad：前端手工输入一个不存在的期号仍然提交订单；订单必须从 open 期号中选择，后端也必须再次校验。
- Bad：前端传 `amountMinor` 给后端并由后端直接保存；订单金额必须由后端根据注数和单注金额计算。
- Bad：机器人购彩绕过订单接口直接写订单；后续机器人必须复用订单创建校验。

### 6. 必要测试

- 后端需要覆盖订单创建时按玩法规则引擎计算注数、金额和展开投注。
- 后端需要覆盖 open 期号允许投注，closed/drawn/cancelled 期号拒绝投注。
- 后端需要覆盖彩种未配置或停用单玩法时拒绝创建订单。
- 后端需要覆盖取消待开奖订单，以及重复取消被拒绝。
- 后端需要覆盖订单列表默认过滤机器人账户订单，打开开关后返回机器人账户订单。
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
- `GET /api/user/bet/orders`：读取当前登录用户自己的投注订单、当前用户参与且已满单成单的合买投注订单，以及当前用户已认购但尚未生成真实订单的合买记录；支持 `page/pageSize` 分页和 `view=orders|groupBuy` 分视图过滤。
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
      "positionSelectLimits": [{ "positionKey": "first", "maxSelectCount": 7 }],
      "odds": "9.50",
      "unitAmount": "2.00"
    }
  ]
}
```

`positionSelectLimits` 会从彩种玩法配置透传到手机端下注页。手机端必须按 `positionKey` 限制对应位置的选号数量；数组为空或缺失时表示不限制。后端批量下单仍会复用订单服务再次校验，不能只依赖前端禁用按钮。

下注页配置选择当前期号时，`round.status=selling` 只能用于 `status=open` 且 `saleStopAt` 仍晚于当前时间的期号。若期号仍是 `open` 但已经超过 `saleStopAt`，后端必须把它作为 `round.status=opening` 返回，保留 `issue/scheduledDrawAt/saleStopAt` 供页面展示“开奖中”，并触发手机端开盘轮询。这样可以覆盖调度短暂滞后、WebSocket 丢失或接口刷新落后的边界情况。

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

当 `orderSource=groupBuy` 且当前登录用户参与了对应合买计划时，用户注单列表必须额外返回 `groupBuyPlanId`、`groupBuyPlanStatus`、`participationAmountMinor` 和 `participationShareCount`，其中 `groupBuyPlanId` 用于手机端注单详情懒加载合买计划参与人列表，`groupBuyPlanStatus` 用于展示合买计划状态，`participationAmountMinor` 表示当前用户在该合买订单或合买计划中的实际认购金额，`participationShareCount` 表示当前用户累计认购份数；如果同一用户对同一合买计划有多条参与记录，需要按参与记录累加。已成单合买中奖后还必须返回 `participationPayoutMinor`，表示当前用户按参与比例实际分到的派奖金额；该值优先来自真实 `payoutCredit` 资金流水，历史缺失流水时可按合买参与比例和最后一名承接余数规则计算展示兜底。普通独立订单不返回这些合买参与字段，手机端不能把合买真实订单的 `amountMinor` 或整单 `payoutMinor` 当成当前用户实际参与金额或个人中奖金额展示。

用户注单列表的归属口径必须同时覆盖两类数据：

- `order.userId` 等于当前登录用户的独立投注订单。
- 当前登录用户出现在合买计划 `participants` 中，且该计划已经通过 `orderId` 关联真实投注订单时，对应 `orderSource=groupBuy` 的订单。

当前登录用户出现在合买计划 `participants` 中，但该计划尚未通过 `orderId` 关联真实投注订单时，用户注单列表也必须返回一条特殊合买认购记录：

- `orderSource=groupBuy`。
- `groupBuyPendingPlan=true`。
- `status` 在手机端展示为“合买认购中”，结果展示为“未成单”。
- `createdAt` 使用当前用户最后一次认购时间参与注单时间线排序。
- 金额展示必须使用 `participationAmountMinor`，数量展示必须使用 `participationShareCount`。

用户注单列表的 `view` 查询参数用于手机端 `/orders` 页面分组：

- 不传 `view` 时返回独立下注、已成单合买订单和未成单合买认购的混合列表，保留旧调用兼容。
- `view=orders` 只返回真实已下单注单，包括独立下注订单和已满单成单的合买投注订单，不包含 `GB-` 开头的未成单合买认购映射记录。
- `view=groupBuy` 只返回未形成真实投注订单的合买认购记录，包括未满单、待成单或已取消但当前用户曾认购的合买计划。
- 分页必须在 `view` 过滤之后执行，保证“我的注单”和“我的合买”两个 Tab 各自拥有稳定页码。

“我的合买”接口仍然负责展示计划进度和大厅/我的合买视图；“我的注单”负责展示用户资金已经参与的下注或认购时间线，两者不是互斥关系。

手机端注单记录必须按该字段展示“独立下单”或“合买下单”，不能只用订单号、资金流水或旧系统 `source_name` 猜测。

下注页“加入购彩篮”是把当前草稿加入本地待提交购物篮，不是跨彩种组合投注。前端加入和提交时都必须校验购彩篮内所有单据属于同一个彩种和同一期号。

### 4. 校验与错误矩阵

| 条件 | 预期行为 |
|------|----------|
| 未登录读取下注配置或下单 | HTTP 401，返回未授权 |
| 下注页彩种不存在 | HTTP 404，返回彩种不存在 |
| 下注页彩种停售 | HTTP 400，返回 `彩种已停售` |
| `open` 期号已过封盘时间 | 下注配置返回 `round.status=opening`，手机端不允许继续投注并轮询下一期 |
| 批量下单 `orders` 为空 | HTTP 400，返回 `请先选择投注内容` |
| 批量下单超过 50 笔 | HTTP 400，返回 `一次最多提交 50 笔投注` |
| 购彩篮混入不同彩种 | 手机端拦截，提示 `购彩篮只能提交同一个彩种的投注`，不请求后端 |
| 购彩篮混入旧期号 | 手机端拦截，提示清空购彩篮后重新选择，不请求后端 |
| 请求期号不存在或非 `open` | HTTP 404/400，沿用订单期号校验错误 |
| 玩法、号码类型、选号或赔率无效 | HTTP 400，沿用订单和玩法规则引擎错误 |
| 选号超过玩法位置上限 | HTTP 400，返回 `{位置}最多选择 N 个号码` |
| 当前用户余额不足 | HTTP 400/409，沿用财务账户余额校验错误 |
| 扣款失败 | 回滚本次未入账订单，返回财务错误 |
| 用户参与合买且计划已成单 | `GET /api/user/bet/orders` 返回对应 `orderSource=groupBuy` 注单 |
| 用户参与合买且计划已成单 | 返回 `groupBuyPlanId`，手机端注单详情可据此拉取参与人列表 |
| 用户参与合买且计划已成单 | 返回 `participationAmountMinor` 作为当前用户参与金额 |
| 用户参与合买且中奖已派奖 | 返回 `participationPayoutMinor` 作为当前用户个人派奖金额 |
| 用户参与合买但计划未满单 | 返回 `groupBuyPendingPlan=true` 的合买认购记录，手机端显示“合买认购中/未成单” |

### 5. Good / Base / Bad Cases

- Good：进入销售中的 `txffc` 下注页，读取到 `round.status=selling`、最近开奖和所有已启用玩法赔率。
- Good：前端提交 `positions`、`numbers`、`bankerNumbers/dragNumbers` 或 `bigSmallOddEven`，后端复用订单规则计算注数和扣款。
- Good：直选组合前端使用 `positionGridKind=direct_combination` 多选数字，并按排列数显示注数；后端仍以 `selection.numbers` 展开排列投注。
- Good：玩法配置返回 `positionSelectLimits=[{positionKey:"first",maxSelectCount:7}]` 时，手机端只限制第一位最多 7 个数字，未配置的其它位置保持不限制。
- Good：用户独立下注后注单记录展示 `orderSource=direct` 和“独立下单”；用户作为参与人认购的合买满单成单后，即使真实订单 `userId` 是发起人，注单记录也展示 `orderSource=groupBuy` 和“合买下单”。
- Good：用户只认购 30 元合买份额，即使真实合买订单总额是 200 元，手机端注单记录也展示“参与金额 30 元”。
- Good：用户认购的合买还未满单时，“我的注单”也能看到一条“合买认购中”的记录，并展示认购份数和参与金额。
- Good：合买真实订单总派奖 19 元时，认购 300 元和 20 元的参与人看到的中奖金额必须按各自份额分配，不能都显示 19 元。
- Good：用户切换彩种或期号变化后，购彩篮不能继续提交旧彩种或旧期号单据。
- Base：没有 open 期号时，下注页返回 `round.status=opening`，手机端轮询下一期，不允许提交。
- Base：只有已过封盘时间的 open 期号时，下注页返回该期号但状态为 `opening`，手机端展示“开奖中”并轮询下一期。
- Bad：手机端继续把 `play_code/numbers/amount` 发到旧 `/bet/place-batch`；该接口不是当前系统契约。
- Bad：用户端批量下单允许传 `userId`；这会让用户冒充他人下单。

### 6. 必要测试

- 后端运行 `cargo check --manifest-path backend/Cargo.toml`。
- 后端测试 `cargo test --manifest-path backend/Cargo.toml mobile_bet -- --nocapture`，覆盖当前期、最近开奖、已启用玩法和直选组合配置。
- 后端测试需要覆盖普通订单来源为 `direct`，合买满单生成订单来源为 `groupBuy`。
- 后端测试需要覆盖用户参与别人发起的合买计划后，满单生成的真实投注订单会出现在该用户注单列表。
- 后端测试需要覆盖用户参与未满单合买计划后，该计划会以 `groupBuyPendingPlan=true` 出现在用户注单列表。
- 后端测试需要覆盖合买注单返回当前用户 `participationAmountMinor`，多条参与记录需要累加。
- 后端测试需要覆盖合买注单返回当前用户 `participationPayoutMinor`，并确认有资金流水时优先按实际入账金额展示。
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

- `api68-fc3d`：`fc3d` 福彩 3D 默认使用 API68 全国彩接口，`lotCode=10041`，响应中按 `result.data[].preDrawIssue` 匹配后台期号，使用 `preDrawCode` 作为开奖号码。
- `api68-pl3`：`pl3` 体彩排列3默认使用 API68 全国彩接口，endpoint 为 `https://api.api68.com/QuanGuoCai/getLotteryInfo1.do`，`lotCode=10043`，不再复用福彩 3D 来源。
- `api68-pl5`：`pl5` 体彩排列5默认使用 API68 全国彩接口，endpoint 为 `https://api.api68.com/QuanGuoCai/getLotteryInfo.do`，`lotCode=10044`。
- `api68-au5`：`au5` 澳洲幸运5默认使用 API68 CQShiCai 单彩种接口，`lotCode=10010`，响应中按 `result.data.preDrawIssue` 匹配后台期号，使用 `preDrawCode` 作为英文逗号分隔开奖号码。
- API68 批量接入彩种：`bjpk10`、`tjssc`、`xjssc`、`gd11x5`、`au10`、`au20`、`jx11x5`、`js11x5`、`ah11x5`、`sh11x5`、`ln11x5`、`hb11x5`、`gx11x5`、`jl11x5`、`nmg11x5`、`zj11x5`。这些来源按彩种分别绑定 API68 的 PKS、CQShiCai、ElevenFive 或 LuckTwenty endpoint；已停用的快3和北京快乐8不应重新出现在默认来源中。
- `kj-txffc`：`txffc` 腾讯分分彩默认使用 KJAPI 接口，`lotKey=txffc`，响应中按 `result.data.preDrawIssue` 匹配后台期号，使用 `preDrawCode` 作为英文逗号分隔开奖号码；生成下一期时优先读取 `result.data.drawIssue` 和 `result.data.drawTime`。
- `preDrawCode` 必须继续经过后端开奖号码校验，保存和返回仍统一为英文逗号分隔格式。
- API68 解析器必须兼容 `result.data` 为数组或单对象两种形态；单对象接口还应读取 `drawIssue` 和 `drawTime` 作为下一期锚点。
- 暂未配置外部源的 API 彩种仍保留本地生成器占位能力，仅用于当前内存演示阶段；生产接入时需要显式配置来源。

开奖源响应：

```json
{
  "id": "api68-fc3d",
  "name": "API68 福彩 3D",
  "mode": "api",
  "provider": "api68",
  "lotCode": "10041",
  "endpoint": "https://api.api68.com/QuanGuoCai/getLotteryInfoList.do",
  "editable": true,
  "reusableForLotteryIds": ["fc3d"]
}
```

保存开奖源请求：

```json
{
  "id": "api68-fc3d",
  "name": "API68 福彩 3D",
  "provider": "api68",
  "lotCode": "10041",
  "endpoint": "https://api.api68.com/QuanGuoCai/getLotteryInfoList.do",
  "reusableForLotteryIds": ["fc3d"]
}
```

`endpoint` 可为空；为空时后端按供应商写入默认 endpoint。福彩 3D 默认来源写入 `draw_sources` 表，endpoint 为 `https://api.api68.com/QuanGuoCai/getLotteryInfoList.do`；体彩排列3默认来源写入 `https://api.api68.com/QuanGuoCai/getLotteryInfo1.do`；体彩排列5默认来源写入 `https://api.api68.com/QuanGuoCai/getLotteryInfo.do`；澳洲幸运5默认来源写入 `draw_sources` 表，endpoint 为 `https://api.api68.com/CQShiCai/getBaseCQShiCai.do`；腾讯分分彩默认来源写入 `draw_sources` 表，endpoint 为 `https://kjapi.net/hall/hallajax/getLotteryInfo`。后续修改 endpoint 必须通过后台“开奖源配置”或开奖源 API 写入数据库，不通过环境变量覆盖。`platform` 来源也会出现在 `GET /draw-sources` 中，但 `editable=false`，不支持通过 API 源配置接口修改。

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
- Good：`pl3` 创建同一期号 `2026143` 后调用 `PATCH /draw` 传 `{}`，后端通过 `api68-pl3` 的 `lotCode=10043` 获取体彩排列3开奖结果。
- Good：`pl5` 创建体彩排列5期号后调用 `PATCH /draw` 传 `{}`，后端通过 `api68-pl5` 的 `lotCode=10044` 获取五位开奖号码。
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
- 跨层联调需要请求开奖源、保存复用彩种、创建 `fc3d/pl3/pl5` 期号、封盘、API 开奖、手动开奖，并在管理后台页面确认结果回显。

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

- `GET /api/admin/settlements?page=1&pageSize=20`
- `GET /api/admin/settlements/{id}`
- `POST /api/admin/settlements/draw-issues/{id}`
- `GET /api/admin/orders` 和 `GET /api/admin/orders/{id}` 的订单响应新增结算字段。
- `GET /api/admin/dashboard` 的 `recentOrders` 新增结算字段。

### 3. 契约

所有接口继续使用统一 API 信封，金额字段必须使用最小货币单位整数。派奖金额使用订单创建时保存的 `oddsBasisPoints` 赔率快照计算，不能在结算时重新读取当前彩种赔率。

结算批次列表返回分页结构：

```json
{
  "items": [],
  "totalCount": 0,
  "page": 1,
  "pageSize": 20,
  "totalPages": 0
}
```

`items` 中每一项为结算批次响应结构。

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
      "username": "demo_user",
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
| 结算批次列表 `page` 超过最大页 | 返回最后一页 |
| 结算批次列表为空 | 返回空 `items`，`totalCount=0`，`totalPages=0` |

### 5. Good / Base / Bad Cases

- Good：`fc3d` 期号开奖 `0,2,3`，同期开奖的 `threeDirect` 订单选号 `023`，订单赔率快照 `oddsBasisPoints=104000`，结算后订单状态为 `won`，`matchedBets=["023"]`，单注 `200` 分派发 `2080` 分。
- Good：同期开奖的未命中订单结算后状态为 `lost`，`matchedBets=[]`，`payoutMinor=0`。
- Good：同一期号存在已取消订单时，取消订单不参与结算且状态保持 `cancelled`。
- Good：计奖派奖页面请求 `GET /api/admin/settlements?page=1&pageSize=20`，表格展示第一页结算批次，总数来自 `totalCount`。
- Base：结算批次当前保存在内存订单仓储，服务重启后清空；这适合当前后台流程验证。
- Bad：路由函数直接判断中奖或修改订单状态；结算逻辑必须留在服务层/仓储层并复用玩法规则引擎。
- Bad：结算路由绕过资金服务直接修改用户余额；派奖入账必须通过资金仓储生成 `payoutCredit` 流水。
- Bad：前端继续把 `GET /api/admin/settlements` 当成 `SettlementRun[]` 消费；分页后必须读取 `data.items`。

### 6. 必要测试

- 后端需要覆盖中奖订单结算为 `won`，未中奖订单结算为 `lost`。
- 后端需要覆盖已取消订单不参与结算。
- 后端需要覆盖未开奖期号拒绝结算和同一期号重复结算拒绝。
- 后端需要运行 `cargo fmt --check`、`cargo check`、`cargo test`。
- 前端需要运行 `npm run build`。
- 跨层联调需要完成“创建订单 → 创建/开奖期号 → 执行计奖派奖 → 查询订单列表和结算批次”。
- 修改结算批次列表时，需要确认 `useSettlements` 消费分页信封，页面复用公共 `PageControls`。

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
  "username": "demo_user",
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
- `rechargeCredit`：充值入账，金额为正数。
- `rechargeRebateCredit`：充值返利入账，金额为正数。
- `withdrawalFreeze`：提现冻结，金额为负数。
- `withdrawalPayout`：提现打款，冻结资金出账。
- `withdrawalReject`：提现驳回解冻，金额为正数。
- `groupBuyDebit`：合买认购扣款，金额为负数。
- `groupBuyRefund`：合买退款，金额为正数。
- `redPacketDebit`：聊天大厅红包支出，金额为负数。
- `redPacketCredit`：聊天大厅红包领取入账，金额为正数。

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
2. 订单服务必须通过 `OrderRepository::create_with_debit` 或 `create_many_with_debit` 同时创建订单和写入 `orderDebit` 流水。
3. PostgreSQL 模式下订单表、订单运行序号、资金账户和资金流水必须使用同一个 SQLx 事务保存。
4. 批量下注任一订单校验或扣款失败时，整批订单都不创建、不扣款，不能留下部分生效订单。

订单取消资金流：

1. 取消订单必须通过 `OrderRepository::cancel_with_refund` 同时校验 `orderDebit`、标记订单 `cancelled` 并写入 `orderRefund` 流水。
2. PostgreSQL 模式下订单状态和退款流水必须使用同一个 SQLx 事务保存。
3. 同一订单重复退款必须拒绝或保持幂等，不能重复加钱。

结算派奖资金流：

1. 开奖结算必须通过 `OrderRepository::settle_with_payouts` 同时生成结算批次、回写订单状态、写入 `payoutCredit` 并标记合买计划结算状态。
2. 资金仓储只对 `isWinning=true` 且 `payoutMinor > 0` 的订单写入 `payoutCredit`。
3. `payoutCredit` 的 `referenceId` 使用结算批次和订单组合，避免重复入账。
4. PostgreSQL 模式下订单、资金和合买计划结算状态必须使用同一个 SQLx 事务保存。

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
| 批量下注中后续订单余额不足 | HTTP 400，整批订单不创建、不扣款 |
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
  "saleCloseLeadSeconds": 1
}
```

`now` 使用 `YYYY-MM-DD HH:mm:ss`。`saleCloseLeadSeconds` 可省略，默认 `1`，表示封盘时间为开奖前 1 秒。

`preview-generation` 和 `generate-batch` 请求体：

```json
{
  "lotteryId": "fc3d",
  "now": "2026-06-02 20:00:00",
  "count": 5,
  "saleCloseLeadSeconds": 1
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
7. 如果彩种绑定了外部 API 开奖源，后端必须先读取当前彩种独立绑定来源的最新 `preDrawIssue`，并用该数字期号递增生成未来期号；例如福彩 3D 使用 `api68-fc3d`，体彩排列3使用 `api68-pl3`，体彩排列5使用 `api68-pl5`，不能再让排列 3 复用福彩 3D 的期号锚点。
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
- Good：`pl3` 使用 `api68-pl3` 来源时，按体彩排列3接口返回的最新期号生成下一期。
- Good：`pl5` 使用 `api68-pl5` 来源时，按体彩排列5接口返回的最新期号生成下一期，号码类型为 `fiveDigit`。
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
- 后端需要覆盖 API68 最新 `preDrawIssue` 驱动福彩 3D、体彩排列3和体彩排列5真实期号生成。
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
  "saleCloseLeadSeconds": 1
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
    "saleCloseLeadSeconds": 1
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
5. 每轮必须先执行封盘快阶段，只处理到期 `open -> closed` 和封盘流单退款；封盘阶段不能等待 API 开奖、结算或机器人。
6. 封盘快阶段完成后，再扫描 `saleEnabled=true` 的彩种，确保每个彩种至少有 `config.futureIssueCount` 个未来可投注 `open` 期号。
7. 未来期号判断只统计同彩种、状态为 `open`，并且 `scheduledAt > now` 的期号；`closed` 期号已经封盘，不能当作下一期缓冲，否则封盘后不会立即开盘下一期。
8. 周期彩种补期必须沿用最新本地 `scheduledAt` 的固定节拍；调度晚执行时不能以当前调度时刻重新起步，避免 60 秒彩种的新期倒计时被压缩。
9. 补期继续调用 `generate_draw_issue_batch`，不在调度服务里重新实现开奖计划算法。
10. 封盘和补期完成后必须立即发布 `lottery.issue_closed` 与 `lottery.issue_opened`，不能等 API 开奖、订单结算、派奖入账或机器人执行完成后再统一发布。
11. 开奖结算慢阶段在开盘推送后执行，处理 API 开奖、平台开奖、订单结算、派奖入账和合买结算；`lottery.draw_result` 在慢阶段完成对应期号开奖后发布。
12. 补期完成后必须调用 `run_group_buy_robots` 执行已启用合买机器人；机器人执行不能放在补期前，否则刚补出的 open 期号无法被机器人使用。
13. 机器人执行产生的 `ledgerEntries` 要计入本轮调度 `ledgerEntryCount`，并通过实时事件推送用户余额变化；机器人产生的订单要推送用户订单变化。
14. `saleEnabled=false` 彩种不会自动补期，也不会被合买机器人发起计划，会记录为跳过彩种或机器人跳过项。
15. 调度周期成功或失败都要写入调度运行历史，页面通过状态接口读取历史，而不是解析日志。
16. 调度周期成功或失败都使用 `tracing` 结构化日志记录，不暴露原始请求体或敏感信息；成功日志中的统计字段必须使用中文键名，包括封盘耗时、补期耗时、开奖结算耗时、机器人耗时、机器人新增合买、机器人满单、机器人生成订单和机器人跳过项。
17. 后台调度循环使用固定节拍追赶执行，不能在本轮执行完成后再额外 sleep 一个完整周期；如果本轮耗时超过配置周期，需要记录中文 warning 并立即追赶下一轮。
18. API 开奖期号到期开奖前需要先按彩种读取开奖源最新期号；如果当前期号与最新期号都是纯数字，且 `最新期号 - 当前期号 > 5`，本轮自动开奖必须停止请求该旧期号开奖号码，只把期号写入 `skippedIssues`，原因说明“停止重试旧期号”。同一轮内同一彩种的最新期号查询需要缓存，避免多个旧期号重复请求开奖源。

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
| API 旧期号落后开奖源最新期号超过 5 期 | 不再请求旧期号开奖号码，写入跳过期号明细 |
| API 旧期号落后开奖源最新期号等于 5 期 | 仍按原开奖源逻辑请求开奖号码 |
| 调度未启用时查询状态 | HTTP 200，`enabled=false`，历史为空 |
| 最近运行超过 20 条 | 只保留最新 20 条，旧记录从内存仓储移除 |
| 状态仓储锁异常 | HTTP 500，返回统一错误信封 |

### 5. Good / Base / Bad Cases

- Good：服务启动后即使初始配置禁用，管理后台保存 `enabled=true` 后也会在不重启服务的情况下开始自动调度。
- Good：后台保存 `enabled=true` 和 `intervalSeconds=1` 后，本地启动服务会从数据库恢复配置，并自动为销售开启彩种补齐未来期号。
- Good：调度跑过一轮后，`GET /api/admin/draw-scheduler/status` 返回最新 `SCH...` 记录，管理后台“常驻调度”显示成功状态和运行摘要。
- Good：已有到期开奖期号时，单轮调度先执行封盘并补齐下一期，立即推送开盘，再继续处理开奖和结算。
- Good：已有期号刚到封盘时间但未到开奖时间时，单轮调度先把当前期转为 `closed`，再生成下一期 `open`，保证销售链路继续有可投注期号。
- Good：平台 60 秒彩种上一期为 `20:18:27`，即使调度到 `20:18:52` 才执行，下一期仍生成 `20:19:27`，不会生成 `20:19:52`。
- Good：WebSocket 事件顺序为 `lottery.issue_closed`、`lottery.issue_opened`、`lottery.draw_result`，新期开奖不被开奖结果和结算拖后。
- Good：销售中且开启合买的彩种在补出 open 期号后，同轮调度可以执行合买机器人并创建本期机器人合买。
- Good：API 彩种存在大量历史旧期号时，落后最新期号超过 5 期的旧期号只记录跳过原因，不再逐个请求旧期号开奖号码，避免拖慢平台开奖彩种。
- Base：默认关闭适合本地开发和测试，不会让后台循环干扰手动 API 冒烟。
- Bad：把 `closed` 期号算作未来缓冲；这会让当前期封盘后没有新的 `open` 期号可投注。
- Bad：把 `lottery.issue_opened` 放到 API 开奖、结算或机器人之后统一推送；这会让手机端收到新期时倒计时已经被慢阶段吃掉。
- Bad：在调度服务里复制一套封盘、开奖、结算或开奖计划计算逻辑；这些必须继续复用自动开奖阶段函数和 `generate_draw_issue_batch`。
- Bad：调度器直接手写合买机器人发单逻辑；机器人执行必须在 `group_buy_robot` 服务中复用合买和订单服务。
- Bad：管理后台为了显示调度状态去解析服务日志；页面必须调用 `GET /api/admin/draw-scheduler/status`。

### 6. 必要测试

- 后端需要覆盖调度默认关闭时仍创建后台任务，但不执行开奖调度。
- 后端需要覆盖后台保存 `enabled=true` 后，已启动后台任务无需重启即可执行。
- 后端需要覆盖默认配置写入数据库、数据库配置恢复和无效配置拒绝。
- 后端需要覆盖销售开启彩种自动补齐未来期号，销售关闭彩种跳过。
- 后端需要覆盖未来期号缓冲已满足时不重复生成。
- 后端需要覆盖到期期号先封盘、补齐未来期号并推送开盘，再执行开奖结算。
- 后端需要覆盖当前期到封盘时间后会生成下一期 `open` 期号，不能因为当前期 `closed` 仍未开奖就跳过补期。
- 后端需要覆盖调度晚执行时周期彩种仍保持固定开奖节拍。
- 后端需要覆盖实时事件顺序先发布封盘和开盘，再发布开奖结果。
- 后端需要覆盖 API 旧期号超过最新期号 5 期后停止重试，以及刚好相差 5 期时仍保持原重试逻辑。
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
  "agentUsername": "agent_alpha",
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
6. 后台用户列表、详情、创建、更新和状态变更响应在 `agentId` 外补充 `agentUsername`，用于用户维护页直接展示上级代理用户名；没有上级代理或代理账号不存在时返回 `null`。
7. 用户管理接口只需要 `users` 权限，不允许让用户管理页额外依赖需要 `rebates` 权限的邀请管理接口。
8. 本阶段不保存管理员密码，不提供真实登录、JWT、菜单拦截或权限鉴权。

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
  "description": "按彩种开盘时间模拟普通用户购彩",
  "groupBuyFillStrategy": "rhythm",
  "groupBuyFillBeforeDrawSeconds": 15,
  "deletable": true
}
```

字段契约：

1. `kind` 只允许 `groupBuy` 或 `purchase`。
2. `status` 只允许 `enabled`、`paused`、`disabled`。
3. `lotteryIds` 必须至少包含一个有效彩种 ID；后端保存时会去重并按稳定顺序返回。
4. `DashboardSummary.robots` 必须从 `RobotRepository` 读取，不允许 dashboard 使用独立静态机器人函数。
5. 机器人响应返回 `deletable` 字段，表示后台是否允许删除该配置；前端不能自行硬编码受保护 ID。
6. `DELETE /api/admin/robots/{id}` 只能删除普通机器人配置；核心内置机器人返回冲突错误，运营需要通过 `PATCH /api/admin/robots/{id}/status` 改为 `paused` 或 `disabled` 停止执行。
7. `groupBuyFillStrategy` 只允许 `rhythm` 或 `beforeDraw`，旧请求未传时默认 `rhythm`。
8. `groupBuyFillBeforeDrawSeconds` 表示开奖前多少秒触发补满，范围为 1 到 86400；只有 `groupBuyFillStrategy=beforeDraw` 时参与执行判断。
9. `POST /api/admin/robots/run` 只执行已启用的 `groupBuy` 机器人，不执行 `purchase` 机器人。
10. 合买机器人执行结果字段：
   - `now`：本轮执行时间，格式为 `YYYY-MM-DD HH:mm:ss`。
   - `createdPlans`：本轮新创建的合买计划。
   - `filledPlans`：本轮补满并关联订单的合买计划。
   - `createdOrders`：本轮由满单合买生成的真实投注订单。
   - `ledgerEntries`：本轮机器人自动授信、合买认购产生的资金流水。
   - `skippedItems`：本轮跳过项，每项包含 `robotId`、`robotName`、`lotteryId`、`issue` 和 `reason`。
11. 合买机器人计划 ID 必须按“机器人 ID + 彩种 ID + 期号”确定性生成，同一期重复执行不能重复创建计划。
12. 合买机器人必须使用当前合买链路：校验彩种开售、合买开启、open 期号、封盘时间、启用玩法和注数报价，再创建计划，并按配置策略补单；满单后成单并写入 `groupBuyDebit`。
13. 合买机器人使用系统账户 `U90001` 出资；发起计划或补单前余额不足时，后端必须先通过 `ManualAdjustment` 自动授信补余额并记录“机器人账户自动授信补余额”，随后再执行真实扣款，不允许出现未扣款计划。
14. 合买机器人必须扫描同彩种当前期非机器人发起的 `draft/open` 未满单计划；`G-ROBOT-` 开头的机器人计划不得被其他机器人交叉补单。
15. `rhythm` 策略使用封盘前 90 秒窗口：距离封盘 90-61 秒目标进度 40%，60-31 秒目标进度 60%，30-16 秒目标进度 80%，最后 15 秒才允许补到 100% 并触发满单成单；用户或后台发起的合买在封盘点到开奖前仍保留兜底补满能力。
16. `beforeDraw` 策略不按阶段追加认购，而是在距离开奖时间小于等于 `groupBuyFillBeforeDrawSeconds` 时一次性补满剩余金额；如果当前时间已经超过开奖时间则跳过。
17. 合买机器人每次补单都必须生成新的参与记录 ID，不允许复用同一个机器人参与记录一次性覆盖剩余金额。
18. 用户端合买计划响应必须隐藏 `G-ROBOT-` 开头的机器人计划真实发起人和机器人标题：`initiatorDisplay` 返回按计划 ID 稳定生成的普通会员展示名，`title` 返回“彩种 第期号期合买”，不得把 `U90001`、`agent_alpha`、机器人名称或“合买机器人”暴露给手机端；后台机器人执行结果和后台合买管理仍可保留真实配置用于运营排查。

### 4. 校验与错误矩阵

| 条件 | 预期行为 |
|------|----------|
| 机器人 ID 为空 | HTTP 400，返回 `robot id is required` |
| 机器人名称为空 | HTTP 400，返回 `robot name is required` |
| 机器人说明为空 | HTTP 400，返回 `robot description is required` |
| `lotteryIds` 为空 | HTTP 400，返回 `at least one robot lottery is required` |
| `lotteryIds` 包含不存在彩种 | HTTP 404，返回 `lottery ... not found for robot` |
| `groupBuyFillBeforeDrawSeconds` 为 0 或超过 86400 | HTTP 400，返回合买机器人开奖前补满秒数错误 |
| 创建重复机器人 ID | HTTP 409，返回重复机器人错误 |
| 更新路径 ID 与机器人 ID 不一致 | HTTP 400，返回 `path id must match robot id` |
| 查询、更新不存在机器人 | HTTP 404，返回机器人不存在 |
| 删除不存在机器人 | HTTP 404，返回机器人不存在 |
| 删除 `deletable=false` 的核心内置机器人 | HTTP 409，返回“内置机器人配置不能删除，请改为暂停或禁用” |
| 删除 `deletable=true` 的普通机器人 | HTTP 200，返回被删除的机器人配置，列表和 dashboard 刷新后不再展示 |
| 手动执行时系统机器人资金账户不存在 | HTTP 404，返回合买机器人资金账号不存在 |
| 手动执行时彩种停售或未开启合买 | HTTP 200，进入 `skippedItems`，不创建计划 |
| 手动执行时没有可销售 open 期号 | HTTP 200，进入 `skippedItems`，不创建计划 |
| 已创建合买但未进入封盘前 90 秒补单窗口 | HTTP 200，进入 `skippedItems`，不追加机器人参与记录、不成单 |
| 当前合买进度已达到本阶段目标 | HTTP 200，进入 `skippedItems`，等待下一阶段 |
| 手动执行时机器人余额不足 | 自动写入机器人授信流水，再继续创建计划或补单并扣款 |

### 5. Good / Base / Bad Cases

- Good：创建 `purchase` 机器人并绑定 `fc3d`、`ssc60`，响应按标准字段返回，dashboard 同步显示该机器人。
- Good：通过 `PATCH /api/admin/robots/{id}/status` 把机器人从 `paused` 改为 `enabled`，列表立即显示启用状态。
- Good：删除后台新建的普通机器人配置后，`GET /api/admin/robots` 和 dashboard 不再返回该配置。
- Good：尝试删除核心内置机器人时返回冲突错误，运营仍可暂停或禁用该机器人。
- Good：`ssc60` 开售且开启合买、存在未封盘 open 期号时，`POST /api/admin/robots/run` 可以先创建确定性机器人合买计划；未进入封盘前 90 秒补单窗口时只记录跳过原因，不立即补满。
- Good：进入补单窗口后，合买机器人按 40%、60%、80%、100% 的阶段目标追加机器人参与记录，最后 15 秒才补满计划并生成真实投注订单。
- Good：用户或后台已经发起同彩种当前期未满单合买时，合买机器人同样按临近封盘阶段目标追加机器人参与记录，不一次性补足剩余金额。
- Good：同一期重复执行 `POST /api/admin/robots/run` 时返回“本期机器人合买计划已处理”等跳过原因，不重复创建计划。
- Good：用户端查看机器人发起的合买计划时，只看到普通会员式发起人和通用合买标题，不会看到机器人账号、机器人名称或机器人说明。
- Good：机器人账户余额被扣到不足时，执行结果 `ledgerEntries` 先出现正向 `manualAdjustment` 授信流水，再出现 `groupBuyDebit` 扣款流水。
- Base：无数据库环境下使用内存机器人仓储，服务重启后恢复种子机器人配置。
- Bad：机器人页面直接读取 dashboard 静态 `robots`，保存后列表与首页摘要会漂移。
- Bad：机器人配置保存时不校验彩种存在，后续真实执行会对不存在彩种下单或发起合买。
- Bad：机器人执行绕过 `group_buy_flow` 或订单报价服务，手写投注内容展开、注数和单注金额。
- Bad：用户端合买列表或详情直接返回 `agent_alpha`、`U90001`、机器人名称或包含“机器人”的标题。

### 6. 必要测试

- 后端需要覆盖机器人创建、状态变更和绑定彩种去重。
- 后端需要覆盖普通机器人可删除、核心内置机器人拒绝删除。
- 后端需要覆盖无彩种拒绝保存。
- 后端需要覆盖绑定不存在彩种拒绝保存。
- 后端需要覆盖合买机器人创建计划后不会在补单窗口外立即补满。
- 后端需要覆盖合买机器人在封盘前 90 秒窗口内按 40%、60%、80%、100% 节奏补单，并在最终阶段生成真实订单。
- 后端需要覆盖合买机器人可以按相同节奏补满非机器人发起的当前期未满单计划。
- 后端需要覆盖同一期重复执行不会重复创建机器人合买计划。
- 后端需要覆盖用户端合买 DTO 对机器人计划的发起人展示和标题脱敏，并确认普通用户发起的合买不受影响。
- 后端需要覆盖机器人账户余额不足时会自动授信补余额，并且不会创建未扣款计划。
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
5. 充值成功后会按 `defaultRechargeRebateBasisPoints` 给符合条件的上级代理写入 `rechargeRebateCredit` 流水；`rechargeTiered` 尚未维护独立阶梯表时沿用默认比例发放。
6. 同一充值订单的 `rechargeRebateCredit` 只能发放一次，幂等引用固定为 `recharge-rebate:{充值单号}`；后续邀请关系或上级代理变更不能让同一充值单再次给新代理返利。

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
- `GET /api/user/support/conversations`
- `GET /api/user/support/conversations/{id}`
- `POST /api/user/support/conversations/{id}/messages`
- `POST /api/user/support/conversations/{id}/read`

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
  "userUnreadCount": 0,
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
  "content": "已为您核对订单。",
  "messageType": "text",
  "imageUrl": null
}
```

后台图片回复字段：

```json
{
  "adminId": "A10001",
  "content": "请查看这张凭证截图。",
  "messageType": "image",
  "imageUrl": "https://oss.example.test/support-proof.png"
}
```

删除已解决会话：

- `DELETE /api/admin/support/conversations/{id}`
- 成功时返回被删除的客服会话快照。
- 仅允许 `status=resolved` 的会话被删除。

字段契约：

1. `status` 只允许 `open`、`pending`、`resolved`、`closed`。
2. `priority` 只允许 `normal`、`urgent`。
3. 创建会话时 `userId` 必须引用用户仓储中的已有用户，后端根据用户仓储回填 `username`。
4. 更新会话时 `assignedAdminId` 可以为空；非空时必须引用管理员仓储中的已有管理员，后端回填 `assignedAdminName`。
5. 后台回复时 `adminId` 必须引用已有管理员，消息作者为 `admin`。
6. 客服消息返回 `messageType`，文本消息为 `text`，图片消息为 `image`。
7. 图片消息必须提供 `imageUrl`，且只能保存 `http/https` 图片链接；`content` 是可选说明文字。
8. 后台客服页面按状态 Tabs 区分会话列表，筛选只影响后台列表展示，不改变后端会话状态。
9. 客服消息通过 `support.message_created` 实时事件同步到会话所属用户和后台客服连接，手机端按 `messageType` 展示文本或图片。
10. `unreadCount` 只代表后台客服侧未读消息数，用户发消息时递增，后台客服回复或关闭/解决会话时清零。
11. `userUnreadCount` 只代表用户侧未读消息数，后台客服回复时递增，用户回复或调用 `POST /api/user/support/conversations/{id}/read` 后清零。
12. 用户侧已读接口必须校验会话归属，归属不匹配时返回 404；该接口只清理用户侧未读，不影响后台客服侧 `unreadCount`。
13. 后台客服会话列表必须按处理优先级返回：`unreadCount > 0` 的会话排在最前，其次按最后一条消息时间、更新时间、创建时间倒序，最后按会话 ID 倒序兜底。
14. 后台删除客服会话只允许 `resolved` 状态；删除成功后必须发布 `support.conversation_deleted`，事件只携带 `conversationId` 和 `userId`，用于客户端移除本地会话。

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
| 文本回复内容为空 | HTTP 400，返回 `support reply content is required` |
| 图片回复缺少图片链接 | HTTP 400，返回客服图片链接不能为空 |
| 图片回复链接不是 `http/https` | HTTP 400，返回客服图片链接必须是 `http` 或 `https` 地址 |
| 查询、更新、回复不存在会话 | HTTP 404，返回会话不存在 |
| 删除未解决会话 | HTTP 400，返回只有已解决的客服会话可以删除 |
| 删除已解决会话 | HTTP 200，删除会话并返回被删除快照 |
| 用户标记他人客服会话已读 | HTTP 404，返回会话不存在 |

### 5. Good / Base / Bad Cases

- Good：创建 `CS-API-001` 绑定 `U10001`，响应自动带上 `username=demo_user` 和首条用户消息。
- Good：把会话分配给 `A10001`，响应自动带上 `assignedAdminName=admin`。
- Good：客服回复后消息列表新增 `admin` 消息，`unreadCount` 清零。
- Good：客服回复后 `userUnreadCount` 递增，手机端在线客服入口显示未读红点；用户打开该会话并调用已读接口后 `userUnreadCount` 清零。
- Good：后台上传图床图片后用 `messageType=image` 和 `imageUrl` 发送，后台和手机端历史消息都展示图片缩略图。
- Good：用户发送新客服消息后，该会话的 `unreadCount` 递增，并在后台会话列表移动到未读队列前列。
- Good：客服把会话保存为已解决后，后台点击“删除会话”，接口删除该会话并广播 `support.conversation_deleted`。
- Base：无数据库环境下使用内存客服仓储，服务重启后恢复种子会话。
- Bad：前端直接提交 `username` 或 `assignedAdminName` 并让后端信任，会导致用户/管理员改名后数据漂移。
- Bad：只把图片 URL 拼进文本内容，导致手机端无法按图片消息渲染，也无法区分图片说明文字。
- Bad：允许删除处理中会话，会导致客服工单和用户沟通记录在未完成时丢失。

### 6. 必要测试

- 后端需要覆盖创建、更新分配和后台回复。
- 后端需要覆盖后台图片回复保存 `messageType=image` 和 `imageUrl`。
- 后端需要覆盖客服回复增加 `userUnreadCount`，用户标记已读只清理用户侧未读并保留后台侧 `unreadCount`。
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
5. `rebateEnabled` 表示该邀请关系是否参与充值返利发放；充值成功时只有 `status=active` 且 `rebateEnabled=true` 的人工邀请记录会触发返利。
6. 同一个代理邀请码可以创建多条不同被邀请人的邀请关系；重复关系仍按邀请人和被邀请人组合拒绝。
7. 同一被邀请人存在后台邀请记录时，充值返利以后台邀请记录为准；记录禁用或关闭返利时不再回退到用户表 `agentId`。

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

- `GET /api/admin/group-buy/plans?page=1&pageSize=20&includeRobotData=false`
- `GET /api/admin/group-buy/plans/{id}`
- `POST /api/admin/group-buy/plans`
- `DELETE /api/admin/group-buy/plans/clear`
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

后台计划列表响应使用分页结构：

```json
{
  "items": [
    {
      "id": "G202606020001",
      "lotteryId": "fc3d",
      "lotteryName": "福彩 3D",
      "orderId": null,
      "issue": "20260605001",
      "ruleCode": "threeDirect",
      "title": "福彩 3D 第20260605001期合买",
      "initiatorUserId": "U90001",
      "initiatorUsername": "agent_alpha",
      "totalAmountMinor": 100000,
      "filledAmountMinor": 72000,
      "shareCount": 1000,
      "status": "open",
      "createdAt": "2026-06-02 09:00:00"
    }
  ],
  "totalCount": 0,
  "page": 1,
  "pageSize": 20,
  "totalPages": 0
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
14. 后台计划列表不传 `page/pageSize` 时允许返回全量列表，用于内部调试；管理后台页面必须显式传入分页参数。
15. 用户端 `/api/user/group-buy/plans`、`/api/user/group-buy/plans/{id}` 和 `/api/user/group-buy/my` 返回的 `initiatorDisplay` 必须是脱敏展示名：普通用户和机器人计划都只保留首尾字符，中间用 `*` 替代；后台合买管理、资金流水和审计仍保留真实 `initiatorUserId/initiatorUsername`。
16. 用户端合买计划响应必须返回 `participantMinAmountMinor`，手机端认购金额用它和 `shareAmountMinor` 共同校正最小可认购金额。
17. 参与人最低认购金额适用于普通追加认购；如果当前剩余金额已经低于该最低值，允许用户按剩余金额一次性全包尾单。
18. 新增参与记录后如果仍未满单，剩余金额不能小于参与人最低认购金额；否则该计划会留下无人可认购的小尾巴，后端必须拒绝本次认购并提示用户增加金额或选择全包。
19. 后台 `GET /api/admin/group-buy/plans`、用户端 `/api/user/group-buy/plans` 和 `/api/user/group-buy/my` 都必须由后端仓储统一按 `issue` 倒序返回；同一期多条计划按 `createdAt`、计划 ID 倒序稳定排列，前端不得改成升序。
20. 后台 `GET /api/admin/group-buy/plans` 不传 `includeRobotData` 时等同于 `false`，默认过滤 `initiatorUserId` 为系统合买机器人账户的计划；传 `includeRobotData=true` 时才展示机器人发起计划。机器人作为参与人补单的普通用户发起计划不能被过滤掉。
21. 后台计划列表摘要必须返回 `createdAt`，后台合买计划列表需要直接显示该创建时间，方便运营核对计划生成顺序。
22. 用户端 `/api/user/group-buy/plans`、`/api/user/group-buy/plans/{id}` 和 `/api/user/group-buy/my` 返回的 `initiatorAvatarUrl` 只面向普通用户发起计划，从当前用户资料按 `initiatorUserId` 动态读取；机器人计划必须返回空字符串，避免暴露机器人账号头像。
23. 用户端 `/api/user/group-buy/plans/{id}` 返回 `participants` 参与人摘要，字段包含 `id`、脱敏 `displayName`、`amountMinor`、`shareCount`、`isMine` 和 `createdAt`；用户端不得返回完整用户名、用户 ID 或机器人账号给普通用户页面。

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
| 参与金额等于剩余金额但剩余金额低于 `participantMinAmountMinor` | 允许认购并满单 |
| 参与金额会留下低于 `participantMinAmountMinor` 的剩余金额 | HTTP 400，返回需要增加认购金额或选择全包 |
| 参与金额超过剩余可认购金额 | HTTP 400，返回超额参与错误 |
| 计划不是 `draft` 或 `open` | HTTP 400，返回计划不可参与 |
| 满员计划关联真实订单失败 | 回滚新建计划或新增参与记录，已创建的未入账订单必须移除 |
| 后台取消已开奖或已取消的合买真实订单 | HTTP 400，返回已开奖或已取消的合买订单不能取消 |
| 后台计划列表 `page <= 0` 或缺失 | 使用第 1 页 |
| 后台计划列表 `pageSize <= 0` 或缺失 | 使用默认每页 20 条 |
| 后台计划列表页码超过最大页 | 返回最后一页 |
| 后台计划列表未传 `includeRobotData` | 默认过滤机器人发起计划 |
| 后台计划列表传 `includeRobotData=true` | 返回普通用户和机器人发起计划 |

### 5. Good / Base / Bad Cases

- Good：`fc3d` 开启合买，`U90001` 发起 `100000` 分计划并认购 `10000` 分，后端自动生成发起人参与记录和 `1000` 份总份额。
- Good：手机端当前用户发起或参与合买后，后端扣减可用余额，写入 `groupBuyDebit` 资金流水，并返回最新合买计划和余额。
- Good：追加 `U10001` 参与记录后，如果 `filledAmountMinor == totalAmountMinor`，后端自动把计划状态改为 `filled`，创建真实投注订单并在响应里返回 `orderId`。
- Good：计划剩余 500 分且参与人最低认购 1000 分时，用户认购 500 分可以直接全包满单。
- Good：计划剩余 1500 分且参与人最低认购 1000 分时，用户认购 1000 分会留下 500 分尾单，后端拒绝并要求增加金额或全包。
- Good：自动化封盘时取消未满员计划，按每条参与记录退回认购金额，重复执行不会重复退款。
- Good：开奖结算时识别合买订单，中奖金额按参与金额比例拆给参与用户，普通订单仍按订单用户派奖。
- Good：后台合买管理按 `page/pageSize` 请求计划列表，响应 `items` 只包含当前页，`totalCount` 返回全部计划数。
- Good：后台和手机端合买计划列表都按期号倒序展示，分页前已经完成排序，最新期号优先出现在第一页。
- Good：后台合买计划列表显示每条计划的创建时间，且详情加载、创建计划和新增参与人后本地摘要仍保留 `createdAt`。
- Good：后台合买管理默认只展示非机器人发起计划，打开“显示机器人数据”后才纳入机器人发起计划；机器人补单参与的用户计划仍保持可见。
- Good：普通用户 `regular_user` 发起的用户端合买响应中 `initiatorDisplay` 返回 `r**********r`，机器人计划也返回同样脱敏后的普通会员式展示名。
- Good：普通用户发起的用户端合买响应返回该用户当前 `initiatorAvatarUrl`，机器人计划返回空头像并由手机端显示脱敏名首字。
- Base：无数据库环境下使用内存合买仓储，服务重启后恢复种子合买计划；数据库模式下使用 `group_buy_plans`、`group_buy_participants` 和 `ledger_entries` 持久化。
- Bad：前端自行计算 `shareCount` 并提交给后端，后续会与彩种合买配置漂移。
- Bad：直接把 dashboard 的 `groupBuyPlans` 写成静态函数，页面创建计划后首页摘要不会同步。
- Bad：用户端继续请求旧 `/group-buys/*` 路径，当前后端没有该旧系统接口。
- Bad：满单后创建普通订单并再次调用 `debit_order`；这会导致用户既被合买认购扣款，又被订单扣款。
- Bad：用户端合买列表直接展示 `initiatorUsername`、机器人账号或完整昵称，导致用户可以识别真实发起人或机器人身份。

### 6. 必要测试

- 后端需要覆盖创建合买计划并自动写入发起人参与记录。
- 后端需要覆盖未开启合买彩种被拒绝。
- 后端需要覆盖发起人认购低于最低比例被拒绝。
- 后端需要覆盖添加参与记录后自动满单。
- 后端需要覆盖超额参与被拒绝。
- 后端需要覆盖低于参与人最低认购金额的最后尾单可被全包，以及普通认购不能留下低于最低认购金额的尾单。
- 后端需要覆盖合买扣款写入 `groupBuyDebit` 且相同参与记录不会重复扣款。
- 后端需要覆盖满单创建真实订单并回写 `orderId`。
- 后端需要覆盖封盘流单退款写入 `groupBuyRefund` 且幂等。
- 后端需要覆盖合买中奖按参与人份额拆分派奖。
- 后端需要覆盖后台合买计划分页响应的当前页切片、总数和总页数。
- 后端需要覆盖合买计划摘要列表和详情列表均按期号倒序返回。
- 后端需要覆盖后台合买计划默认过滤机器人发起计划，打开 `includeRobotData` 后才返回。
- 后端需要覆盖用户端普通用户和机器人合买计划的 `initiatorDisplay` 脱敏，断言不会返回完整昵称、机器人账号或包含“机器人”的展示名。
- 后端需要覆盖普通用户合买返回发起人头像、机器人合买不返回真实头像。
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

```rust
fn user_group_buy_initiator_display(plan: &GroupBuyPlan) -> String {
    plan.initiator_username.clone()
}
```

这个写法会把完整用户昵称或机器人账号暴露给手机端合买大厅。

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

```rust
fn user_group_buy_initiator_display(plan: &GroupBuyPlan) -> String {
    let display_name = if is_robot_group_buy_plan(plan) {
        robot_group_buy_initiator_display(plan)
    } else {
        plan.initiator_username.clone()
    };

    mask_group_buy_initiator_display(&display_name)
}
```

用户端合买响应统一返回脱敏展示名，真实发起人只留在后台和审计链路。

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

- 触发条件：新增或修改彩种控制台的开奖号码控制、控制范围、开奖服务控制优先级、自动开奖控制逻辑。
- 范围：后端开奖控制配置 API、开奖服务读取控制号码、自动开奖手动彩种跳过规则、后台订单信息查看、前端控制台类型和 SideSheet 表单。
- 控制配置支持内存模式和 PostgreSQL 业务表模式；接口契约不随持久化模式变化。

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
    "targetScope": "lottery",
    "targetIssue": null,
    "targetOrderId": null,
    "updatedAt": null
  }
]
```

`PUT /api/admin/draw-controls/{lotteryId}` 请求体：

```json
{
  "enabled": true,
  "drawNumber": "2,4,7",
  "targetScope": "issue",
  "targetIssue": "202606052200",
  "targetOrderId": null
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
  "targetScope": "issue",
  "targetIssue": "202606052200",
  "targetOrderId": null,
  "updatedAt": "unix:1780475520"
}
```

开奖号码继续使用英文逗号分隔格式。后端保存时会规范化号码，例如 `247` 保存为 `2,4,7`；中文逗号输入也会被规范化为英文逗号。

控制范围：

- `targetScope=lottery`：控制整个彩种后续开奖，保存时清空 `targetIssue` 和 `targetOrderId`。
- `targetScope=issue`：只控制指定期号，`targetIssue` 必填，且后台保存前必须确认该期号属于当前彩种。
- `targetScope=order`：按目标订单绑定控制期号，`targetOrderId` 必填；后台保存前必须确认订单属于当前彩种且状态为 `pendingDraw`，并把 `targetIssue` 自动补齐为该订单期号。
- 订单范围不是“只给单个订单单独结算不同号码”，因为一期只有一个开奖号码；它表示控制号码只在该订单所在期号开奖时生效，并在控制台记录目标订单便于运营审计。
- 关闭控制时，后端必须清空控制目标，避免历史目标字段被误读。

开奖优先级：

1. 如果彩种控制配置 `enabled=true`、有合法 `drawNumber`，且控制范围命中当前开奖期号，手动触发开奖和自动开奖都优先使用控制号码。
2. 如果控制关闭，`platform` 彩种使用平台生成器。
3. 如果控制关闭，`api` 彩种使用已绑定的 API 开奖源。
4. 如果控制关闭，`manual` 彩种自动任务缺少管理员号码时继续跳过。

彩种控制台可以复用 `GET /api/admin/orders` 查看用户下单信息，默认不包含机器人订单；SideSheet 中展示当前彩种近期订单、当前期订单，并允许对待开奖订单一键选择为控制目标。

### 4. 校验与错误矩阵

| 条件 | 预期行为 |
|------|----------|
| `lotteryId` 不存在 | HTTP 404，返回彩种不存在 |
| `enabled=true` 且 `drawNumber` 为空 | HTTP 400，返回控制开奖号码必填 |
| `targetScope=issue` 且 `targetIssue` 为空 | HTTP 400，返回控制期号不能为空 |
| `targetScope=issue` 且期号不属于当前彩种 | HTTP 404 或 400，返回期号校验错误 |
| `targetScope=issue` 且期号已开奖或已取消 | HTTP 200，自动保存为关闭控制并清空控制目标 |
| `targetScope=order` 且 `targetOrderId` 为空 | HTTP 400，返回目标订单不能为空 |
| `targetScope=order` 且订单不属于当前彩种 | HTTP 400，返回目标订单不属于当前彩种 |
| `targetScope=order` 且订单不是 `pendingDraw` | HTTP 400，返回只能控制待开奖订单 |
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
- Good：运营在控制台选择某个待开奖订单作为目标，后端自动绑定该订单期号，控制号码只在该期开奖时生效。
- Good：运营只想控制下一期时选择 `targetScope=issue`，该期完成后其它期号继续走平台或 API 开奖源。
- Base：控制关闭时，现有平台生成器和 API68 来源行为保持不变。
- Bad：只在前端保存控制状态，不让后端开奖服务读取；这会导致页面看起来控制成功但自动开奖仍按原来源开奖。
- Bad：前端直接绕过后端校验写入不符合彩种号码类型的号码。
- Bad：订单范围控制时只保存订单号、不保存订单期号；开奖服务无法判断当前期是否命中控制范围。
- Bad：把订单范围理解为同一期不同订单使用不同开奖号码，这会破坏一期一个开奖结果的核心规则。

### 6. 必要测试

- 后端需要覆盖平台开奖使用控制号码。
- 后端需要覆盖 API 开奖源被控制号码覆盖。
- 后端需要覆盖控制号码按号码类型校验。
- 后端需要覆盖期号范围控制只命中目标期号。
- 后端需要覆盖订单范围控制会绑定目标订单期号，并拒绝非待开奖订单。
- 后端需要覆盖手动彩种自动任务在控制号码启用时不跳过并完成开奖。
- 前端需要运行 `npm run build`，确认 `LotteryDrawControl`、保存请求类型、订单信息展示和控制范围表单与接口字段一致。
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
  targetScope: "issue",
  targetIssue: "202606052200",
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
if let Some(draw_number) = active_draw_control_number(&issue)? {
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
- `GET /api/user/realtime`
- `GET /api/admin/realtime?token=<管理员登录 token>`
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

`channel` 只能是 `rainbowEpay` 或 `customerService`。金额继续使用最小货币单位。彩虹易支付返回 `paymentUrl`，客服直充返回 `supportConversationId` 并同步创建客服会话。后台系统设置的“支付方式开关”负责维护 `recharge_rainbow_epay_enabled`、`recharge_customer_service_enabled` 和 `recharge_rainbow_epay_pay_types`；当彩虹易支付总开关开启但 `payTypes` 为空时，用户端配置必须把 `rainbowEpay.enabled` 返回为 `false`，创建彩虹易支付订单也必须返回 HTTP 400。

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

客服直充实时聊天规则：

- 创建客服直充订单后，后端同步创建客服会话、关联 `supportConversationId`，并发布 `support.message_created`。
- 用户调用 `POST /api/user/support/conversations/{id}/messages` 后，消息落库，再发布 `support.message_created` 给本人和后台客服。
- 后台调用 `POST /api/admin/support/conversations/{id}/messages` 后，消息落库，再发布 `support.message_created` 给会话所属用户和后台客服。
- 后台调用 `PUT /api/admin/support/conversations/{id}` 调整状态、优先级或分配客服后，必须发布 `support.conversation_updated` 给会话所属用户和后台客服。
- 当会话处于 `pending`、`resolved` 或 `closed` 时，用户再次发送消息必须把会话恢复为 `open`，避免客服后台仍显示“等待用户”。
- 充值页必须消费 `user.recharge_changed` 和 `user.balance_changed`，客服确认入账后实时刷新充值订单和余额。
- WebSocket 只负责实时通知；断线重连后仍必须通过 HTTP 拉取会话历史和充值订单状态。

### 4. 校验与错误矩阵

| 条件 | 预期行为 |
|------|----------|
| 未登录调用用户端充值或客服接口 | HTTP 401 |
| 充值金额低于 `recharge_min_amount_minor` | HTTP 400 |
| 充值金额高于 `recharge_max_amount_minor` | HTTP 400 |
| 彩虹易支付未开启 | HTTP 400 |
| 彩虹易支付开启但未配置任何 `payTypes` | HTTP 400，用户端充值配置中 `rainbowEpay.enabled=false` |
| 彩虹易支付网关、商户号或密钥仍为占位值 | HTTP 400 |
| `payType` 不在后台配置列表中 | HTTP 400 |
| 客服直充未开启 | HTTP 400 |
| 用户访问他人的客服会话 | HTTP 404 |
| 客服消息内容为空 | HTTP 400 |
| 支付通知签名无效或金额不匹配 | HTTP 400 |
| 重复支付通知 | 保持幂等，不重复生成 `rechargeCredit` 流水 |
| 重复支付通知且上级代理关系已变化 | 保持幂等，不重复生成 `rechargeRebateCredit` 流水，也不改派给新代理 |
| 后台确认非客服直充订单 | HTTP 400 |
| 后台重复确认已入账客服直充订单 | 保持幂等，不重复生成 `rechargeCredit` 流水 |
| WebSocket 断线或错过客服消息 | 前端通过 HTTP 重新拉取会话详情恢复完整历史 |

### 5. Good / Base / Bad Cases

- Good：客服直充创建订单后返回 `waitingCustomerService`，后台在线客服可看到会话，用户端可继续发消息。
- Good：客服直充会话任意一方发送消息后，另一方通过 WebSocket 收到 `support.message_created` 并刷新消息列表。
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
- 后端需要覆盖客服消息实时事件和后台实时受众。
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
- 后台用户列表：`GET /api/admin/users?page=1&pageSize=20&sortBy=id&sortDirection=desc`
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
- `GET /api/admin/users` 返回分页结构 `items/totalCount/page/pageSize/totalPages`，其中 `items[].balanceMinor` 应以 `financial_accounts.available_balance_minor` 为准。
- `GET /api/admin/users`、`GET /api/admin/users/{id}`、用户创建、更新和状态变更响应必须返回 `agentUsername` 派生字段；该字段只供后台展示，不作为保存请求字段。
- 用户列表支持 `sortBy` 和 `sortDirection` 查询排序；`sortBy` 白名单为 `id`、`username`、`email`、`kind`、`status`、`balanceMinor`、`agentId`、`inviteCode`，`sortDirection` 只允许 `asc` 或 `desc`，未传或传空字符串时默认按 `desc` 降序。
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
| 用户列表 `sortBy` 不在白名单 | HTTP 400，返回不支持的用户排序字段 |
| 用户列表 `sortDirection` 不是 `asc/desc` | HTTP 400，返回不支持的用户排序方向 |

### 5. Good / Base / Bad Cases

- Good：用户先绑定银行卡提现方式，再提交 `POST /api/user/withdrawals`，返回 `pending` 提现申请，资金账户可用余额减少、冻结余额增加。
- Good：后台用户维护列表中余额来自财务账户；财务手动调账后刷新用户维护页能看到新余额。
- Good：后台用户维护页请求 `GET /api/admin/users?page=1&pageSize=20&sortBy=balanceMinor&sortDirection=desc`，按余额倒序展示第一页用户，总数来自 `totalCount`。
- Good：用户存在 `agentId=U90001` 时，后台用户维护列表的上级代理列展示 `agent_alpha` 和 `U90001`，不再只显示代理 ID。
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

## 场景：手机端首页高频极速配置

### 1. 范围 / 触发条件

- 触发条件：手机端首页需要由后台控制“高频极速”模块是否展示，以及展示哪些彩种。
- 范围：`GET /api/lottery/home`、`system_settings`、管理后台系统设置、手机端首页推荐区。

### 2. 签名

- 首页接口：`GET /api/lottery/home`
- 系统设置开关：`mobile_home_featured_enabled`
- 系统设置标题：`mobile_home_featured_title`
- 系统设置彩种：`mobile_home_featured_lottery_codes`

### 3. 契约

`mobile_home_featured_enabled` 默认值必须是 `false`，手机端首页默认不展示高频极速模块。只有该值为 `true`，且 `mobile_home_featured_lottery_codes` 至少命中一个销售中彩种时，`GET /api/lottery/home` 才返回 `settings.featuredEnabled=true` 和非空 `featuredSection.lotteries`。

`mobile_home_featured_lottery_codes` 使用英文逗号分隔彩种 ID，后端按配置顺序返回，只保留当前销售中的彩种，不自动按开奖周期兜底补彩种。

手机端首页卡片不得展示合买标签、合买按钮或合买大厅入口；合买入口只属于合买专用页面或下注页合买模式。

### 4. 校验与错误矩阵

| 条件 | 预期行为 |
|------|----------|
| 开关为 `false` | 首页 `featuredEnabled=false`，高频极速模块隐藏 |
| 开关为 `true` 但未选彩种 | 首页 `featuredEnabled=false`，模块隐藏 |
| 开关为 `true` 且选中销售中彩种 | 按后台配置顺序返回高频极速彩种 |
| 配置中包含停售彩种 | 停售彩种不进入首页高频极速 |
| 到达开奖时间 | 手机端静默刷新首页或消费新期号实时事件，不长期停在“开奖中” |

### 5. Good / Base / Bad Cases

- Good：后台开启高频极速并选择 `au5,txffc`，两个彩种均销售中，首页按 `au5`、`txffc` 顺序展示。
- Base：后台保持默认关闭，首页只展示轮播、跑马灯和分类分组，不渲染高频极速模块。
- Bad：后端因为存在 5 分钟内开奖的销售中彩种就自动返回高频极速；这会绕过后台开关。
- Bad：手机端首页显示合买标签或合买入口；这会把首页推荐区和合买专用流程混在一起。

### 6. 必要测试

- 后端需要覆盖高频极速默认关闭。
- 后端需要覆盖系统设置解析开关、标题和彩种顺序。
- 手机端需要运行 `npm run build`，确认首页实时事件和倒计时刷新逻辑可编译。
- 管理后台需要运行 `npm run build`，确认系统设置多选彩种配置可编译。

### 7. Wrong vs Correct

#### 错误

```rust
let featured_lotteries = selling_cards
    .iter()
    .filter(|card| card.draw_interval.is_some_and(|interval| interval <= 300))
    .take(3)
    .cloned()
    .collect::<Vec<_>>();
```

这个写法会让高频极速默认自动展示，后台无法控制是否显示和显示哪些彩种。

#### 正确

```rust
let settings = state.access.settings().await?;
let featured_config = mobile_featured_config_from_settings(&settings);
let home = build_mobile_lottery_home(lotteries, categories, issues, featured_config);
```

首页接口必须先读取后台系统设置，再按配置开关和彩种 ID 列表构建高频极速模块。

---

## 场景：后台财务分页与提现审核接口

### 1. 范围 / 触发条件

- 触发条件：财务管理页展示资金账户、充值订单、资金流水或提现申请，或者后台审核提现申请。
- 范围：`/api/admin/finance-overview`、`/api/admin/financial-accounts`、`/api/admin/recharge-orders`、`/api/admin/ledger-entries`、`/api/admin/withdrawal-orders`、`FinanceRepository`、`WithdrawalRepository`、管理后台财务页面。
- 机器人数据口径：财务总览、资金账户和资金流水默认隐藏系统机器人账户数据，只有页面开关传入 `includeRobotData=true` 时才展示。

### 2. 签名

- `GET /api/admin/finance-overview?includeRobotData=false`
- `GET /api/admin/financial-accounts?page=1&pageSize=20&includeRobotData=false`
- `GET /api/admin/recharge-orders?page=1&pageSize=20`
- `GET /api/admin/ledger-entries?page=1&pageSize=20&includeRobotData=false`
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

资金账户必须在分页前按用户编号倒序排序，让最新创建的用户优先展示；充值订单、资金流水和提现申请必须在分页前按创建时间倒序排序。同一秒产生的财务事件按业务编号倒序兜底，保证第一页永远优先展示最新财务事件。时间排序需要兼容标准 `YYYY-MM-DD HH:mm:ss` 和历史 `unix:秒` 两种格式。

不传 `includeRobotData` 时等同于 `false`，财务总览、资金账户和资金流水必须排除 `U90001` 等系统机器人账户；开关打开时才纳入机器人自动授信、合买扣款和机器人订单相关流水，方便审计。

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
| 未传 `includeRobotData` | 默认过滤系统机器人账户和机器人流水 |
| `includeRobotData=true` | 返回真实用户数据和机器人账户数据 |
| 提现申请不存在 | HTTP 404 |
| 提现申请已通过再驳回 | HTTP 400 |
| 提现申请已驳回再通过 | HTTP 400 |
| 冻结余额不足 | HTTP 400，不改变提现状态 |

### 5. Good / Base / Bad Cases

- Good：财务管理页按分页请求资金账户，表格同时展示用户名和用户 ID，并且第一页优先展示用户编号最大的最新用户。
- Good：充值订单、资金流水、提现申请都有分页控件，并且第一页按创建时间展示最新记录，翻页不会一次性拉取所有历史记录。
- Good：默认进入财务管理页时，财务总览、资金账户和资金流水都不包含机器人账户；打开“显示机器人数据”后再纳入机器人流水。
- Good：待审核提现点击“通过”后状态变为已通过，冻结余额减少，资金流水出现提现打款。
- Good：待审核提现点击“驳回”后状态变为已驳回，冻结余额退回可用余额，资金流水出现提现驳回解冻。
- Bad：前端拿全量数组后自行分页；数据量增大后会拖慢财务页面。
- Bad：后台只改提现申请状态，不写资金流水；这会破坏财务审计链路。

### 6. 必要测试

- 后端需要覆盖提现通过、提现驳回和反向审核拒绝。
- 后端需要覆盖机器人账户默认被财务口径过滤、打开开关后纳入总览。
- OpenAPI 测试需要覆盖财务总览、提现申请列表和提现审核路径。
- 管理后台需要运行 `npm run build`，确认分页响应、用户名字段和提现状态枚举类型一致。

---

## 场景：手机端用户头像设置

### 1. 范围 / 触发条件

- 触发条件：登录用户在手机端“我的账户”点击头像上传图片，或已有图片链接需要写回个人资料。
- 范围：`users.avatar_url`、`UserSummary.avatarUrl`、`PUT /api/user/avatar`、`POST /api/user/avatar/upload`、系统图床配置和手机端 `mobile/src/api/user.ts`。

### 2. 签名

- `PUT /api/user/avatar`
- 认证方式：用户 Bearer Token。
- 请求体：JSON。

```json
{
  "avatarUrl": "https://cdn.example.com/avatar.png"
}
```

- `POST /api/user/avatar/upload`
- 认证方式：用户 Bearer Token。
- 请求体：`multipart/form-data`，图片字段默认使用 `file`，服务端会读取 `image_bed_upload_field` 配置并兼容默认字段。

### 3. 契约

- 两个接口都返回统一信封，`data.user.avatarUrl` 为保存后的头像链接。
- `PUT /api/user/avatar` 允许空字符串，表示清空头像；非空值必须是 `http` 或 `https` 链接。
- `POST /api/user/avatar/upload` 必须读取后台系统设置中的图床上传地址、Token、上传字段名和结果链接字段；接口只保存图床返回的有效图片链接，不把第三方完整响应直接写入用户资料。
- 图床未配置、上传字段缺失、文件不是图片、返回字段不存在或返回链接非法时，必须返回中文错误信息。
- 后台用户维护接口不能因为旧表单没有头像字段而清空用户头像；头像由当前用户自己的接口维护。

### 4. 验证

- 后端需要覆盖头像保存成功和非法头像链接拒绝。
- OpenAPI 测试需要覆盖 `/user/avatar` 和 `/user/avatar/upload`。
- 手机端需要运行 `npm run build`，确认头像上传 API 类型、个人中心上传入口和登录态刷新可编译。

---

## 场景：手机端邀请中心汇总接口

### 1. 范围 / 触发条件

- 触发条件：手机端打开 `invitation-center` 页面，或用户复制邀请码、查看直属用户与返利摘要。
- 范围：`GET /api/user/invitations/summary`、`AccessRepository` 用户代理关系、`InviteRepository` 邀请记录、`FinanceRepository` 资金流水、手机端 `mobile/src/api/user.ts`。

### 2. 签名

- `GET /api/user/invitations/summary`
- 认证方式：用户 Bearer Token。
- 请求体：无。

### 3. 契约

接口返回统一信封，`data` 字段使用 `camelCase`：

```json
{
  "canInvite": true,
  "invitationCode": "ABCDEFGH",
  "directCount": 2,
  "activeDirectCount": 1,
  "totalDirectDepositMinor": 15000,
  "totalPaidCommissionMinor": 350,
  "rebateMode": "immediate",
  "defaultRechargeRebateBasisPoints": 300,
  "directUsers": []
}
```

直属用户项：

```json
{
  "id": "U90012",
  "username": "demo_user",
  "status": "active",
  "inviteStatus": "active",
  "rebateEnabled": true,
  "totalDepositMinor": 10000,
  "createdAt": "2026-06-05 19:00:00"
}
```

规则：

- 每个用户都有 `invitationCode`，但普通用户 `canInvite=false`，普通用户邀请码不能作为有效邀请来源。
- 只有 `UserKind::Agent` 且邀请策略 `agentsCanInvite=true` 时，`canInvite=true`。
- 直属用户必须合并两类来源：后台邀请记录里的 `inviterUserId`，以及注册时写入用户表的 `agentId`。
- 同一直属用户同时存在两类来源时，后台邀请记录优先，因为它包含人工维护的 `inviteStatus`、`rebateEnabled` 和 `createdAt`。
- `totalDirectDepositMinor` 和直属用户 `totalDepositMinor` 只统计正向 `rechargeCredit` 资金流水。
- `totalPaidCommissionMinor` 只统计当前代理自己的正向 `rechargeRebateCredit` 资金流水，不能用直属充值金额推算或伪造返利金额。
- 充值返利的幂等引用只绑定充值单号，不绑定代理 ID；后台调整邀请关系后，同一充值单不能在邀请中心形成第二笔返利。

### 4. 校验与错误矩阵

| 条件 | 预期行为 |
|------|----------|
| 未登录访问 | HTTP 401 |
| 普通用户访问 | 返回自己的邀请码，`canInvite=false`，直属列表为空 |
| 代理用户访问且策略开启 | `canInvite=true`，可返回直属用户统计 |
| 代理用户访问且策略关闭 | `canInvite=false`，仍可查看已有直属统计 |
| 直属用户没有充值流水 | `totalDepositMinor=0` |
| 金额汇总溢出 | HTTP 500，并返回中文业务错误 |

### 5. Good / Base / Bad Cases

- Good：用户通过代理邀请码注册，只写入 `agentId`，邀请中心仍能展示到代理的直属下级。
- Good：管理员手动维护邀请记录后，邀请中心使用记录里的邀请状态和返利开关。
- Base：普通用户看到邀请码但复制时提示无可用邀请权限。
- Bad：手机端继续请求旧 `/auth/invitations/summary` 或继续消费 `can_invite/direct_users` 这类 snake_case 字段。
- Bad：前端自行读取资金流水并计算直属充值；统计口径必须由后端统一。

### 6. 必要测试

- 后端需要覆盖代理权限判断、邀请记录与 `agentId` 合并、充值流水汇总。
- OpenAPI 核心路径测试需要包含 `/user/invitations/summary`。
- 手机端需要运行 `npm run build`，确认 TypeScript 类型与 `camelCase` 字段一致。

---

## 场景：聊天大厅红包与合买分享

### 1. 范围 / 触发条件

- 触发条件：手机端公共聊天大厅需要发送普通文本、红包消息，或把本人参与的合买计划分享给所有在线会员。
- 范围：`/api/user/chat-hall/messages`、`/api/user/chat-hall/red-packets`、`/api/user/chat-hall/group-buy-plans`、聊天大厅数据库表、资金账户流水、手机端 WebSocket 消息归一化。
- 聊天大厅是公开大厅，不属于客服会话；红包收发是资金业务，必须与聊天消息持久化在同一事务内完成。

### 2. 签名

- 拉取大厅消息：`GET /api/user/chat-hall/messages`
- 发送文本消息：`POST /api/user/chat-hall/messages`
- 发送红包：`POST /api/user/chat-hall/red-packets`
- 领取红包：`POST /api/user/chat-hall/red-packets/{id}/claim`
- 分享合买计划：`POST /api/user/chat-hall/group-buy-plans`
- 实时事件：`chat_hall.message_created`
- 数据表：`chat_hall_messages`、`chat_hall_red_packets`、`chat_hall_red_packet_claims`
- 资金流水类型：`redPacketDebit`、`redPacketCredit`

### 3. 契约

聊天大厅消息必须包含 `messageType` 和 `payload`：

```json
{
  "id": "chat-000001",
  "userId": "U90001",
  "username": "demo",
  "avatarUrl": "https://cdn.example.com/avatar.png",
  "content": "恭喜发财，大吉大利",
  "messageType": "redPacket",
  "payload": {},
  "createdAt": "2026-06-07 17:20:00"
}
```

`messageType` 只允许：

- `text`：普通文本，`payload=null`。
- `redPacket`：红包卡片，`payload` 包含 `redPacketId`、`amountMinor`、`claimCount`、`claimedCount`、`remainingAmountMinor`、`greeting`、`claims`。
- `groupBuyPlan`：合买计划卡片，`payload` 包含 `planId`、`lotteryId`、`lotteryName`、`issue`、`playName`、`totalShares`、`soldShares`、`progressPercent`、`initiatorName`、`status`。

发送红包请求体：

```json
{
  "amountMinor": 1000,
  "claimCount": 5,
  "greeting": "恭喜发财，大吉大利"
}
```

分享合买计划请求体：

```json
{
  "planId": "GBP000001"
}
```

后端必须校验当前用户属于该合买计划的参与用户列表，不能分享别人完全无关的合买计划。红包扣款、红包记录、聊天消息和资金流水必须在同一 PostgreSQL 事务里保存；如果使用内存仓储测试，也必须保持同样的业务顺序。

### 4. 校验与错误矩阵

| 条件 | 预期行为 |
|------|----------|
| 未登录发送、领取或分享 | HTTP 401 |
| 红包金额小于等于 0 | 返回业务错误，不扣款、不保存消息 |
| 红包数量小于 1 或大于 100 | 返回业务错误，不扣款、不保存消息 |
| 红包金额小于红包数量 | 返回业务错误，保证每份至少 1 分 |
| 红包祝福语超过 60 字 | 返回业务错误 |
| 用户余额不足 | 返回资金业务错误，不保存红包消息 |
| 发包人领取自己的红包 | 返回业务错误，不入账 |
| 同一用户重复领取同一红包 | 返回业务错误，不重复入账 |
| 红包已领完 | 返回业务错误，不入账 |
| 分享不存在的合买计划 | 返回业务错误 |
| 分享非本人参与的合买计划 | 返回业务错误 |
| 领取红包成功 | 增加 `redPacketCredit` 流水，广播更新后的同 ID 红包消息 |
| 发送红包成功 | 增加 `redPacketDebit` 流水，广播红包消息 |

### 5. Good / Base / Bad Cases

- Good：用户发送 10 元 5 个红包，后端扣减余额、写入红包和聊天消息，并向所有在线会员广播 `redPacket` 消息。
- Good：用户领取红包后，手机端收到同一个消息 ID 的 `chat_hall.message_created`，用新 payload 替换旧消息，红包领取进度实时刷新。
- Good：用户把自己发起或参与的合买计划分享到大厅，其他用户看到卡片后可以进入合买大厅查看计划。
- Base：普通文本仍走 `messageType=text`，不需要 payload。
- Bad：把红包内容拼成纯文本，例如“红包:10元”，这会让手机端无法展示领取状态，也无法做资金幂等。
- Bad：红包扣款和聊天消息分两次保存且没有事务，失败时会出现扣款成功但消息不存在，或消息存在但未扣款。
- Bad：分享合买计划时只校验计划存在，不校验当前用户是否参与。

### 6. 必要测试

- 后端需要覆盖聊天文本消息、红包发送、红包领取、重复领取、发包人领取、合买计划分享。
- 后端需要运行 `cargo test --manifest-path backend/Cargo.toml chat_hall -- --nocapture`。
- OpenAPI 核心路径测试需要包含红包发送、红包领取和合买计划分享接口。
- 实时事件测试需要确认 `chat_hall.message_created` 仍是公开事件，并能携带 `messageType` 与 `payload`。
- 手机端需要运行 `npm run build`，确认聊天大厅红包卡片、合买计划卡片、WebSocket 归一化类型可编译。
- 修改数据库结构时，需要给新增表和字段添加中文注释。

### 7. Wrong vs Correct

#### 错误

```rust
let message = chat_hall.send_message(user, "红包 10 元".to_string())?;
finance.debit(user.id.clone(), 1000)?;
```

这个写法把资金变动和聊天消息拆开处理，并且让手机端只能看到普通文本，无法展示红包状态。

#### 正确

```rust
let message = chat_hall.send_red_packet(&finance, user, request)?;
```

服务层用专门的红包接口统一校验、扣款、写入红包 payload、保存资金流水，并发布可被手机端识别的结构化消息。

---

## 场景：后台充值导出与历史记录清理

### 1. 范围 / 触发条件

- 触发条件：后台运营需要导出用户充值记录，或在测试、运营维护场景中清理充值、提现、投注、合买计划列表历史记录。
- 范围：后台财务管理、订单管理、合买管理、充值仓储、提现仓储、订单仓储、合买仓储、计奖派奖历史。
- 记录清理只处理历史记录，不做余额冲正，不删除资金流水，不重置流水号。

### 2. 签名

- 导出充值记录：`GET /api/admin/recharge-orders/export`
- 清除充值记录：`DELETE /api/admin/recharge-orders/clear`
- 清除提现记录：`DELETE /api/admin/withdrawal-orders/clear`
- 清除投注记录：`DELETE /api/admin/orders/clear`
- 清除合买计划列表：`DELETE /api/admin/group-buy/plans/clear`

清理接口统一响应：

```json
{
  "deletedCount": 12
}
```

充值导出返回 `text/csv; charset=utf-8`，内容必须带 UTF-8 BOM，字段包含订单 ID、用户 ID、用户名、充值渠道、支付方式、金额、状态、外部交易号、客服会话 ID、创建时间和入账时间。

### 3. 契约

- `/recharge-orders/export` 必须导出全部充值订单，不受当前页面分页限制。
- `/recharge-orders/clear` 允许清除所有充值订单历史，但不得回滚已入账余额或删除充值资金流水。
- `/withdrawal-orders/clear` 在存在 `Pending` 提现申请时必须返回业务错误，要求管理员先审核或驳回，避免冻结余额失去对应申请。
- `/orders/clear` 在存在 `PendingDraw` 投注订单时必须返回业务错误，要求管理员先开奖结算或取消订单，避免已扣款订单失去结算机会。
- `/group-buy/plans/clear` 只删除 `Cancelled` 和 `Settled` 合买计划；`Draft`、`Open` 或 `Filled` 等未结算计划必须自动跳过并保留，接口不应因为存在未结算计划而整体失败，避免合买扣款、退款或派奖失去业务记录。
- 清除投注记录时必须同步清除 `order_settlement_runs` 和 `order_settlements` 对应内存快照/数据库记录，避免结算历史引用不存在的订单。
- 清理充值、提现、投注记录后必须保留 `next_sequence` 或 `next_settlement_sequence`，后续新单号不能重复。

### 4. 校验与错误矩阵

| 条件 | 预期行为 |
|------|----------|
| 管理员没有财务权限访问充值/提现导出清理 | HTTP 403 |
| 管理员没有订单权限清理投注记录 | HTTP 403 |
| 导出充值记录时没有数据 | 返回只包含表头的 CSV |
| 清理充值记录时没有数据 | 返回 `deletedCount=0` |
| 清理提现记录且存在待审核申请 | 返回业务错误，不删除任何提现记录 |
| 清理投注记录且存在待开奖订单 | 返回业务错误，不删除任何订单或结算记录 |
| 清理投注记录且订单均已结束 | 删除订单和结算批次，返回删除订单数量 |
| 清理合买记录且存在未完成计划 | 返回业务错误，不删除任何合买计划或参与记录 |
| 清理合买记录且计划均已取消或已结算 | 删除合买计划和参与记录，返回删除计划数量 |

### 5. Good / Base / Bad Cases

- Good：运营先导出充值 CSV，再清理充值记录，用户余额和资金流水仍保持不变。
- Good：提现列表有待审核申请时点击清除，后台提示先审核或驳回，冻结余额仍有业务记录可追溯。
- Good：投注订单已全部结算后点击清除，订单管理和计奖派奖历史同时清空。
- Good：合买计划均已取消或已结算后点击清除，合买计划和参与记录清空，资金流水、投注订单和派奖记录保留。
- Base：没有历史记录时点击清除，返回 `deletedCount=0` 并刷新页面。
- Bad：清理投注订单但保留结算批次，导致计奖派奖页面展示孤儿结算。
- Bad：清理提现待审核申请但不解冻资金，导致用户冻结余额无法追溯。
- Bad：清理后重置流水号，导致后续充值单、提现单或投注单 ID 重复。

### 6. 必要测试

- 后端需要覆盖充值清理保留流水号、提现待审核拒绝清理、投注待开奖拒绝清理、已结算投注清理同时移除结算批次。
- 后台需要运行 `npm run build`，确认新增按钮、Blob 下载和清理 API 类型可编译。
- 修改接口后必须同步更新 `admin/src/api/client.ts` 和对应页面刷新逻辑。

---

## 场景：后台手动同步 API 开奖源

### 1. 范围 / 触发条件

- 触发条件：后台彩种控制台中 API 彩种的本地待开奖期号和外部开奖源出现偏移，运营点击“立即同步”。
- 范围：`POST /api/admin/lotteries/{id}/sync-draw-source`、开奖源最新期号读取、开奖期号仓储、彩种控制台刷新。
- 只适用于 `drawMode=api` 的彩种；平台开奖和手动开奖彩种不得使用该接口。

### 2. 签名

- 手动同步开奖源：`POST /api/admin/lotteries/{id}/sync-draw-source`
- 请求体：无。
- 响应体：统一信封内返回 `DrawSourceSyncResult`，包含 `apiSnapshot`、`targetIssue`、`generatedIssues`、`updatedIssues`、`cancelledIssues`、`keptIssues` 和中文 `message`。

### 3. 契约

- 后端必须读取当前彩种绑定的 API 开奖源，按 `preDrawIssue/preDrawTime` 和 `drawIssue/drawTime` 计算当前可销售目标期。
- 封盘提前秒数必须使用后台调度配置 `saleCloseLeadSeconds`，与常驻调度保持一致。
- 如果目标期号本地不存在，创建新的 `open` 期号。
- 如果目标期号本地存在且尚未开奖，更新为 `open` 并校准 `scheduledAt/saleClosedAt`。
- 同彩种其它 `open/closed` 期号如果没有待开奖订单，自动标记为 `cancelled`。
- 同彩种其它 `open/closed` 期号如果存在待开奖订单，必须保留到 `keptIssues`，不得静默取消或退款。
- 同步完成后必须发布 `lottery.issue_opened`，让后台和手机端可以刷新当前期号。

### 4. 校验与错误矩阵

| 条件 | 预期行为 |
|------|----------|
| 彩种不存在 | 返回 `not found` |
| 彩种不是 API 开奖模式 | 返回业务错误“只有 API 开奖彩种可以同步开奖源” |
| 彩种未绑定 API 开奖源 | 返回业务错误“当前彩种没有绑定 API 开奖源” |
| API 请求失败或响应无法解析 | 返回业务错误，并保留原始错误详情到中文日志 |
| API 目标期号已在本地开奖 | 返回冲突错误，不修改本地期号 |
| 存在无订单旧期 | 同步时取消旧期 |
| 存在有待开奖订单旧期 | 同步时保留旧期并在结果中提示 |

### 5. Good / Base / Bad Cases

- Good：`txffc` 本地停在旧期，点击同步后生成 API 返回的下一期，并取消没有订单的旧 open 期。
- Good：旧期已有用户待开奖订单，点击同步后目标期正常生成，旧期保留在 `keptIssues`，管理员可以后续人工处理订单。
- Base：目标期已经存在但时间偏移，点击同步只更新目标期时间和状态。
- Bad：同步按钮直接删除有订单旧期，导致已扣款订单失去开奖/退款路径。
- Bad：平台开奖彩种也调用 API 同步接口，导致本地调度彩种被外部接口规则影响。

### 6. 必要测试

- 后端需要覆盖 API 同步生成目标期、取消无订单旧期、保留有订单旧期。
- 前端需要运行 `npm run build`，确认彩种控制台按钮、同步结果 Toast 和类型契约可编译。

---

## 场景：开奖调度 API 读取并发化

### 1. 范围 / 触发条件

- 触发条件：常驻开奖调度一轮内存在多个 API 彩种或多个 API 期号需要读取开奖源。
- 范围：`run_draw_automation`、API 最新期号读取、API 开奖号码读取、串行开奖结算链路。
- 并发只允许发生在外部 API 读取阶段，不能扩大到资金和订单写入阶段。

### 2. 契约

- 调度器必须先收集本轮到期开奖候选，再并发读取 API 最新期号。
- API 最新期号读取结果用于旧期号距离判断；读取失败时保持保守行为，不执行距离跳过，继续尝试读取开奖号码。
- 对未被旧期规则跳过、且没有后台控奖号码的 API 期号，并发读取 API 开奖号码。
- API 开奖号码读取失败只跳过对应期号，并写入 `skippedIssues`，不得中断其它候选期号。
- 开奖期状态写入、订单结算、派奖入账、合买状态更新必须按候选期号原顺序串行执行。
- 后台控奖号码优先级必须高于并发预取的 API 开奖号码。
- API 彩种可通过 `LotteryKind.apiDrawDelaySeconds` 配置开奖源延迟秒数；该延迟只影响“是否进入 API 开奖请求候选”，不能改变 `scheduledAt`、`saleClosedAt` 或移动端倒计时。
- 未到 `scheduledAt + apiDrawDelaySeconds` 时，调度器仍可正常封盘，但不得请求第三方 API，也不得把“等待延迟到点”写成跳过明细。

### 3. 错误与边界

| 条件 | 预期行为 |
|------|----------|
| 某个 API 最新期号读取失败 | 记录中文 warning，该彩种本轮不做旧期距离跳过 |
| 某个 API 开奖号码读取失败 | 该期写入跳过原因，其它期继续开奖 |
| API 期号超过最新期号 5 期 | 不预取旧期开奖号码，直接跳过旧期 |
| 后台控奖号码已启用 | 不预取 API 开奖号码，开奖时使用控奖号码 |
| API 彩种配置延迟且尚未到延迟后时间 | 只做封盘等到期动作，不进入 API 最新期号或开奖号码预取 |
| 非 API 彩种 | 不进入 API 预取流程 |

### 4. 必要测试

- 后端需要覆盖 API 旧期跳过、API 缺失当前期跳过、已预取号码可完成 API 开奖。
- 后端需要覆盖配置开奖源延迟时“先封盘不开奖、延迟到点后再请求 API 并开奖”。
- 后端完整测试必须继续通过，确保并发读取没有破坏资金、订单和派奖串行一致性。

---

## 场景：开奖调度快路径不得被慢业务阻塞

### 1. 范围 / 触发条件

- 触发条件：常驻开奖调度每秒执行，彩种到达封盘时间或需要生成下一期。
- 范围：`spawn_draw_scheduler`、`close_due_draw_issues`、`refund_closed_unfilled_group_buys`、`draw_due_issues`、`ensure_*_future_draw_issues`、手机端首页和下注页当前期选择。
- 目标：保证封盘、补下一期和 `issue_opened` 推送优先完成，避免下注页长期停在“开奖中”。

### 2. 快慢阶段契约

- 快阶段只能执行：
  - 到期 `open` 期号改为 `closed`。
  - 非 API 彩种本地补齐未来 `open` 期号。
  - API 彩种并发预览下一期，再串行写入新 `open` 期号。
  - 发布 `lottery.issue_closed` 和 `lottery.issue_opened`。
- 慢阶段才能执行：
  - 封盘未满员合买取消与退款。
  - API/平台/手动开奖。
  - 订单结算、派奖入账、合买结算。
  - 合买机器人和购彩机器人。
- 常驻调度中慢阶段必须后台执行并串行互斥；慢阶段未结束时，下一轮只能跳过慢阶段启动，不能阻塞快阶段。
- 慢阶段仍需发布余额、订单和开奖相关实时事件，但不得影响下一期生成。

### 3. 持久化契约

- `draw_issues` 的创建、封盘、开奖和取消属于高频路径，必须使用单行 upsert 持久化。
- 高频路径不得调用会删除并重写整张 `draw_issues` 表的全量保存函数。
- 全量保存只允许用于低频批处理或显式同步场景，使用前需要确认不会在每秒调度路径触发。

### 4. 当前期选择契约

- 手机端首页当前期选择顺序：
  - 优先选择 `status=open` 且 `saleClosedAt > now` 的最早可售期。
  - 没有可售期时，选择最新的 `closed` 或已过封盘时间的 `open` 期，展示为等待开奖。
  - 最后才展示最近已开奖期。
- 手机端下注页当前期选择顺序：
  - `status=open` 且 `saleClosedAt > now` 返回 `selling`。
  - 已过封盘时间的 `open` 或 `closed` 返回 `opening`，触发前端静默轮询。
  - 前端不能只依赖 `round.status=opening` 才轮询；`selling` 但本地判断 `saleStopAt <= now` 时也必须轮询。

### 5. 错误与边界

| 条件 | 预期行为 |
|------|----------|
| 慢阶段还在执行 | 快阶段继续按配置周期执行，慢阶段启动被跳过并记录 debug 日志 |
| 封盘时存在未满员合买 | 快阶段只关闭期号，退款在慢阶段处理 |
| API 下一期预览慢或失败 | 不影响非 API 彩种补期；该 API 彩种写入跳过原因 |
| `draw_issues` 数量增长到上万条 | 单次封盘/补期不得因全表重写拖到数十秒 |
| 首页存在多个历史 `closed` 期号 | 不得选择最早旧期作为当前期 |

### 6. 必要测试

- 后端需要覆盖平台彩种在 API 预览失败时仍能生成下一期。
- 后端需要覆盖首页当前期不会被历史 `closed` 旧期压住。
- 后端需要覆盖封盘流单退款仍会在完整自动化链路中执行。
- 本地联调需要至少观察 `魔力分分彩` 和 `腾讯分分彩` 跨过一次封盘点，确认 `/api/lottery/home` 与 `/api/user/bet/page-config/{lottery_id}` 都返回下一期 `selling`。

---

## 场景：后台手动刷新内存缓存

### 1. 范围 / 触发条件

- 触发条件：管理员手动清空或直接修改 PostgreSQL 业务表后，需要让运行中的后端快照型仓储重新读取数据库。
- 范围：后台系统设置页、`AppState` 仓储刷新入口、各快照型仓储的 `reload_from_database` 方法。
- 不适用场景：普通业务变更仍应通过后台或用户端 API 完成，不应依赖直接改库加刷新缓存。

### 2. 签名

- 接口：`POST /api/admin/system-settings/cache/reload`
- 权限：`PermissionScope::SystemSettings`
- 响应：

```json
{
  "reloadedModules": ["用户权限与系统设置"],
  "databaseDirectModules": ["彩种配置"],
  "skippedModules": [],
  "refreshedAt": "2026-06-10 22:46:00"
}
```

### 3. 契约

- 后端必须重新加载所有配置了 `BusinessDatabase` 的快照型仓储，并用数据库内容替换当前内存快照。
- 彩种配置在 PostgreSQL 模式下是数据库直读，返回到 `databaseDirectModules`，不得伪装为已替换内存。
- 访问控制仓储必须最后刷新，避免当前维护接口执行中途先替换管理员会话。
- 未配置 `DATABASE_URL` 时返回业务错误“当前服务未启用数据库持久化，无法刷新内存缓存”。
- 刷新成功后后台页面需要重新拉取系统设置、用户、管理员、角色等数据。

### 4. 错误与边界

| 条件 | 预期行为 |
|------|----------|
| 数据库模式运行 | 返回已刷新模块和数据库直读模块 |
| 内存模式运行 | 返回业务错误，不显示成功 Toast |
| 某个仓储加载失败 | 接口返回该仓储的中文错误，不继续提示成功 |
| 管理员会话表被手动清空 | 本次请求可返回，后续请求可能需要重新登录 |
| 调度器正在运行 | 刷新只替换仓储快照，不主动暂停调度 |

### 5. 必要测试

- 后端需要运行 `cargo fmt --check`、`cargo check` 和相关路由或仓储测试。
- 后台需要运行 `npm run build`，确认接口类型和按钮交互编译通过。
- 本地联调时应在 PostgreSQL 模式调用接口，确认返回 `reloadedModules` 和 `databaseDirectModules`。
