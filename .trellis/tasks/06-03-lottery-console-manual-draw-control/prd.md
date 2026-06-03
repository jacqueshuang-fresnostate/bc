# 彩种控制台手动控制开奖号码

## Goal

在“彩种控制台”为每个彩种提供开奖号码控制能力。运营开启某个彩种的控制状态并输入开奖号码后，该彩种到期开奖时应优先使用控制台配置的号码完成开奖；关闭控制后恢复原有平台随机或 API 来源开奖逻辑。

## What I Already Know

- 用户要求“彩种控制台 每个彩种都可以控制开奖号码”，开启控制后按输入号码开奖。
- 架构设计最早已定义“控制指定号码”为开奖模式之一：可以配置某个彩种某个期号的特定号码。
- 当前已有“彩种控制台”页面，复用彩种列表和开奖期号列表，支持倒计时、开奖号码展示和状态筛选。
- 当前开奖服务已经支持 `drawNumber` 英文逗号分隔校验，3 位彩种必须 3 个数字，5 位彩种必须 5 个数字。
- 自动开奖服务会处理到期 `platform` 和 `api` 期号，`manual` 期号缺少号码时跳过。

## Requirements

- 每个彩种都能在“彩种控制台”开启或关闭开奖号码控制。
- 开启控制时必须配置合法开奖号码，格式继续使用英文逗号分隔，例如 `2,4,7`、`7,8,9,4,2`。
- 后端负责按彩种号码类型校验控制号码，前端只做辅助提示。
- 当某个彩种开启控制并存在控制号码时，该彩种的到期开奖优先使用控制号码。
- 控制号码需要同时影响后台手动执行开奖和自动调度开奖，避免两个开奖入口行为不一致。
- 控制台保存后应刷新服务端事实数据并在页面回显控制状态、控制号码和更新时间。
- 关闭控制后，平台彩种恢复本地生成器，API 彩种恢复第三方开奖源。
- 保持已有用户未提交的 `admin/vite.config.ts`、`backend/src/main.rs` 和 `.idea/` 不被纳入本任务。

## Acceptance Criteria

- [ ] 后端新增彩种开奖控制配置读取/保存接口，统一响应信封，字段使用 `camelCase`。
- [ ] 服务层校验空彩种、未知彩种、号码长度、非数字和号码格式错误。
- [ ] `platform` / `api` 到期开奖时，如果彩种控制开启，则使用控制号码并完成后续结算/入账链路。
- [ ] 控制台页面可对每个彩种打开 SideSheet 或等价维护入口，保存控制状态和号码。
- [ ] 控制台卡片能展示控制状态与控制号码，保存成功后刷新。
- [ ] `架构设计.md` 和 `TODO.md` 记录本次功能、问题和完成时间。
- [ ] `cargo fmt --check`、`cargo check`、`cargo test`、`npm run build` 通过。

## Out Of Scope

- 不在本任务新增数据库持久化；当前按现有内存仓储阶段实现。
- 不新增 WebSocket/SSE。
- 不做期号级多号码队列或按具体期号预设多个未来号码；本任务先做彩种级当前控制号码。
- 不改变开奖号码英文逗号分隔约定。

## Technical Notes

- 相关后端规范：`.trellis/spec/backend/api-contracts.md`、`.trellis/spec/backend/error-handling.md`、`.trellis/spec/backend/logging-guidelines.md`、`.trellis/spec/backend/quality-guidelines.md`。
- 相关前端规范：`.trellis/spec/frontend/component-guidelines.md`、`.trellis/spec/frontend/hook-guidelines.md`、`.trellis/spec/frontend/state-management.md`、`.trellis/spec/frontend/type-safety.md`。
- 跨层数据流：控制台表单 -> API 请求 -> 后端控制仓储 -> 开奖服务读取 -> 开奖期号结果 -> 控制台轮询展示。
