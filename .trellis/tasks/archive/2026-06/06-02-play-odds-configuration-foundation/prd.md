# 玩法与赔率配置完善 PRD

## 背景

用户指出当前系统存在三个问题：

1. 玩法需要有地方查看和编辑。
2. 每个彩种的每个玩法赔率都可能不一样。
3. 玩法需要清楚区分 3 个号码和 5 个号码，且要确认玩法是否全部落地。

当前代码状态：

- 后端玩法规则引擎已经覆盖 `3个号码玩法规则说明.md` 中的 5 个 3 位玩法。
- 后端玩法规则引擎已经覆盖 `5个玩法规则说明.md` 中的 19 个 5 位玩法。
- 管理后台“玩法规则”页面只能做规则评估，不能编辑彩种玩法配置和赔率。
- 彩种管理只配置玩法分类，例如直选、组三、组六，不能精确到每个玩法。
- 结算派奖仍使用写死在订单服务里的基础倍数，不支持按彩种和玩法配置赔率。

## 目标

1. 在彩种模型中新增精确到“玩法”的配置，支持每个玩法启用/禁用和配置赔率。
2. “玩法规则”页面升级为玩法查看和赔率编辑页面，清楚区分 3 位玩法和 5 位玩法。
3. 页面可以选择具体彩种，查看该彩种所有适用玩法及赔率，并保存配置。
4. 订单创建时校验具体玩法是否在该彩种启用，而不是只校验玩法分类。
5. 订单创建时快照当前玩法赔率，后续结算使用订单快照赔率，避免赔率修改影响历史订单。
6. 结算派奖金额使用订单上的赔率计算，不再使用写死的后端固定倍数。
7. 更新 `架构设计.md`、API 规范和 `TODO.md`，所有文档输出保持中文。

## 玩法覆盖结论

### 3 位玩法

来自 `3个号码玩法规则说明.md`：

- 直选：已落地为 `threeDirect`
- 组三复式：已落地为 `threeGroupThree`
- 组三胆拖：已落地为 `threeGroupThreeBanker`
- 六组复式：已落地为 `threeGroupSix`
- 六组胆拖：已落地为 `threeGroupSixBanker`

### 5 位玩法

来自 `5个玩法规则说明.md`：

- 前三/中三/后三直选：已落地为 `fiveFrontDirect`、`fiveMiddleDirect`、`fiveBackDirect`
- 前三/中三/后三直选组合：已落地为 `fiveFrontDirectCombination`、`fiveMiddleDirectCombination`、`fiveBackDirectCombination`
- 前三/中三/后三组三复式：已落地为 `fiveFrontGroupThree`、`fiveMiddleGroupThree`、`fiveBackGroupThree`
- 前三/中三/后三组三胆拖：已落地为 `fiveFrontGroupThreeBanker`、`fiveMiddleGroupThreeBanker`、`fiveBackGroupThreeBanker`
- 前三/中三/后三组六复式：已落地为 `fiveFrontGroupSix`、`fiveMiddleGroupSix`、`fiveBackGroupSix`
- 前三/中三/后三组六胆拖：已落地为 `fiveFrontGroupSixBanker`、`fiveMiddleGroupSixBanker`、`fiveBackGroupSixBanker`
- 大小单双：已落地为 `fiveBigSmallOddEven`

本阶段重点不是补公式，而是让这些玩法在后台可查看、可按彩种配置、可参与真实派奖。

## 赔率表示

赔率使用整数 basis points：

- 字段名：`oddsBasisPoints`
- `10000` 表示 `1.00` 倍。
- `100000` 表示 `10.00` 倍。
- 派奖公式：`命中投注数 × 单注金额 × oddsBasisPoints / 10000`

选择这个表示是为了避免浮点金额误差，同时保留小数赔率能力。

## 后端需求

### 彩种玩法配置

`LotteryKind` 新增 `playConfigs`：

```json
{
  "ruleCode": "threeDirect",
  "enabled": true,
  "oddsBasisPoints": 100000
}
```

要求：

- 每个彩种只包含与自身 `numberType` 匹配的玩法。
- 3 位彩种展示 5 个 3 位玩法。
- 5 位彩种展示 19 个 5 位玩法。
- 未启用的玩法仍可保留配置，便于后台查看和后续开启。
- 至少需要有一个启用玩法。
- `oddsBasisPoints` 必须大于 0。
- `playCategories` 继续作为分类摘要存在，但后端需要根据启用玩法保持它与 `playConfigs` 一致。

### 数据库

- 新增迁移，为 `lotteries` 表增加 `play_configs JSONB NOT NULL DEFAULT '[]'`。
- 旧数据如果 `play_configs` 为空，后端读取时按号码类型和玩法分类生成默认玩法配置。

### 订单创建

- 创建订单时必须找到该彩种对应 `ruleCode` 的启用玩法配置。
- 未启用玩法返回业务错误。
- 订单保存当前 `oddsBasisPoints` 快照。
- 订单响应和 dashboard 最近订单需要返回 `oddsBasisPoints`。

### 结算派奖

- 结算使用订单快照 `oddsBasisPoints` 计算 `payoutMinor`。
- 结算单笔结果返回 `oddsBasisPoints`。
- 移除写死的 `payout_multiplier_for_rule` 作为真实结算依据。

### 玩法目录

- `GET /api/admin/play-rules` 继续返回全量玩法目录。
- 玩法目录需要携带 `category`，方便前端分组和同步彩种分类。

## 前端需求

1. `PlayRulesPage` 改为“玩法与赔率配置”工作台。
2. 页面顶部支持选择 3 位玩法 / 5 位玩法视图。
3. 页面展示对应号码类型的完整玩法目录数量和规则说明。
4. 页面支持选择彩种，并显示该彩种所有适用玩法配置。
5. 每行玩法可以启用/禁用，并编辑赔率倍数。
6. 保存后调用现有彩种更新接口，刷新彩种和 dashboard。
7. 页面仍保留规则评估能力，方便验证某个玩法注数和中奖判断。
8. 订单管理和计奖派奖页面显示订单/结算使用的赔率。

## 验收标准

1. `GET /api/admin/play-rules` 返回 3 位 5 个玩法、5 位 19 个玩法，并带有玩法分类。
2. `GET /api/admin/lotteries` 返回每个彩种的 `playConfigs`。
3. 管理后台“玩法规则”页面可以按 3 位/5 位切换查看玩法目录。
4. 管理后台可以选择彩种，编辑该彩种某个玩法的赔率并保存。
5. 创建订单时，如果玩法未启用，后端拒绝创建。
6. 创建订单时，订单响应包含 `oddsBasisPoints`。
7. 修改彩种赔率后，新订单使用新赔率；旧订单结算仍使用创建时快照赔率。
8. 结算派奖金额按订单快照赔率计算，结算结果返回 `oddsBasisPoints`。
9. `cargo fmt --check`、`cargo check`、`cargo test`、`npm run build` 通过。
10. 更新 `架构设计.md`、`.trellis/spec/backend/api-contracts.md` 和 `TODO.md`。

## 非目标

1. 不实现真实奖金上限、单期赔付上限和风控审核。
2. 不实现赔率变更审批、发布版本和历史审计。
3. 不实现手机端玩法配置。
4. 不实现数据库中的订单持久化。
5. 不重写已经通过测试的玩法公式，只补可配置性和展示体验。

## 风险与约束

1. 彩种当前支持 PostgreSQL 持久化，但订单、开奖、结算和资金仍是内存仓储。
2. 旧数据库中已有彩种时，新增 `play_configs` 后需要后端自动补默认配置。
3. 赔率变更不能影响已创建订单，因此订单必须保存赔率快照。
4. 玩法配置和玩法分类容易漂移，后端需要归一化，不能只依赖前端。
