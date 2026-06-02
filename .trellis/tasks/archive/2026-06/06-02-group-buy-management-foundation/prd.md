# 合买配置与计划基础

## 目标

把管理后台“合买配置”从占位入口升级为可操作的合买计划管理页面。当前先实现后台创建、查看和维护合买计划及参与记录，校验每个彩种已有的合买配置，让运营可以看到计划金额、份额、发起人认购、参与人认购和满单进度，为后续合买订单、资金冻结、派奖分账和合买机器人执行打基础。

## 已知信息

* 用户要求继续完善整个彩票管理后台，所有项目文档使用中文输出。
* `架构设计.md` 的合买需求要求每个彩种可开启/关闭合买，配置份额最低金额、发起人最低起购比例和参与人最低购买金额。
* 彩种模型已经包含 `GroupBuyConfig`：`enabled`、`minShareAmountMinor`、`initiatorMinPercent`、`participantMinAmountMinor`。
* dashboard 里已有“合买配置”入口，但当前 `App.tsx` 未接真实页面，会进入 `PlaceholderPage`。
* dashboard 的 `groupBuyPlans` 目前是静态假数据，未接真实仓储。
* 当前订单、财务、计奖派奖已经有基础能力，但合买份额扣款、撤单退款和中奖分账仍在后续范围。

## 临时假设

* 本阶段只做后台合买计划和参与记录管理，不创建真实投注订单，不冻结/扣减用户资金。
* 合买计划仍使用内存仓储，服务重启后恢复种子计划。
* 创建合买计划必须选择已存在且开启合买的彩种。
* 发起人和参与人必须是已存在用户。
* 发起人认购金额必须满足彩种的 `initiatorMinPercent`，参与人认购金额必须满足 `participantMinAmountMinor`。
* 份额数量由 `totalAmountMinor / minShareAmountMinor` 推导，金额必须能被最小份额金额整除。
* 计划状态支持 `draft`、`open`、`filled`、`cancelled`、`settled`；本阶段只允许后台维护状态，不自动执行派奖。

## 需求

* 后端新增合买计划领域模型、参与记录模型和内存仓储。
* 后端新增合买计划接口：列表、详情、创建、更新状态/备注、添加参与记录。
* 创建合买计划时校验彩种存在、合买已开启、金额大于 0、金额能按最小份额拆分、发起人存在、发起人认购满足比例、关系 ID 不重复。
* 添加参与记录时校验计划存在、计划处于 `open` 或 `draft`、用户存在、金额满足参与人最低金额、认购后不超过计划总金额。
* 认购达到计划总金额时，计划自动进入 `filled` 状态。
* dashboard 的 `groupBuyPlans` 改为读取真实合买仓储。
* 前端新增合买类型、API client、hook 和“合买配置”真实页面。
* 页面展示彩种合买配置、计划列表、计划详情、参与记录、创建计划表单和添加参与记录表单。
* 保存后刷新 dashboard，确保“合买配置”入口和概览数据一致。
* 同步更新 `架构设计.md`、`TODO.md` 和 `.trellis/spec/backend/api-contracts.md`。

## 验收标准

* [ ] `GET /api/admin/group-buy/plans` 返回合买计划列表。
* [ ] `GET /api/admin/group-buy/plans/{id}` 返回计划详情和参与记录。
* [ ] `POST /api/admin/group-buy/plans` 可创建有效合买计划，并自动写入发起人参与记录。
* [ ] `PUT /api/admin/group-buy/plans/{id}` 可更新状态和备注。
* [ ] `POST /api/admin/group-buy/plans/{id}/participants` 可添加参与记录，满额后计划状态变为 `filled`。
* [ ] 未开启合买彩种、未知用户、发起人认购比例不足、参与金额低于最低值、超额参与和重复 ID 返回业务错误。
* [ ] “合买配置”入口显示真实页面并可维护合买计划。
* [ ] `cargo fmt --check`、`cargo check`、`cargo test`、`npm run build` 通过。
* [ ] API 冒烟和浏览器验证可创建/更新计划、添加参与记录，页面无控制台错误。

## 完成定义

* 代码实现完成并通过质量检查。
* 中文文档、架构说明、TODO 记录同步更新。
* Trellis 任务归档并记录 journal。
* 不提交用户已有无关脏改：`admin/vite.config.ts`、`backend/src/main.rs`、`.idea/`。

## 本阶段不包含

* 创建真实投注订单、扣款、冻结金额、撤单退款。
* 开奖后按份额分账和派奖入账。
* 手机端用户参与合买页面。
* 合买机器人真实发起计划或辅助满单。
* PostgreSQL 合买计划表、参与记录表和审计表。

## 技术备注

* 相关后端文件：`backend/src/domain/lottery.rs`、`backend/src/domain/order.rs`、`backend/src/services/lottery.rs`、`backend/src/services/dashboard.rs`、`backend/src/routes/admin.rs`、`backend/src/app.rs`。
* 相关前端文件：`admin/src/App.tsx`、`admin/src/api/client.ts`、`admin/src/pages/LotteryManagementPage.tsx`、`admin/src/types/dashboard.ts`。
* 相关规格：`.trellis/spec/backend/api-contracts.md`、`.trellis/spec/backend/error-handling.md`、`.trellis/spec/frontend/component-guidelines.md`、`.trellis/spec/frontend/hook-guidelines.md`。
