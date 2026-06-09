# 类型安全

> 本项目 TypeScript 类型安全模式。

---

## 概览

管理后台全量使用 TypeScript。共享响应和仪表盘类型放在 `src/types/`；接口函数返回带类型的 Promise。

---

## 类型组织

- 跨组件类型放在 `src/types/`。
- 组件专用 props 接口放在组件附近。
- 与后端响应字段名保持完全一致。
- 当行为基于状态分支时，使用可辨识联合类型。

---

## 校验

首个里程碑可以信任本地后端演示响应。后续接收用户输入表单或集成外部 API 时，需要在业务关键页面使用运行时校验。

如果引入校验库，优先选择 Zod，并同步更新本文件示例。

---

## 常见模式

使用类型化 API 信封：

```ts
export interface ApiEnvelope<T> {
  success: boolean;
  data: T;
  message: string;
}
```

使用显式联合：

```ts
export type DrawMode = 'platform' | 'api' | 'manual';
```

---

## 跨层 API 契约

管理后台类型必须与 `.trellis/spec/backend/api-contracts.md` 中记录的后端接口契约一致。

- `DashboardSummary` 必须覆盖 `/api/admin/dashboard` 的所有顶层字段。
- 后端 `camelCase` 字段名不能在前端改写成 `snake_case`。
- 金额字段保留最小货币单位字段名，例如 `amountMinor`、`balanceMinor`。
- 管理后台面向运营输入金额时使用“元”字符串，提交前统一通过 `src/utils/moneyInput.ts` 转换为 `amountMinor` 等最小货币单位字段；不要让运营表单直接输入“分”。
- 返利比例使用 `defaultRechargeRebateBasisPoints`，不要使用浮点百分比字段。

---

## 禁用模式

- 避免使用 `any`。
- 避免在未校验 API 响应时使用 `as DashboardSummary` 这种宽泛断言。
- 避免用不同拼写重复定义后端枚举名称。
