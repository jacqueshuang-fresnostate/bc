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
- 客服会话消息流应使用 Semi UI `Chat` 组件承载，不手写消息气泡列表；当后台页面只需要展示历史消息、回复输入由业务表单承担时，需要设置 `renderInputArea={() => null}` 和 `enableUpload={false}`，避免出现重复输入区或默认上传控件警告。
- 手机端彩票卡片展示开奖号码时，不能按卡片变体写死 3 位；必须优先使用真实 `latestResult.length`，再结合后端 `resultCount` 决定展示数量，兼容 3 位和 5 位彩种。没有开奖结果时才按该数量用期号尾号或 `?` 补位。
- 手机端首页“高频极速”推荐区的开奖号码必须使用固定正圆号码球，样式需要同时约束 `width`、`height`、`aspect-ratio: 1 / 1` 和 `border-radius: 9999px`，不要只依赖文字内容或内边距撑开形状。
- 手机端充值页必须以 `GET /api/user/recharge/config` 返回的后台充值配置为准，只展示已开启的 `rainbowEpay` 和 `customerService` 渠道；彩虹易支付使用后端 `payTypes`，客服直充创建订单后跳转绑定的客服会话。不要再调用旧 `/payment/*` 接口，也不要在后端未配置时展示 USDT 或快捷充值模式。

错误示例：

```vue
<div v-for="digit in digits(3)">{{ digit }}</div>
```

正确示例：

```ts
const displayCount = Math.max(lottery.latestResult?.length || 0, lottery.resultCount || 0, 3)
const displayDigits = roundDigits(lottery, displayCount)
```

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
