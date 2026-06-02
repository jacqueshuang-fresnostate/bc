# 机器人配置基础管理

## 目标

把管理后台“合买机器人”和“购彩机器人”从占位入口升级为可操作的机器人配置页面。当前先实现机器人配置的列表、创建、更新、状态切换和适用彩种维护，让运营能看到哪些机器人启用、暂停或禁用，并为后续真实自动发起合买、满单辅助和模拟购彩执行打基础。

## 已知信息

* 用户要求继续完善整个彩票管理后台，所有项目文档使用中文输出。
* `架构设计.md` 明确包含合买机器人和购彩机器人：合买机器人可在开盘期间发起合买、辅助满单；购彩机器人可在开盘期间模拟用户购彩。
* 当前 `backend/src/domain/robot.rs` 只有 `RobotConfigSummary` 摘要模型，dashboard 使用 `services/dashboard.rs` 静态 `robots()` 数据。
* 当前前端“合买机器人”和“购彩机器人”仍走 `PlaceholderPage`。
* 当前订单、开奖期号、资金和调度已有基础链路，但合买计划真实执行尚未落地。

## 临时假设

* 本阶段只做机器人配置管理，不执行真实机器人任务。
* 机器人配置仍使用内存仓储，服务重启后恢复种子数据。
* 机器人可绑定多个彩种；保存时需要校验彩种存在。
* `groupBuy` 机器人和 `purchase` 机器人进入同一个“机器人配置”页面，侧边栏入口不同但页面按类型定位。
* 机器人状态支持 `enabled`、`paused`、`disabled`。

## 需求

* 后端新增机器人内存仓储，保存机器人配置并与 dashboard 共享数据。
* 后端新增机器人接口：列表、详情、创建、更新、删除、状态变更。
* 后端保存机器人时校验 ID、名称、类型、至少一个彩种、彩种存在性、重复 ID。
* 前端新增机器人类型、API client、hook 和真实页面。
* “合买机器人”和“购彩机器人”入口进入同一个页面，并按入口默认筛选对应类型。
* 页面展示机器人列表、状态、类型、绑定彩种、说明，并支持新增、编辑、启停/暂停、删除。
* 同步更新 `架构设计.md`、`TODO.md` 和 `.trellis/spec/backend/api-contracts.md`。

## 验收标准

* [ ] `GET /api/admin/robots` 返回机器人列表。
* [ ] `POST /api/admin/robots` 可创建绑定有效彩种的机器人，dashboard 同步显示。
* [ ] `PUT /api/admin/robots/{id}` 可更新机器人名称、类型、彩种和说明。
* [ ] `PATCH /api/admin/robots/{id}/status` 可切换启用、暂停、禁用。
* [ ] 无彩种或绑定不存在彩种时返回业务错误。
* [ ] “合买机器人”和“购彩机器人”入口显示真实页面并按类型定位。
* [ ] `cargo fmt --check`、`cargo check`、`cargo test`、`npm run build` 通过。
* [ ] API 冒烟和浏览器验证可创建/编辑机器人，页面无控制台错误。

## 完成定义

* 代码实现完成并通过质量检查。
* 中文文档、架构说明、TODO 记录同步更新。
* Trellis 任务归档并记录 journal。
* 不提交用户已有无关脏改：`admin/vite.config.ts`、`backend/src/main.rs`、`.idea/`。

## 本阶段不包含

* 真实机器人定时执行。
* 自动创建合买计划、自动满单、自动下投注单。
* 机器人执行日志、风控限额、失败重试和审计。
* PostgreSQL 机器人配置表。
* 手机端。

## 技术备注

* 相关后端文件：`backend/src/domain/robot.rs`、`backend/src/services/dashboard.rs`、`backend/src/routes/admin.rs`、`backend/src/app.rs`。
* 相关前端文件：`admin/src/App.tsx`、`admin/src/api/client.ts`、`admin/src/pages/PlaceholderPage.tsx`、`admin/src/types/dashboard.ts`。
* 相关规格：`.trellis/spec/backend/api-contracts.md`、`.trellis/spec/backend/error-handling.md`、`.trellis/spec/frontend/component-guidelines.md`、`.trellis/spec/frontend/hook-guidelines.md`。
