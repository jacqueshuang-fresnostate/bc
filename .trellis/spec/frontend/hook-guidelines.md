# Hook 规范

> 本项目 hook 使用方式。

---

## 概览

hook 负责可复用的状态逻辑。页面组件调用 hook，然后渲染返回状态。

---

## 自定义 Hook 模式

- hook 必须以 `use` 开头。
- 返回带命名字段的小对象。
- API 页面需要把 loading、error、data、refresh 放在同一 hook 中。
- 避免把导航副作用隐藏在通用 hook 中。

示例：

```tsx
const { data, loading, error, refresh } = useDashboard();
```

---

## 数据获取

首个里程碑可以通过 `api/client.ts` 中的 `fetch` 获取数据。如果服务端状态变复杂，在多个 hook 复制缓存/重试逻辑之前引入 React Query。

不要在多个组件中直接调用 `fetch`；接口行为集中放在 `api/`。

---

## 命名约定

- `useDashboard` 用于仪表盘状态。
- `useLotteries` 用于彩种列表状态。
- `useOrders` 用于订单列表状态。
- hook 文件名和 hook 名称保持一致。

---

## 常见错误

- 不要创建返回含糊 tuple 位置的 hook。
- 如果 hook 变成长生命周期或频繁刷新，不要忽略请求取消。
- 不要在多个 hook 中重复拼接 API URL。
