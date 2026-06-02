# 错误处理

> 本项目后端错误处理方式。

---

## 概览

路由可见的失败统一使用单一 API 错误类型。内部可以使用更具体的错误，但公开处理函数需要转换成一致的 JSON 响应信封。

---

## 错误类型

在 `backend/src/error.rs` 定义 crate 级 `ApiError` 枚举。

预期变体：

- `BadRequest`
- `Unauthorized`
- `Forbidden`
- `NotFound`
- `Conflict`
- `Internal`

每个变体携带一段简短消息，用于日志和 API 响应。

---

## 处理模式

- 服务和路由代码优先使用 `Result<T, ApiError>`。
- 底层错误转换后使用 `?` 传播。
- 面向用户的消息要清晰，但不要暴露内部细节。
- `ApiError` 到 Axum 响应的转换集中放在一个地方。

示例：

```rust
pub type ApiResult<T> = Result<T, ApiError>;
```

---

## API 错误响应

错误响应使用和成功响应一致的顶层结构：

```json
{
  "success": false,
  "data": null,
  "message": "Lottery not found"
}
```

HTTP 状态码表达严重程度，响应信封让前端处理保持简单。

---

## 常见错误

- 请求处理路径中不要调用 `unwrap()` 或 `expect()`。
- 不要让不同接口返回不一致的 JSON 错误结构。
- 不要把密钥、SQL、令牌或堆栈信息泄露到 API 响应中。
