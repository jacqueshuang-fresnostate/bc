# 开奖期号与开奖源基础

## 目标

补齐彩票系统的开奖期号与开奖源基础能力：后端可以创建开奖期号、关闭销售、按彩种开奖模式生成或录入开奖号码、查询开奖记录；管理后台可以进入“开奖期号与开奖源”页面维护期号和执行开奖。这个阶段为后续订单计奖、派奖、资金流水和机器人执行提供开奖结果入口。

## 已知信息

- 当前彩种已有 `drawMode` 和 `schedule` 配置，支持 `platform`、`api`、`manual` 三种开奖模式。
- 当前 dashboard 有静态 `drawSources`，但没有真实期号、开奖记录或开奖操作。
- 当前订单已支持创建、查询和取消，但订单不会因为开奖自动计奖。
- 当前工作区存在用户或 IDE 产生的未提交变更：`admin/vite.config.ts`、`backend/src/main.rs`、`.idea/`。本任务会保留这些变更，不主动提交或覆盖。
- 项目文档必须使用中文输出。

## 本轮范围

### 后端

- 新增开奖领域模型：
  - 开奖期号状态。
  - 开奖期号详情。
  - 创建期号请求。
  - 执行开奖请求。
- 新增内存开奖仓储：
  - 创建期号。
  - 列表查询。
  - 详情查询。
  - 关闭销售。
  - 执行开奖。
  - 取消未开奖期号。
- 新增 API：
  - `GET /api/admin/draw-sources`
  - `GET /api/admin/draw-issues`
  - `GET /api/admin/draw-issues/{id}`
  - `POST /api/admin/draw-issues`
  - `PATCH /api/admin/draw-issues/{id}/close`
  - `PATCH /api/admin/draw-issues/{id}/draw`
  - `PATCH /api/admin/draw-issues/{id}/cancel`
- 开奖号码校验：
  - 3 位彩种开奖号码必须是 3 位数字。
  - 5 位彩种开奖号码必须是 5 位数字。
- 开奖模式规则：
  - `manual` 彩种必须由管理员录入开奖号码。
  - `platform` 彩种由本地平台生成器生成开奖号码。
  - `api` 彩种本阶段使用本地模拟 API 生成器生成开奖号码，不请求外部网络。
- 暂不触发订单计奖或派奖。

### 管理后台

- 新增“开奖期号与开奖源”真实页面。
- “开奖模式”和“开奖时间”两个导航入口都进入该页面。
- 页面展示开奖源说明、期号列表、状态、开奖号码和开奖模式。
- 页面支持创建期号、关闭销售、执行开奖、取消期号。
- 手动开奖模式支持录入开奖号码，平台/API 模式不需要录入号码。

## 暂不包含

- 不接真实第三方开奖 API。
- 不实现定时任务自动创建期号。
- 不实现自动封盘。
- 不计奖、不派奖、不更新订单状态。
- 不写入数据库。
- 不实现开奖源配置 CRUD。
- 不实现开奖记录删除。

## 数据契约草案

创建期号请求：

```json
{
  "lotteryId": "fc3d",
  "issue": "2026156",
  "scheduledAt": "2026-06-02 21:00:15",
  "saleClosedAt": "2026-06-02 20:59:45"
}
```

执行开奖请求：

```json
{
  "drawNumber": "247"
}
```

平台/API 开奖模式可以不传 `drawNumber`，后端会生成号码。

开奖期号响应：

```json
{
  "id": "D000000000001",
  "lotteryId": "fc3d",
  "lotteryName": "福彩 3D",
  "issue": "2026156",
  "numberType": "threeDigit",
  "drawMode": "api",
  "scheduledAt": "2026-06-02 21:00:15",
  "saleClosedAt": "2026-06-02 20:59:45",
  "status": "open",
  "drawNumber": null,
  "drawnAt": null,
  "createdAt": "unix:1780388000"
}
```

## 验收标准

- [x] 后端支持创建期号、列表、详情、关闭销售、执行开奖和取消期号。
- [x] 后端能按彩种号码类型校验手动开奖号码长度。
- [x] 平台开奖和 API 开奖能生成符合号码类型长度的数字字符串。
- [x] 手动开奖模式缺少开奖号码时会返回错误。
- [x] 已开奖期号不能重复开奖或取消。
- [x] 管理后台新增真实“开奖期号与开奖源”页面。
- [x] “开奖模式”和“开奖时间”导航入口都能进入该页面。
- [x] 浏览器可以创建期号并执行开奖。
- [x] `cargo fmt --check`、`cargo check`、`cargo test`、`npm run build` 通过。

## 完成定义

- 开奖业务逻辑集中在服务层，不写进路由处理函数。
- 开奖 API 返回统一 API 信封。
- 前端 API client、类型、hook 与后端契约一致。
- 架构、TODO、规格和任务记录同步更新。
- 完成本阶段后提交 Git，并归档任务。

## 技术方案

- 后端新增 `domain/draw.rs` 和 `services/draw.rs`。
- `AppState` 增加 `draws: DrawRepository`。
- `routes/admin.rs` 增加 draw source 和 draw issue API。
- `services/dashboard.rs` 暴露 `draw_sources()`，让 dashboard 和独立接口复用同一份开奖源摘要。
- 前端新增 `types/draws.ts`、`useDraws`、`DrawManagementPage`。
- `App.tsx` 中把 `draw-modes` 和 `schedules` 都指向 `DrawManagementPage`。

## 风险与约束

- 本阶段的 API 开奖是本地模拟生成，不代表真实第三方 API 对接。
- 开奖号码会成为后续计奖派奖的核心输入，必须在后端校验号码长度和数字格式。
- 期号当前是内存模式，服务重启后会丢失，后续需要数据库持久化。
- 开奖后暂不更新订单状态，避免在没有奖金表和派奖流程时产生错误资金状态。

## 技术备注

- 已阅读：`AGENTS.md`、`架构设计.md`、现有彩种 `drawMode`/`schedule`、dashboard 静态开奖源、订单基础模块。
- 相关后端文件：`backend/src/domain/lottery.rs`、`backend/src/services/dashboard.rs`、`backend/src/routes/admin.rs`、`backend/src/app.rs`。
- 相关前端文件：`admin/src/App.tsx`、`admin/src/components/AppShell.tsx`、`admin/src/types/dashboard.ts`、`admin/src/pages/LotteryManagementPage.tsx`。
- 任务创建时间：2026-06-02 HKT。

## 完成记录

- 完成时间：2026-06-02 16:11:08 HKT。
- 后端新增 `DrawRepository` 和开奖 API，服务层集中处理期号状态流转、号码长度校验、手动开奖必填校验、平台/API 本地号码生成。
- 前端新增 `DrawManagementPage`、`useDraws`、`types/draws.ts` 和 draw API client，侧边栏“开奖模式”和“开奖时间”均进入真实页面。
- API 验证：`draw-sources`、创建期号、封盘、API 开奖和手动开奖均通过。
- 浏览器验证：在 `http://127.0.0.1:5174/` 创建 `20260602001` 并开奖，页面回显 `978` 和“已开奖”。
- 质量验证：`cargo fmt --check`、`cargo check`、`cargo test`、`npm run build` 均通过。
