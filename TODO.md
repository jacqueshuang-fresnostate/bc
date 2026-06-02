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

## 2026-06-02 15:37:14 HKT

- 完成任务：实现 `06-02-play-rule-engine-foundation` 玩法规则引擎阶段，新增后端玩法规则领域模型和服务层，支持 3 位直选、组三复式、组三胆拖、组六复式、组六胆拖，以及 5 位前/中/后 3 直选、直选组合、组三、组六、胆拖和大小单双；新增 `GET /api/admin/play-rules` 与 `POST /api/admin/play-rules/evaluate`，并在管理后台新增“玩法规则”真实页面。
- 解决问题：彩票后台此前只有彩种入口和静态占位，缺少订单、计奖、派奖复用的核心规则能力；本次把注数计算、投注展开和中奖判断放到后端服务层，避免后续投注和派奖依赖前端临时计算。实现中保留了用户已有的 `admin/vite.config.ts`、`backend/src/main.rs` 端口改动和 `.idea/` 文件，不纳入本阶段提交。
- 验证结果：`curl` 冒烟测试确认规则目录、3 位直选评估和 5 位大小单双评估返回统一 API 信封且命中结果正确；浏览器打开 `http://127.0.0.1:5174/` 后进入“玩法规则”页面并计算出 `247` 命中；`cargo fmt --check`、`cargo check`、`cargo test`、`npm run build` 均通过。
- 后续动作：下一阶段应优先实现订单与投注模块，把规则引擎接入订单创建、投注金额校验和投注明细保存；随后继续推进开奖源、期号、计奖、派奖、用户资金、合买和机器人流程。

## 2026-06-02 15:54:58 HKT

- 完成任务：实现 `06-02-order-betting-foundation` 订单与投注基础阶段，新增后端订单领域模型、内存订单仓储、订单创建/列表/详情/取消接口；订单创建会读取彩种配置并复用玩法规则引擎计算注数、展开投注和订单金额。管理后台新增“订单管理”真实页面，并在工作台新增“最近订单”展示。
- 解决问题：此前订单管理只是占位，dashboard 最近订单也是静态演示数据，后续开奖、计奖、派奖和机器人没有真实订单入口；本次建立了基础订单数据流，并确保金额由后端按 `stakeCount * unitAmountMinor` 计算，不让前端传最终金额。用户已有的 `admin/vite.config.ts`、`backend/src/main.rs` 端口改动和 `.idea/` 继续保留，不纳入本阶段提交。
- 验证结果：`curl` 冒烟测试通过，确认创建 3 位直选订单得到 `stakeCount=1`、`amountMinor=200`、`expandedBets=["247"]`，订单列表和 dashboard 最近订单能回流；浏览器打开订单管理页成功创建订单，并在工作台看到最近订单；`cargo check`、`cargo test`、`npm run build` 已通过，后端测试增加到 24 个。
- 后续动作：下一阶段建议实现开奖期号与开奖源模块，随后把订单接入计奖、派奖和用户资金流水；订单数据库持久化也需要单独排期。
