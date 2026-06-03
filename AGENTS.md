<!-- TRELLIS:START -->
# Trellis Instructions

These instructions are for AI assistants working in this project.

This project is managed by Trellis. The working knowledge you need lives under `.trellis/`:

- `.trellis/workflow.md` — development phases, when to create tasks, skill routing
- `.trellis/spec/` — package- and layer-scoped coding guidelines (read before writing code in a given layer)
- `.trellis/workspace/` — per-developer journals and session traces
- `.trellis/tasks/` — active and archived tasks (PRDs, research, jsonl context)

If a Trellis command is available on your platform (e.g. `/trellis:finish-work`, `/trellis:continue`), prefer it over manual steps. Not every platform exposes every command.

If you're using Codex or another agent-capable tool, additional project-scoped helpers may live in:
- `.agents/skills/` — reusable Trellis skills
- `.codex/agents/` — optional custom subagents

Managed by Trellis. Edits outside this block are preserved; edits inside may be overwritten by a future `trellis update`.

<!-- TRELLIS:END -->
阅读 架构设计.md 并且进行系统的开发，如果需要对功能的添加修改 也需要对应的去修改 架构设计.md ，并且在开发过程中 需要 在TODO.md 中 把每次完成了什么任务，解决了什么问题，解决问题的时间描述清楚
在 架构设计.md 有个 #后续 后面如果需要添加什么修改什么功能也会在那里进行添加，你需要不断的去查看是否有新的需求
所有面向项目沉淀或交付的文档内容都需要使用中文输出，包括 PRD、TODO、架构说明、开发规格、总结记录和后续新增文档；只有代码标识、命令、路径、第三方库名、协议字段名等必须保持原文的内容可以保留英文。
后续 Git 提交信息也需要使用中文，提交标题和必要的提交说明都应清楚描述本次完成的功能、修复的问题或规则变更。
后续功能验证和联调测试不要通过 Docker 打包镜像来完成，直接本地启动后端和前端服务进行测试；Docker 镜像只用于明确要求的镜像构建、发布或部署验证。
后端本地测试默认使用外部 PostgreSQL，`DATABASE_URL` 通过本地环境变量或被 `.gitignore` 忽略的 `.env.local` 传入，连接模板为 `postgres://root:<密码>@192.168.2.3:15432/postgres`，密码不要写入可提交文件。后端从 `backend/` 目录执行 `cargo run`，启动时会加载项目根目录和 `backend/` 下的 `.env`、`.env.local`；前端从 `admin/` 目录执行 `npm run dev -- --host 127.0.0.1 --port <空闲端口>`，并通过 `admin/.env.local` 的 `VITE_API_BASE_URL` 指向后端。
