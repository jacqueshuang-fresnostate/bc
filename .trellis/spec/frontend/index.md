# 前端开发规范

> React 管理后台的初始项目约定。

---

## 概览

前端是 React + Vite + TypeScript 管理后台，使用 Tailwind CSS 和 Semi UI。页面应构建为密集、清晰、适合运营工作的后台界面，而不是营销型页面。

本项目从文档起步，因此这些规范是当前初始事实来源。真实约定变化时，需要同步更新本目录文档。

---

## 规范索引

| 文档 | 说明 | 状态 |
|------|------|------|
| [目录结构](./directory-structure.md) | 组件、页面、hook 组织 | 初始 |
| [组件规范](./component-guidelines.md) | 组件模式和 props 约定 | 初始 |
| [Hook 规范](./hook-guidelines.md) | 自定义 hook 和数据获取模式 | 初始 |
| [状态管理](./state-management.md) | 本地、全局、服务端和 URL 状态 | 初始 |
| [质量规范](./quality-guidelines.md) | 构建、测试、可访问性 | 初始 |
| [类型安全](./type-safety.md) | TypeScript 约定和校验 | 初始 |

---

## 核心规则

- 优先构建紧凑的后台工作流，不做装饰性布局。
- 复杂控件和表格优先使用 Semi UI。
- Tailwind 用于布局、间距和小范围视觉修饰。
- API 调用需要集中且有类型。
- 当 hook 或类型化客户端更清晰时，不要发明一次性的状态模式。
