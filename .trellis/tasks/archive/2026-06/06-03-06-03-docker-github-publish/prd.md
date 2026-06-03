# Docker 单镜像打包与 GitHub 上传

## 背景

当前项目包含 `backend/` Rust API 和 `admin/` React 管理后台。用户要求上传到 GitHub，并打包为 Docker image，前后端在同一个项目镜像中运行，可以使用 Nginx。

## 目标

1. 新增根目录 Docker 单镜像构建方案。
2. 镜像内编译前端静态资源和后端 Rust release binary。
3. 运行时使用 Nginx 服务前端，并将 `/api` 反向代理到同容器内后端。
4. 提供本地构建和运行说明。
5. 提交 Docker 打包改动，并在拿到 GitHub 远端后推送。

## 非目标

1. 不把 PostgreSQL 打进同一个镜像。
2. 不把用户本地未提交的端口配置和 IDE 配置纳入本阶段提交。
3. 不改变现有 API 路由契约。

## 验收

1. `docker build` 可以成功生成镜像。
2. 容器启动后根路径返回前端页面，`/api/health` 返回后端健康检查。
3. `cargo fmt --check`、`cargo check`、`cargo test`、`npm run build` 保持通过。
