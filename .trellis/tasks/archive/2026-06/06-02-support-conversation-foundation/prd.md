# 在线客服基础管理

## 目标

把管理后台“在线客服”从占位入口升级为可操作的客服会话/工单管理页面。当前先实现会话列表、详情、创建、状态维护、客服分配和后台回复，让运营能处理用户问题并记录处理过程，为后续真实在线聊天、消息推送和客服 SLA 打基础。

## 已知信息

* 用户要求继续完善整个彩票管理后台，所有项目文档使用中文输出。
* `架构设计.md` 把在线客服列为公共功能模块。
* 当前 dashboard 中 `support` 模块仍是 `ModuleStatus::Planned`，前端进入后仍走 `PlaceholderPage`。
* 当前用户权限模块已有用户、管理员和角色数据，可用于客服会话中的用户与客服选择。
* 本项目后台 API 采用统一 API 信封，页面数据通过集中 API client 和 hook 加载。

## 临时假设

* 本阶段只做客服会话/工单的后台管理，不实现实时聊天、WebSocket、访客端入口或消息推送。
* 客服会话仍使用内存仓储，服务重启后恢复种子会话。
* 新建会话时需要绑定已有用户；分配客服时需要绑定已有管理员。
* 会话状态支持 `open`、`pending`、`resolved`、`closed`。
* 会话优先级支持 `normal`、`urgent`。
* 后台回复使用管理员身份写入消息列表，并把会话更新时间刷新。

## 需求

* 后端新增客服会话领域模型和内存仓储。
* 后端新增客服接口：列表、详情、创建、更新状态/优先级/客服分配、追加消息。
* 后端创建会话时校验用户存在，分配客服时校验管理员存在。
* 后端校验会话 ID、用户 ID、主题、消息内容不能为空，重复 ID 拒绝创建。
* 前端新增客服类型、API client、hook 和真实页面。
* “在线客服”入口进入客服会话页面，展示列表、状态、优先级、未读数、分配客服、消息记录和回复表单。
* 保存或回复后刷新页面数据，并刷新 dashboard。
* 同步更新 `架构设计.md`、`TODO.md` 和 `.trellis/spec/backend/api-contracts.md`。

## 验收标准

* [ ] `GET /api/admin/support/conversations` 返回客服会话列表。
* [ ] `POST /api/admin/support/conversations` 可创建绑定有效用户的会话。
* [ ] `PUT /api/admin/support/conversations/{id}` 可更新状态、优先级和分配客服。
* [ ] `POST /api/admin/support/conversations/{id}/messages` 可追加后台回复。
* [ ] 用户不存在、管理员不存在、空主题、空消息和重复 ID 返回业务错误。
* [ ] “在线客服”入口显示真实页面并可维护会话。
* [ ] `cargo fmt --check`、`cargo check`、`cargo test`、`npm run build` 通过。
* [ ] API 冒烟和浏览器验证可创建/更新/回复客服会话，页面无控制台错误。

## 完成定义

* 代码实现完成并通过质量检查。
* 中文文档、架构说明、TODO 记录同步更新。
* Trellis 任务归档并记录 journal。
* 不提交用户已有无关脏改：`admin/vite.config.ts`、`backend/src/main.rs`、`.idea/`。

## 本阶段不包含

* 实时聊天、WebSocket、站内信推送或短信通知。
* 客服排班、SLA、自动分配和机器人客服。
* 文件上传、图片消息和敏感词审核。
* PostgreSQL 客服会话与消息表。
* 手机端用户发起客服入口。

## 技术备注

* 相关后端文件：`backend/src/domain/`、`backend/src/services/dashboard.rs`、`backend/src/routes/admin.rs`、`backend/src/app.rs`。
* 相关前端文件：`admin/src/App.tsx`、`admin/src/api/client.ts`、`admin/src/pages/PlaceholderPage.tsx`、`admin/src/types/dashboard.ts`。
* 相关规格：`.trellis/spec/backend/api-contracts.md`、`.trellis/spec/backend/error-handling.md`、`.trellis/spec/frontend/component-guidelines.md`、`.trellis/spec/frontend/hook-guidelines.md`。
