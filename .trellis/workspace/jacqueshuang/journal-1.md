# Journal - jacqueshuang (Part 1)

> AI development session journal
> Started: 2026-06-02

---



## Session 1: 初始化彩票管理系统骨架

**Date**: 2026-06-02
**Task**: 初始化彩票管理系统骨架
**Branch**: `main`

### Summary

创建 Rust 后端和 React 管理后台基础工程，补齐中文项目规范、API 契约、TODO 记录，并完成构建验证与初始提交。

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `bef1149` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 2: 彩种管理 CRUD

**Date**: 2026-06-02
**Task**: 彩种管理 CRUD
**Branch**: `main`

### Summary

实现彩种内存仓储、管理 API、销售开关和管理后台彩种配置页面，修复 DrawSchedule intervalSeconds 跨层契约，并通过后端测试、前端构建、HTTP 冒烟和浏览器验证。

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `984cf1d` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 3: 彩种数据库持久化

**Date**: 2026-06-02
**Task**: 彩种数据库持久化
**Branch**: `main`

### Summary

为彩种管理新增 SQLx PostgreSQL 可选持久化、lotteries 迁移、统一仓储入口和 DATABASE_URL 启动选择；保留无数据库内存模式，完成后端测试、无数据库冒烟和前端构建。

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `00f5199` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 4: 玩法规则引擎

**Date**: 2026-06-02
**Task**: 玩法规则引擎
**Branch**: `main`

### Summary

实现彩票玩法规则引擎：新增后端规则目录与评估 API，支持 3 位/5 位玩法注数计算、投注展开和中奖判断；新增管理后台玩法规则验证页，并同步架构、TODO 与 API 契约规格。

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `8a15b3d` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 5: 订单与投注基础

**Date**: 2026-06-02
**Task**: 订单与投注基础
**Branch**: `main`

### Summary

实现订单与投注基础：新增后端订单模型、内存订单仓储、订单创建/列表/详情/取消 API，订单创建复用玩法规则引擎计算注数和金额；新增管理后台订单管理页和工作台最近订单展示，并同步架构、TODO 与 API 契约。

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `1fe6ec0` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 6: 开奖期号与开奖源基础

**Date**: 2026-06-02
**Task**: 开奖期号与开奖源基础
**Branch**: `main`

### Summary

完成开奖期号与开奖源基础：新增后端开奖领域模型、内存仓储和开奖 API；管理后台新增开奖期号页面，支持创建、封盘、开奖、取消；同步架构、TODO 和 API 规格，并通过 Rust/前端构建测试及浏览器联调。

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `77bfbc3` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 7: 计奖与派奖基础

**Date**: 2026-06-02
**Task**: 计奖与派奖基础
**Branch**: `main`

### Summary

完成计奖与派奖基础：新增结算领域模型、结算 API、订单状态流转和基础派奖结果；管理后台新增计奖派奖页面，订单页展示结算字段；同步架构、TODO 和 API 规格，并通过 Rust/前端构建测试及浏览器联调。

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `42c5d18` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 8: 用户资金与资金流水基础

**Date**: 2026-06-02
**Task**: 用户资金与资金流水基础
**Branch**: `main`

### Summary

完成用户资金与资金流水基础：新增资金账户、流水、手动调账、订单扣款/取消退款、结算派奖入账和管理后台财务页面；验证 cargo fmt/check/test、npm build、API 冒烟和浏览器财务页。

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `303d9b1` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 9: 玩法与赔率配置完善

**Date**: 2026-06-02
**Task**: 玩法与赔率配置完善
**Branch**: `main`

### Summary

完成彩种单玩法赔率配置、订单赔率快照、结算按快照派奖和玩法规则页赔率维护；验证 3 位 5 个、5 位 19 个玩法，以及 API、浏览器、Rust/前端质量门。

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `55d00a8` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 10: 期号封盘投注校验

**Date**: 2026-06-02
**Task**: 期号封盘投注校验
**Branch**: `main`

### Summary

完成订单创建接入开奖期号 open 状态校验，订单页期号改为 open 期号下拉；验证 open 期号可下单、closed 和不存在期号被拒绝，Rust/前端质量门通过。

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `bb193ea` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 11: 自动封盘开奖结算基础

**Date**: 2026-06-02
**Task**: 自动封盘开奖结算基础
**Branch**: `main`

### Summary

实现后台触发式自动封盘、自动开奖、自动结算和派奖入账入口，补齐管理后台自动任务操作区，并同步中文架构、TODO 和 API 契约。

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `0e5930f` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 12: 自动创建下一期号基础

**Date**: 2026-06-02
**Task**: 自动创建下一期号基础
**Branch**: `main`

### Summary

实现按彩种开奖计划生成下一期号，支持周期、每日、周开奖，管理后台新增生成入口，并同步中文架构、TODO 和 API 契约。

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `9f96356` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 13: 批量预生成期号和计划预览

**Date**: 2026-06-02
**Task**: 批量预生成期号和计划预览
**Branch**: `main`

### Summary

实现开奖期号计划预览和批量生成，新增后端预览/批量接口，前端开奖期号页面新增数量输入、预览计划和批量生成入口，并同步中文架构、TODO 与 API 契约。

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `6f55bb3` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 14: 系统级常驻调度基础

**Date**: 2026-06-02
**Task**: 系统级常驻调度基础
**Branch**: `main`

### Summary

实现后端常驻开奖调度基础，支持通过环境变量启用后台循环，周期性自动封盘、开奖、结算、派奖并补齐未来期号，同时同步中文架构、TODO 和运行契约。

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `90959cd` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 15: 调度运行历史与后台可视化基础

**Date**: 2026-06-02
**Task**: 调度运行历史与后台可视化基础
**Branch**: `main`

### Summary

新增调度状态仓储和 GET /api/admin/draw-scheduler/status 接口；管理后台开奖期号页面新增常驻调度状态、最近运行和历史展示；同步更新架构设计、TODO 和 API 契约。验证通过 cargo fmt --check、cargo check、cargo test、npm run build、API 冒烟和浏览器控制台检查。

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `cc79406` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 16: 后台用户权限基础管理

**Date**: 2026-06-02
**Task**: 后台用户权限基础管理
**Branch**: `main`

### Summary

新增 AccessRepository 内存仓储和用户、管理员、角色、系统设置、注册配置管理接口；dashboard 改为读取同一用户权限仓储；管理后台新增用户权限管理真实页面，接入用户、管理员、角色权限、系统设置、用户注册入口。验证通过 cargo fmt --check、cargo check、cargo test、npm run build、API 冒烟和浏览器控制台检查。

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `e265495` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 17: 机器人配置基础管理

**Date**: 2026-06-02
**Task**: 机器人配置基础管理
**Branch**: `main`

### Summary

完成机器人配置基础管理：新增后端 RobotRepository 和机器人 CRUD/状态接口，dashboard 改为读取共享机器人仓储；管理后台新增机器人配置页面，接入合买机器人和购彩机器人入口，支持类型筛选、彩种绑定、启停、编辑和删除。同步更新架构设计、TODO 和后端 API 契约。验证 cargo fmt --check、cargo check、cargo test、npm run build 通过，并完成 API 与浏览器冒烟。

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `e2051ad` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 18: 邀请返利配置基础管理

**Date**: 2026-06-02
**Task**: 邀请返利配置基础管理
**Branch**: `main`

### Summary

完成邀请返利配置基础管理：新增后端 RebateRepository 和 GET/PUT /api/admin/invite-policy 接口，dashboard 改为读取共享返利仓储；管理后台新增返利配置页面，支持代理邀请、普通用户邀请、返利模式和默认充值返利比例维护。同步更新架构设计、TODO 和后端 API 契约。验证 cargo fmt --check、cargo check、cargo test、npm run build 通过，并完成 API 与浏览器冒烟。

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `559d982` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 19: 在线客服基础管理

**Date**: 2026-06-02
**Task**: 在线客服基础管理
**Branch**: `main`

### Summary

完成在线客服基础管理：新增后端客服会话领域模型、SupportRepository 和客服会话列表/详情/创建/更新/回复接口；管理后台新增在线客服页面，支持会话列表、创建工单、状态优先级维护、客服分配和后台回复。同步更新架构设计、TODO 和后端 API 契约。验证 cargo fmt --check、cargo check、cargo test、npm run build 通过，并完成 API 与浏览器冒烟。

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `3b1ffa6` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 20: 邀请管理基础

**Date**: 2026-06-02
**Task**: 邀请管理基础
**Branch**: `main`

### Summary

实现邀请管理基础能力：后端新增邀请关系模型、内存仓储和管理接口；前端新增邀请管理页面，可查看、创建和更新邀请关系状态、返利资格与备注；同步更新架构说明、TODO 和后端 API 规格，并通过格式、检查、测试、构建、API 冒烟和浏览器验证。

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `5dd2c05` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 21: 合买配置与计划基础

**Date**: 2026-06-02
**Task**: 合买配置与计划基础
**Branch**: `main`

### Summary

实现合买配置与计划基础能力：后端新增合买计划模型、内存仓储和管理接口；dashboard 的 groupBuyPlans 改为读取真实仓储；前端新增合买配置页面，可创建计划、维护状态、查看参与记录并添加参与金额；同步更新架构说明、TODO 和后端 API 规格，并通过格式、检查、测试、构建、API 冒烟和浏览器验证。

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `e681e09` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 22: 调度配置后台编辑

**Date**: 2026-06-02
**Task**: 调度配置后台编辑
**Branch**: `main`

### Summary

实现调度配置后台编辑：新增 PUT /api/admin/draw-scheduler/config，支持在后台保存启用状态、执行周期、未来期号缓冲和封盘提前秒数；已启动调度循环每轮读取最新配置；前端常驻调度卡片新增配置表单和保存能力；同步更新架构说明、TODO 和后端 API 规格，并通过格式、检查、测试、构建、API 冒烟和浏览器验证。

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `1baa1fb` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 23: 用户权限维护侧边栏

**Date**: 2026-06-02
**Task**: 用户权限维护侧边栏
**Branch**: `main`

### Summary

将用户维护、账号维护、角色维护从用户权限管理页面常驻表单改为 SideSheet 打开，补充架构设计、TODO 和前端组件规范，并完成构建与浏览器验证。

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `3b93fd3` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 24: 客服会话使用 Semi Chat

**Date**: 2026-06-02
**Task**: 客服会话使用 Semi Chat
**Branch**: `main`

### Summary

将在线客服消息记录从手写列表改为 Semi UI Chat 组件展示，保留原有后台回复表单，关闭 Chat 默认输入区和上传能力，补充架构设计、TODO 与前端组件规范，并完成构建和浏览器验证。

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `cf87370` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 25: 在线客服仅保留回复入口

**Date**: 2026-06-02
**Task**: 在线客服仅保留回复入口
**Branch**: `main`

### Summary

移除在线客服后台新建会话表单和创建逻辑，页面只保留用户会话列表、Semi Chat 消息记录、状态维护和后台回复入口；清理 hook 的创建函数与用户列表加载，并完成构建和浏览器验证。

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `84bc6d5` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 26: 后台彩种控制台

**Date**: 2026-06-02
**Task**: 后台彩种控制台
**Branch**: `main`

### Summary

新增后台彩种控制台，按彩种展示销售状态、当前期号、封盘倒计时、开奖倒计时和最近开奖号码；入口接入 dashboard 与侧边栏，文档和 TODO 已同步。

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `d511e3a` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 27: 后台登录鉴权与权限拦截

**Date**: 2026-06-02
**Task**: 后台登录鉴权与权限拦截
**Branch**: `main`

### Summary

新增后台登录页、登录/当前管理员/登出接口、内存 Bearer Token 会话、API 权限中间件和按角色权限过滤菜单/工作台模块。

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `1d758e1` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 28: dashboard 数据按权限裁剪

**Date**: 2026-06-03
**Task**: dashboard 数据按权限裁剪
**Branch**: `main`

### Summary

完成 dashboard 数据按权限裁剪：/api/admin/dashboard 读取当前 AdminAuthSession.scopes，后端按 users/orders/finance/admins/roles/systemSettings/lotteries/customerService/robots/rebates 过滤模块、指标和摘要字段；无权限数组清空，财务/注册/返利对象返回安全默认值。已更新 架构设计.md、TODO.md 和 backend api-contracts spec。验证通过 cargo fmt --check、cargo check、cargo test、npm run build，并用低权限 token 做 API 冒烟。

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `ed78d37` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 29: 管理员密码哈希与重置基础

**Date**: 2026-06-03
**Task**: 管理员密码哈希与重置基础
**Branch**: `main`

### Summary

完成管理员密码哈希与重置基础：新增 Argon2id 密码哈希依赖，AccessStore 改为按管理员 ID 保存密码哈希；新增 AdminSaveRequest、AdminPasswordResetRequest 和 PATCH /api/admin/admins/{id}/password；前端账号维护 SideSheet 支持初始密码和重置密码。已更新 架构设计.md、TODO.md 和 backend api-contracts spec。验证通过 cargo fmt --check、cargo check、cargo test、npm run build，并用 API 冒烟确认创建账号必须传密码、重置后旧密码失败、新密码成功且读接口不泄露密码字段。

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `09f4657` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 30: API68 福彩 3D 开奖源接入

**Date**: 2026-06-03
**Task**: API68 福彩 3D 开奖源接入
**Branch**: `main`

### Summary

完成 API68 福彩 3D 开奖源接入：新增 `ApiDrawSourceRepository` 和 API68 响应解析，应用启动时为 `fc3d` 注入 `api68-fc3d`，手动开奖和自动任务共用外部源结果；API68 未命中期号或请求失败时不生成假号码，手动开奖返回统一错误，自动任务写入 `skippedIssues` 后继续处理其他期号。已更新 `架构设计.md`、`TODO.md` 和后端 API 契约规范。

### Main Changes

- 新增 `backend/src/services/draw_api.rs`，支持 API68 `preDrawIssue` 匹配和 `preDrawCode` 解析。
- `DrawRepository` 支持注入 API 开奖源，生产应用默认注入 API68，测试内存仓储保持可无网络运行。
- 自动封盘开奖任务捕获开奖失败并记录跳过原因，避免单个外部源失败拖垮整轮任务。
- `GET /api/admin/draw-sources` 展示 `API68 福彩 3D`，可复用彩种为 `fc3d`。

### Git Commits

| Hash | Message |
|------|---------|
| `7fb3a9e` | feat: 接入 API68 福彩 3D 开奖源 |
| `112c205` | chore(task): archive 06-03-api68-draw-source |

### Testing

- [OK] `cargo fmt --check`
- [OK] `cargo check`
- [OK] `cargo test`，101 个测试通过
- [OK] `npm run build`
- [OK] API 冒烟：`fc3d/2026143` 开奖返回 `3,7,6`；`fc3d/2099999` 返回 404 且不写入号码；自动任务将未命中 API 期号写入 `skippedIssues` 并继续处理平台期号。

### Status

[OK] **Completed**

### Next Steps

- 开奖源配置 CRUD、API68 原始响应留痕、失败重试队列、排列 3 复用福彩 3D 结果映射和开奖期号持久化。


## Session 30: 开奖源配置与多彩种复用

**Date**: 2026-06-03
**Task**: 开奖源配置与多彩种复用
**Branch**: `main`

### Summary

完成后台开奖源配置维护能力，API68 来源可绑定并复用到多个 API 彩种，前端新增开奖源配置面板并完成 API 冒烟验证。

### Main Changes

- 完成后端开奖源配置仓储与 CRUD 接口，默认 `api68-fc3d` 可复用到 `fc3d`、`pl3`。
- 完成前端“开奖源配置”维护面板，可新增、编辑、删除 API68 来源并绑定多个 API 彩种。
- 完成 `pl3/2026143` 复用 API68 `lotCode=10041` 开奖验证，开奖号码为 `3,7,6`。
- 完成重复绑定冲突验证，重复绑定 `fc3d` 返回 HTTP 409。
- 已更新 `架构设计.md`、`TODO.md` 与 `.trellis/spec/backend/api-contracts.md`。
- 验证：`cargo fmt --check`、`cargo check`、`cargo test` 通过；`npm run build` 通过，仅保留既有 chunk size warning；浏览器自动化因当前环境缺少可用 browser tool/Playwright 未执行。


### Git Commits

| Hash | Message |
|------|---------|
| `9fd8f23` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 31: 福彩3D真实期号生成修复

**Date**: 2026-06-03
**Task**: 福彩3D真实期号生成修复
**Branch**: `main`

### Summary

修复福彩 3D 自动生成时间戳期号的问题，改为基于 API68 最新 preDrawIssue 生成真实 7 位期号，并让排列 3 复用同一规则。

### Main Changes

- 新增 API68 最新 `preDrawIssue` 解析能力，开奖匹配和期号生成共用同一响应校验入口。
- 福彩 3D、排列 3 绑定 `api68-fc3d` 来源时，预览、单期生成、批量生成和调度补期都会按外部最新 7 位期号递增。
- 当 API68 当前最新为 `2026143` 时，福彩 3D 下一期生成 `2026144`；本地已有 `2026144` 后继续生成 `2026145`。
- 常驻调度遇到 API 最新期号缺失时跳过对应彩种并记录原因，平台彩种继续补期。
- 已更新 `架构设计.md`、`TODO.md` 与 `.trellis/spec/backend/api-contracts.md`。
- 验证：`cargo fmt --check`、`cargo check`、`cargo test` 通过，后端测试 112 个；API 冒烟使用 `PORT=18102` 验证真实 API68 生成 `2026144/2026145`，`pl3` 预览生成 `2026144`。


### Git Commits

| Hash | Message |
|------|---------|
| `fce7c21` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 32: 开奖期号与开奖源页面优化

**Date**: 2026-06-03
**Task**: 开奖期号与开奖源页面优化
**Branch**: `main`

### Summary

把开奖期号与开奖源页面重排为概览指标、分段工作区和 SideSheet 维护表单，减少长页面拥挤并提升运营扫描效率。

### Main Changes

- 页面新增期号总数、待开奖、已开奖、开奖源/调度状态 4 个概览指标。
- 主工作区拆分为“期号管理”“开奖源配置”“自动任务与调度”三个分段入口。
- 创建期号、执行开奖、开奖源维护和调度配置都改用 Semi UI SideSheet 打开，主页面只保留列表、卡片摘要、状态和操作入口。
- 创建期号表单默认期号改为空，不再展示旧的 `20260602001`；默认时间改为当前时间后一小时，封盘时间为开奖前 30 秒。
- 已更新 `架构设计.md` 和 `TODO.md`。
- 验证：`npm run build` 通过，仅保留既有 chunk size warning；Vite dev server `http://127.0.0.1:5196/` 返回 HTTP 200。当前环境无可用浏览器检查工具，未执行截图级视觉验证。


### Git Commits

| Hash | Message |
|------|---------|
| `e16ffe8` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 33: 彩种控制台状态筛选

**Date**: 2026-06-03
**Task**: 彩种控制台状态筛选
**Branch**: `main`

### Summary

彩种控制台新增本地状态筛选条，可按销售开启、已停售、开盘中、待开奖、已开奖和无当前期过滤彩种卡片。

### Main Changes

- 新增彩种控制台状态筛选：全部、销售开启、已停售、开盘中、待开奖、已开奖、无当前期。
- 每个筛选项显示匹配数量，当前筛选高亮，卡片列表即时按前端本地数据过滤。
- 无匹配结果时显示筛选空状态，不影响原有接口和轮询逻辑。
- 已更新 `架构设计.md` 和 `TODO.md`。
- 验证：`npm run build` 通过，仅保留既有 chunk size warning。


### Git Commits

| Hash | Message |
|------|---------|
| `db54d8c` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 34: 用户管理显示邀请码

**Date**: 2026-06-03
**Task**: 用户管理显示邀请码
**Branch**: `main`

### Summary

用户管理接口新增只读 inviteCodes，后端按邀请人聚合邀请码，前端用户列表新增邀请码列并同步契约文档。

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `b37bbb9` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 35: 全员邀请码中文日志与澳洲5分彩接入

**Date**: 2026-06-03
**Task**: 全员邀请码中文日志与澳洲5分彩接入
**Branch**: `main`

### Summary

用户摘要改为单个 inviteCode，普通用户码创建邀请关系返回无效；后端日志 message 中文化；新增 au5 澳洲 5 分彩和 API68 10010 来源。

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `a4c987c` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 36: Docker 单镜像打包与 GitHub 上传

**Date**: 2026-06-03
**Task**: Docker 单镜像打包与 GitHub 上传
**Branch**: `main`

### Summary

新增前后端同项目 Docker 单镜像部署方案，使用 Nginx 服务管理后台并反向代理后端 API；补充中文部署说明和容器部署 code-spec；完成本地质量检查、Docker 构建和临时容器健康检查。GitHub 上传等待配置远端仓库。

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `d33c04a` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 37: GitHub Actions CI 与 GHCR 发布

**Date**: 2026-06-03
**Task**: GitHub Actions CI 与 GHCR 发布
**Branch**: `main`

### Summary

新增 GitHub Actions CI workflow：push/PR/手动触发时运行 Rust 与前端质量检查并构建 Docker 单镜像；main 分支 push 时发布 ghcr.io/sydneypoole/bc 的 latest 与 sha 标签；同步更新部署说明、架构设计、TODO 和容器部署规范。

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `183ec9b` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 38: 数据库持久化接入

**Date**: 2026-06-03
**Task**: 数据库持久化接入
**Branch**: `main`

### Summary

让 Docker Compose 同时启动 PostgreSQL 与前后端单镜像应用，注入 DATABASE_URL，保留 docker run 外部数据库模式；补充中文部署说明、架构设计、TODO 和后端数据库/部署规范；验证 cargo fmt/check/test、npm run build、Compose healthcheck、/api/health 与 lotteries 表迁移。

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `05f7a94` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 39: 邀请码与澳洲5分彩采集修正

**Date**: 2026-06-03
**Task**: 邀请码与澳洲5分彩采集修正
**Branch**: `main`

### Summary

修正用户维护保存时邀请码被覆盖的问题，邀请管理按代理自动带出邀请码并只展示代理邀请人；新增 API68 澳洲 5 分彩采集预设；后端日志错误字段改为中文化 ApiError::log_message；同步架构设计和 TODO，并通过 cargo fmt/check/test 与 npm run build。

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `9602fab` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 40: 彩种控制台控制开奖号码

**Date**: 2026-06-03
**Task**: 彩种控制台控制开奖号码
**Branch**: `main`

### Summary

新增彩种控制台开奖号码控制接口和 SideSheet 维护入口，控制号码优先覆盖平台/API 开奖并支持手动彩种自动开奖。

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `1c8472d` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 41: 全后台模块数据库持久化

**Date**: 2026-06-03
**Task**: 全后台模块数据库持久化
**Branch**: `main`

### Summary

完成 state_documents 第一阶段持久化并记录用户要求后续改为业务关系表。

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `0d3aa01` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 42: 全业务关系表数据库持久化

**Date**: 2026-06-03
**Task**: 全业务关系表数据库持久化
**Branch**: `main`

### Summary

将所有已落地后台业务从 state_documents 迁移为独立 PostgreSQL 业务表持久化，删除运行时 StateDocumentRepository，补充 BusinessDatabase、业务表迁移、中文架构文档与 TODO，并完成后端和前端质量检查。

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `5950881` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 43: 开奖后自动开盘下一期修复

**Date**: 2026-06-03
**Task**: 开奖后自动开盘下一期修复
**Branch**: `main`

### Summary

修复常驻调度未来期号缓冲统计，把 closed 期号排除出可投注未来期，确保当前期封盘后自动生成下一期 open 期号；补充回归测试、架构设计、TODO 和后端 API 契约规范。

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `bc53bfb` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 44: 后台动态启用开奖调度器

**Date**: 2026-06-03
**Task**: 后台动态启用开奖调度器
**Branch**: `main`

### Summary

开奖调度器改为服务启动时始终创建后台任务，禁用时短轮询等待后台启用；后台保存 enabled=true 后无需环境变量或重启即可执行，并同步测试和中文文档。

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `d0c243c` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 45: 澳洲 5 分彩端到端开奖流程跑通

**Date**: 2026-06-03
**Task**: 澳洲 5 分彩端到端开奖流程跑通
**Branch**: `main`

### Summary

重建最新 Docker Compose 镜像并补齐 PostgreSQL 迁移后，通过后台 API 登录、创建澳洲 5 分彩到期期号和中奖订单、启用常驻调度，验证 API68 自动开奖、订单结算、资金入账和下一期 open 期号补齐全部跑通。

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `2e245d0` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 46: 充值返利真实入账修复

**Date**: 2026-06-06
**Task**: 充值返利真实入账修复
**Branch**: `main`

### Summary

补齐充值成功后给上级代理发放返利的真实资金流水链路，并同步后台、手机端展示、迁移注释和中文文档。

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `8e16f8b` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 47: 补单机器人展示名随机化

**Date**: 2026-06-26
**Task**: 补单机器人展示名随机化
**Branch**: `main`

### Summary

修复补单机器人账号 X90002-X90010 暴露固定用户名的问题，统一机器人账号识别、随机中文展示名、文档和规格口径。

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `6f30a94d` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 48: 调整 Redis 避奖风险池 TTL

**Date**: 2026-06-26
**Task**: 调整 Redis 避奖风险池 TTL
**Branch**: `main`

### Summary

将 Redis 开奖赔付风险池 key 的过期时间从 7 天调整为 12 小时，并同步更新架构设计、后端数据库规格和 TODO 验证记录。

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `1f5203b3` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 49: 修正 Redis 避奖风险池写分

**Date**: 2026-06-26
**Task**: 修正 Redis 避奖风险池写分
**Branch**: `main`

### Summary

修正 Redis 避奖风险池批量 ZINCRBY 的返回解析，避免把新分数列表按空返回处理，并同步记录 TODO 与后端数据库规格。

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `eb18475a` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete
