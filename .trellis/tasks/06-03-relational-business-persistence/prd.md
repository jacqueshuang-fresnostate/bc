# 全业务关系表数据库持久化

## Goal

把当前彩票管理后台已经落地的所有业务模块从 `state_documents` JSONB 状态文档过渡方案升级为 PostgreSQL 业务表持久化。运行时不再读写 `state_documents`，每个业务模块使用自己的表保存数据，避免所有业务状态挤在单张状态表里。

## What I Already Know

- 用户明确要求：所有业务都要数据库持久化，不使用 `state_documents`。
- 当前彩种和玩法赔率已经使用 `lotteries` 关系表。
- 上一阶段其它业务模块通过 `StateDocumentRepository` 保存到 `state_documents`：
  - 用户、管理员、角色、系统设置、注册配置、管理员会话。
  - 订单、结算批次。
  - 开奖期号、开奖源配置、彩种控制台控制号码。
  - 资金账户、资金流水。
  - 合买计划、邀请关系、返利配置、机器人配置、客服会话、调度配置和运行历史。
- 当前仓储仍保留内存 Store 业务逻辑；有数据库时可以从业务表加载 Store，并在写操作后同步保存到对应业务表。
- 用户已有未识别改动 `admin/vite.config.ts`、`backend/src/main.rs` 和 `.idea/`，本任务不纳入提交。

## Assumptions

- “不使用 `state_documents`”指运行时不再依赖单张状态文档表；允许部分复杂字段在业务表列中使用 JSONB，例如权限列表、投注选择、展开投注和绑定彩种列表。
- 本阶段先完成“每个业务模块落到独立业务表”，保留现有内存 Store 的校验与领域逻辑，避免一次性重写全部仓储为逐行 SQL CRUD。
- 旧的 `state_documents` 表不再作为运行时数据源；新迁移会建立业务表。历史迁移文件可以保留以保持 SQLx migration 历史一致。

## Requirements

- 新增 PostgreSQL migration，创建所有当前业务模块需要的业务表。
- 配置 `DATABASE_URL` 后，除彩种 `lotteries` 表外，其它模块必须从业务表读取并写入业务表。
- 后端运行时不能再通过 `StateDocumentRepository` 或 `state_documents` 保存业务状态。
- 未配置 `DATABASE_URL` 时继续使用现有内存模式。
- 空业务表启动时写入原有种子数据；已有业务表数据时不覆盖。
- 每个写操作成功后保存对应业务表状态。
- 保持现有 API 契约和前端字段不变。
- 日志 message 继续使用中文。
- 更新 `架构设计.md`、`TODO.md` 和 `.trellis/spec/backend/database-guidelines.md`，删除“状态文档为当前方案”的描述。

## Acceptance Criteria

- [x] 后端新增业务表迁移，覆盖用户权限、订单、开奖、开奖源、控制号码、资金、合买、邀请、返利、机器人、客服和调度模块。
- [x] 后端运行时不再引用 `StateDocumentRepository` 保存业务状态。
- [x] `DATABASE_URL` 已配置时，所有已落地后台业务模块使用业务表持久化。
- [x] `DATABASE_URL` 未配置时，内存模式仍可运行。
- [x] 空库启动可写入种子数据；已有数据不会被种子覆盖。
- [x] 后端测试覆盖至少一个关系表持久化模块的种子、保存和恢复。
- [x] 文档和 TODO 使用中文更新。
- [x] `cargo fmt --check`、`cargo check`、`cargo test`、`npm run build` 通过。

## Out Of Scope

- 本阶段不改变 API 字段和前端页面结构。
- 本阶段不新增分页、搜索、审计导出或后台数据迁移 UI。
- 本阶段不重写所有业务逻辑为逐行 SQL 事务；先落业务表持久化，后续再补跨模块事务和精细化 CRUD。
- 本阶段不提交用户已有的端口改动和 `.idea/`。

## Technical Notes

- 推荐新增共享数据库连接包装，例如 `BusinessDatabase`，复用一个 `PgPool` 并运行 migrations。
- 可以按模块提供 `load_*_store` / `save_*_store`，业务 Store 仍负责校验。
- 高风险模块后续仍需补事务一致性，尤其订单创建扣款、取消退款、开奖结算派奖和返利入账。
