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
