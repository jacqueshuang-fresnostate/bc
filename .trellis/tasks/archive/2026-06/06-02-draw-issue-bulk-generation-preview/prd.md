# 批量预生成期号和计划预览

## 目标

在已有“按计划生成下一期”能力之上，新增批量预生成多期期号和生成计划预览，让管理员可以先查看系统将按彩种开奖计划生成哪些期号，再一次性创建多期，减少逐期点击和人工核对时间。

## 已知信息

* 用户要求持续完善彩票管理后台，并且文档输出必须使用中文。
* `架构设计.md` 的“自动创建下一期号基础范围”后续项明确包含“批量预生成多期和期号生成计划预览”。
* 当前后端已有 `POST /api/admin/draw-issues/generate-next` 和 `draw_generation` 服务，可按周期、每日、周开奖生成单期。
* 当前管理后台“开奖期号与开奖源”页面已有“按计划生成下一期”按钮，可复用同一处 UI 区域扩展批量功能。
* 开奖号码与期号相关接口已经使用统一 API 信封；手动开奖、平台/API 开奖号码统一使用英文逗号分隔。

## 临时假设

* 本阶段只做后台触发式批量生成和预览，不做系统级常驻调度。
* 批量数量先限制为 1 到 50，避免管理员误生成过多演示期号。
* 预览不会写入仓储；批量生成会写入仓储，并返回实际创建的期号列表。
* 如果预览或生成时遇到已有同彩种同 issue，后端应跳过冲突并继续往后寻找可用期号，而不是返回重复期号。
* 默认封盘提前时间继续沿用 30 秒，请求可覆盖。

## 需求

* 后端新增批量期号生成请求模型，包含彩种 ID、基准时间、生成数量和可选封盘提前秒数。
* 后端新增计划预览接口，返回将生成的 issue、scheduledAt、saleClosedAt，但不创建期号。
* 后端新增批量生成接口，按同一计划创建多期并返回创建结果。
* 单期生成、批量预览和批量生成应复用同一套计划计算逻辑，避免时间规则分叉。
* 前端在开奖期号页面新增生成数量输入、计划预览按钮和批量生成按钮。
* 前端应展示预览结果，并在批量生成成功后刷新期号列表、选中新创建的第一条或最后一条期号。
* `架构设计.md` 和 `TODO.md` 必须同步记录本阶段完成内容与验证结果。

## 验收标准

* [ ] `POST /api/admin/draw-issues/preview-generation` 能返回指定彩种未来 N 期计划，且不改变期号列表。
* [ ] `POST /api/admin/draw-issues/generate-batch` 能创建指定彩种未来 N 期并返回列表。
* [ ] 周期、每日、周开奖三种计划均有后端测试覆盖。
* [ ] 同彩种已有期号作为基线时，批量生成能从最新期开奖时间继续向后生成。
* [ ] 前端可以输入数量，点击预览看到待生成期号，再点击批量生成后列表刷新。
* [ ] `cargo fmt --check`、`cargo check`、`cargo test`、`npm run build` 通过。
* [ ] 浏览器验证开奖期号页面无控制台错误，批量预览和生成可用。

## 完成定义

* 代码实现完成并通过质量检查。
* 中文文档、架构说明、TODO 记录同步更新。
* Trellis 任务归档并记录 journal。
* 不提交用户已有的无关脏改：`admin/vite.config.ts`、`backend/src/main.rs`、`.idea/`。

## 本阶段不包含

* 系统级常驻调度进程。
* 自动任务失败重试队列。
* 开奖 API 源数据审计。
* 开奖期号 PostgreSQL 持久化和数据库唯一约束。
* 手机端购彩页面。

## 技术备注

* 相关后端文件：`backend/src/services/draw_generation.rs`、`backend/src/routes/admin.rs`、`backend/src/domain/draw.rs`。
* 相关前端文件：`admin/src/pages/DrawManagementPage.tsx`、`admin/src/hooks/useDraws.ts`、`admin/src/api/client.ts`、`admin/src/types/draws.ts`。
* 相关规格：`.trellis/spec/backend/api-contracts.md`、`.trellis/spec/backend/error-handling.md`、`.trellis/spec/frontend/component-guidelines.md`、`.trellis/spec/frontend/hook-guidelines.md`。
