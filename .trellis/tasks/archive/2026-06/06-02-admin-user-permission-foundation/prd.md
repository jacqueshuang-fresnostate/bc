# 后台用户权限基础管理

## 目标

把管理后台公共功能里的“用户管理、管理员管理、角色权限、系统设置”从占位入口升级为可操作的基础后台模块。当前先使用内存仓储完成列表、创建、更新和状态维护，让后台可以真实维护用户、后台账号、角色权限范围和注册/系统配置，为后续登录鉴权、审计和 PostgreSQL 权限表打基础。

## 已知信息

* 用户要求继续完善整个彩票管理后台，所有项目文档使用中文输出。
* `架构设计.md` 的公共功能明确包含添加用户、用户信息维护、用户资金查看、管理员管理、角色权限管理和系统设置。
* 当前 `DashboardSummary` 已返回 `users`、`admins`、`roles`、`settings`、`registration`，但它们来自 `services/dashboard.rs` 的静态函数。
* 当前前端只有 `PlaceholderPage` 承接 `users`、`admins`、`roles`、`settings`、`registration` 等入口。
* 当前仍保留用户已有未提交改动：`admin/vite.config.ts`、`backend/src/main.rs` 和 `.idea/`，本任务不纳入提交。

## 临时假设

* 本阶段先做内存模式，不接 PostgreSQL 用户/权限表。
* 本阶段只做后台维护能力，不实现真实登录、密码哈希、JWT、权限拦截和会话管理。
* 用户资金展示先使用用户摘要里的 `balanceMinor`，不在本阶段和资金账户仓储做强同步。
* 管理员账号只维护用户名、角色和状态，不保存密码。
* 系统设置先支持字符串配置；注册配置仍以结构化对象返回和更新。

## 需求

* 后端新增用户权限相关内存仓储，保存用户、管理员、角色、系统设置和注册配置。
* dashboard 改为从同一个仓储读取用户权限数据，避免工作台和管理页面数据漂移。
* 后端新增用户管理接口：列表、创建、更新、状态变更。
* 后端新增管理员管理接口：列表、创建、更新、状态变更。
* 后端新增角色权限接口：列表、创建、更新、删除。
* 后端新增系统设置接口：设置列表、单项更新、注册配置更新。
* 后端校验 ID、名称、状态、角色存在性、权限范围非空、重复 ID、删除被管理员使用的角色等基础规则。
* 前端新增类型、API client、hook 和真实页面，替换用户、管理员、角色、系统设置、用户注册入口的占位展示。
* 页面需要能查看用户、管理员、角色、系统设置和注册配置，并支持新增/更新/状态切换。
* 同步更新 `架构设计.md`、`TODO.md` 和 `.trellis/spec/backend/api-contracts.md`。

## 验收标准

* [ ] `GET /api/admin/users` 返回用户列表，创建/更新/状态变更后 dashboard 用户数据同步变化。
* [ ] `GET /api/admin/admins` 返回管理员列表，管理员角色必须引用已存在角色。
* [ ] `GET /api/admin/roles` 返回角色列表，角色权限范围为空时拒绝保存，被管理员使用的角色拒绝删除。
* [ ] `GET /api/admin/system-settings` 返回系统配置，注册配置可更新并体现在 dashboard。
* [ ] 用户、管理员、角色权限、系统设置、用户注册入口都进入真实后台页面。
* [ ] `cargo fmt --check`、`cargo check`、`cargo test`、`npm run build` 通过。
* [ ] API 冒烟和浏览器验证可创建或更新用户/管理员/角色/设置，页面无控制台错误。

## 完成定义

* 代码实现完成并通过质量检查。
* 中文文档、架构说明、TODO 记录同步更新。
* Trellis 任务归档并记录 journal。
* 不提交用户已有无关脏改：`admin/vite.config.ts`、`backend/src/main.rs`、`.idea/`。

## 本阶段不包含

* 真实登录鉴权、密码哈希、JWT、会话、菜单权限拦截。
* PostgreSQL 用户、管理员、角色、权限和设置表。
* 管理员操作审计日志。
* 客服、机器人、合买、邀请返利的完整业务执行。
* 手机端。

## 技术备注

* 相关后端文件：`backend/src/domain/user.rs`、`backend/src/domain/permission.rs`、`backend/src/routes/admin.rs`、`backend/src/app.rs`、`backend/src/services/dashboard.rs`。
* 相关前端文件：`admin/src/App.tsx`、`admin/src/api/client.ts`、`admin/src/types/dashboard.ts`、`admin/src/pages/PlaceholderPage.tsx`。
* 相关规格：`.trellis/spec/backend/api-contracts.md`、`.trellis/spec/backend/error-handling.md`、`.trellis/spec/frontend/component-guidelines.md`、`.trellis/spec/frontend/hook-guidelines.md`。
