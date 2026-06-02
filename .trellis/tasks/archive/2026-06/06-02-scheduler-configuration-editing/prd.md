# 调度配置后台编辑

## 目标

把常驻调度从“只能通过环境变量配置”升级为后台可查看、可修改的基础配置能力。管理员可以在“开奖期号与开奖源”页面维护调度启用状态、执行周期、未来期号缓冲数量和封盘提前秒数；后端后台循环每轮读取最新配置，让本进程内修改可以热生效，为后续配置发布、回滚、审计和分布式调度治理打基础。

## 已知信息

* 用户要求继续完善整个彩票管理后台，所有项目文档使用中文输出。
* `架构设计.md` 明确记录“调度配置编辑仍然只支持环境变量，后续需要在管理后台做配置查看、修改、发布和回滚流程”。
* 当前 `DrawSchedulerRepository` 已保存 `DrawSchedulerConfig`，`GET /api/admin/draw-scheduler/status` 会返回配置。
* 当前 `spawn_draw_scheduler` 启动后台循环时使用启动时的 `DrawSchedulerConfig` 副本，配置不会热更新。
* 当前前端 `useDrawScheduler` 只查询状态，不支持保存配置。

## 临时假设

* 本阶段只做本进程内内存配置编辑，不写数据库。
* 服务启动时仍从环境变量初始化配置；后台保存后在当前进程热生效，服务重启后恢复环境变量配置。
* 后台循环如果被配置为 `enabled=false`，下一轮 tick 会跳过自动任务并不记录运行历史。
* 如果服务启动时调度为关闭，本阶段只更新仓储状态，不动态启动新的后台 Tokio 循环；后续再实现后台启动/停止控制。管理员仍可通过接口保存启用状态并在状态页看到配置。
* 修改 `intervalSeconds` 在已运行循环中下一个 tick 后生效，当前 tick 间隔不会被立即重建。

## 需求

* 后端新增调度配置更新请求类型和 `PUT /api/admin/draw-scheduler/config`。
* `DrawSchedulerRepository` 新增读取配置和更新配置方法，更新时复用现有校验。
* `spawn_draw_scheduler` 后台循环每轮读取仓储最新配置；`enabled=false` 时跳过执行，`futureIssueCount` 和 `saleCloseLeadSeconds` 热生效。
* 前端新增更新调度配置 API client 方法。
* `useDrawScheduler` 增加保存配置能力。
* “常驻调度”卡片新增配置表单，支持启用状态、执行周期、未来期号、封盘提前秒数编辑与保存。
* 保存后刷新 dashboard/调度状态，确保页面显示最新配置。
* 同步更新 `架构设计.md`、`TODO.md` 和 `.trellis/spec/backend/api-contracts.md`。

## 验收标准

* [ ] `PUT /api/admin/draw-scheduler/config` 可保存有效配置并返回最新 `DrawSchedulerStatus`。
* [ ] `intervalSeconds=0`、`futureIssueCount=0`、`futureIssueCount>50`、`saleCloseLeadSeconds=0` 返回业务错误。
* [ ] 后台循环每轮读取最新配置；更新 `futureIssueCount` 后单轮调度使用新数量。
* [ ] “常驻调度”卡片显示可编辑表单并可保存配置。
* [ ] `cargo fmt --check`、`cargo check`、`cargo test`、`npm run build` 通过。
* [ ] API 冒烟和浏览器验证可保存调度配置，页面无控制台错误。

## 完成定义

* 代码实现完成并通过质量检查。
* 中文文档、架构说明、TODO 记录同步更新。
* Trellis 任务归档并记录 journal。
* 不提交用户已有无关脏改：`admin/vite.config.ts`、`backend/src/main.rs`、`.idea/`。

## 本阶段不包含

* 调度配置 PostgreSQL 持久化。
* 管理员操作审计、发布审批、回滚和配置版本。
* 动态启动/停止后台 Tokio 循环。
* 分布式调度锁、幂等键、失败重试和告警通知。

## 技术备注

* 相关后端文件：`backend/src/services/scheduler.rs`、`backend/src/routes/admin.rs`、`backend/src/app.rs`。
* 相关前端文件：`admin/src/types/scheduler.ts`、`admin/src/hooks/useDrawScheduler.ts`、`admin/src/api/client.ts`、`admin/src/pages/DrawManagementPage.tsx`。
* 相关规格：`.trellis/spec/backend/api-contracts.md`、`.trellis/spec/backend/error-handling.md`、`.trellis/spec/frontend/component-guidelines.md`、`.trellis/spec/frontend/hook-guidelines.md`。
