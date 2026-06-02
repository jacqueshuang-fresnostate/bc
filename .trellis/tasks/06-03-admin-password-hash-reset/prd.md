# 管理员密码哈希与重置基础

## 背景

当前后台已经有登录、Bearer Token 会话和权限拦截，但 `AccessStore` 仍使用全局演示密码 `admin123` 校验所有管理员。新建管理员不会保存单独密码，账号维护页面也不能设置或重置密码。这会让后台鉴权停留在演示阶段，并阻塞后续密码重置、登录失败锁定和持久化凭据。

## 目标

- 后端为每个管理员保存独立密码哈希，不再使用全局明文演示密码。
- 登录时使用管理员自己的密码哈希校验。
- 种子管理员继续可用 `admin/admin123` 登录，便于本地开发和既有冒烟。
- 新建管理员时必须设置初始密码，避免继续产生共享默认凭据。
- 更新管理员资料时可选择不改密码；传入新密码时重置该账号密码。
- 新增独立重置密码接口，供账号维护抽屉明确执行密码重置。
- 前端“账号维护” SideSheet 支持新增账号设置初始密码、编辑账号重置密码。
- 不在任何列表、详情、dashboard 或当前管理员接口返回密码哈希。
- 更新 `架构设计.md`、`TODO.md` 和必要 API 契约规范。

## MVP 范围

### 后端

- 新增管理员保存 DTO，包含 `id`、`username`、`roleId`、`roleName`、`status` 和可选 `password`。
- 新增重置密码请求 DTO，包含 `password`。
- `POST /api/admin/admins` 接收管理员保存 DTO，创建账号时必须传入初始密码并写入密码哈希。
- `PUT /api/admin/admins/{id}` 接收管理员保存 DTO，未传密码时保留原密码哈希，传入密码时更新哈希。
- 新增 `PATCH /api/admin/admins/{id}/password` 重置密码接口。
- `GET /api/admin/admins`、`GET /api/admin/admins/{id}`、dashboard、auth session 和 auth/me 仍只返回 `AdminSummary`，不返回密码哈希。
- 密码最小长度为 8；空白密码或过短密码返回 HTTP 400。
- 哈希使用 Argon2id PHC 字符串格式和随机盐。
- 种子管理员：
  - `admin` 密码为 `admin123`。
  - `locked_admin` 也保留 `admin123`，但锁定状态仍拒绝登录。

### 前端

- API client 新增 `AdminSaveRequest` 与 `resetAdminPassword`。
- `useAccessManagement.saveAdmin` 使用管理员保存 DTO。
- “账号维护” SideSheet：
  - 新建账号时显示“初始密码”输入框。
  - 编辑账号时显示“重置密码”输入框，可留空表示不修改。
  - 新建账号提交时必须输入至少 8 位密码；编辑账号时只在有输入时传 `password`。
- 继续通过 SideSheet 打开账号维护，不回退为页面常驻表单。

## 非目标

- 不接 PostgreSQL 持久化。
- 不实现登录失败次数、锁定策略、验证码、多因素认证。
- 不实现“修改自己密码”页面。
- 不做密码复杂度强制规则，除最小长度外先不限制字符类型。
- 不撤销被重置密码账号的既有 token；后续结合会话持久化统一处理。

## 验收标准

- `admin/admin123` 仍能登录。
- 错误密码登录返回 401。
- 锁定管理员即使密码正确仍返回 403。
- 创建新管理员并设置密码后，新账号可用该密码登录。
- 重置管理员密码后，旧密码登录失败，新密码登录成功。
- 管理员列表、详情、dashboard 和 auth/me 不包含密码哈希或明文密码字段。
- 后端测试覆盖密码哈希登录、错误密码、锁定账号、新建账号密码和重置密码。
- `cargo fmt --check`、`cargo check`、`cargo test`、`npm run build` 通过。
- API 冒烟验证创建账号、重置密码和登录结果。
