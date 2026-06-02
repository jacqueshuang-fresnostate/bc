# 数据库规范

> 本项目的数据库模式和约定。

---

## 概览

首个后端里程碑只使用内存演示数据。后续接入持久化时，优先选择 PostgreSQL + SQLx migrations，让 Rust 类型和 SQL 都保持显式。

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

---

## 场景：彩种数据库持久化

### 1. 范围 / 触发条件

- 触发条件：彩种管理从内存仓储升级为可选 PostgreSQL 持久化。
- 范围：`DATABASE_URL`、SQLx 依赖、`backend/migrations/`、`lotteries` 表、彩种仓储选择。

### 2. 签名

- 环境变量：`DATABASE_URL`
- 迁移目录：`backend/migrations/`
- 首个迁移：`20260602150315_create_lotteries.sql`
- 表名：`lotteries`

### 3. 契约

`DATABASE_URL` 未配置时，后端必须使用内存彩种仓储并能正常启动。

`DATABASE_URL` 已配置时，后端必须：

- 创建 PostgreSQL 连接池。
- 运行 SQLx migrations。
- 如果 `lotteries` 表为空，写入种子彩种。
- 如果 `lotteries` 表已有数据，不覆盖已有彩种。

`lotteries` 表字段：

| 字段 | 类型 | 说明 |
|------|------|------|
| `id` | `text primary key` | 彩种 ID |
| `name` | `text not null` | 彩种名称 |
| `number_type` | `text not null` | `threeDigit` 或 `fiveDigit` |
| `draw_mode` | `text not null` | `platform`、`api` 或 `manual` |
| `schedule` | `jsonb not null` | 当前 API 契约中的开奖时间对象 |
| `sale_enabled` | `boolean not null` | 销售状态 |
| `group_buy` | `jsonb not null` | 当前 API 契约中的合买配置 |
| `play_categories` | `jsonb not null` | 当前 API 契约中的玩法数组 |
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
- 数据库行映射需要测试枚举字符串、`schedule`、`group_buy`、`play_categories` 的序列化/反序列化。
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
