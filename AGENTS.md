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
