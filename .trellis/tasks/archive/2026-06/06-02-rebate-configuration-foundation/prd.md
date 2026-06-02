# 邀请返利配置基础管理

## 目标

把管理后台“返利配置”从占位入口升级为可操作的邀请返利策略配置页面。当前先实现配置的查看、更新和 dashboard 同步，让运营能维护代理邀请开关、普通用户邀请开关、返利模式和默认充值返利比例，为后续真实充值返利发放、代理层级和返利流水打基础。

## 已知信息

* 用户要求继续完善整个彩票管理后台，所有项目文档使用中文输出。
* `架构设计.md` 明确要求普通用户和代理区分，只有代理可以邀请，并且下级充值可返利给上级。
* 当前 `backend/src/domain/rebate.rs` 只有 `InvitePolicySummary` 摘要模型，dashboard 内部静态返回邀请返利策略。
* 当前前端“返利配置”仍走 `PlaceholderPage`，无法维护返利策略。
* 当前用户权限模块已有用户类型 `regular` 和 `agent`，注册配置已有 `agentInviteRequired`。

## 临时假设

* 本阶段只做邀请返利策略配置，不执行真实返利发放。
* 返利配置仍使用内存仓储，服务重启后恢复种子配置。
* 默认保持“只有代理可邀请”，但后台允许开启普通用户邀请，用于运营灰度或测试。
* 返利模式支持 `immediate` 和 `rechargeTiered`，本阶段只维护当前模式和默认充值返利比例，不维护多档阶梯明细。
* 默认充值返利比例使用 basis points 表示，`350` 表示 `3.50%`。

## 需求

* 后端新增返利配置仓储，保存 `InvitePolicySummary` 并与 dashboard 共享数据。
* 后端新增返利配置接口：查询当前策略、更新策略。
* 后端更新时校验至少一种邀请入口开启、返利模式合法、默认充值返利比例在安全范围内。
* 前端新增返利配置类型、API client、hook 和真实页面。
* “返利配置”入口进入可操作页面，展示当前邀请策略、返利模式、默认充值返利比例和注册邀请要求。
* 页面支持保存代理邀请开关、普通用户邀请开关、返利模式和默认充值返利比例。
* 保存后刷新 dashboard，确保首页摘要和返利配置页一致。
* 同步更新 `架构设计.md`、`TODO.md` 和 `.trellis/spec/backend/api-contracts.md`。

## 验收标准

* [ ] `GET /api/admin/invite-policy` 返回当前邀请返利策略。
* [ ] `PUT /api/admin/invite-policy` 可更新邀请开关、返利模式和默认充值返利比例，dashboard 同步显示。
* [ ] 代理邀请和普通用户邀请不能同时关闭。
* [ ] 默认充值返利比例不能超过 `10000` basis points。
* [ ] “返利配置”入口显示真实页面并可保存配置。
* [ ] `cargo fmt --check`、`cargo check`、`cargo test`、`npm run build` 通过。
* [ ] API 冒烟和浏览器验证可更新返利策略，页面无控制台错误。

## 完成定义

* 代码实现完成并通过质量检查。
* 中文文档、架构说明、TODO 记录同步更新。
* Trellis 任务归档并记录 journal。
* 不提交用户已有无关脏改：`admin/vite.config.ts`、`backend/src/main.rs`、`.idea/`。

## 本阶段不包含

* 真实充值事件触发返利发放。
* 代理上下级关系树维护。
* 充值阶梯返利明细、多级返利、返利流水和财务入账。
* PostgreSQL 返利配置表。
* 手机端邀请链接和邀请码生成。

## 技术备注

* 相关后端文件：`backend/src/domain/rebate.rs`、`backend/src/services/dashboard.rs`、`backend/src/routes/admin.rs`、`backend/src/app.rs`。
* 相关前端文件：`admin/src/App.tsx`、`admin/src/api/client.ts`、`admin/src/pages/PlaceholderPage.tsx`、`admin/src/types/dashboard.ts`、`admin/src/pages/DashboardPage.tsx`。
* 相关规格：`.trellis/spec/backend/api-contracts.md`、`.trellis/spec/backend/error-handling.md`、`.trellis/spec/frontend/component-guidelines.md`、`.trellis/spec/frontend/hook-guidelines.md`。
