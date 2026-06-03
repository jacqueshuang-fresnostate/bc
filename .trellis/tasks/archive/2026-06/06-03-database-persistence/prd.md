# 数据库持久化接入

## 背景

当前 Docker 单镜像已经可以运行前端和后端，但默认没有数据库。后端在未配置 `DATABASE_URL` 时使用内存演示数据；目前代码中只有彩种管理已经具备 PostgreSQL + SQLx migrations 仓储基础，其它用户、订单、开奖、资金、权限等模块仍是内存仓储。

## 目标

1. 让 `docker compose up --build` 默认启动 PostgreSQL，并把 `DATABASE_URL` 注入应用容器。
2. 保持单镜像 Dockerfile 不内置数据库，生产数据库仍独立部署。
3. 让已支持数据库的彩种管理在 Compose 部署下使用 PostgreSQL 持久化。
4. 更新中文部署说明、架构设计、TODO 和数据库/部署规范，明确当前数据库覆盖范围。
5. 本阶段不误导为“所有业务数据已持久化”，清楚列出后续仍需迁移的内存模块。

## 非目标

1. 不在本阶段一次性重写用户、订单、开奖、资金、权限、客服、机器人等所有内存仓储。
2. 不把 PostgreSQL 打进应用镜像。
3. 不改变现有 API 响应契约。

## 验收

1. `docker-compose.yml` 包含 PostgreSQL 服务、健康检查、持久化 volume 和应用 `DATABASE_URL`。
2. `docker compose up --build` 后 `/api/health` 正常返回。
3. 有数据库时后端日志显示使用 PostgreSQL 彩种仓储。
4. `backend/migrations` 能在 PostgreSQL 上自动运行并创建 `lotteries` 表。
5. 文档说明 `docker run` 需要外部数据库，`docker compose` 会启动本地数据库。
