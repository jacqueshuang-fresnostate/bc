# SQLx PostgreSQL 迁移调研

## 结论

- 当前后端已经使用 Tokio，因此 SQLx 应启用 `runtime-tokio`。
- 项目数据库规范要求优先使用 PostgreSQL + SQLx migrations，本任务按该方向实现。
- SQLx 官方文档显示当前 docs.rs 最新 crate 页面为 `sqlx 0.9.0`，并提供 PostgreSQL driver、连接池和 `migrate` 支持。
- 本机当前 Rust 版本为 `1.92.0`，Cargo 解析时确认 SQLx `0.9.0` 要求 Rust `1.94.0`，因此本任务实际使用兼容当前工具链的 SQLx `0.8` 系列。
- 为了避免本地没有数据库时阻塞开发和测试，本任务采用可选 `DATABASE_URL`：配置后使用 PostgreSQL 仓储并运行迁移；未配置时继续使用内存仓储。

## 依赖建议

```toml
sqlx = { version = "0.8", default-features = false, features = [
  "runtime-tokio",
  "tls-rustls",
  "postgres",
  "json",
  "migrate",
  "macros"
] }
```

## 迁移策略

- 迁移文件放在 `backend/migrations/`。
- 使用 SQLx migration 支持在服务启动时自动运行迁移。
- 本阶段只新增 `lotteries` 表，保留彩种配置字段的 JSON 结构，避免在玩法和开奖时间模型还会演进时过早拆分多表。

## 参考

- SQLx docs.rs：`https://docs.rs/sqlx/latest/sqlx/`
- SQLx 文档要点：支持 Tokio runtime；提供 PostgreSQL driver；`migrate` macro 可以把迁移嵌入二进制。
- 工具链约束：当前仓库用 Rust `1.92.0` 检查，SQLx `0.9.0` 暂不兼容，因此选择 SQLx `0.8`。
