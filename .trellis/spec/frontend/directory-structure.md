# 目录结构

> 前端代码在本项目中的组织方式。

---

## 概览

管理后台位于 `admin/`。代码围绕应用外壳、页面、共享组件、hooks、API 访问和类型化领域模型组织。

---

## 目录布局

```text
admin/
├── package.json
├── index.html
├── tailwind.config.js
├── vite.config.ts
└── src/
    ├── App.tsx
    ├── main.tsx
    ├── index.css
    ├── api/
    │   └── client.ts
    ├── components/
    │   ├── AppShell.tsx
    │   ├── MetricCard.tsx
    │   └── ModulePanel.tsx
    ├── hooks/
    │   └── useDashboard.ts
    ├── pages/
    │   ├── DashboardPage.tsx
    │   └── PlaceholderPage.tsx
    └── types/
        └── dashboard.ts
```

---

## 模块组织

- `api/` 放 HTTP 客户端辅助函数和接口请求函数。
- `components/` 放不绑定单个页面的共享 UI 组件。
- `hooks/` 放可复用的状态逻辑，尤其是 API 加载状态。
- `pages/` 放路由级或导航级页面。
- `types/` 放跨组件共享的 TypeScript 接口。

---

## 命名约定

- 组件文件使用 `PascalCase.tsx`。
- hook 文件使用 `useSomething.ts`。
- 非组件 TypeScript 模块使用 `camelCase.ts`。
- 类型名使用 `PascalCase`。
- Tailwind 工具类较多的组件仍然需要使用语义清晰的组件名称。

---

## 示例

使用页面组件作为导航目标：

```tsx
export function DashboardPage() {
  return <main>...</main>;
}
```

共享展示控件保持可复用：

```tsx
export function MetricCard(props: MetricCardProps) {
  return <Card>...</Card>;
}
```
