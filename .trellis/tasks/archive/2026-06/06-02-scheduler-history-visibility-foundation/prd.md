# 调度运行历史与后台可视化基础

## 目标

在已有系统级常驻调度基础上，新增调度状态和最近运行历史，让管理后台能看到调度是否启用、当前配置、最近一次运行结果和最近若干轮运行摘要，避免调度只能通过日志和接口侧面判断。

## 已知信息

* 用户要求持续完善彩票管理后台，并且所有项目文档使用中文输出。
* `架构设计.md` 最新后续项明确包含“调度运行历史、后台可视化配置、管理员审计和失败告警”。
* 当前后端已有 `services/scheduler.rs`，支持按环境变量启用后台循环并自动补齐未来期号。
* 当前管理后台“开奖期号与开奖源”页面已有自动任务、创建期号、预览计划和批量生成入口，是展示调度状态的自然位置。
* 当前开奖期号、调度、订单和资金仍主要使用内存仓储；本阶段先做内存运行历史，不做 PostgreSQL 持久化。

## 临时假设

* 本阶段只展示调度状态和最近运行历史，不提供前端编辑调度配置。
* 调度运行历史保留最近 20 条，避免内存无限增长。
* 常驻调度成功和失败都记录历史；调度未启用时状态接口仍返回配置和空历史。
* 手动点击“运行自动任务”仍属于手动执行，不写入常驻调度历史，避免混淆触发来源。

## 需求

* 后端新增调度状态/历史领域模型，使用 `camelCase` 输出。
* 后端新增内存调度历史仓储，保存调度配置、最近运行记录和最后一次运行。
* 后台调度每轮成功或失败后都写入历史记录。
* 后端新增 `GET /api/admin/draw-scheduler/status` 接口，返回启用状态、配置、最近运行记录和统计数量。
* 管理后台“开奖期号与开奖源”页面新增“常驻调度”展示区，显示启用状态、执行周期、未来期号缓冲、最近运行结果和最近运行列表。
* 页面刷新时同步刷新调度状态；手动运行自动任务或批量生成后也刷新调度状态。
* 同步更新 `架构设计.md`、`TODO.md` 和后端 API 契约规格。

## 验收标准

* [ ] `GET /api/admin/draw-scheduler/status` 在调度关闭时返回 `enabled=false`、默认配置和空历史。
* [ ] 调度启用并跑过一轮后，状态接口返回最近运行记录，包含成功/失败、运行时间和封盘/开奖/结算/入账/补期数量。
* [ ] 调度失败时历史记录包含错误消息。
* [ ] 后台页面显示“常驻调度”状态、配置和最近运行记录。
* [ ] `cargo fmt --check`、`cargo check`、`cargo test`、`npm run build` 通过。
* [ ] API 冒烟和浏览器验证可查看调度状态。

## 完成定义

* 代码实现完成并通过质量检查。
* 中文文档、架构说明、TODO 记录同步更新。
* Trellis 任务归档并记录 journal。
* 不提交用户已有的无关脏改：`admin/vite.config.ts`、`backend/src/main.rs`、`.idea/`。

## 本阶段不包含

* 前端编辑调度配置。
* 调度运行历史 PostgreSQL 持久化。
* 分布式锁、失败重试队列、告警推送。
* 管理员操作审计和源数据审计。
* 真实第三方开奖 API 拉取。

## 技术备注

* 相关后端文件：`backend/src/services/scheduler.rs`、`backend/src/routes/admin.rs`、`backend/src/app.rs`。
* 相关前端文件：`admin/src/pages/DrawManagementPage.tsx`、`admin/src/api/client.ts`、`admin/src/types/`、`admin/src/hooks/`。
* 相关规格：`.trellis/spec/backend/api-contracts.md`、`.trellis/spec/backend/error-handling.md`、`.trellis/spec/frontend/component-guidelines.md`、`.trellis/spec/frontend/hook-guidelines.md`。
