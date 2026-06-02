# 目录结构

> 后端代码在本项目中的组织方式。

---

## 概览

后端位于 `backend/`。服务围绕 API 边界和领域概念组织。HTTP 路由组合服务和 DTO；领域模块拥有业务名称和枚举；共享响应、错误和日志辅助代码放在 crate 根附近。

---

## 目录布局

```text
backend/
├── Cargo.toml
└── src/
    ├── main.rs
    ├── app.rs
    ├── error.rs
    ├── response.rs
    ├── domain/
    │   ├── mod.rs
    │   ├── finance.rs
    │   ├── lottery.rs
    │   ├── order.rs
    │   ├── permission.rs
    │   ├── rebate.rs
    │   ├── robot.rs
    │   └── user.rs
    ├── routes/
    │   ├── mod.rs
    │   ├── admin.rs
    │   └── health.rs
    └── services/
        ├── mod.rs
        └── dashboard.rs
```

---

## 模块组织

- `main.rs` 负责启动日志、读取运行配置、构建路由并启动服务。
- `app.rs` 负责构建 Axum 路由和挂载共享状态。
- `routes/` 只放 HTTP 处理函数。处理函数负责验证输入、调用服务并返回响应信封。
- `services/` 放应用用例和演示数据，直到数据库持久化接入。
- `domain/` 放服务和路由共享的领域结构体与枚举。
- `error.rs` 定义 API 错误以及 HTTP 响应转换。
- `response.rs` 定义通用响应信封。

---

## 命名约定

- Rust 模块和文件使用 `snake_case`。
- 结构体和枚举使用 `PascalCase`。
- 路由处理函数使用动作含义清晰的名称，例如 `health`、`dashboard_summary`、`list_lotteries`。
- 暴露给 JSON 的 DTO 字段在前端受益时通过 Serde 使用 `camelCase`。

---

## 示例

聚焦路由模块：

```rust
pub fn router() -> Router {
    Router::new().route("/health", get(health))
}
```

领域类型放在路由模块之外：

```rust
#[derive(Debug, Clone, Serialize)]
pub struct LotteryKind {
    pub id: String,
    pub name: String,
}
```
