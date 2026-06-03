# 全后台模块数据库持久化

## Goal

让彩票管理后台当前已经落地的所有业务模块在配置 `DATABASE_URL` 后都具备 PostgreSQL 持久化能力，避免服务重启后用户、订单、开奖期号、开奖源、资金、权限、客服、机器人、邀请返利、合买和控制台配置丢失。

## What I Already Know

- 当前 Docker Compose 已启动 PostgreSQL，后端配置 `DATABASE_URL` 时会运行 SQLx migrations。
- 目前只有 `lotteries` 彩种和玩法赔率配置使用 PostgreSQL 表持久化。
- 其它模块仍然使用内存仓储：
  - 用户、管理员、角色、系统设置、注册配置、管理员会话。
  - 开奖期号、开奖源配置、彩种控制台控制号码。
  - 订单、结算批次、资金账户、资金流水。
  - 合买计划、邀请关系、返利配置、机器人配置、客服会话、调度配置和运行历史。
- 用户要求“先把所有的先迁移为数据库”，因此第一阶段目标是全模块可持久化，不要求所有模块立即拆成完全范式化业务表。

## Requirements

- 新增 PostgreSQL 迁移，提供通用业务状态持久化表。
- 配置 `DATABASE_URL` 后，除彩种表外的其它业务仓储也必须读取并写入 PostgreSQL。
- 未配置 `DATABASE_URL` 时继续使用现有内存模式，方便本地无数据库开发。
- 数据库为空时各模块写入原有种子数据；数据库已有状态时必须优先恢复数据库状态，不覆盖为种子数据。
- 每个写操作成功后同步保存对应模块状态，保证重启后可恢复。
- 日志继续使用中文 message，结构化字段名可保留英文。
- 保留现有 API 契约和前端字段，不因持久化改动破坏页面。
- 不提交用户已有的 `admin/vite.config.ts`、`backend/src/main.rs` 和 `.idea/`。

## Acceptance Criteria

- [x] 新增通用状态表 migration。
- [x] `AppState::from_env_with_scheduler` 在有 `DATABASE_URL` 时为所有业务仓储启用 PostgreSQL 持久化。
- [x] 用户、权限、订单、开奖、开奖源、控制号码、资金、合买、邀请、返利、机器人、客服、调度配置和运行历史可跨服务重启恢复。
- [x] 无数据库模式仍可运行全部后端测试。
- [x] 至少新增后端测试覆盖通用状态文档读写、种子初始化、保存后重新打开恢复。
- [x] 更新 `架构设计.md`、`TODO.md` 和 `.trellis/spec/backend/database-guidelines.md`。
- [x] `cargo fmt --check`、`cargo check`、`cargo test`、`npm run build` 通过。

## Out Of Scope

- 本阶段不把每个模块拆成独立范式化关系表。
- 本阶段不新增分页、复杂查询索引、审计版本、分布式锁或事务级跨模块一致性。
- 本阶段不改变前端页面或 API 字段结构。

## Technical Notes

- 第一阶段采用 JSONB 状态文档表作为“全模块可持久化”过渡方案。
- 高风险模块后续仍需逐步拆成关系表，尤其是订单、资金流水、结算批次、开奖期号和管理员权限。
- 相关规范：
  - `.trellis/spec/backend/database-guidelines.md`
  - `.trellis/spec/backend/api-contracts.md`
  - `.trellis/spec/backend/logging-guidelines.md`
  - `.trellis/spec/backend/quality-guidelines.md`
  - `.trellis/spec/backend/deployment-guidelines.md`
