# mobile-client

移动端子项目，基于 Vue 3 + TypeScript + Vite，并支持 Tauri 2 原生壳。

## 作用

`mobile-client` 面向终端用户，覆盖以下页面：

- `LoginView.vue`：登录
- `HomeView.vue`：首页彩种入口
- `BetView.vue`：投注页面
- `OrdersView.vue`：订单记录
- `DepositView.vue`：充值
- `SupportView.vue`：客服
- `ProfileView.vue`：个人中心

## 本地开发

```bash
npm install
npm run dev
```

## Web 构建

```bash
npm run build
```

## iOS 无签名 IPA 打包

无签名 IPA 只用于后续企业签名、重签名或第三方签名工具处理，不能直接安装到普通 iPhone。

新电脑首次打包前需要准备：

- macOS + 完整 Xcode，并在 Xcode 中安装 iPhoneOS SDK。
- Rust 工具链和 `rustup`。
- Node.js、`pnpm`。

首次或日常打包统一执行：

```bash
pnpm install
pnpm tauri:build:ios-unsigned -- --api-base https://bcbbc.hippoweb3.net
```

脚本会自动完成这些动作：

- 缺少 iOS 工程时执行 `pnpm tauri ios init --ci`。
- 缺少 `aarch64-apple-ios` Rust target 时自动安装。
- 缺少或需要更新 `src-tauri/gen/apple/Externals/arm64/release/libapp.a` 时，通过 `cargo build --target aarch64-apple-ios --release` 生成真机静态库并复制为 `libapp.a`。
- 构建前端资源，同步后台品牌 Logo，并封装 `Payload/HongFu.app` 为无签名 IPA。

常用参数：

```bash
pnpm tauri:build:ios-unsigned -- --output src-tauri/gen/apple/build/HongFu-unsigned.ipa
pnpm tauri:build:ios-unsigned -- --random-bundle-id
pnpm tauri:build:ios-unsigned -- --force-native-build
pnpm tauri:build:ios-unsigned -- --skip-native-build
```

其中 `--skip-native-build` 只适合确认现有 `libapp.a` 已经匹配当前 Rust/Tauri 代码时使用；新电脑不要使用这个参数。

## Tauri 原生开发

```bash
npx tauri dev
```

## 目录说明

```text
src/
├── api/       # 前端接口请求与响应处理
├── router/    # 路由守卫与页面映射
├── stores/    # Pinia 状态
└── views/     # 页面级组件

src-tauri/
└── ...        # Tauri 原生壳与打包配置
```
