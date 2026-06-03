# 开奖后自动开盘下一期

## Goal

修复彩种一期封盘/结束后不会自动开盘下一期的问题。自动调度每次执行后，必须确保销售中的彩种还有下一期 `Open` 期号可用，避免当前期封盘后控制台和投注链路没有新期可进入。

## What I Already Know

- 用户反馈：彩种不会一期结束了之后开盘下一期。
- 当前后端有开奖调度器 `DrawSchedulerRepository` 和 `run_draw_scheduler_once`。
- 调度器会先执行封盘、开奖、结算，再调用 `ensure_future_draw_issues` 补未来期号。
- 当前实现用 `scheduled_at >= now` 且允许 `Closed` 状态统计未来期号，会把已经封盘但尚未开奖的当前期也算作未来缓冲，导致封盘后不生成下一期。
- 开奖期号已经使用业务表 `draw_issues` 持久化；本修复不需要改变数据库表结构。

## Requirements

- 调度执行后，已封盘的当前期不能再被当成未来开盘缓冲。
- 每个销售开启的彩种在调度执行结束后，应至少保留配置数量的未来 `Open` 期号；默认配置为 1 时，当前期封盘后要自动生成下一期并保持 `Open`。
- 已存在真正未来期号时，不重复生成。
- 手动彩种或无法生成期号的彩种仍按现有错误处理跳过，不影响其它彩种。
- 保持 API 和前端字段不变。
- 更新 `架构设计.md` 和 `TODO.md` 中文记录。

## Acceptance Criteria

- [x] 修复 `ensure_future_draw_issues` / 未来期号统计逻辑。
- [x] 新增或更新后端测试，覆盖当前期封盘后会生成下一期并为 `Open`。
- [x] 后端源码不新增 `state_documents` 运行时依赖。
- [x] 文档和 TODO 使用中文更新。
- [x] `cargo fmt --check`、`cargo check`、`cargo test`、`npm run build` 通过。

## Out Of Scope

- 不改前端页面结构。
- 不新增数据库表或字段。
- 不实现期号级控制队列、审计或推送通知。

## Technical Notes

- 重点检查 `future_issue_count` 是否应只统计 `Open` 且 `scheduled_at > now` 的期号，避免把本轮已经封盘的期号当作未来可投注期号。
- 如果同一轮调度先封盘再补期，需要在补期逻辑中读取本轮自动化后的最新期号状态。
- 测试应复现：存在一个到封盘时间但未到开奖时间的 `Open` 期号，执行调度后该期变为 `Closed`，并生成下一期 `Open`。
