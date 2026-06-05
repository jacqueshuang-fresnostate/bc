# 数据库规范

> 本项目的数据库模式和约定。

---

## 概览

后端支持两种运行模式：未配置 `DATABASE_URL` 时使用内存演示数据；配置 `DATABASE_URL` 时使用 PostgreSQL + SQLx migrations。彩种、玩法赔率和其它已落地后台模块都使用业务表持久化，运行时不再读写 `state_documents`。

不要把数据库访问直接写进 HTTP 处理函数。路由处理函数调用服务，服务再调用仓储或查询模块。

当前后端使用 Rust `1.92.0` 检查，SQLx `0.9.0` 要求 Rust `1.94.0`，因此当前依赖使用 SQLx `0.8.6`。升级 Rust 工具链前不要把 SQLx 提升到 `0.9`。

---

## 查询模式

- SQL 放在仓储或查询模块中，不放在路由处理函数中。
- 任何会改变余额、订单、派奖、返利或机器人购彩记录的操作必须使用事务。
- 使用类型清晰的 ID，或至少使用命名明确的 `String` ID；避免跨层传递匿名元组。
- 接口只查询自己需要的字段。
- 持久化列表接口从第一版开始就需要分页。

---

## 迁移

后续迁移文件应放在 `backend/migrations/`，并和依赖它的代码一起提交。

后端启动时如果配置 `DATABASE_URL`，需要运行 SQLx migrations；未配置时允许使用内存演示仓储。

迁移命名格式：

```text
YYYYMMDDHHMMSS_describe_change.sql
```

如果使用 SQLx 标准模式，每个迁移只需要前向 SQL。若后续选择其他迁移工具，需要先更新本规范。

## 安全字段存储

- `admin_sessions.token` 和 `user_sessions.token` 只能保存登录 Bearer token 的 `sha256:` 摘要，不能保存原始 token。
- 登录接口返回给客户端的原始 token 必须是 opaque token，不能包含用户 ID、管理员 ID、用户名、时间戳或计数器。
- 认证、登出和会话清理都必须先对请求中的原始 token 计算同样的摘要，再访问会话表或会话索引。
- 如果历史迁移发现会话表已经保存过原始 token，应优先清空旧会话并要求重新登录，不能继续让旧明文登录态长期存在。
- `system_settings` 中的 token、key、secret、password 等敏感配置种子值只能为空字符串或“未配置”，不能写入真实第三方密钥。

---

## 命名约定

- 表名使用复数 `snake_case`，例如 `users`、`lotteries`、`lottery_draws`。
- 列名使用 `snake_case`。
- 主键使用 `id`。
- 外键使用 `<singular_table>_id`，例如 `user_id`、`lottery_id`。
- 时间字段使用 `created_at` 和 `updated_at`。
- 金额字段使用整数最小货币单位或 decimal 类型，不能使用浮点数。

---

## 常见错误

- 不要用 `f32` 或 `f64` 存储金额。
- 不要在没有事务的情况下更新余额。
- 不要让机器人操作绕过真实用户订单和财务规则。
- 不要新增数据库行为却不更新 API 和前端假设。
- 不要在路由函数里直接写 SQL；应放在仓储或查询模块。
- 不要假设 `DATABASE_URL` 一定存在；当前本地开发允许无数据库内存模式。
- 不要把图床 token、支付密钥、短信密钥、OpenAPI 密钥等真实敏感值写入种子数据、迁移文件、测试夹具或 TODO/架构文档。

---

## 场景：系统设置敏感配置默认值

### 1. 范围 / 触发条件

- 触发条件：新增或修改 `system_settings` 中的第三方接口 token、支付 key、secret、password、回调签名密钥、对象存储凭据等配置。
- 范围：`seed_settings()`、`system_settings` 迁移、后台系统设置页面、图床/支付/外部 API 配置。

### 2. 签名

- 系统设置表：`system_settings(key, value, description)`
- 图床授权配置：`image_bed_authorization_token`
- 彩虹易支付密钥配置：`recharge_rainbow_epay_key`
- 系统设置接口：`GET /api/admin/system-settings`、`PATCH /api/admin/system-settings/{key}`

### 3. 契约

- 敏感配置的种子默认 `value` 必须为空字符串或“未配置”。
- 迁移文件只能写入占位值，不能写入真实密钥、真实 token、真实商户私钥或用户提供过的完整授权值。
- 发现历史默认值泄露时，后续迁移必须清空该配置，并在 TODO/架构说明中提醒部署后重新配置和轮换密钥。
- 上传、支付等运行时逻辑读取到空敏感配置时必须返回中文业务错误，例如“图床上传 Token 未配置”，不能静默使用假值。

### 4. 校验与错误矩阵

| 条件 | 预期行为 |
|------|----------|
| 空库启动 | 敏感配置为空或“未配置”，后台提示需要配置 |
| 后台保存新密钥 | `PATCH /api/admin/system-settings/{key}` 写入管理员输入值 |
| 运行时调用依赖敏感配置的能力但配置为空 | 返回中文业务错误，不请求第三方接口 |
| 历史数据库已有误写默认密钥 | 后续迁移清空该值，要求管理员重新配置 |
| 源码搜索出现真实 token 前缀或完整密钥 | 审查失败，必须移除并建议轮换密钥 |

### 5. Good/Base/Bad Cases

- Good：`image_bed_authorization_token` 默认值为空，管理员在系统设置中手动填入后再测试上传。
- Base：`recharge_rainbow_epay_key` 默认值为“未配置”，未配置前彩虹易支付下单返回配置错误。
- Bad：把用户提供的真实图床 Bearer token 写入 `seed_settings()` 或迁移文件。

### 6. Tests Required

- 后端变更后运行 `cargo fmt --check`、`cargo check` 和 `cargo test`。
- 安全审查需要运行源码搜索，确认真实 token、key、secret 没有出现在可提交文件中。
- 修改前端系统设置展示时运行管理后台 `npm run build`。

### 7. Wrong vs Correct

#### Wrong

```rust
SystemSetting {
    key: "image_bed_authorization_token".to_string(),
    value: "真实第三方 token".to_string(),
    description: "图床请求 Authorization Token".to_string(),
}
```

#### Correct

```rust
SystemSetting {
    key: "image_bed_authorization_token".to_string(),
    value: String::new(),
    description: "图床请求 Authorization Token（不含 Bearer 前缀，必须在后台手动配置）".to_string(),
}
```

## 场景：用户资金账户生命周期

### 1. 范围 / 触发条件

- 触发条件：新增用户创建入口、用户注册入口、财务余额校验、投注扣款、充值入账或财务账户持久化恢复逻辑。
- 范围：`users`、`financial_accounts`、`FinanceRepository::account_or_create`、用户端注册接口、后台用户创建接口、订单扣款前余额校验。

### 2. 签名

- 用户端注册：`POST /api/user/register`
- 后台新建用户：`POST /api/admin/users`
- 用户余额：`GET /api/user/balance`
- 财务账户表：`financial_accounts`
- 用户表：`users`
- 账户初始化方法：`FinanceRepository::account_or_create(user_id)`

### 3. 契约

所有用户创建成功后必须拥有一条 0 余额资金账户：

```json
{
  "userId": "U90004",
  "availableBalanceMinor": 0,
  "frozenBalanceMinor": 0
}
```

用户端注册和后台新建用户都必须在用户写入成功后调用 `finance.account_or_create(&user.id)`。配置 PostgreSQL 时，财务仓储启动加载必须读取 `users` 表，并为缺失 `financial_accounts` 的历史用户补齐 0 余额账户且持久化，避免旧数据在投注或余额查询时暴露内部缺表错误。

余额校验遇到缺失账户时必须按 0 余额处理，返回余额不足，不返回 `financial account not found` 给用户端。

### 4. 校验与错误矩阵

| 条件 | 预期行为 |
|------|----------|
| 新用户注册成功 | 自动创建 0 余额资金账户 |
| 后台新建用户成功 | 自动创建 0 余额资金账户 |
| 历史用户缺少资金账户 | 服务启动加载财务仓储时自动补齐并保存 |
| 用户余额不足 | 返回余额不足业务错误 |
| 用户 ID 为空 | 返回 `user id is required` |
| 金额小于等于 0 | 返回 `amount must be greater than zero` |

### 5. Good / Base / Bad Cases

- Good：用户注册成功后立即请求 `/api/user/balance`，返回当前用户和 0 余额账户。
- Base：旧数据库中用户已有但账户缺失，服务启动后后台财务账户列表能看到该用户账户。
- Bad：只创建 `users` 记录，不创建 `financial_accounts`，导致投注时报 `financial account not found`。
- Bad：把缺账户错误直接返回给手机端；这暴露了内部持久化结构，也让用户误以为账号异常。

### 6. 必要测试

- 财务仓储单元测试必须覆盖 `account_or_create` 创建 0 余额账户。
- 财务仓储单元测试必须覆盖缺账户用户扣款时返回余额不足。
- 用户注册或后台新建用户链路变更后，需要通过 API 冒烟验证 `/api/user/balance` 能读取新用户账户。

### 7. Wrong vs Correct

#### 错误

```rust
let user = state.access.register_user(payload).await?;
Ok(Json(ApiEnvelope::success(user)))
```

这个写法只创建用户，不创建资金账户，后续投注扣款会在财务模块找不到账户。

#### 正确

```rust
let user = state.access.register_user(payload).await?;
state.finance.account_or_create(&user.id).await?;
Ok(Json(ApiEnvelope::success(user)))
```

用户生命周期入口负责触发财务账户初始化，财务模块负责保证账户存在且可持久化。

---

## 场景：彩种数据库持久化

### 1. 范围 / 触发条件

- 触发条件：彩种管理从内存仓储升级为可选 PostgreSQL 持久化。
- 范围：`DATABASE_URL`、SQLx 依赖、`backend/migrations/`、`lotteries` 表、彩种仓储选择。

### 2. 签名

- 环境变量：`DATABASE_URL`
- 迁移目录：`backend/migrations/`
- 首个迁移：`20260602150315_create_lotteries.sql`
- 玩法赔率迁移：`20260602165000_add_lottery_play_configs.sql`
- 表名：`lotteries`

### 3. 契约

`DATABASE_URL` 未配置时，后端必须使用内存彩种仓储并能正常启动。

`DATABASE_URL` 已配置时，后端必须：

- 创建 PostgreSQL 连接池。
- 运行 SQLx migrations。
- 如果 `lotteries` 表为空，写入种子彩种。
- 如果 `lotteries` 表已有数据，不覆盖已有彩种。

Docker Compose 本地部署必须提供 PostgreSQL 服务和持久化 volume，并把应用 `DATABASE_URL` 指向 Compose 网络内的数据库。彩种管理和玩法赔率配置使用 PostgreSQL `lotteries` 关系表；其它当前已落地业务模块使用各自独立业务表。运行日志和文档必须明确所有运行时业务读写已经脱离 `state_documents`，旧状态文档迁移只作为历史迁移记录保留。

`lotteries` 表字段：

| 字段 | 类型 | 说明 |
|------|------|------|
| `id` | `text primary key` | 彩种 ID |
| `name` | `text not null` | 彩种名称 |
| `number_type` | `text not null` | `threeDigit`、`fiveDigit`、`pk10`、`elevenFive`、`fastThree` 或 `luckTwenty` |
| `draw_mode` | `text not null` | `platform`、`api` 或 `manual` |
| `schedule` | `jsonb not null` | 当前 API 契约中的开奖时间对象 |
| `sale_enabled` | `boolean not null` | 销售状态 |
| `group_buy` | `jsonb not null` | 当前 API 契约中的合买配置 |
| `play_categories` | `jsonb not null` | 当前 API 契约中的玩法数组；暂未接入投注玩法的号码类型保存为空数组 |
| `play_configs` | `jsonb not null default '[]'::jsonb` | 单玩法启用状态和赔率配置数组；暂未接入投注玩法的号码类型保存为空数组 |
| `created_at` | `timestamptz not null default now()` | 创建时间 |
| `updated_at` | `timestamptz not null default now()` | 更新时间 |

### 4. 校验与错误矩阵

| 条件 | 预期行为 |
|------|----------|
| 未配置 `DATABASE_URL` | 使用内存仓储，后端继续启动 |
| 配置 `DATABASE_URL` 且连接失败 | 启动失败，不降级为内存模式 |
| 迁移失败 | 启动失败，不继续提供接口 |
| 数据库中彩种 JSON 无法反序列化 | 返回内部错误信封，不暴露 SQL 或堆栈 |
| 创建重复彩种 ID | 返回 HTTP 409 |

### 5. Good / Base / Bad Cases

- Good：配置测试数据库后启动服务，迁移自动运行，空表写入种子彩种，彩种 CRUD 可以跨重启保留。
- Base：未配置 `DATABASE_URL` 时，服务继续用种子内存数据支持管理后台预览。
- Bad：数据库连接失败时静默降级为内存模式；这会让运维误以为数据已持久化。

### 6. 必要测试

- 无数据库模式需要运行 `cargo test` 并确认内存仓储仍返回种子彩种。
- 数据库行映射需要测试枚举字符串、`schedule`、`group_buy`、`play_categories`、`play_configs` 的序列化/反序列化。
- 数据库集成测试只能在显式配置测试环境变量时运行，避免误写生产数据库。

### 7. Wrong vs Correct

#### 错误

```rust
if let Err(error) = connect_database().await {
    tracing::warn!(%error, "fallback to memory");
    return AppState::new();
}
```

配置了 `DATABASE_URL` 却连接失败时继续降级，会掩盖持久化不可用的问题。

#### 正确

```rust
let Ok(database_url) = std::env::var("DATABASE_URL") else {
    return Ok(AppState::new());
};

let lotteries = LotteryRepository::postgres(&database_url).await?;
```

只有未配置数据库时才使用内存模式；配置后连接失败必须让启动失败。

---

## 场景：全后台业务表持久化

### 1. 范围 / 触发条件

- 触发条件：当前已落地的后台模块需要在配置 `DATABASE_URL` 后跨服务重启恢复数据，并且不能再使用 `state_documents` 作为运行时业务状态入口。
- 范围：`BusinessDatabase`、`backend/migrations/20260603152000_create_business_tables.sql`、所有业务仓储的业务表加载和保存逻辑。

### 2. 签名

- 迁移文件：`backend/migrations/20260603152000_create_business_tables.sql`
- 数据库包装：`backend/src/services/business_database.rs`
- 测试数据库环境变量：`BC_TEST_DATABASE_URL`

### 3. 契约

`DATABASE_URL` 未配置时，后端必须继续使用内存业务仓储。

`DATABASE_URL` 已配置时，后端必须：

- 运行 SQLx migrations。
- 使用各业务表保存当前后台模块状态。
- 数据库中对应业务表为空时写入该模块原有种子状态。
- 数据库已有对应业务表数据时恢复数据库状态，不覆盖为种子数据。
- 写操作成功后用事务保存对应模块涉及的业务表。
- 启动连接或迁移失败时直接启动失败，不静默降级。

当前业务表：

| 模块 | 表 |
|------|------|
| 用户权限 | `users`、`admin_roles`、`admins`、`admin_password_hashes`、`admin_sessions`、`system_settings`、`registration_config`、`access_runtime` |
| 开奖 | `draw_issues`、`draw_controls`、`draw_sources` |
| 订单结算 | `orders`、`order_settlement_runs`、`order_settlements`、`order_runtime` |
| 资金 | `financial_accounts`、`ledger_entries`、`finance_runtime` |
| 合买 | `group_buy_plans`、`group_buy_participants` |
| 邀请返利 | `invite_records`、`rebate_policy` |
| 机器人 | `robot_configs`、`robot_lottery_bindings` |
| 客服 | `support_conversations`、`support_messages` |
| 调度 | `draw_scheduler_config`、`draw_scheduler_runs`、`draw_scheduler_runtime` |

复杂业务字段可以继续用 JSONB 列保存当前 API 契约中的结构，例如角色权限范围、投注选择、展开投注、中奖匹配和 API 开奖源复用彩种；不得把整个模块重新塞回单张状态文档表。

### 4. 校验与错误矩阵

| 条件 | 预期行为 |
|------|----------|
| 业务表为空 | 写入原有种子数据 |
| 业务表已有数据 | 恢复数据库状态，不写入种子覆盖 |
| 枚举字符串或 JSONB 字段反序列化失败 | 返回内部错误，不继续用种子覆盖 |
| 事务保存失败 | 当前接口返回内部错误 |
| 未配置 `DATABASE_URL` | 所有业务仓储继续内存模式 |

### 5. Good / Base / Bad Cases

- Good：后台新增订单、调整资金、保存开奖控制号码后重启服务，配置 `DATABASE_URL` 时数据仍可恢复。
- Base：本地未配置数据库时，全部页面仍可使用种子内存数据。
- Bad：数据库已有业务表数据时重新写入种子，导致用户、订单、资金或控制号码被覆盖。
- Bad：运行时继续引用 `StateDocumentRepository` 或向 `state_documents` 写入业务状态。

### 6. 必要测试

- 无数据库模式必须继续通过 `cargo test`。
- 业务表仓储需要测试至少一个模块的保存和重新加载恢复。
- 数据库集成测试只能在显式配置 `BC_TEST_DATABASE_URL` 时运行，避免误写生产数据库。

### 7. Wrong vs Correct

#### 错误

```rust
let store = AccessStore::seeded();
save_access_store(&database, &store).await?;
```

服务每次启动都直接保存种子状态，会覆盖数据库中已经存在的真实用户、管理员密码哈希、会话和权限配置。

#### 正确

```rust
let store = load_access_store(&database).await?;
```

先尝试读取已有业务表数据；只有业务表为空时才写入种子，避免重启覆盖真实业务数据。

### 8. 后续增强要求

所有运行时业务模块已经迁移为业务表。订单、资金流水、开奖期号、结算批次、管理员权限和高风险开奖控制后续增强时，必须继续补事务一致性、索引、审计字段、分页查询、备份恢复和从历史 `state_documents` 迁移数据的说明。

---

## 场景：充值订单数据库持久化

### 1. 范围 / 触发条件

- 触发条件：新增用户充值、彩虹易支付回调和客服直充订单，需要跨重启保存订单状态、第三方交易号、客服会话绑定和运行时序号。
- 范围：`backend/migrations/20260605006000_create_recharge_orders.sql`、`RechargeRepository`、`FinanceRepository::credit_recharge`、`system_settings` 充值配置键。

### 2. 签名

- 迁移文件：`backend/migrations/20260605006000_create_recharge_orders.sql`
- 业务表：`recharge_orders`
- 运行时表：`recharge_runtime`
- 资金流水类型：`ledger_entries.kind = rechargeCredit`
- 系统设置键：
  - `recharge_min_amount_minor`
  - `recharge_max_amount_minor`
  - `recharge_rainbow_epay_enabled`
  - `recharge_rainbow_epay_gateway_url`
  - `recharge_rainbow_epay_pid`
  - `recharge_rainbow_epay_key`
  - `recharge_rainbow_epay_notify_url`
  - `recharge_rainbow_epay_return_url`
  - `recharge_rainbow_epay_pay_types`
  - `recharge_customer_service_enabled`
  - `recharge_customer_service_message`

### 3. 契约

`recharge_orders` 字段：

| 字段 | 类型 | 说明 |
|------|------|------|
| `id` | `text primary key` | 充值订单 ID |
| `user_id` | `text not null` | 用户 ID |
| `username` | `text not null` | 下单时用户名快照 |
| `channel` | `text not null` | `rainbowEpay` 或 `customerService` |
| `amount_minor` | `bigint not null` | 充值金额（分） |
| `status` | `text not null` | `pending`、`waitingCustomerService`、`paid` 或 `cancelled` |
| `pay_type` | `text` | 彩虹易支付方式 |
| `provider_trade_no` | `text` | 第三方交易号 |
| `payment_url` | `text` | 彩虹易支付跳转地址 |
| `support_conversation_id` | `text` | 客服直充绑定会话 ID |
| `created_at` | `text not null` | 创建时间 |
| `paid_at` | `text` | 入账时间 |

`recharge_runtime` 使用 `key/value` 保存 `next_sequence`。支付网关配置必须在 `system_settings`，不要使用 `API_*` 或支付相关环境变量保存运行时业务配置。

### 4. 校验与错误矩阵

| 条件 | 预期行为 |
|------|----------|
| `amount_minor <= 0` | 数据库约束拒绝，服务层也应先返回 HTTP 400 |
| `channel` 非枚举值 | 数据库约束拒绝，服务层不应生成 |
| `status` 非枚举值 | 数据库约束拒绝，服务层不应生成 |
| 数据库中充值表为空 | 不写种子充值单，返回空订单列表 |
| 数据库已有充值订单 | 启动时恢复订单和 `next_sequence`，不覆盖已有订单 |
| 支付通知重复 | `FinanceRepository::credit_recharge` 按充值单 ID 幂等返回既有流水 |

### 5. Good / Base / Bad Cases

- Good：配置 `DATABASE_URL` 后创建客服直充订单，服务重启后 `GET /api/admin/recharge-orders` 仍能看到该订单。
- Good：彩虹易支付回调成功后，`recharge_orders.status=paid`，同时 `ledger_entries.kind=rechargeCredit`。
- Base：未配置 `DATABASE_URL` 时充值仓储为空内存状态，仍可创建演示充值订单，但不跨重启保留。
- Bad：把充值订单塞回 `state_documents`；这违反“所有业务表持久化”的要求。
- Bad：只更新 `financial_accounts` 不写 `ledger_entries`；后台无法审计充值入账来源。

### 6. 必要测试

- 无数据库模式需要运行 `cargo test`，确认充值仓储可创建彩虹和客服直充订单。
- 资金仓储需要测试同一充值单重复入账只生成一条 `rechargeCredit`。
- PostgreSQL 冒烟需要启动服务并确认迁移执行、充值表可读写、客服会话可绑定。
- SQL 迁移必须给新增表、字段和约束添加中文注释。

### 7. Wrong vs Correct

#### 错误

```rust
let gateway = std::env::var("RAINBOW_EPAY_GATEWAY_URL")?;
```

支付网关配置放在环境变量里，后台无法修改，也不符合业务配置数据库化要求。

#### 正确

```rust
let settings = access.settings().await?;
let recharge_settings = recharge_settings_from_system_settings(&settings);
```

充值运行配置必须从 `system_settings` 读取，由后台维护并随数据库持久化。

#### 错误

```sql
CREATE TABLE recharge_orders (...);
```

新增 SQL 没有字段注释，后续排查和数据库审计会变困难。

#### 正确

```sql
COMMENT ON COLUMN recharge_orders.support_conversation_id IS '客服直充绑定的客服会话 ID';
```

新增业务表必须为表、字段和约束补中文注释。

---

## 场景：提现申请数据库持久化

### 1. 范围 / 触发条件

- 触发条件：用户端新增提现申请接口，提交申请后需要跨重启保存申请状态和收款方式快照。
- 范围：`backend/migrations/20260605007000_create_withdrawal_orders.sql`、`WithdrawalRepository`、`FinanceRepository::freeze_withdrawal`、`FinanceRepository::approve_withdrawal`、`FinanceRepository::reject_withdrawal`。

### 2. 签名

- 迁移文件：`backend/migrations/20260605007000_create_withdrawal_orders.sql`
- 业务表：`withdrawal_orders`
- 运行时表：`withdrawal_runtime`
- 提现冻结流水类型：`ledger_entries.kind = withdrawalFreeze`
- 提现打款流水类型：`ledger_entries.kind = withdrawalPayout`
- 提现驳回解冻流水类型：`ledger_entries.kind = withdrawalReject`

### 3. 契约

`withdrawal_orders` 字段：

| 字段 | 类型 | 说明 |
|------|------|------|
| `id` | `text primary key` | 提现申请 ID |
| `user_id` | `text not null` | 用户 ID |
| `username` | `text not null` | 用户名快照 |
| `method_id` | `text not null` | 用户提现方式 ID |
| `method_type` | `text not null` | `alipay`、`wechat` 或 `bankCard` |
| `account_holder` | `text not null` | 收款账户名快照 |
| `account_number` | `text not null` | 收款账号或银行卡号快照 |
| `bank_name` | `text` | 银行卡所属银行名称 |
| `amount_minor` | `bigint not null` | 提现金额（分） |
| `status` | `text not null` | `pending`、`approved`、`rejected` 或 `cancelled` |
| `created_at` | `text not null` | 创建时间 |
| `reviewed_at` | `text` | 审核时间 |

`withdrawal_runtime` 使用 `key/value` 保存 `next_sequence`。提现申请提交时必须同时冻结财务账户可用余额并写 `withdrawalFreeze` 流水，流水 `reference_id` 使用提现申请 ID。

后台审核提现时必须继续围绕同一个提现申请 ID 保持流水幂等：

- 通过申请：`financial_accounts.frozen_balance_minor -= amountMinor`，写入 `withdrawalPayout` 流水，提现申请状态变为 `approved`，`reviewed_at` 写入审核时间。
- 驳回申请：`financial_accounts.frozen_balance_minor -= amountMinor`，`financial_accounts.available_balance_minor += amountMinor`，写入 `withdrawalReject` 流水，提现申请状态变为 `rejected`，`reviewed_at` 写入审核时间。
- 重复点击同一个审核结果必须返回既有流水或既有状态，不重复扣减或解冻。

### 4. 校验与错误矩阵

| 条件 | 预期行为 |
|------|----------|
| `amount_minor <= 0` | 数据库约束拒绝，服务层也应先返回 HTTP 400 |
| `method_type` 非枚举值 | 数据库约束拒绝 |
| `status` 非枚举值 | 数据库约束拒绝 |
| 数据库中提现表为空 | 不写种子提现单，返回空列表 |
| 数据库已有提现申请 | 启动时恢复申请和 `next_sequence`，不覆盖已有申请 |
| 可用余额不足 | 不创建提现申请，不写冻结流水 |
| 冻结余额不足 | 不允许通过或驳回提现申请 |
| 已通过提现申请再次驳回 | HTTP 400，不改变状态和余额 |
| 已驳回提现申请再次通过 | HTTP 400，不改变状态和余额 |

### 5. Good / Base / Bad Cases

- Good：配置 `DATABASE_URL` 后提交提现申请，重启服务后 `GET /api/user/withdrawals` 仍能看到该申请。
- Good：提现申请保存收款方式快照，后续用户修改提现方式不会改变历史申请的账号信息。
- Good：后台通过提现申请后冻结余额扣减，资金流水生成 `withdrawalPayout`，申请状态为 `approved`。
- Good：后台驳回提现申请后冻结余额退回可用余额，资金流水生成 `withdrawalReject`，申请状态为 `rejected`。
- Base：未配置 `DATABASE_URL` 时使用内存提现仓储，支持本地功能验证但不跨重启保留。
- Bad：提现申请只写 `withdrawal_orders`，不冻结 `financial_accounts`；用户仍能继续使用同一笔余额投注或再次提现。
- Bad：审核提现时只更新申请状态，不处理冻结余额；这会让财务账户和提现状态不一致。
- Bad：把提现申请塞回 `state_documents`；这违反所有运行时业务表持久化要求。

### 6. 必要测试

- 无数据库模式需要覆盖提现仓储生成 `pending` 申请。
- 财务仓储需要覆盖 `withdrawalFreeze` 减少可用余额、增加冻结余额。
- 财务仓储需要覆盖 `withdrawalPayout` 扣减冻结余额，以及 `withdrawalReject` 解冻回可用余额。
- 提现仓储需要覆盖审核状态流转和反向审核拒绝。
- PostgreSQL 冒烟需要确认迁移执行、提现表可读写、冻结余额跨重启恢复。
- SQL 迁移必须给新增表、字段和约束添加中文注释。

### 7. Wrong vs Correct

#### 错误

```sql
CREATE TABLE withdrawal_orders (...);
```

新增表没有字段注释，后续排查和审计困难。

#### 正确

```sql
COMMENT ON COLUMN withdrawal_orders.amount_minor IS '提现金额（分），必须大于 0';
```

新增业务表必须为每个字段和约束补中文注释。

#### 错误

```rust
store.insert_order(order)?;
```

只保存提现申请，不冻结余额，会造成重复使用资金。

#### 正确

```rust
finance.freeze_withdrawal(&order.user_id, order.amount_minor, &order.id).await?;
store.insert_order(order)?;
```

提现申请必须与财务冻结配套执行，后续审核流程再负责打款、驳回或解冻。
