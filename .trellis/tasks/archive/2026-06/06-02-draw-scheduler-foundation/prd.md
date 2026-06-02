# 系统级常驻调度基础

## 目标

在已有后台手动触发“自动任务”和“批量预生成期号”的基础上，新增后端系统级常驻调度基础能力。服务启动后可按环境变量启用后台循环，周期性自动补齐未来期号，并执行到期封盘、开奖、结算和派奖入账，减少管理员必须手动点击的步骤。

## 已知信息

* `架构设计.md` 的后续项明确要求“系统级常驻调度进程，根据服务器时间自动周期执行，不依赖管理员手动点击”。
* 当前已有 `run_draw_automation` 服务，可按传入时间执行封盘、开奖、结算和派奖入账。
* 当前已有 `generate_draw_issue_batch` 服务，可按彩种计划批量生成未来期号。
* `backend/src/main.rs` 当前有用户已有端口改动，本阶段应避免修改该文件；常驻调度可在 `app.rs` 创建 `AppState` 后启动。
* 当前开奖期号、订单、资金和结算仍以内存仓储为主；本阶段只做基础调度，不做 PostgreSQL 事务持久化。

## 临时假设

* 调度默认关闭，使用 `DRAW_SCHEDULER_ENABLED=true` 显式启用，避免本地开发和测试时后台任务自动改写内存数据。
* 调度周期通过 `DRAW_SCHEDULER_INTERVAL_SECONDS` 配置，默认 60 秒，必须大于 0。
* 每个销售开启的彩种至少保留 `DRAW_SCHEDULER_FUTURE_ISSUE_COUNT` 个未来 open/closed 期号，默认 1，最大 50。
* 封盘提前秒数继续沿用 `DRAW_SCHEDULER_SALE_CLOSE_LEAD_SECONDS`，默认 30 秒。
* 每轮调度先执行到期自动任务，再补齐未来期号，避免已开奖/已取消期号继续占用未来期号缓冲。

## 需求

* 新增后端调度配置模型，从环境变量读取启用状态、执行周期、未来期号缓冲数量和封盘提前秒数。
* 新增单轮调度服务，接收显式 `now`，先运行 `run_draw_automation`，再对销售开启的彩种自动补齐未来期号。
* 新增后台循环启动函数，启用后使用 Tokio 定时器周期运行单轮调度，并记录成功/失败日志。
* `app::router_from_env` 创建共享 `AppState` 后按配置启动后台调度任务。
* 不修改 `backend/src/main.rs` 的用户已有端口改动。
* 更新中文架构说明、TODO 和后端 API/运行契约规格。

## 验收标准

* [ ] 未设置 `DRAW_SCHEDULER_ENABLED` 时不会启动后台调度。
* [ ] `DRAW_SCHEDULER_ENABLED=true` 时后端启动后会创建后台调度任务。
* [ ] 单轮调度会先处理到期封盘/开奖/结算，再补齐未来期号。
* [ ] 只为 `saleEnabled=true` 的彩种补齐未来期号。
* [ ] 未来期号数量已满足配置时不会重复生成。
* [ ] 无效环境变量会在启动时给出清晰错误。
* [ ] `cargo fmt --check`、`cargo check`、`cargo test` 通过。
* [ ] API 冒烟或服务日志验证启用调度后能自动补期。

## 完成定义

* 代码实现完成并通过质量检查。
* 中文文档、架构说明、TODO 记录同步更新。
* Trellis 任务归档并记录 journal。
* 不提交用户已有的无关脏改：`admin/vite.config.ts`、`backend/src/main.rs`、`.idea/`。

## 本阶段不包含

* 分布式锁、失败重试队列和告警推送。
* 管理员操作审计和源数据审计。
* 开奖期号、订单、结算、资金流水 PostgreSQL 事务持久化。
* 前端调度配置页面。
* 真实第三方开奖 API 拉取。

## 技术备注

* 相关后端文件：`backend/src/app.rs`、`backend/src/services/automation.rs`、`backend/src/services/draw_generation.rs`、`backend/src/services/lottery.rs`。
* 预计新增后端文件：`backend/src/services/scheduler.rs`。
* 相关规格：`.trellis/spec/backend/api-contracts.md`、`.trellis/spec/backend/error-handling.md`、`.trellis/spec/backend/quality-guidelines.md`、`.trellis/spec/guides/cross-layer-thinking-guide.md`。
