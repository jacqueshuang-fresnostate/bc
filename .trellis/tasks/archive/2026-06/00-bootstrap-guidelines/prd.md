# 引导任务：补齐项目开发规范

**本任务由 AI 执行，开发者通常不需要直接阅读。**

开发者刚在本项目运行了 `trellis init`。`.trellis/` 已经存在，规格目录也已经创建。本任务的目标是把 `.trellis/spec/` 填成项目真实可用的编码约定，让后续 AI 开发任务不再依赖通用默认风格。

---

## 状态

- [x] 填写后端规范
- [x] 填写前端规范
- [x] 添加代码示例

---

## 已完成的规格文件

### 后端规范

| 文件 | 记录内容 |
|------|----------|
| `.trellis/spec/backend/directory-structure.md` | 后端目录、路由、服务、领域模型和辅助模块组织 |
| `.trellis/spec/backend/database-guidelines.md` | 后续数据库、迁移、查询和金额字段约定 |
| `.trellis/spec/backend/error-handling.md` | API 错误类型、传播方式和响应结构 |
| `.trellis/spec/backend/logging-guidelines.md` | tracing 日志级别、结构化字段和敏感信息限制 |
| `.trellis/spec/backend/quality-guidelines.md` | 禁用模式、必须模式、测试要求和审查清单 |

### 前端规范

| 文件 | 记录内容 |
|------|----------|
| `.trellis/spec/frontend/directory-structure.md` | 管理后台组件、页面、hook、API 和类型目录 |
| `.trellis/spec/frontend/component-guidelines.md` | Semi UI、Tailwind、props、样式和可访问性约定 |
| `.trellis/spec/frontend/hook-guidelines.md` | 自定义 hook、数据获取和命名方式 |
| `.trellis/spec/frontend/state-management.md` | 本地状态、服务端状态、导航状态和全局状态边界 |
| `.trellis/spec/frontend/type-safety.md` | TypeScript 类型组织、API 信封和禁用模式 |
| `.trellis/spec/frontend/quality-guidelines.md` | 构建检查、布局可靠性和代码审查清单 |

---

## 当前约定来源

- 项目目前从文档和需求起步，没有历史代码可抽样。
- 本次规范以 `架构设计.md`、用户指定技术栈、首期工程范围和即将创建的代码结构为依据。
- 项目要求所有面向沉淀或交付的文档内容使用中文输出；代码标识、命令、路径、第三方库名和协议字段名可保留英文。

---

## 完成说明

本任务已为后端和前端建立初始可执行规范。后续当真实代码模式变化、数据库接入、认证权限、彩票计算、财务流水、机器人流程等能力落地时，需要同步更新对应 `.trellis/spec/` 文件。
