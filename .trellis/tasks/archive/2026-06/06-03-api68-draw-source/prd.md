# API68 彩种开奖源接入

## Goal

把用户提供的 API68 开奖接口接入后台开奖链路，使 `fc3d` 福彩 3D 在 API 开奖模式下可以按期号拉取真实开奖号码，并继续保持系统统一的英文逗号分隔开奖号码格式。

## What I Already Know

- 用户提供的接口是 `https://api.api68.com/QuanGuoCai/getLotteryInfoList.do?lotCode=10041`。
- 实测接口返回 `errorCode=0`、`result.businessCode=0`，开奖数据位于 `result.data[]`。
- 单期开奖字段包含 `preDrawIssue`、`preDrawCode`、`preDrawTime`，其中 `preDrawCode` 形如 `3,7,6`。
- 项目已有开奖期号仓储、`PATCH /api/admin/draw-issues/{id}/draw`、自动封盘开奖结算任务和统一 API 错误信封。
- 当前 `api` 开奖模式仍使用本地生成器，这是本任务要修正的核心问题。

## Assumptions

- `lotCode=10041` 对应福彩 3D，先绑定现有彩种 `fc3d`。
- API68 返回的期号需要和后台期号 `issue` 做字符串精确匹配，例如 `2026143`。
- API68 未返回对应期号或接口失败时，不能用本地随机/生成器静默兜底。
- 暂不把福彩 3D 结果自动复用给排列 3，后续需要开奖源配置界面时再做显式映射。

## Requirements

- 新增后端 API68 开奖源服务，支持按彩种 ID 和期号获取开奖号码。
- `fc3d` 在 `DrawMode::Api` 开奖时优先请求 API68，并用 `preDrawIssue` 匹配当前期号。
- 拉到的 `preDrawCode` 仍通过既有开奖号码校验，保存为英文逗号分隔格式。
- API68 返回失败、响应结构异常、无对应期号时返回统一 `ApiError`，不得静默生成号码。
- 自动开奖任务遇到外部 API 开奖失败时，应把该期号写入 `skippedIssues` 并继续处理其他期号。
- `GET /api/admin/draw-sources` 需要展示 API68 福彩 3D 开奖源。
- 更新 `架构设计.md`、`TODO.md` 和后端 API 契约规范。

## Acceptance Criteria

- [ ] `fc3d` 期号 `2026143` 调用后台开奖接口后，保存 API68 返回的 `3,7,6`。
- [ ] `fc3d` 不存在于 API68 响应中的期号开奖会返回业务错误，不生成假号码。
- [ ] 自动任务遇到 API68 未命中期号时不会整轮失败，会在 `skippedIssues` 记录原因。
- [ ] `draw-sources` 和 dashboard 开奖源摘要显示 API68 福彩 3D 源。
- [ ] 后端单元测试覆盖 API68 响应解析、期号匹配、失败返回和自动任务跳过。
- [ ] `cargo fmt --check`、`cargo check`、`cargo test`、`npm run build` 通过。

## Out Of Scope

- 开奖源 CRUD 配置界面。
- API68 原始响应持久化、失败重试队列和异常复核流程。
- API68 结果复用给排列 3 的可配置映射。
- 开奖期号数据库持久化。

## Technical Notes

- 主要修改文件预计为 `backend/src/services/draw.rs`、`backend/src/services/automation.rs`、`backend/src/services/dashboard.rs`、`backend/src/app.rs` 和 `backend/Cargo.toml`。
- 需要新增 HTTP client 依赖，优先使用 `reqwest` + `rustls-tls`。
- `DrawRepository::memory()` 继续保留无外部源模式，方便单元测试和本地演示；应用启动时注入默认 API68 源。
- 需遵守 `.trellis/spec/backend/quality-guidelines.md`：开奖结果不能静默兜底。
