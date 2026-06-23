# 容器部署规范

> 后端与管理后台同镜像部署时的可执行契约。

## Scenario: Docker 单镜像与 Nginx 反向代理

### 1. Scope / Trigger

- Trigger: 新增或修改 Dockerfile、Nginx 配置、容器入口脚本、运行环境变量、健康检查或镜像运行方式时必须遵守本规范。
- Scope: 单镜像内包含管理后台静态资源、Rust 后端二进制和 Nginx 网关；数据库、日志采集、对象存储和镜像仓库不打进应用镜像。

### 2. Signatures

- 构建命令：`docker build -t bc-platform:local .`
- 本地运行命令：`docker run --rm -p 8080:80 bc-platform:local`
- Compose 命令：`docker compose up --build`
- GHCR Compose 命令：`BC_IMAGE_TAG=sha-<提交短哈希> docker compose -f docker-compose.ghcr.yml up -d`
- 健康检查：`GET http://127.0.0.1/api/health`

### 3. Contracts

- `BACKEND_PORT`：可选，容器内后端监听端口，默认 `8080`，必须是纯数字。
- `BACKEND_STARTUP_TIMEOUT_SECONDS`：可选，入口脚本等待后端健康检查通过的最长秒数，默认 `60`，必须是纯数字。
- `BACKEND_STARTUP_LOG_INTERVAL_SECONDS`：可选，入口脚本等待后端健康检查期间输出进度日志的间隔秒数，默认 `2`，必须是大于 `0` 的纯数字。
- `APP_PORT`：可选，Compose 模式下宿主机暴露端口，默认 `8080`。
- `BC_IMAGE_TAG`：GHCR Compose 模式必填，必须是 `sha-<提交短哈希>` 或 `v*` 版本标签，不允许使用 `latest`。
- `RUST_LOG`：可选，后端日志级别，默认 `info`。
- `DATABASE_URL`：可选，配置后后端使用 PostgreSQL；未配置时使用内存演示仓储。非空时必须以 `postgres://` 或 `postgresql://` 开头。
- `docker-compose.yml` 必须启动独立 PostgreSQL 服务并把应用 `DATABASE_URL` 指向 Compose 网络内的数据库。
- `docker-compose.ghcr.yml` 必须只拉取 `ghcr.io/sydneypoole/bc:${BC_IMAGE_TAG}`，不能配置 `build`，避免生产服务器重新构建镜像或误用漂移标签。
- Nginx 对外监听 `80`，前端静态资源位于 `/usr/share/nginx/html`。
- 容器运行日志以 Rust 后端 stdout/stderr 为主；Nginx `access_log` 必须关闭，Nginx `error_log` 不得输出到 Docker stdout/stderr。
- Nginx 必须把 `/api/` 反向代理到 `127.0.0.1:${BACKEND_PORT}`。
- Nginx 代理 `/api/` 时必须保留 WebSocket Upgrade 能力，至少包含 `proxy_http_version 1.1`、`Upgrade`、`Connection` 请求头转发，以及覆盖实时心跳间隔的 `proxy_read_timeout`；否则 `GET /api/user/realtime` 会连接失败或无法持续接收开奖推送。
- Nginx 可以开启 gzip 压缩静态文本资源；哈希静态资源目录 `/assets/` 必须使用长期 immutable 缓存，`/index.html` 与 SPA fallback 必须使用 `no-store`，避免手机端加载旧入口文件和新资源文件不匹配。
- 入口脚本必须先启动后端并等待 `http://127.0.0.1:${BACKEND_PORT}/api/health` 通过，再启动 Nginx；后端启动失败时容器必须失败退出，不能只留下 Nginx 返回 502。
- 入口脚本等待健康检查期间必须持续输出中文启动进度，至少包含已等待秒数、剩余秒数、curl 退出码、HTTP 状态和后端进程状态；不得只输出一次“等待后端健康检查通过”后静默等待。
- 入口脚本启动 Nginx 后必须持续监控后端进程；后端退出时需要关闭 Nginx 并让容器退出。
- 非 `/api/` 路径必须使用 SPA fallback 到 `/index.html`。

### 4. Validation & Error Matrix

- `BACKEND_PORT` 为空或包含非数字字符 -> 入口脚本输出中文错误并退出。
- `BACKEND_STARTUP_TIMEOUT_SECONDS` 为空或包含非数字字符 -> 入口脚本输出中文错误并退出。
- `BACKEND_STARTUP_LOG_INTERVAL_SECONDS` 为空、包含非数字字符或等于 `0` -> 入口脚本输出中文错误并退出。
- GHCR Compose 未设置 `BC_IMAGE_TAG` -> Compose 配置阶段直接失败并提示需要指定镜像标签。
- GHCR Compose 使用 `BC_IMAGE_TAG=latest` -> 不符合不可变版本部署要求，应改用 `sha-<提交短哈希>` 或 `v*`。
- 后端未能启动 -> `/api/health` 失败，Docker healthcheck 变为 unhealthy。
- 后端启动失败或运行后退出 -> 容器失败退出，不能继续由 Nginx 对外返回 502。
- Nginx 未按 `BACKEND_PORT` 渲染代理端口 -> 首页可能正常但 `/api/health` 失败。
- Nginx 仍把 access/error log 输出到 stdout/stderr -> `docker logs` 会被网关请求日志刷屏，不符合只观察后端日志的部署要求。
- Nginx 未转发 WebSocket Upgrade 头 -> `/api/user/realtime` 不能升级为 WebSocket，手机端收不到 `lottery.draw_result` 开奖推送。
- `DATABASE_URL` 未配置 -> 后端输出中文日志，使用内存演示仓储。
- `DATABASE_URL` 写成 `用户名:密码@主机:端口/数据库` 或其它缺少协议前缀的格式 -> 后端输出中文配置错误并退出。
- Compose 中 PostgreSQL 未健康 -> 应用容器不得抢先启动，避免连接失败后误判部署成功。

### 5. Good/Base/Bad Cases

- Good: `BACKEND_PORT=18080 docker run --rm -p 8080:80 bc-platform:local`，Nginx 代理到容器内 `18080`，`/api/health` 成功。
- Base: 不传环境变量，后端监听 `8080`，Nginx 对外服务 `80`，`/` 与 `/api/health` 均成功。
- Bad: `BACKEND_PORT=abc docker run ...`，入口脚本拒绝启动并输出 `BACKEND_PORT 必须是数字`。
- Good: 后端连接数据库较慢时，容器日志每 2 秒输出一次健康检查进度和后端进程状态，便于判断是数据库迁移慢、端口未监听还是进程已退出。
- Compose Good: `docker compose up --build` 同时启动 PostgreSQL 和应用，应用日志显示已配置 `DATABASE_URL`。
- Compose Good: `APP_PORT=18081 docker compose up --build` 可在宿主机端口冲突时切换对外端口，容器内仍由 Nginx 监听 `80`。
- Good: 通过容器访问 `ws://127.0.0.1:<host-port>/api/user/realtime` 可以成功升级连接，并持续接收后端公开开奖事件。

### 6. Tests Required

- 每次修改 Dockerfile、Nginx 或入口脚本后运行 `docker build -t bc-platform:local .`。
- 每次修改 Compose 数据库配置后运行 `docker compose up --build`，确认 PostgreSQL healthcheck 和应用健康检查都通过。
- 启动临时容器后验证 `curl -I http://127.0.0.1:<host-port>/` 返回 200。
- 验证 `curl http://127.0.0.1:<host-port>/api/health` 返回 `success=true`。
- 请求首页和 `/api/health` 后检查 `docker logs`，不应出现 Nginx access log 或 Nginx error log 输出。
- 验证 `GET /api/user/realtime` 能通过 Nginx 完成 WebSocket 升级，至少确认连接建立、心跳不断开，并在手动开奖或调度开奖后收到 `lottery.draw_result`。
- 验证 `docker ps` 中临时容器状态为 `healthy`。
- 修改后端或前端构建链路时同步运行 `cargo fmt --check`、`cargo check`、`cargo test`、`npm run build`。

### 7. Wrong vs Correct

#### Wrong

Nginx 配置写死 `127.0.0.1:8080`，但入口脚本允许 `BACKEND_PORT` 改成其它端口。

#### Correct

Nginx 配置使用占位符，入口脚本启动前按 `BACKEND_PORT` 渲染真实端口，并先校验端口格式。

#### Wrong

Nginx 使用官方镜像默认日志配置，把访问日志写入 `/dev/stdout`，错误日志写入 `/dev/stderr`。

#### Correct

Nginx 明确配置 `access_log off;`，并把 `error_log` 指向不会进入 Docker 日志的目标，让容器日志只保留后端服务输出和必要入口脚本提示。

#### Wrong

只代理普通 HTTP 请求，没有配置 `Upgrade` 和 `Connection` 请求头，导致手机端首页可以打开但 WebSocket 开奖推送收不到。

#### Correct

`/api/` 代理同时支持普通 HTTP 和 WebSocket 升级，并把读取超时时间设置为大于后端实时心跳间隔。

## Scenario: 本地服务联调环境变量文件

### 1. Scope / Trigger

- Trigger: 新增或修改本地启动配置、环境变量、`.env.example`、`.gitignore`、后端 env 加载逻辑或前端 Vite env 配置时必须遵守本规范。
- Scope: 本地功能验证直接启动 Rust 后端和 Vite 前端，不通过 Docker 打包镜像；Docker 仅用于明确的镜像构建、发布或部署验证。

### 2. Signatures

- 后端示例文件：`.env.example`
- 后端本地文件：`.env.local`
- 前端示例文件：`admin/.env.example`
- 前端本地文件：`admin/.env.local`
- 后端启动目录：`backend/`
- 前端启动目录：`admin/`

### 3. Contracts

- `.env.local`、`backend/.env.local` 和 `admin/.env.local` 必须被 `.gitignore` 忽略，不能提交真实数据库密码。
- 后端启动时需要加载项目根目录和 `backend/` 下的 `.env`、`.env.local`，并且 shell 中已存在的环境变量优先级高于 env 文件。
- 后端本地测试默认使用外部 PostgreSQL：`postgres://root:<密码>@192.168.2.3:15432/postgres`。
- 前端本地测试通过 `admin/.env.local` 的 `VITE_API_BASE_URL` 指向后端，例如 `http://127.0.0.1:18120`。
- 本地验证服务时使用 `cargo run` 和 `npm run dev`；不要为了普通功能测试执行 Docker 镜像构建。

### 4. Validation & Error Matrix

- `.env.local` 缺失 -> 后端继续使用 shell 环境变量或内存模式。
- `DATABASE_URL` 配置错误 -> 后端启动失败，不降级为内存模式。
- `admin/.env.local` 指向错误端口 -> 前端页面能打开但 API 请求失败。
- env 文件语法错误 -> 后端启动失败，方便及时发现配置问题。

### 5. Good/Base/Bad Cases

- Good: 复制 `.env.example` 为 `.env.local`，配置 PostgreSQL 后在 `backend/` 执行 `cargo run`，日志显示已使用 PostgreSQL。
- Base: 不创建 `.env.local`，只用命令行临时传入 `PORT` 或 `DATABASE_URL`。
- Bad: 把真实 `DATABASE_URL` 密码写进 `.env.example` 或其它可提交文档。

### 6. Tests Required

- 修改后端 env 加载逻辑后运行 `cargo fmt --check`、`cargo check` 和 `cargo test`。
- 修改前端 env 示例后至少运行 `npm run build` 或本地启动前端验证 `VITE_API_BASE_URL` 生效。
- 使用用户指定外部 PostgreSQL 验证时，直接本地启动后端服务并访问 `/api/health`，不要打包 Docker 镜像。

### 7. Wrong vs Correct

#### Wrong

```bash
DATABASE_URL='postgres://root:真实密码@192.168.2.3:15432/postgres' cargo run
```

每次手输完整连接串容易泄露，也不方便前后端联调复用。

#### Correct

```bash
cd backend
cargo run
```

真实连接串保存在被忽略的 `.env.local` 中，命令行只负责启动服务。

## Scenario: GitHub Actions 与 GHCR 镜像发布

### 1. Scope / Trigger

- Trigger: 新增或修改 `.github/workflows/*.yml`、镜像标签规则、CI 检查命令、GHCR 登录或发布权限时必须遵守本规范。
- Scope: GitHub Actions 负责自动质量检查、Docker 单镜像构建和主分支镜像发布；不负责生产服务器部署。

### 2. Signatures

- Workflow 文件：`.github/workflows/ci.yml`
- 触发方式：`push`、`pull_request`、`workflow_dispatch`
- 发布仓库：`ghcr.io/${{ github.repository }}`
- 主分支镜像标签：`sha-<提交短哈希>`
- 版本镜像标签：`v*` Git tag，例如 `v2026.06.23-1`

### 3. Contracts

- Workflow 顶层权限必须包含 `contents: read` 和 `packages: write`。
- Workflow 应使用当前支持 Node.js 24 action runtime 的 action 版本，或显式设置 `FORCE_JAVASCRIPT_ACTIONS_TO_NODE24=true`。
- `quality` job 必须运行 `cargo fmt --check`、`cargo check` 和 `npm run build`；GitHub 打包流程按当前发布效率要求跳过 `cargo test`。
- `docker` job 必须依赖 `quality` job，避免格式、类型检查或前端构建失败仍发布镜像。
- PR 触发时只能构建镜像，不能登录 GHCR 或推送镜像。
- `main` 分支 push 时使用 `secrets.GITHUB_TOKEN` 登录 GHCR，并只推送 `sha-<提交短哈希>` 镜像标签。
- `v*` Git tag push 时使用 `secrets.GITHUB_TOKEN` 登录 GHCR，并推送同名版本镜像标签和 `sha-<提交短哈希>` 标签。
- CI 和部署文档不得发布或引用 `latest` 镜像标签；生产部署必须使用 `sha-<提交短哈希>` 或 `v*` 版本标签。

### 4. Validation & Error Matrix

- `packages: write` 缺失 -> GHCR 推送失败。
- PR 触发时执行登录或推送 -> fork/权限场景容易失败，也可能把未合并代码发布到镜像仓库。
- `docker` job 不依赖 `quality` -> 可能把格式、类型检查或前端构建失败的代码发布为镜像。
- 标签使用 `latest` -> 无法确认当前容器对应的代码提交，容易被缓存或后续推送覆盖。
- 使用已提示 Node.js 20 deprecation 的旧 action 版本 -> 2026-06-16 后可能被 GitHub 强制切换运行时并产生兼容风险。

### 5. Good/Base/Bad Cases

- Good: `main` push 先通过格式检查、类型检查和前端构建，再推送 `sha-xxxxxxx`，GitHub 打包阶段不运行 `cargo test`。
- Good: `git tag v2026.06.23-1 && git push origin v2026.06.23-1` 触发版本镜像发布，GHCR 生成 `v2026.06.23-1` 和 `sha-xxxxxxx`。
- Base: PR 触发构建检查和 Docker 构建，但 `push=false`，不发布镜像。
- Bad: 所有分支 push 都发布 `latest`，导致测试分支覆盖生产候选镜像。

### 6. Tests Required

- 本地修改后端业务逻辑时仍建议运行 `cargo fmt --check`、`cargo check`、`cargo test` 和 `npm run build`；仅修改 GitHub 打包流程时至少检查 YAML、运行差异检查，并确认 workflow 中没有恢复 `cargo test`。
- 本地运行 `docker build -t bc-platform:local .` 确认 Dockerfile 仍能构建。
- 修改 workflow 后检查 YAML 缩进、触发条件、权限、推送条件和镜像标签。
- 推送后在 GitHub Actions 页面确认 workflow 运行通过，并在 GHCR 包页面确认镜像标签存在。

### 7. Wrong vs Correct

#### Wrong

PR 也执行 `docker/login-action` 并把镜像推送到 `latest`。

#### Correct

PR 只构建镜像；只有 `main` 分支 push 或 `v*` Git tag push 时才登录 GHCR 并推送，且镜像标签只允许 `sha-<提交短哈希>` 或 `v*`。
