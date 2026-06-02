# 邀请管理基础

## 目标

把管理后台“邀请管理”从 planned/占位入口升级为可操作的代理邀请关系管理页面。当前先实现邀请关系列表、创建、状态维护、返利资格开关和备注维护，让运营能管理代理与下级用户关系，为后续真实充值返利发放、代理层级树和返利流水打基础。

## 已知信息

* 用户要求继续完善整个彩票管理后台，所有项目文档使用中文输出。
* `架构设计.md` 明确要求用户分为普通用户和代理，默认只有代理可以邀请，并且邀请可关联返利。
* 当前“返利配置”已经有 `InvitePolicySummary`，可配置代理邀请、普通用户邀请和默认返利比例。
* 当前“邀请管理”模块仍为 `ModuleStatus::Planned`，前端进入后仍走 `PlaceholderPage`。
* 当前用户权限模块已有用户类型 `regular`/`agent` 和用户的 `agentId` 字段。

## 临时假设

* 本阶段只做邀请关系管理，不执行真实充值返利发放。
* 邀请关系仍使用内存仓储，服务重启后恢复种子邀请关系。
* 创建邀请关系时根据当前返利配置判断邀请人是否有邀请权限。
* 默认策略下只有代理用户可以作为邀请人；如果后台开启普通用户邀请，则普通用户也可作为邀请人。
* 邀请关系状态支持 `pending`、`active`、`disabled`。
* `rebateEnabled` 只表示该邀请关系是否有返利资格，不代表已经发放返利。

## 需求

* 后端新增邀请关系领域模型和内存仓储。
* 后端新增邀请接口：列表、详情、创建、更新状态/返利资格/备注。
* 创建邀请关系时校验邀请人和被邀请人都存在、不能是同一个用户、邀请人符合当前邀请策略、关系 ID 和邀请码不重复。
* 前端新增邀请类型、API client、hook 和真实页面。
* “邀请管理”入口进入邀请关系页面，展示邀请关系、状态、返利资格、邀请码、邀请人、被邀请人和备注。
* 页面支持新增邀请关系，支持更新状态、返利资格和备注。
* 保存后刷新 dashboard，确保“邀请管理”模块状态和页面一致。
* 同步更新 `架构设计.md`、`TODO.md` 和 `.trellis/spec/backend/api-contracts.md`。

## 验收标准

* [ ] `GET /api/admin/invitations` 返回邀请关系列表。
* [ ] `POST /api/admin/invitations` 可创建有效邀请关系。
* [ ] `PUT /api/admin/invitations/{id}` 可更新状态、返利资格和备注。
* [ ] 默认返利策略下普通用户不能作为邀请人。
* [ ] 用户不存在、邀请人与被邀请人相同、重复 ID、重复邀请码返回业务错误。
* [ ] “邀请管理”入口显示真实页面并可维护邀请关系。
* [ ] `cargo fmt --check`、`cargo check`、`cargo test`、`npm run build` 通过。
* [ ] API 冒烟和浏览器验证可创建/更新邀请关系，页面无控制台错误。

## 完成定义

* 代码实现完成并通过质量检查。
* 中文文档、架构说明、TODO 记录同步更新。
* Trellis 任务归档并记录 journal。
* 不提交用户已有无关脏改：`admin/vite.config.ts`、`backend/src/main.rs`、`.idea/`。

## 本阶段不包含

* 真实充值返利发放、返利流水和财务入账。
* 多级代理树、佣金分润、返利封顶和阶梯明细。
* 邀请码生成服务、手机端邀请链接和二维码。
* PostgreSQL 邀请关系表。

## 技术备注

* 相关后端文件：`backend/src/domain/user.rs`、`backend/src/domain/rebate.rs`、`backend/src/services/rebate.rs`、`backend/src/routes/admin.rs`、`backend/src/app.rs`。
* 相关前端文件：`admin/src/App.tsx`、`admin/src/api/client.ts`、`admin/src/pages/RebateManagementPage.tsx`、`admin/src/types/dashboard.ts`。
* 相关规格：`.trellis/spec/backend/api-contracts.md`、`.trellis/spec/backend/error-handling.md`、`.trellis/spec/frontend/component-guidelines.md`、`.trellis/spec/frontend/hook-guidelines.md`。
