# 质量规范

> 前端开发的代码质量标准。

---

## 概览

前端代码需要类型清晰、易扫描，并且在 loading/error 状态下可靠。管理后台用户需要密集的运营屏幕，不需要装饰型页面。

---

## 禁用模式

- 在组件中分散直接调用 `fetch`。
- 加载或接口失败时显示空屏。
- UI 文本溢出按钮、标签、卡片或导航项。
- 使用负 letter spacing 或随 viewport 缩放字体。
- 管理后台中使用装饰性卡片嵌套或营销型 hero 布局。
- 在应用类型中使用 `any`。

---

## 必须模式

- 前端变更需要运行 TypeScript 构建检查。
- 标准后台控件优先使用 Semi UI。
- 应用外壳必须能访问所有页面入口。
- API base URL 通过 Vite 环境变量配置。
- API 页面必须提供可见 loading 和 error 状态。
- 涉及下注、充值、提现、认购等资金或订单写入的提交动作，必须在请求期间显示明确 loading，并禁用会造成重复提交的主要入口。
- 手机端 Tauri APK 相关变更需要同时跑 Web 构建、`src-tauri` Rust 检查和 `tauri android build --apk --ci`，并确认 APK 包含当前工程路径生成的多 ABI 原生库。
- 手机端脚本约定：`pnpm tauri:build:app` 必须产出已签名的 Android release APK，以保持正常发布体积并支持本地安装验证；release 构建不启用 R8/ProGuard 混淆裁剪，避免 Tauri Android 桥接或插件代码被误裁导致启动闪退；需要调试大包时使用 `pnpm tauri:build:apk:debug`；macOS `.app` 必须使用 `pnpm tauri:build:desktop-app`。

## 场景：手机端 Tauri APK 启动配置

### 1. 范围 / 触发

- 触发：修改 `mobile/src-tauri/tauri.conf.json`、`mobile/src-tauri/src/lib.rs`、Tauri 插件、Android 构建脚本或 capability 权限时，必须按本场景验证。
- 目标：避免 release APK 安装后在 Tauri 初始化阶段直接闪退，且 logcat 只能看到原生 SIGABRT。

### 2. 签名

- 插件注册入口：`mobile/src-tauri/src/lib.rs` 中的 `.plugin(...)` 链。
- Android 配置入口：`mobile/src-tauri/tauri.conf.json`。
- 权限入口：`mobile/src-tauri/capabilities/default.json`。
- 构建命令：`cd mobile && pnpm tauri:build:app`。

### 3. 契约

- `tauri-plugin-store` 和 `tauri-plugin-clipboard-manager` 通过 Rust `.plugin(...)` 注册；当前项目不要在 `tauri.conf.json` 的 `plugins` 节点里写 `"store": {}` 或 `"clipboard-manager": {}` 空对象。
- capability 权限仍保留在 `default.json`，例如 `store:default`、`clipboard-manager:allow-read-text` 和 `clipboard-manager:allow-write-text`。
- Tauri 启动错误需要先写入 Android logcat，日志标签使用 `HongFuMobile`，错误文案使用中文。
- 手机端 HTTP/WS 地址必须区分普通浏览器源和 Tauri 本地源：`http://tauri.localhost`、`tauri:`、`asset:` 不能走相对 `/api`，必须回落到打包域名或构建时 `VITE_API_BASE_URL`。

### 4. 校验与错误矩阵

- `plugins.store` 写成 `{}` -> Android 启动时报 `PluginInitialization("store", "invalid type: map, expected unit")`，应用闪退。
- Rust 注册了 store 插件但 capability 缺少 `store:default` -> 前端调用 Tauri store 时权限失败。
- release APK 未签名 -> 真机无法直接安装验证。
- release 构建启用混淆裁剪且没有 keep 规则 -> 可能裁剪 Tauri Android 桥接或插件代码，导致启动闪退。

### 5. 好 / 基准 / 坏案例

- 好：插件只在 Rust 中注册，`tauri.conf.json` 不写空插件配置，capability 单独声明权限。
- 基准：`pnpm tauri:build:app` 生成 `app-universal-release.apk`，真机启动后 8 秒仍有 `com.hongfu.app` 进程。
- 坏：为了“显式启用插件”在 `tauri.conf.json` 写 `"store": {}`，这会让 Android 端把 map 当成 unit 配置解析并闪退。
- 坏：用 `window.location.protocol.startsWith("http")` 判断是否走相对 `/api`，Tauri Android 的 `http://tauri.localhost` 会被误判为普通浏览器同源。

### 6. 必需测试

- `cd mobile/src-tauri && cargo check`。
- `cd mobile && pnpm tauri:build:app`。
- `apksigner verify --verbose mobile/src-tauri/gen/android/app/build/outputs/apk/universal/release/app-universal-release.apk`。
- 真机或可用模拟器安装启动后，检查 `pidof com.hongfu.app` 有进程，并且 logcat 没有 `HongFuMobile` 启动失败、`PluginInitialization`、`SIGABRT` 或 panic。
- 真机启动后，登录页 logo 和介绍文案需要能读取后端 `GET /api/user/mobile/site-config`；如果仍显示默认文案，优先检查 API base 是否误打到 `tauri.localhost/api`。

### 7. 错误写法与正确写法

#### 错误

```json
{
  "plugins": {
    "store": {},
    "clipboard-manager": {}
  }
}
```

#### 正确

```rust
tauri::Builder::default()
    .plugin(tauri_plugin_store::Builder::default().build())
    .plugin(tauri_plugin_clipboard_manager::init())
```

权限通过 `mobile/src-tauri/capabilities/default.json` 声明，不通过 `tauri.conf.json` 空对象配置声明。

---

## 测试要求

- 前端变更需要运行 `npm run build`。
- 表单、权限、金额展示、彩票计算变成可交互逻辑时，需要增加组件或 hook 测试。
- 显著布局变更且有开发服务器时，需要做浏览器验证。

---

## 代码审查清单

- 页面是否符合后台工作流，且没有过大的装饰元素？
- API 调用是否类型化并集中管理？
- loading 和 error 状态是否可见？
- 文本在移动端和桌面端是否都能放得下？
- 重复 UI 模式是否抽成了可复用组件？
