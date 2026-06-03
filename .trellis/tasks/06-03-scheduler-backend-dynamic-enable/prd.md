# 后台动态启用开奖调度器

## Goal

开奖调度器由后端服务启动时自动创建后台常驻任务，不再要求通过环境变量决定是否创建调度器。后台页面保存“启用”配置后，调度循环应在不重启服务的情况下开始执行。

## What I Already Know

- 用户要求：开奖调度器应该由后台启动，而不是通过环境变量配置来启动。
- 当前 `spawn_draw_scheduler` 在 `config.enabled=false` 时直接返回 `None`，不会创建后台循环。
- 当前管理后台已经有 `PUT /api/admin/draw-scheduler/config`，可以保存 `enabled`、执行周期、未来期号缓冲和封盘提前秒数。
- 当前后台循环每轮会读取 `DrawSchedulerRepository::config()`，所以只要循环存在，后台启用配置可以热生效。
- 如果服务启动时未创建循环，后台保存 `enabled=true` 只会改变配置，不会真正启动自动调度。

## Requirements

- 服务启动时必须创建开奖调度后台循环，即使当前配置为 `enabled=false`。
- `enabled=false` 时循环不执行封盘、开奖、结算、补期，只定期读取配置等待后台启用。
- 后台保存 `enabled=true` 后，不需要重启服务，下一轮循环即可开始执行。
- 环境变量可以继续作为初始默认值，但不再决定后台任务是否存在。
- 保持现有 API 和前端字段不变。
- 后台运行日志 message 继续使用中文。
- 更新 `架构设计.md`、`TODO.md` 和后端 API 契约规范。

## Acceptance Criteria

- [x] `spawn_draw_scheduler` 不再因 `enabled=false` 跳过创建后台任务。
- [x] 后台动态修改 `enabled` 后，已启动循环能读取配置并执行。
- [x] 后端测试覆盖“禁用配置下也创建后台任务”和“后台启用后无需重启即可运行”。
- [x] 文档和 TODO 使用中文更新。
- [x] 不改动用户已有的 `admin/vite.config.ts`、`backend/src/main.rs` 和 `.idea/`。
- [x] `cargo fmt --check`、`cargo check`、`cargo test`、`npm run build` 通过。

## Out Of Scope

- 不新增前端页面结构。
- 不新增数据库表或字段。
- 不实现分布式锁、失败告警或调度审计详情。

## Technical Notes

- 可以保留 `DRAW_SCHEDULER_ENABLED` 作为初始配置默认值，但 `spawn_draw_scheduler` 必须总是创建任务。
- 禁用状态下不要使用过长的初始执行周期等待后台启用，否则页面保存启用后会显得“没生效”；禁用轮询可以使用短周期读取配置。
- 测试中创建的后台任务需要 `abort()`，避免测试进程悬挂。
