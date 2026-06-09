# 状态管理

> 本项目状态管理方式。

---

## 概览

使用能满足需求的最小状态机制。首个管理后台里程碑使用 React 本地状态和类型化数据获取 hook。只有多个远距离页面需要共享可变客户端状态时，才引入全局 store。

---

## 状态分类

- 本地 UI 状态：组件内 `useState`。
- 服务端状态：通过 hook 从后端 API 返回的数据。
- URL/导航状态：当前页面或路由选择。
- 全局状态：实现认证后用于当前管理员身份和权限。

---

## 何时使用全局状态

只在以下场景使用全局状态：

- 当前管理员账号。
- 权限映射或角色能力数据。
- 全应用使用的功能开关或运行配置。

简单表格筛选不要提升为全局状态，除非路由或持久化确实需要。

---

## 服务端状态

- API 调用集中在 `api/`。
- hook 暴露 loading 和 error 状态。
- 演示兜底必须明确；不要在生产路径中静默混合假数据和真实数据。
- 如果轮询、缓存、失效或乐观更新变常见，再引入 React Query。

## 手机端 Pinia 服务端状态缓存

- 手机端通过底部导航频繁进出，首页聚合数据、广告、当前用户资料、充值配置、充值订单、提现方式、提现申请和资金流水不能在每次组件挂载时无条件重新请求。
- 可复用服务端状态应放在 Pinia store 中缓存，页面只消费 store refs，并调用 store 的 `load*` 方法。
- 首页聚合数据建议使用短 TTL，例如 30 秒；广告这类运营配置可使用更长 TTL；余额必须绑定当前用户 ID，切换用户后立即失效。
- 收到 WebSocket 开奖、开盘、封盘推送，或倒计时已经超过开奖时间时，需要使用 `force + silent` 刷新，不能被普通 TTL 阻止。
- 当前用户资料、余额和资金类列表统一由 `mobileUserData` store 维护；首页、下注页、合买页和我的账户等页面不要再各自直接请求 `/user/me`。
- 资金写入或资料修改成功后必须使用 `force` 刷新或直接写回 store，例如下注、发起/认购合买、充值、提现、绑定邮箱、修改密码和上传头像。
- 列表缓存不能用数组长度判断是否有效；空列表也是有效响应，应以 `fetchedAt` 和 TTL 判断是否需要重新请求。
- 有缓存时不要显示整页 loading；过期刷新应静默进行，避免用户返回页面时看到重复加载闪烁。
- 在线客服未读状态由手机端 `supportUnread` Pinia store 维护，来源为当前用户客服会话列表里的 `userUnreadCount`；收到 `support_message_created` 或 `support_conversation_updated` 实时事件时需要 `force + silent` 刷新，进入具体会话后调用已读接口并写回 store，避免个人中心和底部导航红点状态不一致。

## 实时看板状态

- 秒级倒计时、当前时钟和剩余时间属于本地派生状态，页面组件用 `setInterval` 更新 `now`，不要每秒请求后端。
- 服务端事实数据仍通过 typed hook 获取，例如彩种、期号、开奖结果；需要回流新期号或新开奖结果时，由 hook 使用低频轮询刷新。
- 轮询 hook 必须在 `useEffect` 清理 `AbortController` 和 `window.setInterval`，避免切换页面后继续请求。
- 倒计时展示必须从服务端时间字段即时计算，不要把剩余秒数复制到多个本地 state。

示例：

```tsx
const { data, loading, error, refresh } = useRealtimePanel(10_000);
const [now, setNow] = useState(() => new Date());

useEffect(() => {
  const intervalId = window.setInterval(() => setNow(new Date()), 1_000);
  return () => window.clearInterval(intervalId);
}, []);
```

---

## 常见错误

- 不要把同一份服务端数据镜像到多个独立本地状态。
- 不要存储渲染时可以计算的派生值。
- 不要在需求明确前自建缓存层。
