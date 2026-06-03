# GitHub Actions CI 与 Docker 镜像发布

## 背景

项目已经上传到 GitHub，并新增了前后端同镜像 Docker 打包方案，但仓库还没有 GitHub Actions workflow，无法在 push/PR 时自动检查，也无法在主分支发布 Docker image。

## 目标

1. 新增 GitHub Actions workflow。
2. 在 `push` 和 `pull_request` 时运行后端格式检查、类型检查、测试，以及前端生产构建。
3. 在 `push` 和 `pull_request` 时构建 Docker image，确保单镜像方案可被 CI 验证。
4. 在 `main` 分支 push 时把 Docker image 推送到 GitHub Container Registry。
5. 更新中文架构文档、TODO 和 Trellis 部署规范。

## 非目标

1. 不接入第三方镜像仓库。
2. 不配置生产服务器自动部署。
3. 不改变现有 Dockerfile、Nginx 或后端 API 契约。

## 验收

1. `.github/workflows/ci.yml` 存在。
2. workflow 使用 `GITHUB_TOKEN` 登录 GHCR，并在 `main` push 时推送 `ghcr.io/<owner>/<repo>:latest` 和 sha tag。
3. workflow 在 PR 时只构建不推送镜像。
4. 本地能通过基础语法检查，且项目现有本地检查保持通过。
