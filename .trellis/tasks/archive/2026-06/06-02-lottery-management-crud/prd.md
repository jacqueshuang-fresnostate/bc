# 彩种管理 CRUD 与配置页面

## 目标

把首期管理后台里的“彩种管理”入口升级为可操作页面，并在 Rust 后端提供彩种创建、查询、更新、删除和销售开关接口，让开奖模式、开奖时间、玩法分类、合买配置这些核心配置可以通过管理后台维护。

## 已知信息

- 项目已完成 Rust 后端和 React 管理后台基础骨架。
- 后端已有 `LotteryKind`、`DrawMode`、`DrawSchedule`、`GroupBuyConfig`、`PlayCategory` 等领域模型。
- 后端已有统一 API 响应信封和 `ApiError`。
- 前端已有 `/api/admin/dashboard` 类型、API client、工作台和模块入口。
- `架构设计.md` 要求依据 3 位和 5 位玩法开设彩种，并支持平台开奖、API 开奖、指定号码、不同开奖时间、合买配置。
- 当前项目文档必须使用中文输出。

## 临时假设

- 本轮先使用内存仓储保存彩种数据，不接入数据库；服务重启后数据回到初始种子数据。
- 彩种 ID 由后台创建时填写，方便后续与开奖源、订单、配置项关联。
- 本轮实现管理后台常用 CRUD 和销售开关，不实现真实开奖源 API 拉取、计奖或订单联动。
- 前端仅对彩种管理页做真实功能，其他模块仍保持占位页。

## 需求

- 后端新增彩种管理 API：列表、详情、创建、更新、删除、销售开关。
- 后端新增共享应用状态和内存彩种仓储，避免继续只返回静态不可变数据。
- 后端对彩种配置做基础校验：ID/名称不能为空、玩法不能为空、周期开奖秒数大于 0、每日/周开奖时间不能为空、合买金额大于 0、发起人最低比例在 0-100 之间。
- 后端 dashboard 中的彩种数据应来自同一份彩种仓储，保持跨接口一致。
- 前端新增彩种管理页，支持查看彩种列表、选择彩种、编辑基础信息、开奖模式、开奖时间、玩法分类、合买配置、销售开关。
- 前端新增彩种 API client 和 hook，处理 loading、error、saving 状态。
- 更新 `架构设计.md` 和 `TODO.md`，记录本阶段范围、完成内容、问题和时间。
- 如产生新的接口契约或模式，更新 `.trellis/spec/`。

## 验收标准

- [x] `GET /api/admin/lotteries` 返回彩种列表。
- [x] `POST /api/admin/lotteries` 可创建彩种，并做基础校验。
- [x] `PUT /api/admin/lotteries/:id` 可更新彩种配置。
- [x] `PATCH /api/admin/lotteries/:id/sale` 可切换销售状态。
- [x] `DELETE /api/admin/lotteries/:id` 可删除彩种。
- [x] `GET /api/admin/dashboard` 使用同一彩种仓储数据。
- [x] 管理后台“彩种管理”页面可以完成新增、编辑、删除和销售开关操作。
- [x] `cargo fmt --check`、`cargo check`、`cargo test`、`npm run build` 通过。

## 验证记录

- 2026-06-02：HTTP 冒烟测试通过，覆盖彩种列表、创建、销售开关、删除和 dashboard 数量一致性。
- 2026-06-02：浏览器打开 `http://127.0.0.1:5174/`，进入“彩种管理”，完成新增 `UI 冒烟 3D` 和删除操作，列表从 4 条变为 5 条后回到 4 条。
- 2026-06-02：修复 `DrawSchedule` 枚举字段序列化契约，确认后端接受 `intervalSeconds`。

## 完成定义

- 后端接口返回统一 API 信封。
- 前后端 TypeScript/Rust 字段契约一致。
- 关键校验有后端测试覆盖。
- 文档和 TODO 同步更新。
- 完成本阶段后提交 Git。

## 暂不包含

- 数据库持久化和迁移。
- 真实开奖源 API 对接。
- 订单、投注、计奖、派奖联动。
- 权限鉴权。
- 手机端。

## 技术方案

- 后端新增 `AppState`，内部持有 `Arc<RwLock<LotteryStore>>`。
- 后端新增 `services/lottery.rs`，管理内存彩种数据和校验逻辑。
- 后端新增 `routes/admin/lotteries` 相关接口，沿用 `ApiEnvelope` 和 `ApiError`。
- 前端新增 `useLotteries` hook 和 `LotteryManagementPage`，页面内用受控表单维护当前编辑草稿。
- dashboard 和彩种管理页共用后端仓储数据，避免工作台和列表不一致。

## 决策记录

**上下文**：下一步既可以先接数据库，也可以先做彩种配置页面。接数据库需要外部环境和迁移设计，容易拖慢核心业务页面推进。

**决策**：本轮先做内存仓储 + 彩种 CRUD + 管理后台页面，保留后续数据库替换空间。

**影响**：可以快速验证彩种配置流程；缺点是服务重启后数据不持久，后续需要数据库任务接手。

## 技术备注

- 相关后端文件：`backend/src/app.rs`、`backend/src/domain/lottery.rs`、`backend/src/routes/admin.rs`、`backend/src/services/dashboard.rs`。
- 相关前端文件：`admin/src/App.tsx`、`admin/src/api/client.ts`、`admin/src/types/dashboard.ts`。
- API 契约规范：`.trellis/spec/backend/api-contracts.md`。
