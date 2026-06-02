# 组件规范

> 本项目组件构建方式。

---

## 概览

组件应呈现运营控制台的感觉：简洁、可扫描、可预期。导航、卡片、表格、按钮、标签、提示和加载状态优先使用 Semi UI。布局和间距使用 Tailwind。

---

## 组件结构

- 使用命名导出。
- props 接口放在拥有该组件的文件附近，除非多个文件共享。
- 组件职责保持聚焦；页面重复同一模式时再抽取共享展示控件。
- 避免多层卡片嵌套。

示例：

```tsx
interface MetricCardProps {
  label: string;
  value: string;
}

export function MetricCard({ label, value }: MetricCardProps) {
  return <Card>{value}</Card>;
}
```

---

## Props 约定

- 优先使用明确的 prop 名称，避免大型无类型配置对象。
- 只有明确需要插槽时才使用 `ReactNode`。
- 避免用大量可选 props 制造很多视觉模式；行为分叉明显时拆组件。
- 回调 props 使用 `on` 前缀，例如 `onRefresh`。

---

## 样式模式

- Tailwind 用于布局、网格、间距和响应式。
- 能用 Semi UI 组件 prop 表达的变体，优先不用自定义 CSS。
- 页面背景保持克制并服务后台工作。
- 卡片只用于真实分组数据或重复项，不用于装饰页面区块。
- 后台列表页的创建/编辑维护表单如不需要常驻对照，应使用 Semi UI `SideSheet` 打开；主页面保留列表、筛选、统计和“新建/编辑”入口，避免右侧表单长期占用列表扫描空间。
- `SideSheet` 表单保存、删除成功后应关闭抽屉，并沿用页面原有 hook 或 API 刷新链路；切换模块时应关闭已打开的维护抽屉，防止不同模块的编辑状态残留。

> **注意**：当前 `@douyinfe/semi-ui` 包的 `exports` 不暴露 `dist/css/semi.min.css` 作为 bare import。Vite 构建中需要通过相对路径导入完整样式：
>
> ```ts
> import '../node_modules/@douyinfe/semi-ui/dist/css/semi.min.css';
> ```
>
> 如果升级 Semi UI 后官方暴露了新的样式入口，需要先更新本规范，再调整代码。

---

## 可访问性

- 图标按钮需要清晰标签或 tooltip。
- 加载和错误状态必须可见。
- 不要只依赖颜色表达状态。
- 文本在移动端和桌面端都需要可读且不溢出。

---

## 常见错误

- 不要为管理后台创建营销型 hero 页面。
- 不要在仪表盘和面板中使用过大的字体。
- 不要让接口失败时只显示空屏。
