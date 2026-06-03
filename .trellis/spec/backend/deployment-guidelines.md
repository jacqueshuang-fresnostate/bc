# 容器部署规范

> 后端与管理后台同镜像部署时的可执行契约。

## Scenario: Docker 单镜像与 Nginx 反向代理

### 1. Scope / Trigger

- Trigger: 新增或修改 Dockerfile、Nginx 配置、容器入口脚本、运行环境变量、健康检查或镜像运行方式时必须遵守本规范。
- Scope: 单镜像内包含管理后台静态资源、Rust 后端二进制和 Nginx 网关；数据库、日志采集、对象存储和镜像仓库不打进应用镜像。

### 2. Signatures

- 构建命令：`docker build -t bc-platform:latest .`
- 本地运行命令：`docker run --rm -p 8080:80 bc-platform:latest`
- Compose 命令：`docker compose up --build`
- 健康检查：`GET http://127.0.0.1/api/health`

### 3. Contracts

- `BACKEND_PORT`：可选，容器内后端监听端口，默认 `8080`，必须是纯数字。
- `RUST_LOG`：可选，后端日志级别，默认 `info`。
- `DATABASE_URL`：可选，配置后后端使用 PostgreSQL；未配置时使用内存演示仓储。
- Nginx 对外监听 `80`，前端静态资源位于 `/usr/share/nginx/html`。
- Nginx 必须把 `/api/` 反向代理到 `127.0.0.1:${BACKEND_PORT}`。
- 非 `/api/` 路径必须使用 SPA fallback 到 `/index.html`。

### 4. Validation & Error Matrix

- `BACKEND_PORT` 为空或包含非数字字符 -> 入口脚本输出中文错误并退出。
- 后端未能启动 -> `/api/health` 失败，Docker healthcheck 变为 unhealthy。
- Nginx 未按 `BACKEND_PORT` 渲染代理端口 -> 首页可能正常但 `/api/health` 失败。
- `DATABASE_URL` 未配置 -> 后端输出中文日志，使用内存演示仓储。

### 5. Good/Base/Bad Cases

- Good: `BACKEND_PORT=18080 docker run --rm -p 8080:80 bc-platform:latest`，Nginx 代理到容器内 `18080`，`/api/health` 成功。
- Base: 不传环境变量，后端监听 `8080`，Nginx 对外服务 `80`，`/` 与 `/api/health` 均成功。
- Bad: `BACKEND_PORT=abc docker run ...`，入口脚本拒绝启动并输出 `BACKEND_PORT 必须是数字`。

### 6. Tests Required

- 每次修改 Dockerfile、Nginx 或入口脚本后运行 `docker build -t bc-platform:latest .`。
- 启动临时容器后验证 `curl -I http://127.0.0.1:<host-port>/` 返回 200。
- 验证 `curl http://127.0.0.1:<host-port>/api/health` 返回 `success=true`。
- 验证 `docker ps` 中临时容器状态为 `healthy`。
- 修改后端或前端构建链路时同步运行 `cargo fmt --check`、`cargo check`、`cargo test`、`npm run build`。

### 7. Wrong vs Correct

#### Wrong

Nginx 配置写死 `127.0.0.1:8080`，但入口脚本允许 `BACKEND_PORT` 改成其它端口。

#### Correct

Nginx 配置使用占位符，入口脚本启动前按 `BACKEND_PORT` 渲染真实端口，并先校验端口格式。
