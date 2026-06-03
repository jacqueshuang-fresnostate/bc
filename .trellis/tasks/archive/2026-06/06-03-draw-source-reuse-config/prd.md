# 开奖源配置与多彩种复用

## Goal

把当前硬编码的 API68 福彩 3D 开奖源升级为后台可维护的开奖源配置，让一个 API 源可以绑定多个 API 开奖彩种复用同一开奖结果，例如 `fc3d` 和 `pl3` 复用 API68 `lotCode=10041` 返回的 `preDrawCode`。

## What I Already Know

- 上一阶段已接入 API68 全国彩接口：`https://api.api68.com/QuanGuoCai/getLotteryInfoList.do?lotCode=10041`。
- 当前后端 `ApiDrawSourceRepository` 只按单个 `lottery_id` 查找来源，默认 `api68-fc3d` 只绑定 `fc3d`。
- 当前 `GET /api/admin/draw-sources` 只返回摘要，不支持新增、编辑、删除或调整可复用彩种。
- `DrawManagementPage` 已展示开奖源卡片，页面已有彩种列表，可用于配置可复用彩种。
- 用户明确询问“这个 API 的配置可以多个彩种复用么”，并确认继续实现。

## Assumptions

- 本阶段继续使用内存配置，不接 PostgreSQL。
- 默认 API68 源绑定 `fc3d` 和 `pl3`，两个彩种需要使用相同后台期号才能匹配同一条 API68 `preDrawIssue`。
- 同一 API 开奖彩种同一时间只能绑定一个 API 源，避免开奖来源歧义。
- 目前只支持 API68 provider；后续接入更多供应商时再扩展 provider 枚举和表单字段。

## Requirements

- 新增可维护的 API 开奖源配置仓储，支持列表、创建、更新和删除。
- 开奖源配置字段至少包含：`id`、`name`、`provider`、`mode`、`lotCode`、`endpoint`、`reusableForLotteryIds`、`editable`。
- 默认种子源为 `api68-fc3d`，名称为 `API68 福彩 3D/排列 3`，`provider=api68`，`lotCode=10041`，可复用彩种为 `fc3d`、`pl3`。
- `DrawRepository` 执行 API 开奖时，应按 `reusableForLotteryIds` 查找配置，命中后用该配置的 `lotCode` 请求 API68。
- 新增后台接口：
  - `GET /api/admin/draw-sources`
  - `POST /api/admin/draw-sources`
  - `PUT /api/admin/draw-sources/{id}`
  - `DELETE /api/admin/draw-sources/{id}`
- 保存开奖源时校验：
  - ID、名称、`lotCode`、复用彩种不能为空。
  - `lotCode` 只能是数字。
  - 复用彩种必须存在，且必须是 `drawMode=api`。
  - 同一个彩种不能同时绑定多个 API 源。
- 管理后台“开奖期号与开奖源”页新增配置能力：查看来源详情、选择复用彩种、保存配置、新建来源、删除来源。
- dashboard 和开奖源页面都读取同一份动态配置，不再用静态 API 源摘要。

## Acceptance Criteria

- [ ] `GET /api/admin/draw-sources` 默认返回 `api68-fc3d`，`reusableForLotteryIds` 包含 `fc3d` 和 `pl3`。
- [ ] 创建 `pl3/2026143` 期号并触发 API 开奖，会复用 API68 `10041` 返回的 `3,7,6`。
- [ ] 后台页面可以把 API68 源的复用彩种保存为 `fc3d` 或 `fc3d+pl3`。
- [ ] 保存重复绑定彩种的第二个 API 源会被拒绝。
- [ ] 删除某个 API 源后，它绑定的彩种回到未配置外部源的占位生成行为。
- [ ] `cargo fmt --check`、`cargo check`、`cargo test`、`npm run build` 通过。

## Out Of Scope

- 开奖源 PostgreSQL 持久化。
- API68 原始响应留痕、重试队列、人工复核和告警。
- 不同彩种之间的期号映射转换规则；本阶段仍按相同字符串期号匹配。
- 接入 API68 之外的其他 provider。

## Technical Notes

- 后端主要修改 `backend/src/services/draw_api.rs`、`backend/src/services/draw.rs`、`backend/src/routes/admin.rs`、`backend/src/services/dashboard.rs`、`backend/src/app.rs`、`backend/src/domain/lottery.rs`。
- 前端主要修改 `admin/src/types/dashboard.ts`、`admin/src/api/client.ts`、`admin/src/hooks/useDraws.ts`、`admin/src/pages/DrawManagementPage.tsx`。
- 需要更新 `.trellis/spec/backend/api-contracts.md`、`架构设计.md` 和 `TODO.md`。
