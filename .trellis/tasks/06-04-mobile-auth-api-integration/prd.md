# 手机端登录注册接口对接 PRD

## 目标

将新加入的 `mobile` 手机端工程接入当前后端用户接口，完成登录、用户名注册、邮箱注册和登录态恢复的基础链路，让用户注册后可以进入手机端业务页面。

## 已知信息

- 后端用户公开接口位于 `/api/user/register`、`/api/user/login`、`/api/user/forgot-password`、`/api/user/reset-password`。
- 后端受保护用户接口位于 `/api/user/me`、`/api/user/logout`、`/api/user/bind-email`、`/api/user/password/change` 等路径。
- 后端统一返回 `ApiEnvelope<T>`，真实业务数据在 `data` 字段内。
- 后端登录返回 `UserAuthSession`，字段为 `token` 和 `user`，当前没有刷新令牌。
- 手机端当前仍调用旧的 `/api/auth/*`，并假设返回 `access_token`、`refresh_token`。
- 后端注册配置已有后台维护接口，但手机端缺少公开读取入口。

## 假设

- 手机端后续将作为同项目中的独立前端工程运行，开发时通过 `VITE_API_BASE` 或同源 `/api` 访问后端。
- 邮箱注册当前不需要邮箱验证码；注册开关由后台注册配置控制。
- 后端不新增刷新令牌，本次移动端以单 token 会话保存为准。
- 忘记密码、绑定邮箱、资金等后续页面可继续按后端现有用户接口逐步对接，本次优先保证登录注册入口和登录后基础用户信息可用。

## 需求

1. 手机端登录表单调用 `/api/user/login`，请求字段使用 `loginKey`、`password`。
2. 手机端用户名注册调用 `/api/user/register`，请求字段使用 `username`、`password`、`inviteCode`。
3. 手机端邮箱注册调用 `/api/user/register`，请求字段使用 `email`、`password`、`inviteCode`。
4. 手机端必须兼容后端统一响应信封，从 `data` 中读取 `token` 和 `user`。
5. 手机端登录态保存为单 token，并保存当前用户摘要；刷新页面后能恢复 token。
6. 后端新增公开注册配置读取接口，手机端根据 `emailEnabled`、`usernameEnabled`、`agentInviteRequired` 控制注册入口与邀请码必填提示。
7. 登录后业务页中读取当前用户的接口改为 `/api/user/me`，避免仍依赖旧 `/api/auth/me`。
8. 用户看到的错误提示优先使用后端统一响应的 `message`。

## 验收标准

- 未登录进入受保护页面会跳转登录页。
- 用户名注册成功后自动登录并进入首页。
- 邮箱注册开启时可以使用邮箱注册；邮箱注册关闭时不展示邮箱注册入口。
- 登录成功后手机端请求会携带 `Authorization: Bearer <token>`。
- 刷新页面后仍能从本地存储恢复 token。
- 后端公开注册配置接口返回统一响应信封和 `camelCase` 字段。
- `mobile` 构建检查通过，后端格式化和检查通过。

## 不在本次范围

- 邮箱验证码发送与校验。
- 刷新令牌机制。
- 第三方 OAuth 登录。
- 手机端全部业务页面的完整接口联调。
- 原生 Tauri 打包发布。

## 技术说明

- 移动端 API 基础路径保持 `baseURL: ${API_BASE}/api`，调用时使用 `/user/*`。
- 单 token 仍沿用 `accessToken` 字段名保存，减少路由守卫和请求拦截器改动范围。
- 后端注册配置复用现有 `RegistrationConfig` 领域模型，不新增重复 DTO。
- 后端新接口仅公开注册开关，不公开管理员或安全敏感配置。
