# 后台登录鉴权与权限拦截基础

## 背景

当前后台已经有用户、管理员、角色权限和系统设置维护能力，但管理后台本身没有登录态，也没有把管理员角色权限绑定到菜单、工作台模块和 API 访问。`架构设计.md` 多处把“真实登录鉴权、菜单权限拦截和接口鉴权”列为后续任务，因此本阶段先建立基础鉴权闭环。

## 目标

- 新增后台登录、当前管理员、登出接口。
- 前端新增登录页，未登录时不进入管理后台。
- 前端 API client 自动携带 Bearer Token。
- 登录成功后保存当前管理员身份、角色和权限范围。
- 菜单、工作台模块入口按当前角色权限过滤。
- 后端对主要 `/api/admin/**` 接口做基础 Bearer Token 鉴权。
- 登录失败、账号锁定、权限不足时返回统一 API 信封错误。
- 更新 `架构设计.md`、`TODO.md` 和必要规格文档。

## 非目标

- 不接 PostgreSQL 权限表。
- 不实现密码哈希、密码重置、多因素认证、验证码和登录失败锁定策略。
- 不实现管理员操作审计。
- 不实现细粒度按钮级权限。
- 不引入 JWT 第三方库；本阶段用内存会话 token。

## 现有事实

- 后端 `AccessRepository` 已有 `AdminSummary`、`AdminRole`、`PermissionScope`、管理员状态和角色权限数据。
- 种子管理员：
  - `admin` / `A10001` / `role-super` / active。
  - `locked_admin` / `A10002` / `role-ops` / locked。
- 当前公开错误类型已有 `Unauthorized` 和 `Forbidden`。
- 当前前端模块入口全部来自 `/api/admin/dashboard` 的 `moduleGroups`。
- 当前前端 API 调用集中在 `admin/src/api/client.ts`。

## 临时约定

- 种子管理员默认密码使用 `admin123`，仅作为无数据库阶段的登录演示凭据。
- 新建管理员默认密码同样使用 `admin123`，直到后续接入密码设置、哈希和重置流程。
- 会话 token 保存在后端内存仓储，前端存入 `localStorage`；服务重启后 token 失效，需要重新登录。
- 登录接口不需要 token；其他 `/api/admin/**` 管理接口需要 token。

## 权限映射

- `users`、`registration` → `users`
- `admins` → `admins`
- `roles` → `roles`
- `settings` → `systemSettings`
- `orders`、`settlements` → `orders`
- `finance` → `finance`
- `support` → `customerService`
- `lottery-console`、`lotteries`、`draw-modes`、`schedules`、`group-buy`、`play-rules`、`draw-automation`、`draw-scheduler` → `lotteries`
- `group-buy-robot`、`purchase-robot` → `robots`
- `invite`、`rebate` → `rebates`

## 验收标准

- 未登录打开前端时显示登录页。
- 使用 `admin` / `admin123` 可登录并进入系统概览。
- 登录后侧边栏和工作台模块只显示当前角色允许的模块。
- `locked_admin` / `admin123` 登录返回账号不可用错误。
- 不带 token 请求 `/api/admin/dashboard` 返回 HTTP 401 和统一 API 信封。
- 带 `role-ops` 范围 token 访问无权限接口时返回 HTTP 403。
- `npm run build`、`cargo fmt --check`、`cargo check`、`cargo test` 通过。
- 浏览器验证登录、刷新保持登录态、登出回到登录页。
