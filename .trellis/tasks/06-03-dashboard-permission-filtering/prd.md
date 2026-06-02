# dashboard 数据按权限裁剪

## 背景

上一阶段已经完成后台登录、Bearer Token 鉴权、API 权限中间件和前端菜单过滤。但 `/api/admin/dashboard` 为了让系统概览可访问，没有要求具体业务权限，当前仍会在响应中带回完整的用户、管理员、角色、财务、彩种、订单、机器人、邀请返利等摘要数据。低权限管理员即使菜单看不到，也可能通过 dashboard 响应拿到无权限领域数据。

## 目标

- 后端 `/api/admin/dashboard` 根据当前管理员 `PermissionScope` 裁剪响应数据。
- dashboard 模块组、模块入口和指标在后端就按权限过滤。
- 无权限领域的数组字段返回空数组。
- 无权限领域的对象字段返回零值/安全默认值，避免泄露真实配置。
- 保留 dashboard 页面基础可用性，登录后仍能看到自己有权限的概览。
- 前端已有菜单过滤可继续保留，作为二次防护。
- 更新 `架构设计.md`、`TODO.md` 和必要 API 契约规范。

## 权限裁剪规则

- `users`：保留 `users`、`registration` 和“用户总数”指标；无权限时清空 `users`，`registration` 返回关闭状态。
- `admins`：保留 `admins`；无权限时清空。
- `roles`：保留 `roles`；无权限时清空。
- `systemSettings`：保留 `settings`；无权限时清空。
- `orders`：保留 `recentOrders`、`settlements` 模块和“今日订单”指标；无权限时清空订单。
- `finance`：保留 `finance`、`financialAccounts` 和“平台余额”指标；无权限时金额为 0、账户为空。
- `customerService`：保留客服模块入口；dashboard 当前无客服摘要字段。
- `lotteries`：保留 `lotteries`、`drawSources`、`groupBuyPlans`、彩票相关模块和“已配置彩种”指标；无权限时清空。
- `robots`：保留 `robots`；无权限时清空。
- `rebates`：保留 `invitePolicy` 和邀请返利模块；无权限时返回关闭邀请和 0 返利比例。

## 非目标

- 不改变 `/api/admin/dashboard` 的顶层字段结构。
- 不新增数据库持久化。
- 不做按钮级权限。
- 不修改已经存在的业务接口权限中间件。
- 不实现管理员操作审计。

## 验收标准

- 超级管理员登录后 dashboard 仍能看到完整模块和摘要。
- 运营角色只拥有 `users`、`orders`、`lotteries` 时，dashboard 不返回管理员、角色、财务、机器人、返利等敏感数组数据。
- 运营角色 dashboard 模块组只包含用户/订单/彩票相关入口。
- 后端新增测试覆盖 dashboard 按 scopes 裁剪。
- `cargo fmt --check`、`cargo check`、`cargo test`、`npm run build` 通过。
- API 冒烟验证低权限 token 请求 dashboard 的裁剪结果。
