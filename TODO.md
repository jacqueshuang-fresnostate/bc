# TODO

## 2026-06-02 13:36:05 HKT

- 完成任务：启动彩票系统首个开发任务 `06-02-admin-backend-foundation`，并创建本轮 PRD。
- 解决问题：项目当前只有需求文档、规则文档和 Trellis 配置，没有可运行工程；先把开发范围收敛为“后端 + 管理后台 MVP 骨架”，避免一次性铺开全部业务导致边界不清。
- 后续动作：补齐初始开发规格，创建 Rust 后端和 React 管理后台工程，并同步更新 `架构设计.md`。

## 2026-06-02 13:40:23 HKT

- 完成任务：在 `AGENTS.md` 中加入“项目文档使用中文输出”的规则，并把本轮新增/改写的 PRD 与开发规格文档改为中文。
- 解决问题：此前 Trellis 规格文档沿用了英文模板，与用户要求的中文文档输出规则不一致；现在已统一为中文，代码标识、命令、路径、第三方库名和协议字段名保留原文。
- 后续动作：继续完成 Rust 后端和 React 管理后台基础工程，并在实现过程中继续维护 `TODO.md`。

## 2026-06-02 13:50:30 HKT

- 完成任务：创建 Rust 后端工程 `backend/`，实现 Axum API、统一响应信封、健康检查、管理后台概览接口、核心领域模型和演示数据服务；创建 React 管理后台 `admin/`，实现应用外壳、侧边导航、系统概览、模块入口、接口加载和错误状态。
- 解决问题：项目没有可运行工程和跨层接口契约；本次建立了 `/api/health`、`/api/admin/dashboard` 以及前端对应类型，保证管理后台可以先围绕真实 API 结构运行起来。`8080` 被本机其他进程占用，联调改用后端 `18080` 和前端 `5174`，避免影响已有服务。
- 验证结果：`cargo fmt`、`cargo check`、`cargo test`、`npm run build` 均通过；浏览器打开 `http://localhost:5174/` 后确认工作台、彩种开奖源、用户管理入口正常显示，点击“用户管理”可进入占位页面，控制台无错误。
- 后续动作：进入质量复查，确认文档、规格、架构说明与代码保持一致；下一阶段可开始接入数据库、认证权限或彩种管理真实 CRUD。

## 2026-06-02 13:52:50 HKT

- 完成任务：完成 Trellis 质量复查和规格沉淀，新增 `.trellis/spec/backend/api-contracts.md`，记录 `/api/health`、`/api/admin/dashboard`、统一响应信封、`PORT`、`VITE_API_BASE_URL`、金额最小单位和返利 basis points 契约；同时补充前端类型安全规范和 Semi UI 样式导入注意事项。
- 解决问题：构建过程中发现 `tsc -b` 会生成 `vite.config.js`、`vite.config.d.ts` 和 `*.tsbuildinfo` 等副产物，已改为 `tsc --noEmit` 双配置检查，避免构建污染源码目录；前端错误提示也从固定 `8080` 改为检查 `VITE_API_BASE_URL`，适配非默认端口联调。
- 验证结果：重新运行 `cargo fmt --check`、`cargo check`、`cargo test`、`npm run build` 均通过。
- 后续动作：项目根目录当前不是 Git 仓库，无法按 Trellis Phase 3.4 生成工作提交；后续如果需要完整任务归档和提交记录，需要先在项目根或包目录初始化/进入 Git 仓库。

## 2026-06-02 14:16:33 HKT

- 完成任务：实现 `06-02-lottery-management-crud` 彩种管理阶段，新增后端内存彩种仓储、彩种 CRUD 与销售开关接口，并把管理后台“彩种管理”入口替换为可新增、编辑、删除和切换销售状态的真实页面。
- 解决问题：此前彩种只存在于 dashboard 静态演示数据中，无法维护配置；本次用共享 `LotteryStore` 让列表接口和 dashboard 使用同一份数据。接口联调时发现 `DrawSchedule` 枚举变体字段没有按前端契约接受 `intervalSeconds`，已通过 `rename_all_fields = "camelCase"` 修复，并新增序列化/反序列化测试。
- 验证结果：HTTP 冒烟测试通过，确认 `GET/POST/PATCH/DELETE /api/admin/lotteries` 和 `/api/admin/dashboard` 数据一致；浏览器验证通过，彩种管理页从 4 条新增到 5 条再删除回 4 条；`cargo fmt --check`、`cargo check`、`cargo test`、`npm run build` 均通过。
- 后续动作：提交 Git；下一阶段可进入数据库持久化、开奖源配置或鉴权权限。

## 2026-06-02 15:10:45 HKT

- 完成任务：实现 `06-02-lottery-database-persistence` 彩种数据库持久化阶段，新增 SQLx PostgreSQL 依赖、`lotteries` 表迁移、统一彩种仓储入口和 PostgreSQL 彩种仓储；后端会根据 `DATABASE_URL` 自动选择数据库模式或内存模式。
- 解决问题：上一阶段彩种数据服务重启后会丢失；本次在配置数据库时可持久化彩种 CRUD 和销售状态，同时保留无数据库 fallback。实现中发现 SQLx `0.9.0` 要求 Rust `1.94.0`，当前工具链是 Rust `1.92.0`，已改用兼容的 SQLx `0.8.6` 并记录到 PRD 和调研文档。
- 验证结果：无 `DATABASE_URL` 启动后端成功，`/api/health`、`/api/admin/lotteries` 和 `/api/admin/dashboard` 冒烟测试通过；`cargo fmt --check`、`cargo check`、`cargo test` 通过，后端 11 个测试全绿；`npm run build` 通过。
- 后续动作：同步数据库/API 规格并完成 Git 提交；下一阶段可进入开奖源配置、数据库容器化联调或鉴权权限。
