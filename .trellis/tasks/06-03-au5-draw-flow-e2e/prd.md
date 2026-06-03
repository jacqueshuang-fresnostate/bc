# 澳洲 5 分彩端到端开奖流程跑通

## 目标

把澳洲 5 分彩从部署、数据库迁移、后台配置、期号生成、自动封盘、API 开奖、结算和下一期开盘完整跑通，并记录导致“到达开奖时间一直等待开奖”的真实原因。

## 已知情况

- 当前运行中的 Docker 容器不是最新迁移状态，PostgreSQL 只看到早期 `lotteries` 表。
- 当前容器里的开奖调度配置为 `enabled=false`，`runCount=0`。
- 澳洲 5 分彩开奖源应使用 API68：
  - endpoint: `https://api.api68.com/CQShiCai/getBaseCQShiCaiList.do`
  - `lotCode=10010`
- API68 返回的开奖号码字段为 `preDrawCode`，期号字段为 `preDrawIssue`，系统开奖时需要用本地期号精确匹配 `preDrawIssue`。

## 要求

- 使用最新代码重新构建并启动 Docker Compose。
- PostgreSQL 必须运行最新迁移，包含 `draw_issues`、`draw_sources`、`draw_scheduler_config` 等业务表。
- 后台接口能登录默认管理员账号。
- 后台配置开启常驻开奖调度，无需环境变量启用。
- 澳洲 5 分彩必须存在且销售开启，开奖源必须绑定 `api68-au5`。
- 生成澳洲 5 分彩期号后，自动任务能在开奖时间到达后完成 API 开奖。
- 开奖后需要自动补齐下一期可投注 `open` 期号。
- 如果发现代码或部署缺口，需要修复，并同步更新 `架构设计.md`、`TODO.md`。

## 验收标准

- [x] Docker Compose 使用最新镜像启动成功，应用和 PostgreSQL healthy。
- [x] `_sqlx_migrations` 包含最新业务表迁移。
- [x] `draw_issues`、`draw_sources`、`draw_scheduler_config` 表存在。
- [x] `api68-au5` 开奖源存在，绑定 `au5`。
- [x] `au5` 彩种存在且 `saleEnabled=true`。
- [x] `PUT /api/admin/draw-scheduler/config` 开启后，调度状态 `enabled=true` 且 `runCount` 增长。
- [x] 澳洲 5 分彩至少一个期号成功从 `open/closed` 进入 `drawn`，并保存英文逗号分隔开奖号码。
- [x] 开奖后至少存在下一期 `open` 期号。
- [x] 记录本次跑通结果和发现的问题。

## 非目标

- 不改手机端。
- 不接入新的第三方供应商。
- 不实现 API68 原始响应长期留痕。
- 不处理生产多实例调度锁。
