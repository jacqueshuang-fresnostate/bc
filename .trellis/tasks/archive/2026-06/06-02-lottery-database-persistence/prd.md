# 彩种数据库持久化

## 目标

把上一阶段的内存彩种仓储升级为可持久化的 PostgreSQL + SQLx 仓储，让彩种新增、编辑、删除和销售状态切换在配置 `DATABASE_URL` 后可以保存到数据库；同时保留无数据库环境下的内存模式，保证本地开发、测试和前端预览不被外部数据库阻塞。

## 已知信息

- 当前项目已完成 Rust 后端、React 管理后台和彩种管理 CRUD。
- 后端彩种数据目前由 `Arc<RwLock<LotteryStore>>` 保存，服务重启会恢复种子数据。
- `架构设计.md` 的 `# 后续` 明确下一步包括把内存彩种仓储替换为数据库持久化。
- `.trellis/spec/backend/database-guidelines.md` 约定优先使用 PostgreSQL + SQLx migrations。
- 当前文档必须使用中文输出。

## 调研引用

- [`research/sqlx-postgres-migrations.md`](research/sqlx-postgres-migrations.md)：SQLx PostgreSQL、Tokio runtime、迁移支持和本任务依赖建议。

## 临时假设

- 本阶段只持久化彩种配置，不持久化订单、开奖历史、机器人、用户、财务等数据。
- 彩种配置中的 `schedule`、`groupBuy`、`playCategories` 先使用 PostgreSQL `jsonb` 保存，保留当前前后端契约，后续如果玩法和开奖时间模型稳定，再拆成多表。
- `DATABASE_URL` 未配置时，后端继续使用内存仓储并记录日志；`DATABASE_URL` 配置后，后端连接 PostgreSQL、运行迁移并使用数据库仓储。
- 数据库为空时自动写入当前种子彩种；数据库已有彩种时不覆盖已有数据。
- 本阶段不要求启动或配置本机 PostgreSQL 容器，数据库联调仅在 `DATABASE_URL` 可用时执行。

## 需求

- 后端新增 SQLx 依赖，支持 PostgreSQL 连接池和 migrations。
- 后端新增 `backend/migrations/`，创建 `lotteries` 表。
- 后端新增数据库彩种仓储，提供和当前内存仓储一致的列表、详情、创建、更新、删除、销售开关能力。
- 后端启动时根据 `DATABASE_URL` 选择仓储：
  - 未配置：使用内存仓储。
  - 已配置：连接 PostgreSQL、运行迁移、必要时写入种子彩种、使用数据库仓储。
- 后端现有 `/api/admin/lotteries` 与 `/api/admin/dashboard` 响应契约保持不变。
- 后端错误处理继续使用 `ApiError` 和统一 API 信封，不把数据库内部错误细节暴露给前端。
- 更新 `架构设计.md` 和 `TODO.md`，记录本阶段范围、完成内容、问题和时间。
- 如产生新的环境变量、迁移策略或数据库契约，更新 `.trellis/spec/`。

## 验收标准

- [x] `backend/migrations/` 包含 `lotteries` 表迁移，表名和字段命名符合数据库规范。
- [x] 未配置 `DATABASE_URL` 时，后端可以启动并继续返回种子彩种。
- [x] 配置 `DATABASE_URL` 时，后端会创建连接池、运行迁移、使用数据库仓储。
- [x] 数据库为空时会写入种子彩种；数据库非空时不会覆盖已有彩种。
- [x] 彩种 CRUD 和销售开关接口在内存模式下继续通过。
- [x] 数据库仓储的映射、校验和错误处理有测试覆盖；需要数据库的测试必须能在无 `DATABASE_URL` 时安全跳过。
- [x] `GET /api/admin/dashboard` 继续使用同一份彩种仓储数据。
- [x] 管理后台无需修改或仅做必要适配，生产构建通过。
- [x] `cargo fmt --check`、`cargo check`、`cargo test`、`npm run build` 通过。

## 验证记录

- 2026-06-02：无 `DATABASE_URL` 启动后端成功，请求 `/api/health`、`/api/admin/lotteries`、`/api/admin/dashboard` 均通过，彩种数量为 4。
- 2026-06-02：`cargo fmt --check`、`cargo check`、`cargo test` 通过，后端 11 个测试全绿；`BC_TEST_DATABASE_URL` 未配置时数据库集成测试安全跳过。
- 2026-06-02：`npm run build` 通过，前端无需字段适配。
- 2026-06-02：SQLx `0.9.0` 因 Rust `1.92.0` 工具链不兼容已调整为 SQLx `0.8.6`。

## 完成定义

- 后端仓储入口可以根据环境选择内存或 PostgreSQL。
- 数据库迁移文件和依赖它的代码一起提交。
- 现有 API 契约不破坏，前端彩种管理页面无需改字段。
- 文档、TODO、Trellis 任务记录同步更新。
- 完成本阶段后提交 Git，并归档任务。

## 暂不包含

- 本阶段不创建 Docker Compose 或本机 PostgreSQL 安装脚本。
- 不实现订单、开奖历史、用户、财务、机器人等数据表。
- 不拆分彩种玩法、开奖时间、合买配置为独立关系表。
- 不实现鉴权和权限控制。
- 不实现生产级备份、回滚和数据迁移演练。

## 技术方案

- 使用兼容当前 Rust `1.92.0` 工具链的 `sqlx = "0.8"`，启用 `runtime-tokio`、`tls-rustls`、`postgres`、`json`、`migrate`、`macros`。
- 新增仓储抽象，让路由层不关心当前使用内存还是 PostgreSQL。
- PostgreSQL 表字段：
  - `id text primary key`
  - `name text not null`
  - `number_type text not null`
  - `draw_mode text not null`
  - `schedule jsonb not null`
  - `sale_enabled boolean not null`
  - `group_buy jsonb not null`
  - `play_categories jsonb not null`
  - `created_at timestamptz not null default now()`
  - `updated_at timestamptz not null default now()`
- 使用服务层现有校验逻辑，数据库仓储写入前仍先校验 `LotteryKind`。
- 用 `serde_json` 在数据库行和领域模型之间做显式映射。

## 决策记录

**上下文**：可以立即强制所有环境都依赖 PostgreSQL，也可以先提供可选数据库模式。强制数据库更接近最终形态，但会让没有本地数据库的开发、测试和前端预览立即中断。

**决策**：本阶段使用 `DATABASE_URL` 作为开关。有数据库时使用 PostgreSQL 持久化；没有数据库时继续内存模式。

**影响**：可以安全推进持久化代码，同时保持项目随时可运行；缺点是需要在文档和日志中明确当前存在两种运行模式。

## 技术备注

- 相关后端文件：`backend/src/app.rs`、`backend/src/main.rs`、`backend/src/routes/admin.rs`、`backend/src/services/lottery.rs`、`backend/Cargo.toml`。
- 相关文档：`.trellis/spec/backend/database-guidelines.md`、`.trellis/spec/backend/api-contracts.md`、`架构设计.md`、`TODO.md`。
- 本任务是跨层/基础设施任务，完成后必须更新代码规格。
