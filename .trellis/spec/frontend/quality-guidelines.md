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
- 手机端 Tauri iOS 模拟器构建使用 `pnpm tauri:build:ios-sim`；首次构建前需要执行 `pnpm tauri:ios:init`，并确保本机已安装 iOS Rust targets、Xcode iOS Simulator runtime 和 `xcodegen`。如果 Xcode/SwiftPM 因 `safe.bareRepository is 'explicit'` 无法读取 SwiftPM bare repo 缓存，构建前临时设置 `git config --global safe.bareRepository all`，构建后恢复原值；真机/IPA 构建还必须配置 Apple Developer Team、签名证书和 provisioning profile。
- 手机端 Tauri iOS 的 `productName`、Xcode `PRODUCT_NAME` 和可执行文件名必须使用英文内部名，中文桌面显示名通过 `CFBundleDisplayName=盛源` 配置；不要让 `.app` 目录或可执行文件名直接使用中文，避免 iOS/WKWebView 初始化阶段原生崩溃。企业签名后需要解包确认仍是 `Payload/HongFu.app/HongFu`，如果签名平台改成 `Payload/盛源.app/盛源`，该包会继续闪退。
- 手机端打包版动态品牌、Logo、介绍和轮播广告都依赖当前 `API_BASE` 指向的后端；发布包必须确认 `GET /api/user/mobile/site-config` 和 `GET /api/user/mobile/advertisements` 命中同一个后台配置库。品牌配置在启动和首页进入时要支持强制刷新，避免后台更新后继续显示默认 Logo 或旧标题。
- 手机端无签名 IPA 打包需要同步后台 `site-config` 的 Logo 到 `mobile/dist/app-logo.png`、`mobile/dist/logo.svg`、`mobile/dist/mobile-branding.json` 和临时 iOS `AppIcon.appiconset`；首屏先读取包内 `mobile-branding.json`，再静默刷新后台配置。平台 Logo、首页 Banner、彩种 Logo 等图床图片必须优先使用通用远程图片缓存组件，避免 Tauri App 内反复请求同一张网络图片。

## 场景：手机端样式分片兼容

### 1. 范围 / 触发

- 触发：修改 `mobile/vite.config.ts`、移动端路由懒加载、页面级 `<style scoped>` 或 Tauri/WebView 打包样式加载策略时，必须按本场景验证。
- 目标：避免部分安卓浏览器、低版本 WebView 或企业签名 App 出现页面内容已渲染但登录页、首页、底部导航等样式退回默认 HTML 的问题。

### 2. 签名

- Vite 构建配置：`mobile/vite.config.ts` 必须设置 `base = './'` 和 `build.cssCodeSplit = false`，并通过构建插件把主 CSS 内联到 `dist/index.html`。
- 验证命令：`cd mobile && pnpm build`。

### 3. 契约

- 移动端打包产物必须把页面级 CSS 合并进首屏 CSS 包，不依赖 `LoginView-*.css`、`HomeView-*.css`、`LayoutView-*.css` 等异步 CSS chunk。
- Tauri Android 打包产物的 `dist/index.html` 必须使用相对路径加载 JS 和图标，例如 `./assets/index-*.js`、`./logo.svg`；不能生成 `/assets/...` 这类站点根路径资源。
- Tauri Android 打包产物的主 CSS 必须内联到 `dist/index.html` 的 `<style data-tauri-inline-css>`，不能只保留外链 `<link rel="stylesheet" href="./assets/style-*.css">`。
- 路由预加载工具可以继续预热 JS 页面 chunk，但不能作为页面样式加载成功的唯一保障。
- 新增页面可以继续写 `<style scoped>`，但构建后这些样式必须进入统一 CSS 产物。

### 4. 校验与错误矩阵

- `cssCodeSplit=true` -> 部分手机可能只加载页面 JS，没有加载异步 CSS chunk，表现为输入框和按钮使用浏览器默认边框。
- 构建后仍生成 `/assets/...` 或外链主 CSS -> 部分 Tauri Android WebView 可能已执行 JS 但跳过 CSS 资源，登录页和首页会退回接近默认 HTML 样式。
- 首页或登录页出现局部裸样式 -> 优先检查构建产物是否重新出现页面级 CSS chunk。
- 单个页面 `<style scoped>` 依赖异步加载 -> 在 WebView 弱网或缓存异常时可能丢失关键布局。

### 5. 好 / 基准 / 坏案例

- 好：`dist/index.html` 使用相对脚本路径并包含 `<style data-tauri-inline-css>`，`dist/assets` 中没有 `.css` 文件，也没有 `LoginView-*.css`、`HomeView-*.css` 这类页面 CSS 分片。
- 基准：登录页在打包 App、手机浏览器和普通 Vite preview 中都保持圆角卡片、填充按钮、紧凑输入框样式。
- 坏：只依赖 `preloadMobileRoutes()` 提前动态导入页面，让异步 CSS chunk 在空闲时间“尽量加载”。

### 6. 必需测试

- `cd mobile && pnpm build`。
- 构建后检查 `mobile/dist/index.html` 包含 `data-tauri-inline-css`，不包含 `rel="stylesheet"`，且 `mobile/dist/assets` 没有 `.css` 文件。
- 有真机条件时，优先验证低版本安卓 WebView、企业签名 iOS 包或用户反馈机型的登录页和首页。

### 7. 错误写法与正确写法

#### 错误

```ts
export default defineConfig({
  plugins: [vue(), tailwindcss()],
})
```

#### 正确

```ts
export default defineConfig({
  base: './',
  plugins: [vue(), tailwindcss(), inlineBuiltCssForTauri()],
  build: {
    cssCodeSplit: false,
  },
})
```

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
