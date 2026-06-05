# TODO

## 2026-06-05 12:34 HKT 手机端下注页接入当前投注接口

- 完成任务：把手机端下注页和注单记录接入当前后端用户端投注接口。
- 解决问题：
  - 下注页仍请求旧 `/bet/page-config/{code}`，后端当前没有该路由，无法加载真实玩法、期号和赔率。
  - 批量下注仍提交旧 `/bet/place-batch` 的 `play_code/numbers/amount` 结构，无法复用当前订单、玩法规则和财务扣款链路。
  - 注单记录仍请求旧 `/bet/orders`，用户端无法读取当前订单仓储里的投注记录。
- 具体实现：
  - 后端新增 `MobileBetPageConfig`、用户端投注批量请求/响应结构，以及 `services/mobile_bet.rs` 下注页配置聚合服务。
  - 用户路由新增 `GET /api/user/bet/page-config/{lottery_id}`、`GET /api/user/bet/orders` 和 `POST /api/user/bet/orders`。
  - 用户端下单从登录会话读取用户 ID，先整体校验期号、玩法、赔率和余额，再逐单创建订单并扣款；扣款失败会移除未入账订单。
  - 手机端新增 `mobile/src/api/bet.ts`，统一封装下注配置、批量下单和注单记录归一化。
  - 动态下注页改为读取新接口，并把位置宫格、直选组合、复式、胆拖和大小单双转换成后端 `selection`。
  - 新增 `direct_combination` 位置宫格类型，直选组合按排列数计算注数，并在注单详情中展开显示。
  - OpenAPI、Trellis 后端 API 契约、前端组件规范和架构说明已同步记录新投注接口契约。
- 验证记录：
  - `cargo fmt --manifest-path backend/Cargo.toml` 通过。
  - `cargo check --manifest-path backend/Cargo.toml` 通过。
  - `cargo test --manifest-path backend/Cargo.toml mobile_bet -- --nocapture` 通过。
  - `cargo test --manifest-path backend/Cargo.toml openapi_document_contains_core_paths -- --nocapture` 通过。
  - `cargo test --manifest-path backend/Cargo.toml -- --nocapture` 通过，188 条后端测试全部成功；仍有既有 `LotteryCategory` 未使用导入 warning。
  - `cd mobile && npm run build` 通过。

## 2026-06-05 12:08 HKT 手机端充值页体验优化

- 完成任务：优化手机端 `deposit` 页面，让充值流程更像移动端钱包操作页。
- 解决问题：原页面虽然已接入当前充值模式，但仍偏表单堆叠，充值渠道需要弹层选择，金额输入缺少快捷金额，最近充值记录缺少后续操作入口，用户完成充值路径不够直观。
- 具体实现：
  - `DepositView.vue` 顶部新增余额与充值订单摘要，展示账户余额、可用渠道、待处理订单和已入账订单数量。
  - 充值方式改为直接展示渠道卡片，用户可在“彩虹易支付”和“客服直充”之间直接切换，不再通过底部弹层选择。
  - 充值金额区新增快捷金额按钮，按后台单笔充值上下限过滤可选金额。
  - 底部新增固定提交栏，实时展示本次充值金额和当前渠道提示，主操作始终可见。
  - 最近充值记录新增“继续支付”和“联系客服”操作入口，待支付订单和客服直充订单可以继续处理。
  - 前端组件规范补充手机端充值页的界面交互约束。
- 验证记录：
  - `cd mobile && npm run build` 通过。

## 2026-06-05 11:55 HKT 手机端充值页接入当前充值模式

- 完成任务：把手机端 `deposit` 页面改为依据后台当前充值配置展示和下单。
- 解决问题：充值页仍使用旧的 `/payment/methods`、`/payment/fiat/create-order`、`/payment/usdt/create-order` 和 `fiat/usdt` 模式，和当前后端的“彩虹易支付 / 客服直充”充值体系不一致。
- 具体实现：
  - `mobile/src/api/user.ts` 新增充值配置、充值订单、创建充值订单、客服会话列表、客服会话详情和用户回复接口封装。
  - `DepositView.vue` 改为读取 `GET /api/user/recharge/config`，只展示后台开启的 `rainbowEpay` 和 `customerService` 渠道。
  - 彩虹易支付按后台 `payTypes` 展示支付方式，创建订单后打开后端返回的 `paymentUrl`。
  - 客服直充创建订单后跳转到 `/support?conversationId=...`，让用户直接进入对应客服会话继续沟通。
  - `SupportView.vue` 改为接入当前 `/api/user/support/conversations` 会话接口，支持从充值页带入会话 ID 后继续发送文字消息。
  - 前端组件规范补充手机端充值页必须以后台充值配置为准，不能继续调用旧支付接口或展示未配置的 USDT 模式。
- 验证记录：
  - `cd mobile && npm run build` 通过。

## 2026-06-05 11:47 HKT 高频极速开奖号码正圆展示

- 完成任务：把手机端首页“高频极速”模块的开奖号码改为固定正圆号码球展示。
- 解决问题：仅依赖 Tailwind 圆角和尺寸类时，后续如果内容、内边距或样式覆盖变化，号码球可能呈现为非正圆。
- 具体实现：
  - `HomeDrawCard.vue` 新增 `home-result-ball` scoped 样式，强制设置固定尺寸、`aspect-ratio: 1 / 1`、`border-radius: 9999px` 和不收缩。
  - 高频极速推荐大卡使用 `home-result-ball--featured`，小卡使用 `home-result-ball--secondary`。
  - 前端组件规范补充高频极速开奖号码必须使用固定正圆号码球的约束。
- 验证记录：
  - `cd mobile && npm run build` 通过。

## 2026-06-05 11:43 HKT 手机端高频极速开奖号码位数兼容

- 完成任务：修复手机端首页“高频极速”模块 5 位开奖号码只显示 3 位的问题。
- 解决问题：
  - `HomeDrawCard.vue` 的推荐大卡和小卡原先写死 `digits(3)`，5 位彩种会被截断为 3 位。
  - `roundDigits` 使用真实开奖结果数组补位时没有拷贝，存在把 `latestResult` 原数组补上 `?` 的风险。
- 具体实现：
  - 首页彩票卡片统一使用 `latestResult.length`、后端 `resultCount` 和默认 3 的最大值计算展示位数。
  - 推荐大卡、推荐小卡和分组卡片统一使用 `displayDigits` 渲染开奖号码，兼容 3 位和 5 位。
  - `roundDigits` 改为复制真实开奖结果后再补位，且真实结果长度大于兜底值时不截断。
  - 调整号码球尺寸和换行能力，避免 5 位号码在移动端卡片内溢出。
  - `.trellis/spec/frontend/component-guidelines.md` 已补充彩票卡片开奖号码位数规范。
- 验证记录：
  - `cd mobile && npm run build` 通过。

## 2026-06-05 11:38 HKT 手机端首页移除“全部 ›”入口

- 完成任务：移除手机端首页推荐区和分类分组标题右侧的“全部 ›”按钮。
- 解决问题：首页标题区重复出现“全部 ›”跳转入口，用户要求手机端统一去掉该文案和箭头。
- 具体实现：
  - 删除 `mobile/src/views/HomeView.vue` 推荐区标题右侧的“全部 ›”按钮。
  - 删除分类分组标题右侧的“全部 ›”按钮。
  - 移除按钮唯一使用的 `openAllLotteries` 方法，避免留下无用代码。
- 验证记录：
  - `cd mobile && npm run build` 通过。

## 2026-06-05 02:30 HKT 登录会话 Token 随机化与摘要落库

- 完成任务：修复用户和管理员登录 token 暴露账号信息、时间戳和计数器的问题。
- 解决问题：
  - 用户登录 token 原先类似 `user-U10001-时间戳-序号`，管理员 token 原先类似 `adm-A10001-时间戳-序号`，可读且可预测。
  - 数据库会话表保存原始 Bearer token，一旦数据库被查看就能直接拿到可用登录态。
- 具体实现：
  - 新增 `sha2` 直接依赖，使用 `Sha256` 计算会话 token 摘要。
  - 登录签发 `bcst_` 前缀的 32 字节强随机 token，不再拼接用户 ID、管理员 ID、时间戳或计数器。
  - `admin_sessions.token` 和 `user_sessions.token` 只保存 `sha256:` 摘要；认证和登出时对请求 token 先计算摘要再处理。
  - 新增迁移 `20260605009000_hash_login_session_tokens.sql`，删除历史明文会话并更新 SQL 字段中文注释。
  - 新增管理员和用户会话 token 回归测试，验证返回 token 不含账号 ID，仓储不保存原始 token。
- 验证记录：
  - `cargo fmt --manifest-path backend/Cargo.toml` 通过。
  - `cargo check --manifest-path backend/Cargo.toml` 通过。
  - `cargo test --manifest-path backend/Cargo.toml access_repository_hashes -- --nocapture` 通过。
  - `cargo test --manifest-path backend/Cargo.toml` 通过，186 个后端测试全部通过；日志中仍有既有 `LotteryCategory` 未使用导入警告。

## 2026-06-05 00:47 HKT 手机端彩种分组与开奖历史接口补齐

- 完成任务：补齐手机端彩种分组、最新开奖和开奖历史接口，并接入相关手机端页面。
- 解决问题：
  - 全部彩种页、开奖历史页和合买创建入口仍直接请求旧 `/lottery/groups`、`/lottery/history/latest`、`/lottery/history` 裸响应。
  - 本项目后端没有这些用户端彩票接口，手机端页面在本地后端环境下会拿不到彩种分组和开奖记录。
- 具体实现：
  - 后端 `routes/lottery.rs` 新增 `GET /lottery/groups`、`GET /lottery/history/latest`、`GET /lottery/history`。
  - 彩种分组只返回销售中彩种；开奖历史只返回销售中彩种的已开奖且有开奖号码记录。
  - `mobile/src/api/lottery.ts` 新增分组、最新开奖和开奖历史的类型化封装。
  - `AllLotteryView.vue`、`useLotteryHistory.ts`、`useBettingRound.ts`、`features/group-buy/api.ts` 改为复用统一彩票 API client。
  - OpenAPI 文档同步新增三条用户端彩票路径。
- 验证记录：
  - `cargo fmt --manifest-path backend/Cargo.toml --check` 通过。
  - `cargo check --manifest-path backend/Cargo.toml` 通过。
  - `cargo test --manifest-path backend/Cargo.toml routes::lottery -- --nocapture` 通过。
  - `cargo test --manifest-path backend/Cargo.toml openapi_document_contains_core_paths -- --nocapture` 通过。
  - `cd mobile && npm run build` 通过。

## 2026-06-05 00:33 HKT 手机端首页销售中彩种分组接口

- 完成任务：接好手机端首页彩种接口，返回所有销售中的彩种、分类分组和最近开奖号码。
- 解决问题：
  - 手机端首页原先请求 `/api/lottery/home`，但本项目后端没有实际挂载该接口。
  - 首页分组展示只取固定前两个分组，无法展示所有销售中的彩种分类。
  - 首页卡片缺少从本地后端聚合出的最近开奖号码，仍依赖旧接口字段假设。
- 具体实现：
  - 后端新增 `routes/lottery.rs`，挂载 `GET /api/lottery/home`。
  - 后端新增 `services/mobile_home.rs`，统一组合彩种、分类、当前期号和最近已开奖期号。
  - `domain/mobile.rs` 新增手机端首页响应结构，字段统一通过 `camelCase` 输出。
  - OpenAPI 文档新增 `/lottery/home`，核心路径测试同步覆盖。
  - 手机端新增 `mobile/src/api/lottery.ts`，首页使用类型化 `fetchLotteryHomepage()`。
  - `HomeView.vue` 改为动态渲染接口返回的全部分类分组。
  - `HomeDrawCard.vue` 和 `useHomepageDrawUpdates.ts` 切换为 `camelCase` 首页字段。
- 验证记录：
  - `cargo fmt --manifest-path backend/Cargo.toml` 通过。
  - `cargo check --manifest-path backend/Cargo.toml` 通过。
  - `cargo test --manifest-path backend/Cargo.toml mobile_home -- --nocapture` 通过。
  - `cargo test --manifest-path backend/Cargo.toml openapi_document_contains_core_paths -- --nocapture` 通过。
  - `cd mobile && npm run build` 通过。

## 2026-06-04 23:36 HKT 手机端提现申请记录接口对接

- 完成任务：把手机端提现页接入用户提现申请记录接口。
- 解决问题：此前手机端只调用提现申请提交接口，用户提交后看不到自己的申请记录、审核状态和收款账户快照，容易误以为接口未生效。
- 具体实现：
  - `WithdrawView.vue` 引入 `fetchWithdrawalOrders`，加载提现页时与余额、收款账户一起请求。
  - 提现提交成功后重新刷新余额、提现方式和提现申请记录。
  - 新增“提现申请记录”区块，展示最近 6 条提现申请。
  - 申请记录展示状态中文文案、金额、创建时间、审核时间、收款方式、收款账户快照和申请单号。
- 验证记录：
  - `cd mobile && npm run build` 通过。

## 2026-06-04 23:23 HKT 财务管理 Tabs 功能分组

- 完成任务：把后台财务管理页面改为使用 Tabs 对功能进行划分。
- 解决问题：财务管理页原先把资金账户、手动调账、充值订单、提现管理和资金流水连续堆叠，页面过长，财务人员切换功能不够直观。
- 具体实现：
  - `FinanceManagementPage.tsx` 引入 Semi UI `Tabs`。
  - 顶部财务指标继续作为全局摘要保留。
  - 新增“账户与调账”“充值订单”“提现管理”“资金流水”四个标签页。
  - 每个标签显示对应列表数量，原分页、调账、充值确认、提现通过/驳回操作保持不变。
- 验证记录：
  - `cd admin && npm run build` 通过；Vite 仍提示既有 chunk 体积超过 500kB。

## 2026-06-04 23:09 HKT 财务管理分页与提现管理

- 完成任务：完善后台财务管理，给资金账户、充值订单、资金流水增加分页，并新增提现管理和提现审核能力。
- 解决问题：
  - 资金账户只显示用户 ID，没有用户名，财务人员不方便识别用户。
  - 资金账户、充值订单、资金流水一次性展示全量数据，数据增多后页面扫描和加载都会变差。
  - 用户端已经能提交提现申请，但后台没有提现管理入口处理申请。
- 具体实现：
  - 后端新增 `FinancePage<T>` 分页响应和 `AdminFinancialAccountSummary`，资金账户分页接口返回用户名。
  - 新增 `GET /api/admin/finance-overview`，财务页顶部指标从后端总览读取，避免被当前页数据影响。
  - `GET /api/admin/financial-accounts`、`GET /api/admin/recharge-orders`、`GET /api/admin/ledger-entries`、`GET /api/admin/withdrawal-orders` 支持 `page/pageSize`。
  - 新增提现审核接口 `POST /api/admin/withdrawal-orders/{id}/approve` 和 `POST /api/admin/withdrawal-orders/{id}/reject`。
  - 提现通过写入 `withdrawalPayout` 流水并扣减冻结余额；提现驳回写入 `withdrawalReject` 流水并把冻结余额退回可用余额。
  - 管理后台财务页新增分页控件和“提现管理”表格，待审核提现可直接通过或驳回。
  - OpenAPI、Trellis API 契约和数据库规范已同步新增财务分页与提现审核场景。
- 验证记录：
  - `cargo check --manifest-path backend/Cargo.toml` 通过。
  - `cargo test --manifest-path backend/Cargo.toml finance::tests -- --nocapture` 通过。
  - `cargo test --manifest-path backend/Cargo.toml withdrawal::tests -- --nocapture` 通过。
  - `cargo test --manifest-path backend/Cargo.toml openapi_document_contains_core_paths -- --nocapture` 通过。
  - `cd admin && npm run build` 通过；Vite 仍提示既有 chunk 体积超过 500kB。

## 2026-06-04 22:34 HKT 手机端安全中心 Tabs 分类

- 完成任务：把手机端“安全中心”的绑定邮箱和修改密码拆分为两个标签页。
- 解决问题：安全中心原先把账号信息、绑定邮箱和修改密码连续堆叠在同一页面，入口不够清晰，用户容易在两个安全操作之间混淆。
- 具体实现：
  - `SecurityCenterView.vue` 新增 `activeTab` 状态，使用 Vant `van-tabs` 和 `van-tab` 分别承载“绑定邮箱”和“修改密码”。
  - “绑定邮箱”Tab 保留账号信息、当前邮箱、绑定状态、绑定邮箱表单和已绑定/未开放提示。
  - “修改密码”Tab 独立展示当前密码、新密码、确认新密码和提交按钮。
  - 邮箱绑定成功后自动切换到“修改密码”Tab，方便用户继续完成密码安全操作。
- 验证记录：
  - `cd mobile && npm run build` 通过。

## 2026-06-04 22:28 HKT 后台用户维护字段收紧与用户提现申请接口

- 完成任务：移除后台用户维护中用户 ID、账户余额、邀请码的编辑能力，并新增用户端提现申请接口。
- 解决问题：
  - 用户维护页可以直接编辑余额，绕过财务管理和资金流水审计。
  - 用户维护页可以编辑用户 ID 和邀请码，不符合用户 ID/邀请码不可变的业务要求。
  - 手机端申请提现调用 `/user/withdrawals`，但后端没有对应接口。
- 具体实现：
  - `AccessManagementPage.tsx` 中用户 ID、账户余额、邀请码改为只读展示；余额提示必须通过财务管理手动调账。
  - 后端 `AccessStore::update_user()` 强制保留原 `balanceMinor` 和 `inviteCode`，防止绕过前端直接修改。
  - 后台用户列表和用户详情返回时，用财务账户 `availableBalanceMinor` 覆盖用户摘要余额，确保用户维护展示的是财务账户余额。
  - 新增 `WithdrawalOrderSummary`、`CreateWithdrawalOrderRequest` 和 `WithdrawalRepository`，支持用户提现申请列表和创建。
  - 新增 `GET /api/user/withdrawals` 和 `POST /api/user/withdrawals`；创建申请时校验提现方式归属，并冻结用户可用余额。
  - 财务流水新增 `withdrawalFreeze`，提现申请成功后可用余额减少、冻结余额增加，资金流水写入提现申请 ID。
  - 新增迁移 `20260605007000_create_withdrawal_orders.sql`，创建 `withdrawal_orders`、`withdrawal_runtime` 并补全中文注释。
  - 手机端提现提交改为调用 `createWithdrawalOrder({ methodId, amountMinor })`。
  - OpenAPI 文档新增用户提现申请列表与提交接口。
  - Trellis 后端 API 契约和数据库规范已同步新增提现申请场景。
- 当前边界：
  - 本阶段只完成用户提交提现申请并冻结余额；后台提现审核、驳回解冻、确认打款和提现记录筛选后续继续完善。
- 验证记录：
  - `cargo check --manifest-path backend/Cargo.toml` 通过。
  - `cargo test --manifest-path backend/Cargo.toml access_repository_update_preserves_balance_and_invite_code -- --nocapture` 通过。
  - `cargo test --manifest-path backend/Cargo.toml store_freezes_withdrawal_once -- --nocapture` 通过。
  - `cargo test --manifest-path backend/Cargo.toml withdrawal_store -- --nocapture` 通过。
  - `cargo test --manifest-path backend/Cargo.toml openapi_document_contains_core_paths -- --nocapture` 通过。
  - `cargo test --manifest-path backend/Cargo.toml -- --nocapture` 通过，177 条测试全部成功；仍有 4 个既有 `LotteryCategory` 未使用导入 warning。
  - `cd admin && npm run build` 通过；Vite 仍提示既有 chunk 体积超过 500kB。
  - `cd mobile && npm run build` 通过。

## 2026-06-04 21:57 HKT 手机端提现方式管理接口对接

- 完成任务：把手机端“提现管理”的提现方式列表、新增、编辑、删除和设默认接入后端真实接口。
- 解决问题：手机端提现管理原先按旧接口读取 `items/config`，并使用 `method_type`、`account_no`、`bank/usdt`、`/default` 等旧字段和路由；当前后端真实接口返回统一响应信封和 `camelCase` 字段，导致页面无法正常管理提现方式。
- 具体实现：
  - `mobile/src/api/user.ts` 新增提现方式类型和 `fetchWithdrawalMethods()`、`createWithdrawalMethod()`、`updateWithdrawalMethod()`、`deleteWithdrawalMethod()`。
  - `WithdrawalMethodsView.vue` 改为使用 `alipay`、`wechat`、`bankCard` 三种后端支持类型，并提交 `methodType`、`accountHolder`、`accountNumber`、`bankName`、`isDefault`。
  - 银行卡保存前校验银行名称；支付宝和微信不再提交无效银行字段。
  - 设置默认提现方式改为调用 `PUT /api/user/withdrawal-methods/{method_id}` 并传入 `isDefault=true`，不再调用不存在的 `/default` 子路由。
  - `WithdrawView.vue` 读取收款账户列表时复用同一 API 封装，确保提现申请页能展示管理页维护的收款账户。
- 当前边界：
  - 后端当前还没有用户提现申请提交路由，`WithdrawView.vue` 的真正提现提交动作需要后续新增提现订单接口后继续完整接入。
- 验证记录：
  - `cd mobile && npm run build` 通过。

## 2026-06-04 21:53 HKT 手机端个人中心移除快捷充值渠道

- 完成任务：删除手机端个人中心的“快捷充值渠道”模块。
- 解决问题：个人中心钱包卡片下方仍展示“USDT 极速充值 / RMB支付”等快捷充值渠道，不符合当前手机端页面需求。
- 具体实现：
  - 移除 `ProfileView.vue` 中的 `QuickActionGrid` 引用和快捷充值渠道渲染区块。
  - 移除个人中心对 `/payment/methods` 的快捷渠道配置请求，避免页面继续加载已删除模块的数据。
  - 保留钱包卡片中的充值、提现入口，不影响用户进入充值页面。
- 验证记录：
  - `cd mobile && npm run build` 通过。

## 2026-06-04 21:49 HKT 新用户资金账户自动初始化

- 完成任务：修复新注册用户或后台新建用户缺少资金账户导致的 `financial account not found`。
- 解决问题：手机端测试用户 `U90004` 已有用户记录，但 `financial_accounts` 中没有对应账户；后续余额校验、投注扣款或财务读取会报 `not found: financial account \`U90004\` not found`。
- 具体实现：
  - 用户端注册接口 `POST /api/user/register` 成功创建用户后立即调用 `finance.account_or_create()` 初始化 0 余额资金账户。
  - 后台新建用户接口成功创建用户后同样初始化 0 余额资金账户。
  - PostgreSQL 财务仓储启动加载时会扫描 `users` 表，对已有用户中缺失 `financial_accounts` 的记录自动补 0 余额账户并持久化。
  - 财务余额校验遇到历史缺失账户时按 0 余额处理，返回 `insufficient available balance`，不再向用户暴露内部账户缺失错误。
  - 新增财务单元测试覆盖“缺账户用户下注返回余额不足”和“账户初始化创建 0 余额账户”。
- 验证记录：
  - `cargo fmt --manifest-path backend/Cargo.toml --check` 通过。
  - `cargo check --manifest-path backend/Cargo.toml` 通过。
  - `cargo test --manifest-path backend/Cargo.toml finance::tests -- --nocapture` 通过，9 条财务测试全部成功。
  - `cargo test --manifest-path backend/Cargo.toml -- --nocapture` 通过，173 条测试全部成功；测试构建仍有 4 个既有 `LotteryCategory` 未使用导入 warning。
  - 使用用户提供的 PostgreSQL 本地启动后端，注册新用户 `acctfix_80954862` 得到 `U90005`，调用 `/api/user/balance` 返回 0 余额资金账户。
  - 通过后台 `/api/admin/financial-accounts` 验证历史用户 `U90004` 已自动补齐 `{ availableBalanceMinor: 0, frozenBalanceMinor: 0 }`。

## 2026-06-04 21:41 HKT 手机端轮播接口对接

- 完成任务：把手机端首页轮播接入后端公开广告接口。
- 解决问题：手机端首页原先只从旧的 `/api/lottery/home` 聚合数据读取 `banners`，没有使用后台“广告管理”维护的 `GET /api/user/mobile/advertisements`，导致后台配置的手机端轮播广告无法在手机端首页展示。
- 具体实现：
  - `mobile/src/api/user.ts` 新增 `MobileAdvertisement` 类型和 `fetchMobileAdvertisements()`，统一通过 `ApiEnvelope` 解析 `GET /api/user/mobile/advertisements`。
  - `mobile/src/views/HomeView.vue` 新增 `mobileAdvertisements` 状态，首页加载时并发请求首页数据和手机端轮播广告。
  - 将后端广告字段 `imageUrl`、`linkUrl`、`sortOrder` 映射为首页现有轮播 UI 使用的 `image_url`、`link_url` 数据形状。
  - 首页轮播展示条件改为“存在有效手机端广告即展示”，不再依赖旧首页聚合数据中的 `banners_enabled`。
  - `HomepageBanner.id` 类型扩展为 `string | number`，兼容后端广告 ID。
- 验证记录：
  - `cd mobile && npm run build` 通过。

## 2026-06-04 20:19 HKT 手机端登录注册接口对接

- 完成任务：把新加入的 `mobile` 手机端工程接入当前后端用户登录、注册和基础会话接口。
- 解决问题：手机端原先调用旧的 `/api/auth/*` 接口，并按 `access_token/refresh_token` 读取登录结果；当前后端真实接口位于 `/api/user/*`，登录返回 `token/user`，导致手机端无法完成注册、登录和登录后当前用户读取。
- 具体实现：
  - 后端新增公开接口 `GET /api/user/register-options`，返回 `usernameEnabled`、`emailEnabled` 和 `agentInviteRequired`，供手机端注册页按后台配置展示入口。
  - OpenAPI 文档同步新增“注册配置”接口，并补充公开接口不需要 Bearer Token 的测试断言。
  - 移动端新增 `mobile/src/api/user.ts`，集中封装统一响应信封解析、注册配置、登录、注册、当前用户、绑邮箱、改密、找回密码和手机端站点配置读取。
  - 移动端鉴权 store 改为单 token 会话保存，持久化 `access_token` 和当前用户摘要；401 时清理本地会话并回到登录页。
  - 登录页改为调用 `/api/user/login` 和 `/api/user/register`，字段使用 `loginKey`、`inviteCode` 等后端 `camelCase` 契约；邮箱注册不再调用旧验证码接口。
  - 登录页品牌信息改为读取 `/api/user/mobile/site-config`，使用后台配置的平台名称、Logo 和介绍。
  - 首页、彩种列表、投注页、历史页、个人中心、提现页、安全中心和合买创建余额读取统一改用 `/api/user/me` 的适配结果。
  - 安全中心的绑邮箱和修改密码改为对接 `/api/user/bind-email`、`/api/user/password/change`；找回密码页改为按后端当前重置令牌流程调用 `/api/user/forgot-password` 和 `/api/user/reset-password`。
- 验证记录：
  - `cd mobile && npm run build` 通过。
  - `cargo fmt --manifest-path backend/Cargo.toml` 已执行。
  - `cargo check --manifest-path backend/Cargo.toml` 通过。
  - `cargo test --manifest-path backend/Cargo.toml openapi_document -- --nocapture` 通过。
  - `cargo test --manifest-path backend/Cargo.toml -- --nocapture` 通过，171 条测试全部成功；仍有 4 个既有 `LotteryCategory` 未使用导入 warning。
  - 使用本地后端 `PORT=18130` 和用户提供的 PostgreSQL 连接验证：`/api/health`、`/api/user/register-options`、`/api/user/mobile/site-config` 和 OpenAPI 注册配置路径均返回成功。
  - 使用唯一测试账号 `mobiletest_75509722` 完成用户名注册、登录和 `/api/user/me` 查询，登录用户 ID 一致，返回随机邀请码 `5CXLVLXC`。
  - 启动移动端 dev server `http://127.0.0.1:5210/`，`/login` 返回 HTTP 200；当前环境未暴露 in-app Browser 工具，因此未做截图级验证。
- 发现的残留问题：
  - 本地后端连接外部 PostgreSQL 启动时，开奖调度器持续输出“开奖调度器历史记录写入失败 error=内部错误：开奖调度历史数据保存失败”。本次未修改调度持久化，需要后续单独排查数据库调度历史写入失败原因。

## 2026-06-04 19:48 HKT Docker 数据库连接串错误提示优化

- 完成任务：把 Docker 后端启动时 `DATABASE_URL` 格式错误导致的 `RelativeUrlWithoutBase` 改成明确中文配置错误。
- 解决问题：用户在镜像启动日志中看到 `Error: Configuration(RelativeUrlWithoutBase)`，无法直接判断是数据库连接串缺少 `postgres://` 或 `postgresql://` 前缀。
- 具体实现：
  - 后端新增 `DATABASE_URL` 读取与格式校验，非空时必须以 `postgres://` 或 `postgresql://` 开头。
  - 空 `DATABASE_URL` 继续视为未配置，使用内存演示仓储。
  - 主入口调整启动顺序：先初始化路由和数据库依赖，再绑定端口并打印“后台接口服务已开始监听”。
  - 部署规范和 `架构设计.md` 同步记录 `DATABASE_URL` 格式契约。
- 验证记录：
  - `cd backend && cargo fmt` 已执行。
  - `cd backend && cargo fmt --check` 通过。
  - `cd backend && cargo check` 通过。
  - `cd backend && cargo test database_url -- --nocapture` 通过；仍有 4 个既有 `LotteryCategory` 未使用导入 warning。
  - `docker build -t bc-platform:latest .` 通过。
  - 使用错误 `DATABASE_URL=root:123456@192.168.2.3:15432/postgres` 启动临时容器，容器按预期退出，日志输出中文错误“DATABASE_URL 配置无效：必须以 postgres:// 或 postgresql:// 开头”。
  - 错误连接串场景下不再提前输出“后台接口服务已开始监听”，避免误判后端已经成功监听。
  - 使用新镜像启动正常临时容器，`/api/health` 返回 `success=true`，容器状态为 `running healthy`。

## 2026-06-04 19:08 HKT Docker 镜像后端 502 修复

- 完成任务：修复单镜像部署时后端失败但 Nginx 继续运行导致接口 502 的问题。
- 解决问题：此前入口脚本启动后端后立即启动 Nginx，不等待后端健康，也不监控后端进程；当数据库连接、迁移或后端初始化失败时，容器仍然对外服务前端静态页，接口请求会表现为 502。
- 具体实现：
  - `docker/entrypoint.sh` 新增后端健康检查等待逻辑，通过 `http://127.0.0.1:${BACKEND_PORT}/api/health` 后才启动 Nginx。
  - 新增 `BACKEND_STARTUP_TIMEOUT_SECONDS`，默认 60 秒，且启动时校验必须为数字。
  - Nginx 启动后持续监控后端和 Nginx 两个进程；后端退出会关闭 Nginx 并让容器失败退出。
  - `Dockerfile` 新增 `BACKEND_STARTUP_TIMEOUT_SECONDS=60`，并把 Docker healthcheck `start-period` 调整为 60 秒。
  - `.trellis/spec/backend/deployment-guidelines.md` 和 `架构设计.md` 已同步容器启动契约。
- 验证记录：
  - `sh -n docker/entrypoint.sh` 通过。
  - `docker build -t bc-platform:latest .` 通过。
  - 使用新镜像启动临时容器 `bc-502-smoke`，`curl http://127.0.0.1:18082/api/health` 返回 `success=true`。
  - 临时容器首页 `curl -I http://127.0.0.1:18082/` 返回 200，容器状态为 `running healthy`。
  - 使用错误 `DATABASE_URL` 启动临时容器 `bc-502-fail`，容器按预期退出，日志显示“后端服务启动失败，退出码：1”，不再留下 Nginx 返回 502。

## 2026-06-04 18:02 HKT 手机端平台名称配置

- 完成任务：补齐手机端设置中的平台名称配置。
- 解决问题：此前手机端配置只有 Logo 和介绍，缺少手机端页面展示所需的平台名称。
- 具体实现：
  - 后端 `seed_settings()` 新增 `mobile_platform_name` 默认配置，已有数据库启动时会自动补齐。
  - 手机端公开接口 `GET /api/user/mobile/site-config` 新增 `platformName` 字段。
  - 管理后台“手机端设置”Tab 新增“平台名称”输入与保存按钮。
  - OpenAPI 文档的手机端站点配置说明补充平台名称。
  - `架构设计.md` 同步更新手机端配置字段清单和验收标准。
- 验证记录：
  - `cd backend && cargo fmt` 已执行。
  - `cd backend && cargo check` 通过。
  - `cd backend && cargo fmt --check` 通过。
  - `cd backend && cargo test mobile_site_config -- --nocapture` 通过；仍有 4 个既有 `LotteryCategory` 未使用导入 warning。
  - `cd backend && cargo test openapi -- --nocapture` 通过；OpenAPI 路径测试仍通过。
  - `cd admin && npm run build` 通过；Vite 仍提示既有 chunk 体积超过 500kB。
  - `git diff --check` 通过。

## 2026-06-04 17:28 HKT 手机端 Logo 与介绍配置

- 完成任务：在系统设置中新增手机端 Logo 图片和站点介绍配置。
- 解决问题：此前后台没有地方维护手机端基础品牌展示信息，手机端也没有公开接口读取 Logo 和介绍。
- 具体实现：
  - 后端 `seed_settings()` 新增 `mobile_logo_image_url` 和 `mobile_site_intro`，已有数据库启动时会自动补齐缺失配置。
  - 新增手机端公开接口 `GET /api/user/mobile/site-config`，返回 `logoImageUrl` 和 `intro`。
  - OpenAPI 文档新增“手机端站点配置”接口记录。
  - 管理后台系统设置新增“手机端设置”Tab，Logo 使用公共图床上传组件，介绍使用 Semi UI `Input` 编辑保存。
  - 未配置 Logo 使用“未配置”占位，手机端接口会把该占位转换为空值。
- 验证记录：
  - `cd backend && cargo fmt` 已执行。
  - `cd backend && cargo check` 通过。
  - `cd backend && cargo fmt --check` 通过。
  - `cd backend && cargo test openapi -- --nocapture` 通过；仍有 4 个既有 `LotteryCategory` 未使用导入 warning。
  - `cd backend && cargo test mobile_site_config -- --nocapture` 通过；覆盖未配置 Logo 隐藏和真实 Logo 链接返回。
  - `cd admin && npm run build` 通过；Vite 仍提示既有 chunk 体积超过 500kB。
  - `git diff --check` 通过。

## 2026-06-04 16:49 HKT 系统设置 Tabs 分类优化

- 完成任务：把系统设置页改为按功能分类的 Semi UI `Tabs` 展示。
- 解决问题：此前系统设置把图床、充值、注册安全、返利和基础配置纵向堆叠在同一页，配置项多时扫描和维护不够清晰。
- 具体实现：
  - 系统设置配置项继续按功能分组，但展示方式从多组卡片改为 `Tabs.TabPane`。
  - “注册配置”移动到“注册与安全”Tab 内。
  - “图床上传测试”移动到“图床设置”Tab 内。
  - 保留配置搜索，搜索结果会按命中的功能 Tab 显示。
  - 配置项列表抽成 `SettingFields`，注册配置抽成 `RegistrationSettingsPanel`，图床测试抽成 `ImageBedTestPanel`。
  - 系统设置作为一级菜单进入时使用独立页头，不再显示用户、管理员、角色维护入口和用户权限指标卡。
- 验证记录：
  - `cd admin && npm run build` 通过；Vite 仍提示既有 chunk 体积超过 500kB。

## 2026-06-04 16:29 HKT 彩虹易支付与客服直充

- 完成任务：新增用户充值体系，支持后台配置彩虹易支付和客服直充。
- 解决问题：此前用户端没有充值配置、充值下单、支付通知入账和客服直充聊天流程，后台财务也没有充值订单查看入口。
- 具体实现：
  - 后端新增充值领域模型、充值仓储和充值订单持久化表 `recharge_orders`，并补充 `recharge_runtime` 保存充值订单序号。
  - 系统设置新增充值上下限、彩虹易支付网关、商户号、密钥、通知/返回地址、支付方式、客服直充开关和客服直充文案。
  - 用户端新增充值配置、充值订单列表、创建充值订单接口；客服直充创建订单后同步创建客服会话。
  - 用户端新增客服会话列表、会话详情和发送消息接口，用户只能访问自己的客服会话。
  - 彩虹易支付通知支持 GET 和 POST 表单回调，验签成功且金额一致后写入 `rechargeCredit` 资金流水并给用户余额入账。
  - 后台财务管理支持对客服直充订单执行“确认入账”，确认后写入充值流水并增加用户余额。
  - 后台财务管理新增充值订单表，资金流水新增“充值入账”类型。
  - OpenAPI 新增后台充值订单、用户端充值、充值回调和用户端客服接口说明。
  - `.trellis/spec/backend/api-contracts.md` 补充用户端充值与客服直充接口契约。
  - `.trellis/spec/backend/database-guidelines.md` 补充充值订单数据库持久化契约。
- 验证记录：
  - `cd backend && cargo fmt` 已执行。
  - `cd backend && cargo check` 通过。
  - `cd backend && cargo fmt --check` 通过。
  - `cd backend && cargo test -- --nocapture` 通过，166 个测试全部通过；仍有 4 个既有 `LotteryCategory` 未使用导入 warning。
  - `cd admin && npm run build` 通过；Vite 仍提示既有 chunk 体积超过 500kB。
  - 使用 `PORT=18162 DATABASE_URL=postgres://root:123456@192.168.2.3:15432/postgres cargo run` 启动本地后端成功，确认新迁移随服务启动执行。
  - PostgreSQL 冒烟：注册测试用户、读取充值配置、创建客服直充订单、查看用户客服会话、发送用户客服消息、后台查询充值订单、后台确认客服直充入账、用户余额和充值流水检查均成功。
  - 冒烟结束后已删除测试用户 `smoke_recharge_1640`、测试充值单 `R000000000002` 和测试客服会话 `CS-RCH-R000000000002`。

## 2026-06-04 16:07 HKT 广告图长方形上传预览

- 完成任务：优化广告管理的广告图片上传区域，改成长方形横幅预览。
- 解决问题：此前广告图片上传复用头像式方形预览，不符合手机端轮播广告的横幅图片形态。
- 具体实现：
  - 公共图片上传组件 `ImageUploadAvatar` 新增 `previewShape="banner"` 横幅预览模式。
  - 广告管理 SideSheet 的“广告图片”字段启用横幅模式，上传前后都显示长方形区域。
  - 彩种 LOGO 的 `uploadAdd` 模式保持不变。
- 验证记录：
  - `cd admin && npm run build` 通过；Vite 仍提示既有 chunk 体积超过 500kB。
  - 当前会话没有可调用的浏览器工具，已通过构建产物确认横幅预览样式和广告页 `previewShape="banner"` 已生效。

## 2026-06-04 14:36 HKT 彩种默认停售与合买关闭

- 完成任务：调整彩种 SQL 和后端种子默认值，让所有默认彩种都是停售状态，并且默认关闭合买。
- 解决问题：此前部分默认彩种初始化后就是开售且合买开启，可能导致调度器或运营流程在未配置前就开始处理彩种。
- 具体实现：
  - 后端 `seed_lotteries()` 默认 `saleEnabled=false`。
  - 后端默认 `groupBuy.enabled=false`，保留合买阈值参数用于后台开启后的默认值。
  - 新增迁移 `backend/migrations/20260605005000_default_lotteries_closed.sql`，设置 `lotteries.sale_enabled` 和 `lotteries.group_buy` 的 SQL 默认值，并将已有彩种统一改为停售和关闭合买。
  - 调整合买和调度相关测试，测试需要开售或开启合买时显式设置前置状态。
- 验证记录：
  - `cd backend && cargo fmt --check` 通过。
  - `cd backend && cargo check` 通过。
  - `cd backend && cargo test -- --nocapture` 通过，159 个测试全部通过；仍有 4 个既有 `LotteryCategory` 未使用导入 warning。
  - 使用 `DATABASE_URL=postgres://root:123456@192.168.2.3:15432/postgres PORT=18158 cargo run` 启动后端成功，说明迁移已随服务启动执行。
  - 通过后台 API 登录并查询 `/api/admin/lotteries`，确认当前 PostgreSQL 中 22 个彩种 `saleEnabled=true` 数量为 0，`groupBuy.enabled=true` 数量为 0。

## 2026-06-04 14:25 HKT 广告管理与手机端轮播接口

- 完成任务：新增后台“广告管理”模块，并补齐手机端轮播广告公开接口。
- 解决问题：此前后台没有地方维护手机端首页轮播广告，手机端也没有可读取当前广告配置的接口。
- 具体实现：
  - 后端新增广告领域模型和 `AdvertisementRepository`，支持内存模式与 PostgreSQL 持久化模式。
  - 新增数据库迁移 `backend/migrations/20260605004000_create_advertisements.sql`，创建 `advertisements` 表，并为表、字段和约束补齐中文注释。
  - 后台新增 `GET/POST /api/admin/advertisements`、`GET/PUT/DELETE /api/admin/advertisements/{id}`，使用 `systemSettings` 权限控制。
  - 用户端新增公开接口 `GET /api/user/mobile/advertisements`，只返回启用、未过期且已到开始时间的手机端轮播广告。
  - 管理后台新增 `广告管理` 页面，支持列表、新增、编辑、删除、启停、排序、展示时间和公共图床上传轮播图。
  - OpenAPI 新增“广告管理”和“用户端内容”标签，并补充后台广告 CRUD 与用户端轮播读取接口。
- 验证记录：
  - `cd backend && cargo fmt && cargo fmt --check` 通过。
  - `cd backend && cargo check` 通过。
  - `cd backend && cargo test advertisement -- --nocapture` 通过，覆盖广告创建、更新、删除、启用筛选和时间窗口校验。
  - `cd backend && cargo test openapi -- --nocapture` 通过，OpenAPI 已包含后台广告和用户端轮播接口。
  - `cd backend && cargo test -- --nocapture` 通过，158 个测试全部通过；仍有 4 个既有 `LotteryCategory` 未使用导入 warning。
  - `cd admin && npm run build` 通过；Vite 仍提示既有 chunk 体积超过 500kB。
  - 使用 `DATABASE_URL=postgres://root:123456@192.168.2.3:15432/postgres PORT=18157 cargo run` 启动后端，确认 `_sqlx_migrations` 已执行 `20260605004000 create advertisements`。
  - API 冒烟：登录后台后创建测试广告 `AD000001`，`GET /api/user/mobile/advertisements` 能返回该广告；删除后用户端接口不再返回该广告。
  - 数据库检查：`advertisements` 表已创建，表注释存在，11 个字段均有中文注释；测试广告已删除，当前广告表为空。

## 2026-06-04 14:11 HKT 移除停用 API68 北京快乐8

- 完成任务：删除 API68 北京快乐8（`bjkl8`）的默认彩种、默认开奖源和后台开奖源预设。
- 解决问题：北京快乐8已经确认不再使用，如果继续保留会导致后台仍可误配置 `api68-bjkl8`，调度器也可能继续对该彩种生成无效期号。
- 具体实现：
  - 后端 `seed_lotteries()` 移除 `bjkl8`，默认种子彩种数量从 23 调整为 22。
  - 后端 `extra_api68_draw_sources()` 移除 `api68-bjkl8` 默认开奖源。
  - 管理后台“开奖源预设”删除北京快乐8采集入口。
  - 新增迁移 `backend/migrations/20260605003000_remove_deprecated_bjkl8_lottery.sql`，清理已落库的北京快乐8彩种、开奖源、开奖期号、控制号码、机器人绑定和合买计划；历史订单、结算和资金流水不删除。
- 验证记录：
  - `cd backend && cargo fmt && cargo fmt --check` 通过。
  - `cd backend && cargo check` 通过。
  - `cd backend && cargo test seeded -- --nocapture` 通过，覆盖默认彩种数量、默认开奖源和共用开奖源测试。
  - `cd backend && cargo test -- --nocapture` 通过，155 个测试全部通过；仍有 4 个既有 `LotteryCategory` 未使用导入 warning。
  - `cd admin && npm run build` 通过；Vite 仍提示既有 chunk 体积超过 500kB。
  - 使用 `DATABASE_URL=postgres://root:123456@192.168.2.3:15432/postgres PORT=18156 cargo run` 启动后端，确认 `_sqlx_migrations` 已执行 `20260605003000 remove deprecated bjkl8 lottery`；数据库中 `bjkl8` 和 `api68-bjkl8` 查询结果均为空，当前彩种数量为 22、开奖源数量为 19。
  - 本地启动验证时不再出现 `bjkl8` 的期号生成冲突日志；仍观察到既有“开奖调度器历史记录写入失败”，该问题与本次删除北京快乐8无关，后续单独排查。

## 2026-06-04 13:59 HKT 移除停用 API68 快3彩种

- 完成任务：删除 API68 安徽快3、北京快3、福建快3、广西快3、河北快3、湖北快3、吉林快3、江苏快3、内蒙古快3的默认彩种、默认开奖源和后台开奖源预设。
- 解决问题：上述快3 API 已不可用，如果继续保留会导致后台误配置失效采集源，调度器也可能继续尝试无效彩种。
- 具体实现：
  - 后端 `seed_lotteries()` 移除 9 个快3彩种，默认种子彩种数量从 32 调整为 23。
  - 后端 `extra_api68_draw_sources()` 移除对应 `api68-*k3` 开奖源，并删除不再使用的 API68 快3 endpoint 常量。
  - 管理后台“开奖源预设”删除对应快3采集入口。
  - 新增迁移 `backend/migrations/20260605002000_remove_deprecated_fast_three_lotteries.sql`，清理已落库的彩种、开奖源、开奖期号、控制号码、机器人绑定和合买计划；历史订单、结算和资金流水不删除。
- 验证记录：
  - `cd backend && cargo fmt && cargo fmt --check` 通过。
  - `cd backend && cargo check` 通过。
  - `cd backend && cargo test seeded -- --nocapture` 通过，覆盖默认彩种数量、默认开奖源和共用开奖源测试。
  - `cd backend && cargo test -- --nocapture` 通过，155 个测试全部通过；仍有 4 个既有 `LotteryCategory` 未使用导入 warning。
  - `cd admin && npm run build` 通过；Vite 仍提示既有 chunk 体积超过 500kB。
  - 使用 `DATABASE_URL=postgres://root:123456@192.168.2.3:15432/postgres PORT=18155 cargo run` 启动后端，确认 `_sqlx_migrations` 已执行 `20260605002000 remove deprecated fast three lotteries`；数据库中 9 个快3彩种和 9 个 `api68-*k3` 开奖源查询结果均为空，当前彩种数量为 23。

## 2026-06-04 13:18 HKT 错误日志保留原始英文详情

- 完成任务：调整后端错误日志规则，保留错误详情原文，不再因为包含英文就输出“错误详情已按中文日志规则隐藏”。
- 解决问题：调度器、数据库或第三方接口出错时，日志只显示“资源冲突：错误详情已按中文日志规则隐藏”，无法判断实际失败原因。
- 具体实现：
  - `ApiError::log_message()` 改为输出中文错误前缀加原始详情。
  - 彩种数据库、枚举和 JSON 转换日志的结构化 `error` 字段改为记录真实 `sqlx` / `serde_json` 错误。
  - `.trellis/spec/backend/logging-guidelines.md` 明确日志 message 必须中文，但错误字段可保留英文原始详情用于排障。
- 验证记录：
  - `cd backend && cargo fmt && cargo fmt --check` 通过。
  - `cd backend && cargo check` 通过。
  - `cd backend && cargo test api_error_log_message -- --nocapture` 通过；测试编译仍提示 4 个既有 `LotteryCategory` 未使用导入 warning。

## 2026-06-04 13:05 HKT 修复新增彩种数据库号码类型约束

- 完成任务：新增数据库迁移更新 `lotteries_number_type_check`，允许 `pk10`、`elevenFive`、`fastThree`、`luckTwenty` 写入 `lotteries.number_type`。
- 解决问题：服务连接 PostgreSQL 启动时，新增 API68 彩种种子插入会被旧约束拦截，日志表现为“彩种数据库操作失败”，服务直接启动失败。
- 具体实现：
  - 新增迁移 `backend/migrations/20260605001000_update_lottery_number_type_check.sql`，先删除旧约束，再写入包含 6 个号码类型的新约束。
  - 为新约束补充 SQL 注释，说明该约束限制系统支持的号码类型枚举。
  - 后端补充号码类型落库名称测试，避免新增号码类型后忘记同步数据库约束。
- 验证记录：
  - `cd backend && cargo check` 通过。
  - 使用 `DATABASE_URL=postgres://root:123456@192.168.2.3:15432/postgres PORT=18141 cargo run` 本地启动成功，不再出现“彩种数据库操作失败”。
  - 已确认远程 PostgreSQL `_sqlx_migrations` 记录 `20260605001000 update lottery number type check` 成功，`lotteries` 已补齐 32 个彩种。

## 2026-06-04 12:47 HKT API68 批量彩种接入

- 完成任务：按用户提供的 API68 接口批量新增北京PK10、天津时时彩、新疆时时彩、广东11选5、江苏快3、澳洲幸运10、澳洲幸运20、北京快乐8、各省 11 选 5、各省快3等彩种，并为这些彩种补齐默认开奖源配置。
- 解决问题：此前系统只支持少量 3 位/5 位彩种，新提供的 PK10、11选5、快3、快乐8/幸运20 接口无法在后台彩种管理、开奖源配置、期号调度和彩种控制台中正确落地。
- 具体实现：
  - 后端 `LotteryNumberType` 新增 `pk10`、`elevenFive`、`fastThree`、`luckTwenty`，并按号码类型校验开奖号码长度、范围和是否去重。
  - `seed_lotteries()` 新增 26 个 API68 彩种，内存种子总数更新为 32；PostgreSQL 启动时会补齐缺失彩种，不覆盖已有同 ID 彩种。
  - `draw_sources` 默认源新增本次 API68 批量彩种来源；已有数据库启动时会补齐缺失默认源，不覆盖已绑定彩种的现有来源。
  - API68 解析器兼容 `result.data` 数组和单对象响应，并读取单对象响应中的 `drawIssue`、`drawTime` 作为下一期锚点。
  - 澳洲幸运5默认 endpoint 更新为 `https://api.api68.com/CQShiCai/getBaseCQShiCai.do`。
  - 管理后台新增共享号码类型工具，彩种管理、开奖期号、开奖源预设、彩种控制台和概览页均能正确展示新增号码类型。
  - PK10、11选5、快3、快乐8/幸运20 当前先接开奖采集、期号调度和控制号码；投注玩法暂不伪造，后续补玩法规则时再扩展。
- 验证记录：
  - `cd backend && cargo fmt && cargo fmt --check` 通过。
  - `cd backend && cargo check` 通过。
  - `cd backend && cargo test api68 -- --nocapture` 通过，覆盖 API68 数组/单对象响应解析和新增默认开奖源。
  - `cd backend && cargo test normalize_draw_number_supports_new_lottery_number_types -- --nocapture` 通过。
  - `cd backend && cargo test seeded_lotteries_include_requested_api68_lotteries -- --nocapture` 通过。
  - `cd backend && cargo test -- --nocapture` 通过，154 个测试全部通过；仍有 4 个既有 `LotteryCategory` 未使用导入 warning。
  - `cd admin && npm run build` 通过；Vite 仍提示已有 chunk 体积超过 500kB。

## 2026-06-04 12:11 HKT 后端接入 OpenAPI 文档能力

- 完成任务：新增后端 OpenAPI 文档入口和 Swagger UI 页面，方便接口联调与后续移动端/前端按统一契约开发。
- 解决问题：此前项目没有可访问的 OpenAPI 规范，接口路径、鉴权方式和请求体只能从代码中查找；本次把当前健康检查、管理后台和用户端接口整理为可读取的 OpenAPI 3.1 文档。
- 具体实现：
  - 新增 `GET /api/openapi.json`，返回 OpenAPI JSON。
  - 新增 `GET /api/docs`，返回 Swagger UI 页面并指向 `/api/openapi.json`。
  - OpenAPI 文档按中文模块标签分组，受保护接口统一声明 `bearerAuth`。
  - 新增 `backend/src/routes/openapi.rs`，并为文档生成、路径参数、请求体、响应体、安全方案等方法补充中文注释。
- 验证记录：
  - `cargo fmt` 已执行。
  - `cargo fmt --check` 通过。
  - `cargo check` 通过。
  - `cargo test openapi -- --nocapture` 通过，覆盖核心路径、安全方案、Swagger UI 指向和路径参数提取。
  - `cargo test -- --nocapture` 通过，后端 150 个测试全部通过；测试编译仍提示 4 个既有 `LotteryCategory` 未使用导入 warning。
  - 使用 `PORT=18132 DATABASE_URL=postgres://root:123456@192.168.2.3:15432/postgres cargo run` 启动本地后端后，`curl http://127.0.0.1:18132/api/openapi.json` 返回 OpenAPI JSON，`curl -I http://127.0.0.1:18132/api/docs` 返回 `200 OK`。
  - 本地启动过程中调度器仍打印“开奖调度器历史记录写入失败”，该日志与 OpenAPI 文档入口无关，后续可单独排查调度历史表/数据库状态。

## 2026-06-04 11:57 HKT 邀请码改为字母数字且保证唯一

- 完成任务：将用户邀请码自动生成规则从纯大写字母调整为 8 位大写字母数字组合，并继续保证每个用户的邀请码唯一。
- 解决问题：此前自动邀请码只会生成纯字母，和“随机字母加数字”的最新要求不一致；邀请关系种子也引用旧代理示例码，空库演示数据容易继续出现旧格式。
- 具体实现：
  - 自动生成字符集改为 `A-Z + 0-9`。
  - 生成结果必须同时包含大写字母和数字，避免生成纯字母或纯数字的邀请码。
  - 生成时检查现有用户集合，遇到重复会重新生成；用户保存时继续执行唯一性校验。
  - 种子用户邀请码更新为包含数字的固定示例码，并同步邀请关系种子的代理邀请码。
- 验证记录：
  - `cargo fmt` 已执行。
  - `cargo fmt --check` 通过。
  - `cargo check` 通过。
  - `cargo test invite_code -- --nocapture` 通过，覆盖种子码格式、自动生成唯一邀请码、重复邀请码拒绝和普通用户邀请码无效。
  - `cargo test -- --nocapture` 通过，后端 146 个测试全部通过；测试编译仍提示 4 个既有 `LotteryCategory` 未使用导入 warning。

## 2026-06-04 11:45 HKT 机器人配置改为 SideSheet

- 完成任务：将“机器人配置”里的新增和编辑维护表单从页面右侧常驻卡片改为 Semi UI `SideSheet` 抽屉。
- 解决问题：此前机器人列表和配置维护表单同屏堆叠，占用列表扫描空间；现在只在新增或编辑时打开抽屉。
- 具体实现：
  - 页面顶部新增“新增配置”按钮，点击后按当前机器人类型初始化表单并打开“新增机器人配置”抽屉。
  - 点击机器人名称或列表“编辑”按钮时加载该机器人数据并打开“编辑机器人配置”抽屉。
  - 保存成功或删除成功后自动关闭抽屉，并继续刷新工作台概览。
  - 切换外层机器人模块时关闭已打开抽屉，避免编辑状态残留。
- 验证记录：
  - `cd admin && npm run build` 通过。
  - 浏览器验证 `http://127.0.0.1:5176/`：进入“机器人配置”时没有常驻 `.semi-sidesheet`。
  - 浏览器验证点击“新增配置”后打开“新增机器人配置” SideSheet，点击列表“编辑”后打开“编辑机器人配置” SideSheet。

## 2026-06-04 11:33 HKT 彩种新增编辑改为 SideSheet

- 完成任务：将“彩种管理”里的新增彩种和编辑彩种表单从页面右侧常驻卡片改为 Semi UI `SideSheet` 抽屉。
- 解决问题：此前彩种列表和新增/编辑表单同屏堆叠，占用列表扫描空间；运营只想维护某个彩种时再打开表单。
- 具体实现：
  - 点击顶部“新增彩种”按钮时清空表单并打开“新增彩种”抽屉。
  - 点击列表中的彩种名称或“编辑”按钮时加载该彩种数据并打开“编辑彩种”抽屉。
  - 保存成功或删除成功后自动关闭抽屉，并继续刷新工作台概览。
  - 主页面保留彩种列表、快速改分类、分类管理、玩法配置和刷新入口。
- 验证记录：
  - `cd admin && npm run build` 通过。
  - 浏览器验证 `http://127.0.0.1:5176/`：进入“彩种管理”时没有常驻 `.semi-sidesheet`。
  - 浏览器验证点击“新增彩种”后打开“新增彩种” SideSheet，点击列表“编辑”后打开“编辑彩种” SideSheet。

## 2026-06-04 11:29 HKT Semi Input 与 Select 尺寸对齐

- 完成任务：修正全局 `.semi-input-wrapper.form-input` 样式，让 Semi `Input` 的高度和左右内边距与 Semi `Select` 保持一致。
- 解决问题：此前 Semi `Input` wrapper 被兼容样式压到 32px，高度低于 `Select` 的 40px，并且 wrapper 左右 padding 为 0，导致同一表单里输入框和下拉框不齐。
- 具体实现：
  - `.semi-input-wrapper.form-input` 调整为 `min-height: 40px`、`display: flex`、`align-items: center`、`padding: 0 10px`。
  - `.semi-input-wrapper.form-input .semi-input` 调整为 `height: 20px`、`line-height: 20px`、`padding: 0`，让文字区域与 Select 文本行高一致。
- 验证记录：
  - `cd admin && npm run build` 通过。
  - 浏览器验证系统设置页：`.semi-input-wrapper.form-input` 与 `.semi-select.form-input` 高度均为 40px，左右 padding 均为 10px。

## 2026-06-04 11:24 HKT 系统设置枚举项改为下拉框

- 完成任务：将“系统设置”中“注册与安全”和“返利设置”的枚举配置改为 Semi UI `Select` 下拉框。
- 解决问题：`email_registration_enabled` 和 `recharge_rebate_mode` 原来使用普通文本输入，运营容易填入非标准值；现在只能从明确选项中选择。
- 具体实现：
  - `email_registration_enabled` 提供“开启邮箱注册 / 关闭邮箱注册”两个选项，保存值仍为 `true / false`。
  - `recharge_rebate_mode` 提供“立即返利 / 充值阶梯返利”两个选项，保存值仍为 `immediate / rechargeTiered`。
  - 若数据库已有历史非标准值，页面会追加“当前值”选项用于展示，避免打开页面时丢失现有值。
- 验证记录：
  - `cd admin && npm run build` 通过。
  - 浏览器验证 `http://127.0.0.1:5176/`：“注册与安全”和“返利设置”共渲染 2 个 `.semi-select.form-input`。
  - 浏览器验证邮箱注册下拉包含“开启邮箱注册 / 关闭邮箱注册”，返利模式下拉包含“立即返利 / 充值阶梯返利”。

## 2026-06-04 11:14 HKT 全局文本输入统一为 Semi Input

- 完成任务：将管理后台内所有文本类、数字类、密码类原生 `<input>` 统一替换为 `@douyinfe/semi-ui` 的 `Input` 组件。
- 解决问题：此前后台页面虽然下拉框已统一为 Semi UI，但文本输入仍大量使用原生 `<input>`，导致输入框样式、交互和回调语义不一致。
- 具体实现：
  - 覆盖页面：
    - `AccessManagementPage`
    - `DrawManagementPage`
    - `FinanceManagementPage`
    - `GroupBuyManagementPage`
    - `InviteManagementPage`
    - `LoginPage`
    - `LotteryConsolePage`
    - `LotteryManagementPage`
    - `OrderManagementPage`
    - `PlayRulesPage`
    - `RebateManagementPage`
    - `RobotManagementPage`
  - 为相关页面引入 `import { Input } from '@douyinfe/semi-ui';`。
  - 将 Semi `Input` 的 `onChange(value)` 回调适配到原有表单状态更新逻辑。
  - `admin/src/index.css` 为 `.semi-input-wrapper.form-input` 增加兼容样式，清除旧原生 `.form-input` 叠加到 Semi wrapper 的 `padding` 与 `min-height`，避免输入框高度和内边距异常。
  - 保留 checkbox 类型原生 `<input>`，因为其不属于 Semi `Input` 文本输入组件范围。
- 验证记录：
  - `cd admin && npm run build` 通过。
  - `rg -n "<input\\b" admin/src` 剩余项均为 `type="checkbox"`。
  - 浏览器验证 `http://127.0.0.1:5176/` 的“订单管理”表单，文本输入渲染为 `.semi-input-wrapper / .semi-input`。
  - 浏览器验证 `.semi-input-wrapper.form-input` 的 wrapper `padding-left/right=0px`、`min-height=0px`，内层 `.semi-input` 保留 Semi 默认 `12px` padding。

## 2026-06-04 11:05 HKT 彩种 Logo 上传精简为 semi-upload-add

- 完成任务：将“彩种管理”新增/编辑表单中的 LOGO 上传入口精简为 Semi UI 图片上传的 `semi-upload-add` 样式入口。
- 解决问题：此前彩种 LOGO 上传复用了完整图片上传面板，会显示上传说明、当前文件、清空按钮等额外内容；用户反馈彩种上传 LOGO 只需要显示 `semi-upload-add`。
- 具体实现：
  - `admin/src/components/ImageUploadAvatar.tsx` 增加 `variant="uploadAdd"` 精简模式，内部使用 `Upload listType="picture"` 生成 `semi-upload-add / semi-upload-picture-add` 上传入口。
  - `admin/src/pages/LotteryManagementPage.tsx` 的 LOGO 字段切换到 `uploadAdd` 模式，只保留上传方块；上传成功后仍回填 `form.logoUrl`。
  - 移除彩种表单中 LOGO 下方的“图床上传字段名”只读展示，字段名继续在内部按系统设置使用。
- 验证记录：
  - `cd admin && npm run build` 通过。
  - 浏览器验证 `http://127.0.0.1:5176/`：“彩种管理”中存在 `.lottery-logo-upload .semi-upload-add` 和 `.semi-upload-picture-add`，旧的 LOGO 上传说明文案与“图床上传字段名”不再显示。

## 2026-06-04 10:26 HKT 公共图片上传组件复用

- 完成任务：新增公共图片上传组件 `ImageUploadAvatar`，将系统设置的图床上传测试和彩种编辑的 Logo 上传统一改为复用同一组件。
- 解决问题：此前图床测试和彩种 Logo 上传各自维护 `Upload + Avatar + Toast + IconCamera`、文件预览、上传状态、返回链接提取和错误提示逻辑，后续修改图床上传体验时容易两边行为不一致。
- 具体实现：
  - `admin/src/components/ImageUploadAvatar.tsx` 统一承载图片选择、头像预览、上传中提示、上传结果展示、复制链接、打开图片、清空图片和配置缺失提示。
  - `admin/src/pages/AccessManagementPage.tsx` 的“图床上传测试”改为直接使用公共组件，保留上传地址、字段名、返回链接字段的配置展示。
  - `admin/src/pages/LotteryManagementPage.tsx` 的新增/编辑彩种 Logo 上传改为使用公共组件，上传成功后回填 `form.logoUrl`，保存彩种后持久化。
  - 清理两个页面内重复的文件预览、图片链接提取、上传错误和头像蒙层 helper。
- 验证记录：
  - `cd admin && npm run build` 通过。
  - 浏览器验证 `http://127.0.0.1:5176/`：“彩种管理”可见“点击图片区域上传 LOGO”，“系统设置”可见“点击图片区域选择并测试上传”。

## 2026-06-04 10:18 HKT 彩种分类抽屉化与 Logo 上传组件优化

- 完成任务：优化“彩种管理”页面结构，将“彩种分类管理”从彩种列表上方移出，改为顶部“分类管理”按钮打开独立 `SideSheet` 维护。
- 完成任务：按 Semi UI `Upload + Avatar + Toast + IconCamera` 风格优化新增/编辑彩种里的 Logo 上传组件。
- 解决问题：此前彩种分类管理卡片和彩种列表堆在同一页面区域，运营扫描彩种列表时被分类维护表单干扰；Logo 上传仍是原生文件输入和按钮，不符合前面确定的图床上传组件样式。
- 具体实现：
  - `admin/src/pages/LotteryManagementPage.tsx` 新增分类管理抽屉，分类新增、编辑、删除都在抽屉内完成。
  - 彩种管理顶部新增“分类管理”按钮，主页面保留彩种列表、快速改分类和新增/编辑彩种表单。
  - Logo 上传改为 Semi UI `Upload` 包裹 `Avatar`，hover 显示 `IconCamera` 相机蒙层。
  - `Upload.customRequest` 继续调用后台图床代理 `uploadImageBedFile`，上传成功后自动写入 `form.logoUrl`，并通过 `Toast` 提示成功/失败。
- 验证记录：
  - `cd admin && npm run build` 通过。
  - 浏览器验证 `http://127.0.0.1:5176/` 的“彩种管理”页面：可见“分类管理”按钮，分类配置默认不与列表同屏堆叠；新增/编辑彩种中显示头像式 Logo 上传入口；点击“分类管理”可打开分类维护抽屉。

## 2026-06-04 09:57 HKT 图床上传测试组件头像化优化

- 完成任务：按 Semi UI 示例风格优化“图床上传测试”组件，改为 `Upload + Avatar + Toast + IconCamera` 的图片上传入口。
- 解决问题：原测试组件是通用文件选择/拖拽形态，测试图床时不够直观；现在点击头像式图片区域即可选择并上传，hover 时显示相机图标，成功/失败通过 Toast 即时提示。
- 具体实现：
  - `admin/src/pages/AccessManagementPage.tsx` 引入 Semi UI `Avatar`、`Upload`、`Toast` 和 `@douyinfe/semi-icons` 的 `IconCamera`。
  - 使用 `Upload.customRequest` 继续调用已有 `uploadImageBedFile`，保证上传仍走后台图床代理和数据库中的图床配置。
  - 上传成功后展示图片链接、图片预览、复制链接、打开图片和原始响应折叠区。
  - 配置缺失时展示中文提示，并阻止上传，避免无效请求。
- 验证记录：
  - `cd admin && npm run build` 通过。

## 2026-06-04 01:10 HKT 全项目下拉控件统一 UI 标准

- 完成任务：将管理后台中所有原生 HTML `<select>` 替换为 `@douyinfe/semi-ui` 的 `Select` 组件，确保筛选、状态选择、角色/玩法/彩种下拉等交互在同一 UI 体系下运行。
- 解决问题：此前项目内仍存在部分原生 `select`，与 Semi UI 设计规范和统一交互风格不一致，影响可维护性与可用性一致性。
- 具体实现：
  - 覆盖页面：
    - `admin/src/pages/AccessManagementPage.tsx`
    - `admin/src/pages/DrawManagementPage.tsx`
    - `admin/src/pages/GroupBuyManagementPage.tsx`
    - `admin/src/pages/InviteManagementPage.tsx`
    - `admin/src/pages/LotteryManagementPage.tsx`
    - `admin/src/pages/OrderManagementPage.tsx`
    - `admin/src/pages/PlayRulesPage.tsx`
    - `admin/src/pages/RebateManagementPage.tsx`
    - `admin/src/pages/RobotManagementPage.tsx`
    - `admin/src/pages/SettlementManagementPage.tsx`
    - `admin/src/pages/SupportManagementPage.tsx`
  - 每个页面补充 `Select` 与 `Select.Option` 的导入与使用，不再使用原生 `select` 元素。
  - 将 `onChange` 回调从事件对象改为 `Select` 值回调，统一按 `string`/类型转换处理，避免 `undefined` 与类型兼容问题。
  - 涉及状态值（销售状态、角色、彩种、玩法、优先级、管理员分配等）均已保持原有语义映射。
- 验证记录：
  - 已执行 `cd admin && npm run build`（含 `tsc --noEmit` 与 `vite build`）验证通过。
  - 全局搜索确认 `rg -n "<select|</select>" admin/src` 无匹配结果。

## 2026-06-04 23:58 HKT 彩种分类管理界面补齐

- 完成任务：在“彩种管理”页面补齐“彩种分类”新增/编辑/删除入口，新增分类后直接可在列表与表单中使用；修复分类下拉从静态枚举切换为后端配置数据源。
- 解决问题：用户反馈“没有地方可以编辑添加彩种分类”，之前仅有分类显示而无维护入口，且分类选择是写死常量导致新增分类无法落地。
- 具体实现：
  - 后端：
    - 已有 `GET/POST/PUT/DELETE /api/admin/lottery-categories` 接口接入前端，不再仅依赖前端静态常量。
    - `LotteryKind.category` 继续使用字符串编码，避免固定枚举约束。
  - 前端：
    - `admin/src/api/client.ts` 新增分类配置接口方法：
      - `fetchLotteryCategories`
      - `createLotteryCategory`
      - `updateLotteryCategory`
      - `deleteLotteryCategory`
    - 新增 `admin/src/hooks/useLotteryCategories.ts` 统一管理分类列表与增改删状态。
    - `admin/src/pages/LotteryManagementPage.tsx` 新增“彩种分类管理”区块：
      - 可查看现有分类列表；
      - 可新增分类；
      - 可编辑分类名称；
      - 可删除分类（含保护提示）。
    - 彩种列表快速改分类下拉和表单分类下拉改为使用后端分类数据。
- 验证记录：
  - 代码静态联调后执行 `cd admin && npm run build`，`cd backend && cargo check`。

## 2026-06-04 21:50 HKT 彩种支持上传 Logo 链路

- 完成任务：在彩种管理页补齐“每个彩种可上传 logo”能力，并在列表与编辑页回显图片；并确保彩种 API/仓储/数据库都持久化 `logoUrl`。
- 解决问题：先前彩种管理仅支持文字字段，运营无法直接在后台给每个彩种绑定视觉标识，后续导入到前端卡片或看板时缺少图像信息。
- 具体实现：
  - 后端：
    - `backend/src/domain/lottery.rs` 增加 `LotteryKind.logo_url`，并在 `seed_lotteries` 与测试级构造体里补默认值。
    - `backend/src/services/lottery.rs` 的 `list/get/create/update` SQL 增加 `logo_url` 字段读写。
    - 新增迁移 `backend/migrations/20260604202000_add_lottery_logo_url.sql`，为 `lotteries` 增加 `logo_url TEXT NOT NULL DEFAULT ''`。
    - 新增 `comment` 脚本补齐字段注释。
  - 前端：
    - `admin/src/types/dashboard.ts` 的 `LotteryKind` 增加 `logoUrl`。
    - `admin/src/App.tsx` 传递系统设置到 `LotteryManagementPage`。
    - `admin/src/pages/LotteryManagementPage.tsx` 新增 Logo 显示、文件选择、上传按钮，并复用图床配置 `image_bed_upload_field`。
    - 上传后将返回的图片链接回填到 `form.logoUrl`，并随保存同步下发到后端。
- 验证记录：
  - `cd backend && cargo check` 通过。
  - `cd admin && npm run build` 通过。
- 后续动作：
  - 若需要按 `image_bed_result_url_field` 自定义读取字段，在该页增加可选覆盖输入项；目前复用系统设置默认值。
  - 可继续扩展为“logo 上传预览失败重试和图片链接校验提示”。

## 2026-06-04 23:58 HKT 彩种 Logo 能力本地回归确认

- 完成任务：对“每个彩种可上传 Logo”链路做本地回归，确认后端持久化、前端回显与构建联动已可用。
- 解决问题：上次只做了静态联动，需要再确认字段映射和构建验证无报错。
- 具体动作：
  - 复核 `LotteryKind.logo_url` 在域模型、仓储 SQL、迁移脚本中的读写链路。
  - 复核 `LotteryManagementPage` 列表与编辑页：新增/编辑可选 LOGO 上传、缩略图展示、清空与保存回传。
  - 补充 `架构设计.md` 后续记录，保持需求变更可追溯。
- 验证记录：
  - `cargo check -q`（backend）通过。
  - `cd admin && npm run build` 通过。

## 2026-06-04 23:59 HKT 彩种分类可直接编辑入口补齐

- 完成任务：在彩种管理列表页补充“快速修改分类”入口，避免需要先进入编辑态才能更改分类。
- 解决问题：当前分类虽有下拉框，但位于编辑表单，运维高频操作不够直接，用户反馈“没有地方可以编辑分类”。
- 具体实现：
  - 在“彩种列表”增加“快速改分类”列。
  - 每行展示分类下拉框，直接调用更新接口改写 `category` 并刷新列表。
  - 已选中的彩种在列表与表单内同步更新，避免编辑态显示与列表状态不一致。
- 验证记录：
  - `cd admin && npm run build` 通过。

## 2026-06-05 00:10 HKT 系统设置独立顶级分类

- 完成任务：把“系统设置”从“公共功能”中独立拆分为单独一级分类，在侧边栏中独立展示。
- 解决问题：当前“系统设置”和“用户管理 / 管理员管理 / 角色权限”放在同一分组，难以快速找到配置入口。
- 具体实现：
  - 在后端 `backend/src/services/dashboard.rs` 的 `module_groups()` 中新增独立 `settings` 分组。
  - 将 `settings` 模块从 `common` 分组移除，放到独立分组。
  - 保持 `settings` 权限校验、路由入口、保存与读取逻辑不变。
- 验证记录：
  - `cargo check` 通过。
  - `cargo test` 通过。
  - `cd admin && npm run build` 通过。

## 2026-06-04 23:55 HKT 图床返回链接字段可配置

- 完成任务：图床上传返回为图片链接时，可通过配置项 `image_bed_result_url_field` 指定返回字段路径（如 `links.download`），避免前端拿到不稳定的原始回包。
- 解决问题：此前接口默认将整段回包透传，遇到返回直接为链接时运维无法直接在统一字段里读取；同时不同图床结构差异（如直接返回 `{"file":{"url":...}}`）也导致联调困难。
- 具体实现：
  - `backend/src/services/access.rs` 在系统设置种子中新增 `image_bed_result_url_field`，并给出默认值 `links.download`。
  - `backend/src/routes/admin.rs` 的 `POST /api/admin/image-bed/upload` 新增：
    - 读取 `image_bed_result_url_field`，按“点号路径”从图床响应 JSON 取值；
    - 取值失败时返回中文提示，确保字段缺失可快速定位；
    - 未配置该字段时兼容返回原始响应。
  - `admin/src/pages/AccessManagementPage.tsx` 的图床设置会显示该配置项，运维可直接在系统设置里修改生效。
- 验证记录：
  - `cargo check -q` 通过。

## 2026-06-04 23:24 HKT 系统设置页面体验优化

- 完成任务：优化“系统设置”页面的编辑体验，减少配置查找和编辑负担。
- 解决问题：原有表格需要横向滚动，配置项较多时难以快速定位；缺少搜索能力，配置分类不直观。
- 具体实现：
  - `admin/src/pages/AccessManagementPage.tsx` 的 `SettingsSection` 重构为“分组卡片 + 搜索过滤”方式。
  - 增加 `系统设置` 页面内置分组规则（图床设置 / 注册与安全 / 返利设置 / 基础设置），并增加“搜索配置项/说明”入口。
  - 支持筛选结果为空的友好提示，避免空白区误解。
- 验证记录：
  - `cd admin && npm run build` 通过。

## 2026-06-04 23:08 HKT 图床上传测试前端能力补齐

- 完成任务：在管理员“系统设置”页新增图床上传测试入口，支持选择图片并调用 `POST /api/admin/image-bed/upload` 验证配置。
- 解决问题：当前仅有后台接口和数据库配置但缺少可直接验证链路，运维无法在后台快速确认 `image_bed_*` 配置与供应商联通性，配置回归时也缺少上传结果观测。
- 具体实现：
  - `admin/src/api/client.ts` 新增 `uploadImageBedFile(file, uploadFieldName)`，使用 `multipart/form-data` 直连后台图床上传接口并透传返回。
  - `admin/src/pages/AccessManagementPage.tsx` 在“系统设置”页新增“图床上传测试”卡片：
    - 显示当前生效上传字段名（默认 `file`）；
    - 支持选择本地图片文件；
    - 点击“测试上传”发起请求并展示返回 JSON；
    - 分离本地错误与列表级全局错误提示。
- 验证记录：
  - 已完成代码接入，待本地启动后台和前端进行一次真实图片上传联调验证。

## 2026-06-04 22:12 HKT 图床上传接口配置能力补齐

- 完成任务：新增管理员后台可配置图床上传接口能力，并提供服务端统一代理接口 `POST /api/admin/image-bed/upload`，将前端上传文件转发到数据库配置的第三方图床。
- 解决问题：图床地址/Token/字段名此前写死在环境里，不够可运维；后续更换供应商或账户时不能即时调整，且上传逻辑与权限链路也缺失。
- 技术实现：
  - 后端：
    - `backend/src/services/access.rs` 在系统设置种子中新增三项图床配置：`image_bed_upload_url`、`image_bed_authorization_token`、`image_bed_upload_field`，并在初始化时自动补齐缺失项。
    - `backend/src/services/access.rs` 新增 `get_setting/setting_value/setting_value_optional`，便于按 key 读取运行时配置。
    - `backend/src/routes/admin.rs` 新增常量与路由 `POST /api/admin/image-bed/upload`，并接入 `SystemSettings` 权限；处理 `multipart/form-data` 文件字段、按配置构建上游请求头（`Authorization: Bearer <token>`）与表单字段名后透传响应。
    - `backend/src/routes/admin.rs` 测试中补充 `image-bed/upload` 对应的权限映射断言。
    - `backend/Cargo.toml` 开启 `axum` 的 `multipart` 与 `reqwest` 的 `multipart` 特性。
  - 配置默认值：
    - 上传地址默认 `https://oss.moonight.cc.cd/api/v1/upload`
    - 上传字段默认 `file`
    - Token 默认按你提供的示例值预填（仅示例示范，可在系统设置中更新）。
- 验证结果：
  - `cargo check -q` 通过。
  - `cargo test -q` 通过（144/144）。
  - `cd admin && npm run build` 通过。

## 2026-06-04 11:20 HKT 用户资金流水接口补齐

- 完成任务：补齐用户端“资金流水列表”接口，新增 `GET /api/user/ledger-entries`，用于查询当前登录用户的资金流水。
- 解决问题：当前已有账户余额、提现方式等接口，但缺少用户可见的流水查询能力，前端/移动端无法拉取个人账变明细，难以展示充值、投注扣款、派奖入账等完整账单闭环。
- 技术实现：
  - 在 `backend/src/routes/user.rs` 增加受保护路由 `/ledger-entries`，挂载到登录态鉴权后链路。
  - 新增 handler `list_ledger_entries`，从会话读取 `user.id` 并调用 `state.finance.user_ledger_entries(&session.user.id)`。
  - 在 `backend/src/services/finance.rs` 新增仓储能力 `user_ledger_entries`，并补充 `ledger_entries_for_user` 的内存过滤实现（按当前实现约定倒序返回）。
  - 新增 `FinanceRepository::user_ledger_entries` 的单元测试，覆盖“只返回指定用户流水、过滤其它用户记录”场景。
- 验证记录：本地运行 `cd backend && cargo test` 全量通过。

## 2026-06-04 10:30 HKT 彩种分类编辑能力

- 完成任务：增加“彩种分类（地方/海外/福利/其他）”字段，支持后台彩种管理页直接维护彩种分类。
- 解决问题：之前彩种只有玩法/开奖参数，无法按运营归类或在列表中快速识别同类彩种，造成后台配置与实际分类口径不一致。
- 技术实现：
  - 后端 `LotteryKind` 增加 `category` 枚举字段，数据库 `lotteries` 新增 `category` 列，并新增 `lotteries_category_check` 校验约束。
  - 彩种持久化 SQL（`list/get/create/update/seed`）同步读写 `category` 字段，含迁移与注释。
  - 期号种子和测试级临时构造彩种补齐分类默认值。
  - 管理后台 `LotteryKind` 类型增加 `category`，彩种管理页新增分类下拉（地方/海外/福利/其他），列表增加分类展示。
  - 首次新增和默认表单新增分类默认值为“地方彩种”。
- 验证记录：对 `backend/src/domain/lottery.rs`、`backend/src/services/lottery.rs`、`admin/src/pages/LotteryManagementPage.tsx`、`admin/src/types/dashboard.ts` 做静态编译检查；后续补充数据库迁移与端到端验证。
- 后续动作：如需按分类在“开奖控制台/期号列表”添加筛选条件，可复用 `category` 字段继续扩展。

## 2026-06-03 23:58 HKT 用户接口补齐

- 完成任务：补齐用户相关后端接口链路，支持“用户注册（用户名/邮箱）”“登录（用户名/邮箱）”“绑定邮箱”“修改密码”“忘记密码重置”“查询用户余额”“提现方式（支付宝、微信、银行卡）”完整流程。
- 解决问题：此前用户端接口在多个版本切换中有部分缺口，用户注册策略、登录标识、密码找回、会话鉴权和提现方式维护未形成统一闭环，前端/联调时缺少可复用的后端契约。
- 技术实现：
  - 保持 `backend/src/routes/user.rs` 路由树完整：`/api/user/register`、`/api/user/login`、`/api/user/forgot-password`、`/api/user/reset-password`、`/api/user/me`、`/api/user/logout`、`/api/user/bind-email`、`/api/user/password/change`、`/api/user/balance`、`/api/user/withdrawal-methods`。
  - `backend/src/services/access.rs` 增加并修复用户生命周期方法：注册、登录、会话解析、登出、绑定邮箱、改密、忘记密码、重置密码、提现方式的增删改查。
  - 新增访问仓储单元测试，覆盖：用户名注册、邮箱注册、仅邮箱开启后用户名注册失败、修改密码、忘记密码与重置、提现方式 CRUD。
  - 清理无用常量与告警：移除未使用的用户会话 TTL/Token 长度常量；修复邀请仓储测试无效变量命名告警。
- 验证：在 `DATABASE_URL=postgres://root:123456@192.168.2.3:15432/postgres` 下运行 `cargo test -q`，通过 143 项测试；无新增失败。
- 后续动作：将用户端接口能力同步补充到移动端 SDK/接口说明文档，并在下一步将该模块接入登录态统一拦截与前端状态管理。

## 2026-06-03 23:46 HKT SQL 字段注释补齐

- 完成任务：为数据库迁移 SQL 全量补齐字段级注释，并整理为新迁移 `backend/migrations/20260603234000_add_all_column_comments.sql`。
- 解决问题：项目中历史迁移新增的多个表及字段未统一注释，维护排查数据库时无法快速识别字段语义。
- 技术说明：
  - 读取全部建表迁移 `20260602150315_create_lotteries.sql`、`20260602165000_add_lottery_play_configs.sql`、`20260603143000_create_state_documents.sql`、`20260603152000_create_business_tables.sql`、`20260603180500_add_scheduler_skip_details.sql`。
  - 逐表逐字段补齐 `COMMENT ON TABLE` 与 `COMMENT ON COLUMN`，覆盖彩种、开奖、订单、财务、管理员、客服、调度、合买、机器人等核心表。
  - 补充已存在约束的字段级说明：如 `lotteries`、`registration_config`、`rebate_policy`、`draw_scheduler_config` 的检查约束。
- 当前状态：脚本已重写为纯 `COMMENT ON` 语法，移除重复与非法语句；待执行数据库迁移时将一次性同步注释信息。

## 2026-06-03 21:46 HKT 后台方法注释具体化

- 完成任务：把后端 `backend/src` 的公共方法注释从占位语句统一改为“具体做什么”的中文说明。
- 解决问题：用户反馈“后台方法注释不具体”，此前大量 `执行xxx方法` 占位文本无法体现业务含义，运维和接手同学无法快速理解每个接口功能。
- 技术说明：通过批量脚本与人工复核，对 `services`、`routes`、`app`、`domain`、`response/error` 等模块公共方法做注释重写，覆盖创建、查询、保存、调度、开奖、财务、权限、玩法、调度启动/运行等关键流程。
- 验证记录：运行 `cargo fmt` 后执行 `cargo test -- --nocapture`，后端测试 138 个通过；保留了既有中文错误测试与功能行为校验。

## 2026-06-03 期号列表分页支持

- 完成任务：为“开奖期号与开奖源”页的期号管理补齐分页能力，支持按彩种筛选后保留分页查询，并显示总期号数。
- 解决问题：在期号量较大时页面列表无分页导致加载缓慢和查找效率低；分页参数未落入持久化查询，调度刷新后也缺少总量与页码展示。
- 技术说明：
  - 后端 `GET /api/admin/draw-issues` 新增分页响应 `DrawIssuePage`，返回 `items/totalCount/page/pageSize/totalPages`。
  - `DrawIssueListQuery` 增加 `page/pageSize`，并在查询参数均未提供时默认返回全部期号（为兼容历史全量调用）。
  - 前端 `admin/src/types/draws.ts` 增加分页类型，`admin/src/api/client.ts` 支持 `page/pageSize` 查询。
  - `admin/src/hooks/useDraws.ts` 解析分页响应并暴露 `issuePage/pageSize/totalCount/totalPages`，`LotteryConsole` 与非期号管理页面继续使用 `items` 进行展示。
  - `admin/src/pages/DrawManagementPage.tsx` 增加分页状态、每页条数选择、上一页/下一页及总数展示，并在筛选彩种变更时回退到第一页。
- 验证记录：`admin` 执行 `npm run build`、`backend` 执行 `cargo check`、`cargo test`，均通过。
- 后续动作：后续可补充“状态字段筛选 + 跳转指定页码 + 分页参数在 URL 同步”，当前先完成基础分页交互。

## 2026-06-03 20:56 HKT 彩种控制台最近开奖号码显示修复

- 完成任务：修复彩种控制台“最近开奖未刷新”表现异常。调整了期号列表聚合规则，`currentIssue` 不再固定取最早 `closed` 期号，而是按“open → 最新 closed → 最新 drawn → 最新 cancelled”顺序回退；并把“开奖号码显示”从“当前期有号码就优先”改为“仅当当前期状态是 `drawn` 且有号码时才标记为本期号码”，否则使用“最近开奖号码”。
- 解决问题：修复后开奖后控制台不会再被历史一期锁死显示，`最近开奖`会实时跟随最新开奖数据更新，避免误判为调度停摆。
- 技术说明：
  - `admin/src/pages/LotteryConsolePage.tsx` 中 `lotteryConsoleItem` 增加按状态分组与时间倒序取最新期号逻辑。
  - 新增 `pickLatestIssue` 辅助函数，避免旧期号（按升序选择）误占“当前期”展示位。
  - `LotteryConsoleCard` 增加 `currentIssueDrawNumber` 判断，明确区分“本期开奖号码”与“最近开奖号码”来源。
  - `admin/src/hooks/useLotteryConsole.ts` 新增页面可见和窗口聚焦后自动触发一次刷新，减少“开奖后等待轮询周期”造成的感知延迟。
- 验证记录：执行 `cd admin && npm run build`，TypeScript 与前端打包通过，未出现编译或打包错误。

## 2026-06-03 23:05 HKT API开售自动对齐期号与时间

- 完成任务：在 `set_lottery_sale` 中实现 API 彩种开售后的自动对齐补期开盘。
- 解决问题：当管理员将 API 彩种从停售切为开售时，系统未立即补齐未来期号，导致刚开售后仍需等待常驻调度下一轮；现在会依据调度配置 `future_issueCount` 和 `saleCloseLeadSeconds` 立即补齐缺口期号。
- 技术说明：
  - `backend/src/routes/admin.rs` 的 `set_lottery_sale` 新增：仅当彩种从停售切到开售且为 `DrawMode::Api` 时触发 `align_api_draw_issue_plan_after_sale_on`。
  - 该方法读取调度配置 `state.scheduler.config()`，统计当前彩种 `status=Open` 且 `scheduledAt > now` 的未来期号数量，按差值调用 `generate_draw_issue_batch`。
  - `generate_draw_issue_batch` 会自动走 API 源期号/开奖时间对齐逻辑，确保新开盘期号与最新外部期号时间一致。
  - 若补齐失败不回滚销售状态变更，并写入中文警告日志，避免管理员无法开售。
- 验证记录：执行 `cargo test`（后端）138 个测试通过。
- 后续动作：补充一条“开售接口返回补齐结果字段”用于前端显示补齐失败原因（当前先保留日志告警）。

## 2026-06-03 22:12 HKT 期号按玩法筛选与停售不调度

- 完成任务：在“开奖期号与开奖源”期号列表页新增玩法筛选入口（按彩种），可按单一玩法或全部玩法查看期号；筛选项默认显示“全部玩法”。
- 完成任务：补齐接口与调度链路，`GET /api/admin/draw-issues` 支持 `lotteryId` 查询参数；自动化调度与补期任务在遇到停售彩种时会跳过处理。
- 解决问题：此前“期号列表”无法按玩法快速定位，停售彩种仍会参与自动封盘/开奖流程，导致后台运维排障困难和调度行为不可控。
- 技术说明：
  - 后端：`backend/src/routes/admin.rs` 的 `list_draw_issues` 新增查询参数提取，支持 `lotteryId`；`backend/src/services/draw.rs` 增加按 `lottery_id` 过滤仓储查询；`automation.rs` 已有停售彩种跳过逻辑；`scheduler.rs` 已在补期期号阶段跳过停售彩种。
  - 前端：`admin/src/types/draws.ts` 新增 `DrawIssueQuery`；`admin/src/api/client.ts` 的 `fetchDrawIssues` 支持可选查询参数；`admin/src/hooks/useDraws.ts` 增加 `refreshWithFilter` 入口；`admin/src/pages/DrawManagementPage.tsx` 的期号管理区增加下拉筛选并联动刷新。
- 验证记录：待执行 `cargo test`、`cargo check`、`npm run build`；重点验证 `GET /api/admin/draw-issues?lotteryId=fc3d` 正常返回、停售彩种在 `POST /api/admin/draw-automation/run` 与常驻调度循环中记录“彩种已停售，跳过自动任务”。
- 后续动作：补充前端筛选状态持久化（URL query 保留筛选值）和后续 UI 增加“号码类型/3位/5位”快捷筛选。

## 2026-06-03 21:02 HKT 开奖源配置改为数据库优先

- 完成任务：修改开奖源加载策略，使 `draw_sources` 在数据库已有数据时不再注入硬编码默认彩种源配置；仅在数据库表为空时执行默认种子回填。
- 解决问题：当前系统在有数据库配置的情况下仍可能被代码内置默认值混入/覆盖判断，影响“数据库配置即权威源”约定；现在数据库优先，避免重复或不一致来源。
- 技术说明：`backend/src/services/draw_api.rs` 的 `load_draw_source_store` 改为仅在存储为空时回填默认源；新增迁移 `backend/migrations/20260603192000_seed_draw_sources.sql` 在空库初始化时写入默认 `draw_sources`（使用 `ON CONFLICT DO NOTHING` 保证已有行不受影响）。
- 影响范围：数据库初始化、开奖源 CRUD 与重启恢复流程。
- 后续动作：清理 `api68_seeded` 场景对生产链路的依赖，统一所有环境都从数据库读取源定义；补充一次迁移回放验证文档。

## 2026-06-03 21:18 HKT 后台代码中文注释补齐

- 完成任务：为后端全部 Rust 文件补充中文注释，明确每个文件/模块职责，提升可读性。
- 解决问题：项目需求为“后台每个地方具体干什么都要中文说明”，当前代码在多人接手时可读性不足，尤其是服务与领域模型入口边界。
- 技术说明：在 `backend/src` 的所有 `.rs` 文件顶部新增 `//!` 中文模块说明，覆盖 `app/main/routes/domain/services` 与其子模块；并针对 `routes/mod.rs`、`services/mod.rs`、`domain/mod.rs` 修正为准确模块聚合职责。
- 影响范围：后端代码可读性、交接和后续维护。
- 后续动作：继续补充函数级中文注释（如关键公共方法、复杂条件分支），按页面对接状态逐步补齐到“每个逻辑点都可直接读懂”。

## 2026-06-03 20:49 HKT 开奖调度器执行日志中文化

- 完成任务：将常驻调度执行成功日志中的英文统计字段（如 `now`、`closed_issues`、`drawn_issues`）统一替换为中文字段名（如“当前时间”“封盘期数”“开奖期数”）。
- 解决问题：日志平台可读性不一致，运维在中文场景下难以快速识别调度结果；现在将 `INFO` 摘要日志改为中文键值，便于一眼判断一轮执行效果。
- 技术说明：`backend/src/services/scheduler.rs` 的 `tracing::info!` 已将结构化字段重命名为中文标签；字段值来源与原有统计逻辑一致。
- 验证记录：`cargo fmt --check`、`cargo test -q`（138 个测试）均通过。

## 2026-06-03 20:12 HKT 开奖等待原因可视化与控制台当前期修正

- 完成任务：排查彩种控制台“到达开奖时间一直等待开奖”的原因，并新增调度跳过明细持久化、后台展示和控制台状态提示。
- 解决问题：本地复现发现 `txffc` 旧期 `202606031202` 已到期开奖，但 KJAPI 当前返回期号已跳到后续期号，最新接口无法补取旧期开奖号码；此前调度历史只展示跳过数量，控制台又优先显示最早 `closed` 期号，导致旧待补期一直压住新的 open 期，看起来像系统不再开盘。
- 技术说明：`draw_scheduler_runs` 新增 `skipped_issues`、`skipped_lotteries` 两个 `jsonb` 字段；`DrawSchedulerRunRecord` 返回跳过期号和彩种原因；自动开奖跳过原因改为中文业务前缀，并在 API 未找到期号时带上当前外部返回期号。
- 管理后台：常驻调度卡片展示最近一轮跳过明细；彩种控制台展示调度启停状态和执行周期，到点后区分“等待开奖源”“等待调度”“调度已关闭”；当前期选择优先展示 open 期号，旧 closed 漏开奖以“待补开奖 N”标签提示。
- 验证记录：`cargo fmt --check`、`git diff --check`、`cargo check`、`cargo test` 137 个测试、`npm run build` 均通过；本地 `18121` 后端连接外部 PostgreSQL 验证调度状态接口返回 `txffc` 跳过原因“当前返回期号 `202606031211`”。
- 后续动作：补开奖源测试连接、原始响应留痕和旧期异常复核入口，允许管理员对外部源已越过的期号进行手动开奖、取消或标记异常。

## 2026-06-03 19:49 HKT 腾讯分分彩 KJAPI 彩种接入

- 完成任务：新增 `txffc` 腾讯分分彩彩种，接入 KJAPI 开奖接口 `https://kjapi.net/hall/hallajax/getLotteryInfo?lotKey=txffc`，并在后台开奖源配置中支持 `kjApi` 供应商和腾讯分分彩采集预设。
- 解决问题：系统此前只支持 API68 格式来源，无法解析 KJAPI 的 `result.data` 对象结构，也无法保存 `txffc` 这种字符串 `lotKey`；现在后端可读取 `preDrawIssue/preDrawCode/preDrawTime/drawIssue/drawTime`，并按供应商返回的下一期开奖时间生成期号。
- 技术说明：API 期号序列升级为 64 位整数，支持 `202606031179` 这类 12 位期号；PostgreSQL 启动时会补齐缺失的默认彩种和开奖源，不覆盖已有同 ID 配置。
- 验证记录：已新增 KJAPI 解析、腾讯分分彩期号生成、已封盘候选期跳过和种子彩种/来源测试；后续继续运行完整后端与前端构建验证。
- 后续动作：补开奖源“测试连接”入口，展示供应商当前期号、下一期期号、服务器时间和解析后的本地开奖计划。

## 2026-06-03 19:28 HKT API68 周期彩种期号时间对齐修正

- 完成任务：修复 API68 周期彩种生成下一期时的开奖时间对齐逻辑，澳洲 5 分彩现在会使用 API68 返回的 `preDrawTime` 作为节奏锚点，并按 `intervalSeconds` 推导后续期号时间。
- 解决问题：此前调度开启后虽然能生成澳洲 5 分彩期号，但 `scheduledAt` 使用服务器当前时间推导，和 API68 实际开奖时间错位，容易出现彩种控制台显示未开盘或到点后持续等待 API 开奖结果。
- 技术说明：`ApiDrawSourceLatestIssue` 增加最新开奖时间；期号生成服务对 API 周期彩种按外部最新期号、外部开奖时间和本地最大期号偏移计算下一期，并跳过已经过了 `saleClosedAt` 的候选期号，避免创建已封盘的 `open` 期。
- 本地验证：通过 `18120` 后端确认调度开启后会生成 open 期号，60 秒时时彩完成封盘、开奖并补下一期；`cargo test` 130 个测试通过，`cargo check` 通过。
- 后续动作：在后台调度运行历史中补充跳过彩种/期号明细，并在开奖源配置页展示 API 最新期号、开奖时间和本地下一期开奖时间。

## 2026-06-03 19:10 HKT 开奖调度后台控制入口

- 完成任务：在管理后台“开奖期号与开奖源”的“自动任务与调度”页签中，为“常驻调度”卡片新增“启动调度”和“关闭调度”直接操作按钮，并保留“修改配置”入口。
- 解决问题：此前调度启停需要进入配置 SideSheet 修改启用复选框，不够直观；现在管理员可以在调度卡片上直接启动或关闭调度，同时仍可进入 SideSheet 调整执行周期、未来期号缓冲和封盘提前秒数。
- 技术说明：启动/关闭按钮复用 `PUT /api/admin/draw-scheduler/config`，只切换 `enabled`，其它调度配置保持当前数据库状态；保存成功后刷新调度状态和 dashboard。
- 后续动作：继续补调度开关二次确认、操作审计、变更原因和多实例分布式锁。

## 2026-06-03 19:09 HKT 开奖调度配置数据库修正

- 完成任务：移除 `DRAW_SCHEDULER_ENABLED`、`DRAW_SCHEDULER_INTERVAL_SECONDS`、`DRAW_SCHEDULER_FUTURE_ISSUE_COUNT` 和 `DRAW_SCHEDULER_SALE_CLOSE_LEAD_SECONDS` 本地 env 配置入口。
- 解决问题：开奖调度启用状态、执行周期、未来期号数量和封盘提前秒数属于后台业务配置，不应该通过环境变量覆盖；现在配置以 `draw_scheduler_config` 数据库表为准，由后台“自动任务与调度”页面保存。
- 技术说明：服务启动时使用 `DrawSchedulerConfig::default()` 作为空库或内存模式种子；配置 PostgreSQL 时会读取 `draw_scheduler_config`，表为空才写入默认配置，已有数据不会被 env 覆盖。
- 后续动作：继续补调度配置变更审计、版本号、审批回滚和多实例分布式锁。

## 2026-06-03 19:02 HKT API68 endpoint 数据库配置修正

- 完成任务：将 API68 全国彩和重庆时时彩 endpoint 从本地 env 配置中移除，并修正后端逻辑为只使用开奖源配置中的 endpoint。
- 解决问题：`API68_QUANGUOCAI_ENDPOINT` 和 `API68_CQSHICAI_ENDPOINT` 属于开奖源业务配置，不应该通过环境变量覆盖；现在默认 endpoint 写入 `draw_sources`，后续修改需要通过后台“开奖源配置”保存到数据库。
- 技术说明：保留 API68 默认 seed 值用于空库初始化，数据库已有开奖源时读取数据库中的 `endpoint`；`.env.example` 和本机 `.env.local` 不再包含 API68 endpoint。
- 后续动作：继续完善开奖源连通性测试、原始响应留痕、endpoint 变更审计和二次确认。

## 2026-06-03 18:59 HKT Git 提交中文规则

- 完成任务：在 `AGENTS.md` 中新增 Git 提交信息使用中文的项目规则。
- 解决问题：此前文档输出已要求中文，但 Git 提交信息仍可能沿用英文；现在明确后续提交标题和必要说明都使用中文，便于项目历史记录统一阅读。
- 后续动作：后续所有提交都使用中文提交信息，并在提交信息中清楚描述本次功能、修复或规则变更。

## 2026-06-03 18:42 HKT 本地 env 文件配置

- 完成任务：新增本地 env 配置方案，后端支持加载项目根目录和 `backend/` 下的 `.env`、`.env.local`，前端新增 `admin/.env.example` 和本机 `admin/.env.local`。
- 解决问题：此前本地测试只能在命令行手动传 `DATABASE_URL`、`PORT` 和 `VITE_API_BASE_URL`，没有可复用的配置文件；现在后端和前端都有明确的本地 env 文件入口。
- 技术说明：真实 PostgreSQL 密码只写入被 `.gitignore` 忽略的 `.env.local`，可提交的 `.env.example` 只保留 `postgres://root:<密码>@192.168.2.3:15432/postgres` 模板；后端 shell 环境变量优先级高于 env 文件。
- 后续动作：使用 `cd backend && cargo run`、`cd admin && npm run dev -- --host 127.0.0.1 --port <空闲端口>` 做本地联调，并继续以外部 PostgreSQL 验证业务流程。

## 2026-06-03 18:32 HKT 本地测试规则更新

- 完成任务：在 `AGENTS.md` 中补充后续测试规则，明确功能验证和联调测试直接本地启动后端、前端服务，不再通过 Docker 打包镜像测试。
- 解决问题：此前端到端验证曾使用 Docker 镜像和 Compose，和用户最新要求“测试时不需要打包成 Docker，直接本地启动服务”不一致；本次把约定写入项目级规则，后续测试默认使用本地服务。
- 技术说明：本地后端测试默认使用用户提供的外部 PostgreSQL，`DATABASE_URL` 通过本地环境变量传入；文档只记录连接模板，不把数据库密码写入仓库文件。
- 后续动作：继续使用 `cargo run` 和 `npm run dev` 做本地联调，并以外部 PostgreSQL 验证业务数据库持久化。

## 2026-06-02 13:36:05 HKT

- 完成任务：启动彩票系统首个开发任务 `06-02-admin-backend-foundation`，并创建本轮 PRD。
- 解决问题：项目当前只有需求文档、规则文档和 Trellis 配置，没有可运行工程；先把开发范围收敛为“后端 + 管理后台 MVP 骨架”，避免一次性铺开全部业务导致边界不清。
- 后续动作：补齐初始开发规格，创建 Rust 后端和 React 管理后台工程，并同步更新 `架构设计.md`。

## 2026-06-02 13:40:23 HKT

- 完成任务：在 `AGENTS.md` 中加入“项目文档使用中文输出”的规则，并把本轮新增/改写的 PRD 与开发规格文档改为中文。
- 解决问题：此前 Trellis 规格文档沿用了英文模板，与用户要求的中文文档输出规则不一致；现在已统一为中文，代码标识、命令、路径、第三方库名和协议字段名保留原文。
- 后续动作：继续完成 Rust 后端和 React 管理后台基础工程，并在实现过程中继续维护 `TODO.md`。

## 2026-06-02 13:50:30 HKT

- 完成任务：创建 Rust 后端工程 `backend/`，实现 Axum API、统一响应信封、健康检查、管理后台概览接口、核心领域模型和演示数据服务；创建 React 管理后台 `admin/`，实现应用外壳、侧边导航、系统概览、模块入口、接口加载和错误状态。
- 解决问题：项目没有可运行工程和跨层接口契约；本次建立了 `/api/health`、`/api/admin/dashboard` 以及前端对应类型，保证管理后台可以先围绕真实 API 结构运行起来。`8080` 被本机其他进程占用，联调改用后端 `18080` 和前端 `5174`，避免影响已有服务。
- 验证结果：`cargo fmt`、`cargo check`、`cargo test`、`npm run build` 均通过；浏览器打开 `http://localhost:5174/` 后确认工作台、彩种开奖源、用户管理入口正常显示，点击“用户管理”可进入占位页面，控制台无错误。
- 后续动作：进入质量复查，确认文档、规格、架构说明与代码保持一致；下一阶段可开始接入数据库、认证权限或彩种管理真实 CRUD。

## 2026-06-02 13:52:50 HKT

- 完成任务：完成 Trellis 质量复查和规格沉淀，新增 `.trellis/spec/backend/api-contracts.md`，记录 `/api/health`、`/api/admin/dashboard`、统一响应信封、`PORT`、`VITE_API_BASE_URL`、金额最小单位和返利 basis points 契约；同时补充前端类型安全规范和 Semi UI 样式导入注意事项。
- 解决问题：构建过程中发现 `tsc -b` 会生成 `vite.config.js`、`vite.config.d.ts` 和 `*.tsbuildinfo` 等副产物，已改为 `tsc --noEmit` 双配置检查，避免构建污染源码目录；前端错误提示也从固定 `8080` 改为检查 `VITE_API_BASE_URL`，适配非默认端口联调。
- 验证结果：重新运行 `cargo fmt --check`、`cargo check`、`cargo test`、`npm run build` 均通过。
- 后续动作：项目根目录当前不是 Git 仓库，无法按 Trellis Phase 3.4 生成工作提交；后续如果需要完整任务归档和提交记录，需要先在项目根或包目录初始化/进入 Git 仓库。

## 2026-06-02 14:16:33 HKT

- 完成任务：实现 `06-02-lottery-management-crud` 彩种管理阶段，新增后端内存彩种仓储、彩种 CRUD 与销售开关接口，并把管理后台“彩种管理”入口替换为可新增、编辑、删除和切换销售状态的真实页面。
- 解决问题：此前彩种只存在于 dashboard 静态演示数据中，无法维护配置；本次用共享 `LotteryStore` 让列表接口和 dashboard 使用同一份数据。接口联调时发现 `DrawSchedule` 枚举变体字段没有按前端契约接受 `intervalSeconds`，已通过 `rename_all_fields = "camelCase"` 修复，并新增序列化/反序列化测试。
- 验证结果：HTTP 冒烟测试通过，确认 `GET/POST/PATCH/DELETE /api/admin/lotteries` 和 `/api/admin/dashboard` 数据一致；浏览器验证通过，彩种管理页从 4 条新增到 5 条再删除回 4 条；`cargo fmt --check`、`cargo check`、`cargo test`、`npm run build` 均通过。
- 后续动作：提交 Git；下一阶段可进入数据库持久化、开奖源配置或鉴权权限。

## 2026-06-02 15:10:45 HKT

- 完成任务：实现 `06-02-lottery-database-persistence` 彩种数据库持久化阶段，新增 SQLx PostgreSQL 依赖、`lotteries` 表迁移、统一彩种仓储入口和 PostgreSQL 彩种仓储；后端会根据 `DATABASE_URL` 自动选择数据库模式或内存模式。
- 解决问题：上一阶段彩种数据服务重启后会丢失；本次在配置数据库时可持久化彩种 CRUD 和销售状态，同时保留无数据库 fallback。实现中发现 SQLx `0.9.0` 要求 Rust `1.94.0`，当前工具链是 Rust `1.92.0`，已改用兼容的 SQLx `0.8.6` 并记录到 PRD 和调研文档。
- 验证结果：无 `DATABASE_URL` 启动后端成功，`/api/health`、`/api/admin/lotteries` 和 `/api/admin/dashboard` 冒烟测试通过；`cargo fmt --check`、`cargo check`、`cargo test` 通过，后端 11 个测试全绿；`npm run build` 通过。
- 后续动作：同步数据库/API 规格并完成 Git 提交；下一阶段可进入开奖源配置、数据库容器化联调或鉴权权限。

## 2026-06-02 15:37:14 HKT

- 完成任务：实现 `06-02-play-rule-engine-foundation` 玩法规则引擎阶段，新增后端玩法规则领域模型和服务层，支持 3 位直选、组三复式、组三胆拖、组六复式、组六胆拖，以及 5 位前/中/后 3 直选、直选组合、组三、组六、胆拖和大小单双；新增 `GET /api/admin/play-rules` 与 `POST /api/admin/play-rules/evaluate`，并在管理后台新增“玩法规则”真实页面。
- 解决问题：彩票后台此前只有彩种入口和静态占位，缺少订单、计奖、派奖复用的核心规则能力；本次把注数计算、投注展开和中奖判断放到后端服务层，避免后续投注和派奖依赖前端临时计算。实现中保留了用户已有的 `admin/vite.config.ts`、`backend/src/main.rs` 端口改动和 `.idea/` 文件，不纳入本阶段提交。
- 验证结果：`curl` 冒烟测试确认规则目录、3 位直选评估和 5 位大小单双评估返回统一 API 信封且命中结果正确；浏览器打开 `http://127.0.0.1:5174/` 后进入“玩法规则”页面并计算出 `247` 命中；`cargo fmt --check`、`cargo check`、`cargo test`、`npm run build` 均通过。
- 后续动作：下一阶段应优先实现订单与投注模块，把规则引擎接入订单创建、投注金额校验和投注明细保存；随后继续推进开奖源、期号、计奖、派奖、用户资金、合买和机器人流程。

## 2026-06-02 15:54:58 HKT

- 完成任务：实现 `06-02-order-betting-foundation` 订单与投注基础阶段，新增后端订单领域模型、内存订单仓储、订单创建/列表/详情/取消接口；订单创建会读取彩种配置并复用玩法规则引擎计算注数、展开投注和订单金额。管理后台新增“订单管理”真实页面，并在工作台新增“最近订单”展示。
- 解决问题：此前订单管理只是占位，dashboard 最近订单也是静态演示数据，后续开奖、计奖、派奖和机器人没有真实订单入口；本次建立了基础订单数据流，并确保金额由后端按 `stakeCount * unitAmountMinor` 计算，不让前端传最终金额。用户已有的 `admin/vite.config.ts`、`backend/src/main.rs` 端口改动和 `.idea/` 继续保留，不纳入本阶段提交。
- 验证结果：`curl` 冒烟测试通过，确认创建 3 位直选订单得到 `stakeCount=1`、`amountMinor=200`、`expandedBets=["247"]`，订单列表和 dashboard 最近订单能回流；浏览器打开订单管理页成功创建订单，并在工作台看到最近订单；`cargo check`、`cargo test`、`npm run build` 已通过，后端测试增加到 24 个。
- 后续动作：下一阶段建议实现开奖期号与开奖源模块，随后把订单接入计奖、派奖和用户资金流水；订单数据库持久化也需要单独排期。

## 2026-06-02 16:11:08 HKT

- 完成任务：实现 `06-02-draw-issue-source-foundation` 开奖期号与开奖源基础阶段，新增后端开奖领域模型、内存开奖仓储、开奖源列表、期号列表/详情/创建/封盘/开奖/取消接口；管理后台新增“开奖期号与开奖源”真实页面，并把“开奖模式”和“开奖时间”两个入口都接入该页面。
- 解决问题：此前开奖源只存在于 dashboard 静态摘要，缺少期号和开奖结果入口，后续计奖、派奖、机器人和资金流水没有可复用的开奖事实来源；本次把开奖号码校验和状态流转放到后端服务层，支持 3 位/5 位号码校验、手动开奖录入、平台/API 本地生成，并阻止已开奖期号重复开奖或取消。用户已有的 `admin/vite.config.ts`、`backend/src/main.rs` 端口改动和 `.idea/` 继续保留，不纳入本阶段提交。
- 验证结果：`curl` 冒烟测试通过，确认 `GET /api/admin/draw-sources`、`POST /api/admin/draw-issues`、封盘、API 开奖生成 3 位号码和手动开奖录入 5 位号码均返回统一 API 信封；浏览器打开 `http://127.0.0.1:5174/` 后进入“开奖模式”页面，成功创建 `20260602001` 并开奖回显号码 `978`；`cargo fmt --check`、`cargo check`、`cargo test`、`npm run build` 均通过，后端测试增加到 28 个。
- 后续动作：下一阶段建议实现计奖与派奖基础，把订单、玩法规则和开奖结果串起来；同时需要把开奖期号持久化到 PostgreSQL，并补真实第三方开奖 API、定时封盘和自动开奖任务。

## 2026-06-02 16:23:42 HKT

- 完成任务：实现 `06-02-settlement-payout-foundation` 计奖与派奖基础阶段，新增后端结算领域模型、结算批次 API、按已开奖期号执行计奖派奖的订单状态流转；管理后台新增“计奖派奖”真实页面，并在订单管理页展示开奖结果、命中投注、派奖金额和结算时间。
- 解决问题：此前订单和开奖之间没有结算链路，订单不会因为开奖结果变成中奖或未中奖；本次让结算流程复用玩法规则引擎，中奖订单更新为 `won`，未中奖订单更新为 `lost`，已取消订单跳过，重复结算同一期号会被拒绝。基础派奖金额使用后端固定倍数表，仅用于验证链路，不代表真实生产赔率；用户已有的 `admin/vite.config.ts`、`backend/src/main.rs` 端口改动和 `.idea/` 继续保留，不纳入本阶段提交。
- 验证结果：`curl` 冒烟测试通过，确认创建 `fc3d` 期号 `2026200`、开奖得到 `023`、创建直选 `023` 订单后执行结算，订单状态变为 `won`，结算批次 `S000000000001` 派奖 `2000` 分；浏览器打开 `http://127.0.0.1:5174/` 后进入“计奖派奖”页面，看到期号、结算批次、订单命中和 `¥20.00` 派奖；`cargo fmt --check`、`cargo check`、`cargo test`、`npm run build` 均通过，后端测试增加到 31 个。
- 后续动作：下一阶段建议实现用户资金与资金流水，把派奖结果真正入账；同时需要补真实赔率/奖金表、期号封盘投注校验、结算持久化和异常复核。

## 2026-06-02 16:36:38 HKT

- 完成任务：开始实现 `06-02-finance-ledger-foundation` 用户资金与资金流水基础阶段，新增后端资金账户、资金流水、手动调账、订单扣款、取消退款和结算派奖入账能力；管理后台新增“财务管理”真实页面、财务 API client、`useFinance` hook 和资金类型。
- 解决问题：此前订单创建不扣余额、取消订单不退款、中奖结算不入账，dashboard 财务摘要也是静态数据；本次让订单、结算和财务管理共用同一份内存资金仓储，并用资金流水记录每次余额变化。订单创建采用“报价和余额预检 → 创建订单 → 扣款 → 扣款失败移除未入资订单”的补偿流程，避免留下无扣款订单。
- 验证结果：阶段性验证已完成 `cargo check`、`cargo test` 和 `npm run build`；后端资金单元测试覆盖投注扣款、余额不足拒绝、取消退款、派奖入账和手动调账。后续还需要完成最终 `cargo fmt --check`、API 冒烟、浏览器验证和 Git 提交归档。
- 后续动作：继续做最终质量检查和联调，确认账户余额、流水、dashboard 和财务页面数据一致；随后提交本阶段代码并归档 Trellis 任务。

## 2026-06-02 16:40:51 HKT

- 完成任务：完成 `06-02-finance-ledger-foundation` 的最终联调验证，确认用户资金、资金流水、订单扣款、取消退款、派奖入账和管理后台财务页面已经形成基础闭环。
- 解决问题：联调时发现后端启动不能把 `DATABASE_URL` 设置为空字符串，否则 SQLx 会按已配置数据库处理并报 `RelativeUrlWithoutBase`；已改用 `env -u DATABASE_URL PORT=18081 cargo run` 启动内存模式，并把该差异记入本次验证过程。
- 验证结果：`cargo fmt --check`、`cargo check`、`cargo test`、`npm run build` 均通过；API 冒烟确认创建订单后 `U10001` 可用余额从 `12000` 降到 `11800` 并生成 `orderDebit`，取消后恢复 `12000` 并生成 `orderRefund`，余额不足用户 `U10004` 创建订单被拒绝；中奖结算后派奖 `2000` 分并生成 `payoutCredit`，`U10001` 可用余额达到 `13800`。浏览器验证 `http://127.0.0.1:5175/` 的“财务管理”页面，资金账户、资金流水、手动调账和用户 `U10001` 均正常显示。
- 后续动作：提交本阶段代码，归档 Trellis 任务并记录开发日志；下一阶段建议接入期号封盘投注校验或资金持久化事务。

## 2026-06-02 17:02:34 HKT

- 完成任务：实现 `06-02-play-odds-configuration-foundation` 玩法与赔率配置阶段，新增彩种 `playConfigs` 单玩法配置、玩法目录 `category` 字段、订单 `oddsBasisPoints` 赔率快照和结算按赔率快照派奖；管理后台“玩法规则”升级为“玩法规则与赔率”，可按 3 位/5 位切换查看玩法、试算规则，并按彩种逐条启用玩法和编辑赔率。
- 解决问题：此前玩法规则页只能试算，无法维护每个彩种每个玩法的赔率；订单和结算使用固定基础倍数，后续调价无法追踪历史订单。本次让赔率落到彩种单玩法，订单创建保存快照，结算使用快照，避免历史订单被后续赔率修改影响。同时核对两份规则文档后确认：3 位玩法为 5 个，`5个玩法规则说明.md` 实际列出 19 个 5 位玩法，当前后端和页面均已按 5/19 全量展示。
- 验证结果：`cargo fmt --check`、`cargo check`、`cargo test`、`npm run build` 均通过；API 冒烟确认 `GET /api/admin/play-rules` 返回 3 位 5 个、5 位 19 个且带 `category`，将 `manual-test.fiveBackDirect` 赔率设为 `123000` 后创建命中订单，订单快照为 `123000`，结算派奖为 `2460` 分；浏览器验证 `http://127.0.0.1:5176/` 的“玩法规则与赔率”页面可切换 5 位玩法，显示 19 个玩法和 `12.30` 倍赔率，移动视口整页宽度保持在 390px，表格只在内部滚动。
- 后续动作：下一阶段建议接入期号封盘投注校验，或者把订单、开奖期号、结算、资金和玩法赔率配置一起升级为 PostgreSQL 事务持久化。

## 2026-06-02 17:09:55 HKT

- 完成任务：修正玩法配置入口可发现性，将 dashboard/侧边栏模块名称从“玩法规则”改为“玩法配置”，页面标题改为“玩法配置与赔率”，并在彩种管理页新增“玩法配置”跳转按钮。
- 解决问题：虽然上一阶段已经有按彩种逐条配置玩法启用状态和赔率的表格，但入口名称仍像规则说明页，导致配置位置不明显；本次让入口、页面标题、保存按钮和架构说明都明确指向“玩法配置”。
- 验证结果：`cargo check`、`cargo test`、`npm run build` 均通过；后端测试 36 个全绿，前端构建确认“彩种管理”到“玩法配置”的跳转参数和页面类型正常。

## 2026-06-02 17:21:32 HKT

- 完成任务：修正开奖号码格式，后端手动开奖、平台/API 自动开奖、玩法规则评估和管理后台默认输入统一使用英文逗号分隔格式，例如 `2,4,7`、`7,8,9,4,2`。
- 解决问题：此前系统主要用 `247`、`78942` 这类紧凑字符串展示和校验开奖号码，与用户要求的逗号分割格式不一致；本次后端保存和返回统一逗号格式，同时兼容读取旧紧凑格式，投注展开和命中投注仍保留紧凑注单编码。
- 验证结果：`cargo fmt --check`、`cargo check`、`cargo test`、`npm run build` 均通过；API 冒烟确认玩法评估接受 `2,4,7` 并命中投注 `247`，手动开奖保存 `7,8,9,4,2`，平台开奖返回类似 `1,3,8` 的逗号格式开奖号码。

## 2026-06-02 17:29:48 HKT

- 完成任务：开始并实现 `06-02-draw-issue-order-guard` 期号封盘投注校验阶段，订单创建必须找到同彩种同 `issue` 的开奖期号，并且只有 `open` 状态允许投注；订单管理页的期号输入改为当前彩种 open 期号下拉框。
- 解决问题：此前订单可以对不存在期号、已封盘期号、已开奖期号或已取消期号继续创建，容易产生无法结算或绕过封盘的异常订单；本次把订单创建和开奖期号销售状态接起来，后端在扣款前再次校验期号状态，前端也只展示可投注期号。
- 验证结果：`cargo fmt --check`、`cargo check`、`cargo test`、`npm run build` 均通过；后端测试增加到 37 个。API 冒烟确认 open 期号 `GUARD20260602OPEN` 可创建订单，closed 期号返回 `draw issue is not open for order creation`，不存在期号返回 `not found for lottery`；浏览器验证订单页期号字段已变为 open 期号下拉框，当前值可选中 `UIOPEN20260602`。

## 2026-06-02 17:43:47 HKT

- 完成任务：实现 `06-02-draw-automation-runner` 自动封盘开奖结算基础阶段，新增 `POST /api/admin/draw-automation/run` 接口和后端自动任务服务；管理后台“开奖期号与开奖源”页面新增“自动任务”操作区，可按传入执行时间触发封盘、开奖、结算和派奖入账。
- 解决问题：此前期号只能由管理员逐个点击封盘、开奖，再到计奖派奖页面手动结算，封盘投注校验虽然已接入，但没有按时间批量推进期号状态的入口；本次让 `open` 且到封盘时间的期号自动变为 `closed`，让到开奖时间的 `platform/api` 期号自动开奖并结算入账，同时让 `manual` 期号缺少开奖号码时只记录跳过原因，不伪造开奖号码。用户已有的 `admin/vite.config.ts`、`backend/src/main.rs` 端口改动和 `.idea/` 继续保留，不纳入本阶段提交。
- 验证结果：`cargo fmt --check`、`cargo check`、`cargo test`、`npm run build` 均通过；后端测试增加到 39 个。API 冒烟确认到期 API 期号自动封盘并开奖为逗号格式 `4,8,7`，生成 1 个结算批次和 1 笔 `payoutCredit` 入账，手动开奖期号返回 `manual draw requires administrator draw number` 跳过原因。浏览器验证 `http://127.0.0.1:5177/` 的“开奖期号与开奖源”页面已显示“自动任务”入口和“运行自动任务”按钮，点击后页面无控制台错误。
- 后续动作：提交本阶段代码，归档 Trellis 任务并记录开发日志；下一阶段建议实现系统级常驻调度、自动创建下一期号、失败重试队列和开奖 API 源数据审计。

## 2026-06-02 18:07:30 HKT

- 完成任务：实现 `06-02-draw-issue-generation-foundation` 自动创建下一期号基础阶段，新增 `POST /api/admin/draw-issues/generate-next` 接口、后端期号生成服务和管理后台“按计划生成下一期”按钮。
- 解决问题：此前自动封盘、自动开奖和自动结算已经能推进已有期号，但仍依赖管理员手动填写期号、开奖时间和封盘时间；本次让后端根据彩种 `DrawSchedule` 自动计算下一期，支持周期开奖、每日固定开奖和周开奖，期号编码统一按开奖时间生成 `YYYYMMDDHHMMSS`，封盘时间默认开奖前 30 秒。用户已有的 `admin/vite.config.ts`、`backend/src/main.rs` 端口改动和 `.idea/` 继续保留，不纳入本阶段提交。
- 验证结果：`cargo fmt --check`、`cargo check`、`cargo test`、`npm run build` 均通过；后端测试增加到 43 个，覆盖周期、每日、周开奖和已有期号作为基线继续生成。API 冒烟确认 `fc3d` 每日开奖生成 `20260603210015`，`ssc60` 周期开奖生成 `20260602200100` 并再次生成 `20260602200200`，`manual-test` 周开奖生成 `20260604210000`。浏览器验证 `http://127.0.0.1:5178/` 的“开奖期号与开奖源”页面已显示“按计划生成下一期”按钮，点击后生成并选中 `20260604210015`，控制台无错误。
- 后续动作：提交本阶段代码，归档 Trellis 任务并记录开发日志；下一阶段建议实现系统级常驻调度、批量预生成多期、自动任务失败重试和开奖期号 PostgreSQL 持久化。

## 2026-06-02 18:22:08 HKT

- 完成任务：实现 `06-02-draw-issue-bulk-generation-preview` 批量预生成期号和计划预览阶段，新增 `POST /api/admin/draw-issues/preview-generation` 与 `POST /api/admin/draw-issues/generate-batch`，管理后台“开奖期号与开奖源”页面新增预生成数量、预览计划和批量生成入口。
- 解决问题：此前系统只能逐次点击生成下一期，管理员无法一次查看未来多期计划，也无法批量补齐 open 期号；本次把单期生成、预览和批量生成统一到后端计划函数，预览不写仓储，批量生成复用开奖期号创建校验，并限制数量为 1 到 50，避免前端自行推导开奖计划。用户已有的 `admin/vite.config.ts`、`backend/src/main.rs` 端口改动和 `.idea/` 继续保留，不纳入本阶段提交。
- 验证结果：`cargo fmt --check`、`cargo check`、`cargo test`、`npm run build` 均通过；后端测试增加到 49 个，覆盖预览不写入、周期批量、已有期号基线、每日批量、周开奖批量和数量边界。API 冒烟确认 `ssc60` 预览 3 期返回 `20260602200100` 到 `20260602200300`，`fc3d` 预览后列表未新增 `fc3d` 期号，随后批量生成 2 期返回 `20260603210015` 和 `20260604210015`，`count=0` 返回数量范围错误。浏览器验证 `http://127.0.0.1:5179/` 的“开奖期号与开奖源”页面已显示“预生成数量”“预览计划”“批量生成”，点击预览显示 5 期计划，点击批量生成后列表新增 `20260605210015` 到 `20260609210015`，页面无接口错误提示。
- 后续动作：提交本阶段代码，归档 Trellis 任务并记录开发日志；下一阶段建议实现系统级常驻调度自动补期、自动生成操作日志、失败重试和冲突审计。

## 2026-06-02 18:44:33 HKT

- 完成任务：实现 `06-02-draw-scheduler-foundation` 系统级常驻调度基础阶段，新增后端 `services/scheduler.rs`，支持通过 `DRAW_SCHEDULER_ENABLED` 等环境变量启用后台循环，周期性执行自动封盘/开奖/结算/派奖，并自动为销售开启彩种补齐未来期号。
- 解决问题：此前自动任务、单期生成和批量预生成都需要管理员手动点击，系统无法在服务运行期间自动推进期号生命周期；本次把常驻调度拆成可测试的单轮调度和后台 Tokio 循环，单轮先复用 `run_draw_automation` 处理到期事项，再复用 `generate_draw_issue_batch` 补齐未来期号，避免复制业务逻辑。调度默认关闭，避免本地开发和测试时后台任务自动改写内存数据；用户已有的 `admin/vite.config.ts`、`backend/src/main.rs` 端口改动和 `.idea/` 继续保留，不纳入本阶段提交。
- 验证结果：`cargo fmt --check`、`cargo check`、`cargo test`、`npm run build` 均通过；后端测试增加到 55 个，覆盖调度默认关闭、环境变量解析、无效配置、销售开启彩种补期、未来缓冲满足不重复生成、到期自动任务先执行再补期。服务冒烟使用 `DRAW_SCHEDULER_ENABLED=true DRAW_SCHEDULER_INTERVAL_SECONDS=1 DRAW_SCHEDULER_FUTURE_ISSUE_COUNT=2 PORT=18086` 启动后，`GET /api/admin/draw-issues` 自动出现 `fc3d`、`pl3`、`ssc60` 各 2 个未来 open 期号，停售的 `manual-test` 未被补期。
- 后续动作：提交本阶段代码，归档 Trellis 任务并记录开发日志；下一阶段建议实现调度运行历史、后台可视化配置、失败重试、告警和分布式锁。

## 2026-06-02 18:55:38 HKT

- 完成任务：实现 `06-02-scheduler-history-visibility-foundation` 调度运行历史与后台可视化基础阶段，新增后端调度状态仓储、运行记录模型和 `GET /api/admin/draw-scheduler/status` 接口；管理后台“开奖期号与开奖源”页面新增“常驻调度”卡片，可查看启用状态、调度配置、最近一次运行摘要和最近运行历史。
- 解决问题：此前常驻调度启用后只能通过日志或期号变化侧面判断是否在运行，管理员无法直接看到最近是否成功、补了多少期、是否跳过停售彩种或是否失败；本次让成功和失败都写入最近 20 条内存历史，并通过 typed API、`useDrawScheduler` hook 和页面状态块展示。手动点击“运行自动任务”仍不写入常驻调度历史，避免混淆自动循环来源；用户已有的 `admin/vite.config.ts`、`backend/src/main.rs` 端口改动和 `.idea/` 继续保留，不纳入本阶段提交。
- 验证结果：`cargo fmt --check`、`cargo check`、`cargo test`、`npm run build` 均通过；后端测试增加到 58 个，覆盖调度历史成功记录、失败记录和最近 20 条保留上限。API 冒烟使用 `DRAW_SCHEDULER_ENABLED=true DRAW_SCHEDULER_INTERVAL_SECONDS=1 DRAW_SCHEDULER_FUTURE_ISSUE_COUNT=1 PORT=18087` 启动后，`GET /api/admin/draw-scheduler/status` 返回 `enabled=true`、最近运行 `SCH...` 和历史记录，`GET /api/admin/draw-issues` 自动出现 3 个未来 open 期号。浏览器验证 `http://127.0.0.1:5180/` 的“开奖期号与开奖源”页面已显示“常驻调度”“已启用”“最近运行”，控制台无错误。
- 后续动作：提交本阶段代码，归档 Trellis 任务并记录开发日志；下一阶段建议实现调度配置后台编辑、调度历史持久化、失败重试、告警、管理员审计和分布式锁。

## 2026-06-02 19:23:50 HKT

- 完成任务：实现 `06-02-admin-user-permission-foundation` 后台用户权限基础管理阶段，新增后端 `AccessRepository` 内存仓储和用户、管理员、角色权限、系统设置、注册配置接口；管理后台新增“用户权限管理”真实页面，把用户管理、管理员管理、角色权限、系统设置和用户注册入口接入可操作界面。
- 解决问题：此前这些公共功能只有 dashboard 静态摘要和占位页，无法真实维护用户、后台账号、角色范围或注册配置；本次让 dashboard 和管理页面共用同一个用户权限仓储，避免摘要与页面数据漂移。管理员保存时提交稳定 `roleId`，后端根据角色仓储回填 `roleName`，避免靠中文角色名反查；已被管理员使用的角色不能删除，注册方式不能全部关闭。用户已有的 `admin/vite.config.ts`、`backend/src/main.rs` 端口改动和 `.idea/` 继续保留，不纳入本阶段提交。
- 验证结果：`cargo fmt --check`、`cargo check`、`cargo test`、`npm run build` 均通过；后端测试增加到 63 个，覆盖用户创建与状态变更、空权限角色拒绝保存、已分配角色拒绝删除、角色改名同步管理员角色名、注册入口不能全部关闭。API 冒烟使用 `PORT=18088` 启动后，成功创建用户 `U20088`、角色 `role-audit`，更新注册配置和邮箱注册设置，dashboard 能返回 `U20088`、`role-audit`、`emailEnabled=true` 和 `agentInviteRequired=true`；删除 `role-super` 返回已分配角色冲突。浏览器验证 `http://127.0.0.1:5181/` 的用户、角色权限和系统设置视图均显示真实数据，控制台无错误。
- 后续动作：提交本阶段代码，归档 Trellis 任务并记录开发日志；下一阶段建议继续落地在线客服、机器人、邀请返利、合买配置，或推进用户权限 PostgreSQL 持久化、真实登录鉴权和管理员审计。

## 2026-06-02 19:32:52 HKT

- 完成任务：实现 `06-02-robot-configuration-foundation` 机器人配置基础管理阶段，新增后端 `RobotRepository` 内存仓储和机器人配置列表、详情、创建、更新、删除、状态接口；管理后台新增“机器人配置”真实页面，把“合买机器人”和“购彩机器人”入口接入同一套可操作页面。
- 解决问题：此前机器人只在 dashboard 静态摘要和占位页里，无法维护启停状态、适用彩种或配置说明；本次让 dashboard 和管理页面共用机器人仓储，保存时校验至少一个有效彩种并拒绝未知彩种，避免后续真实执行绑定不存在彩种。本阶段只做配置，不做真实自动发起合买、辅助满单或下投注单；用户已有的 `admin/vite.config.ts`、`backend/src/main.rs` 端口改动和 `.idea/` 继续保留，不纳入本阶段提交。
- 验证结果：`cargo fmt --check`、`cargo check`、`cargo test`、`npm run build` 均通过；后端测试增加到 66 个，覆盖机器人创建更新、空彩种拒绝和未知彩种拒绝。API 冒烟使用 `PORT=18089` 创建 `R-API-001`，启用 `R-BUY-001`，未知彩种返回业务错误，dashboard 能返回新机器人。浏览器验证 `http://127.0.0.1:5182/` 的“购彩机器人”和“合买机器人”视图显示真实数据，控制台无错误。
- 后续动作：提交本阶段代码，归档 Trellis 任务并记录开发日志；下一阶段建议继续落地在线客服、邀请返利、合买配置，或推进机器人真实执行、执行日志、风控限额、失败重试和审计。

## 2026-06-02 19:41:08 HKT

- 完成任务：实现 `06-02-rebate-configuration-foundation` 邀请返利配置基础管理阶段，新增后端 `RebateRepository` 内存仓储和返利策略查询、更新接口；管理后台新增“返利配置”真实页面，可维护代理邀请、普通用户邀请、返利模式和默认充值返利比例。
- 解决问题：此前返利策略只在 dashboard 中静态展示，“返利配置”入口仍是占位页，运营无法维护返利模式或返利比例；本次让 dashboard 和配置页面共用返利仓储，保存时校验至少保留一种邀请入口，并限制默认充值返利比例不超过 100%。本阶段只做策略配置，不做真实充值返利发放、返利流水或财务入账；用户已有的 `admin/vite.config.ts`、`backend/src/main.rs` 端口改动和 `.idea/` 继续保留，不纳入本阶段提交。
- 验证结果：`cargo fmt --check`、`cargo check`、`cargo test`、`npm run build` 均通过；后端测试增加到 69 个，覆盖返利策略更新、关闭全部邀请入口拒绝和返利比例超过 100% 拒绝。API 冒烟使用 `PORT=18090` 查询默认策略，更新为 `rechargeTiered` 和 `520` basis points，关闭全部邀请入口返回业务错误，dashboard 能返回更新后的返利策略。浏览器验证 `http://127.0.0.1:5183/` 的“返利配置”页面显示真实数据，点击保存无接口错误，控制台无错误。
- 后续动作：提交本阶段代码，归档 Trellis 任务并记录开发日志；下一阶段建议继续落地邀请关系管理、在线客服、合买配置，或推进真实充值返利发放、返利流水、代理层级和持久化。

## 2026-06-02 19:48:18 HKT

- 完成任务：实现 `06-02-support-conversation-foundation` 在线客服基础管理阶段，新增后端客服会话领域模型、`SupportRepository` 内存仓储和客服会话列表、详情、创建、更新、后台回复接口；管理后台新增“在线客服”真实页面，可查看会话、创建工单、分配客服、维护状态并追加回复。
- 解决问题：此前在线客服模块仍是 `planned` 和占位页，运营无法处理用户咨询或记录客服回复；本次将“在线客服”模块状态改为 `scaffolded`，并让新建会话校验用户存在、分配客服和回复校验管理员存在，避免前端伪造用户名或客服名。本阶段只做后台会话/工单记录，不做实时聊天、WebSocket、用户端入口或消息推送；用户已有的 `admin/vite.config.ts`、`backend/src/main.rs` 端口改动和 `.idea/` 继续保留，不纳入本阶段提交。
- 验证结果：`cargo fmt --check`、`cargo check`、`cargo test`、`npm run build` 均通过；后端测试增加到 73 个，覆盖客服会话创建、状态分配、后台回复、未知用户拒绝、未知管理员拒绝和空回复拒绝。API 冒烟使用 `PORT=18091` 创建 `CS-API-001`，分配给 `A10001`，追加后台回复，未知用户和未知管理员均返回业务错误，dashboard 中 `support` 为 `scaffolded` 且“邀请管理”仍为 `planned`。浏览器验证 `http://127.0.0.1:5184/` 的“在线客服”页面显示真实会话、列表、新建会话和消息详情，点击“保存状态”无接口错误，控制台无错误。
- 后续动作：提交本阶段代码，归档 Trellis 任务并记录开发日志；下一阶段建议继续落地邀请管理、合买配置，或推进客服实时聊天、消息持久化、SLA、自动分配和通知。

## 2026-06-02 20:03:12 HKT

- 完成任务：实现 `06-02-invite-management-foundation` 邀请管理基础阶段，新增后端邀请关系领域模型、`InviteRepository` 内存仓储和邀请关系列表、详情、创建、更新接口；管理后台新增“邀请管理”真实页面，可查看代理邀请关系、创建邀请关系、维护状态、返利资格和备注。
- 解决问题：此前“邀请管理”仍是 `planned` 和占位页，代理与下级用户关系无法维护；本次让创建邀请关系校验邀请人和被邀请人存在、默认策略下只允许代理邀请、邀请人与被邀请人不能相同、重复关系和重复邀请码会被拒绝，避免后续返利链路绑定错误关系。本阶段只做邀请关系管理，不做真实充值返利发放、返利流水或财务入账；用户已有的 `admin/vite.config.ts`、`backend/src/main.rs` 端口改动和 `.idea/` 继续保留，不纳入本阶段提交。
- 验证结果：`cargo fmt --check`、`cargo check`、`cargo test`、`npm run build` 均通过；后端测试增加到 77 个，覆盖代理创建与更新、普通用户默认拒绝、未知被邀请人拒绝和重复邀请码拒绝。API 冒烟使用 `PORT=18092` 查询邀请关系、创建临时用户 `U20092` 后创建 `INV-API-001`，更新为停用，普通用户邀请返回 forbidden，dashboard 中 `invite` 为 `scaffolded`。浏览器验证 `http://127.0.0.1:5185/` 的“邀请管理”页面显示真实数据，点击“保存邀请关系”无接口错误，控制台无错误。
- 后续动作：提交本阶段代码，归档 Trellis 任务并记录开发日志；下一阶段建议继续推进合买计划/合买配置真实工作流，或推进真实充值返利发放、代理层级树、邀请码生成服务和持久化。

## 2026-06-02 20:18:43 HKT

- 完成任务：实现 `06-02-group-buy-management-foundation` 合买配置与计划基础阶段，新增后端合买计划领域模型、`GroupBuyRepository` 内存仓储和合买计划列表、详情、创建、状态维护、添加参与记录接口；管理后台新增“合买配置”真实页面，可查看计划、创建计划、维护状态、查看参与记录并追加参与金额。
- 解决问题：此前“合买配置”入口仍走占位页，dashboard 的 `groupBuyPlans` 也是静态假数据；本次让 dashboard 和页面共用合买仓储，创建计划时校验彩种存在且开启合买、发起人存在、金额能按最小份额拆分、发起人认购满足彩种最低比例，添加参与记录时校验用户存在、金额满足参与最低金额且不能超额，满额后自动进入 `filled`。本阶段只做后台计划与参与记录管理，不做真实投注订单、资金冻结/扣款、撤单退款或中奖分账；用户已有的 `admin/vite.config.ts`、`backend/src/main.rs` 端口改动和 `.idea/` 继续保留，不纳入本阶段提交。
- 验证结果：`cargo fmt --check`、`cargo check`、`cargo test`、`npm run build` 均通过；后端测试增加到 82 个，覆盖创建计划、禁用合买彩种拒绝、发起人认购不足拒绝、添加参与记录后满单和超额参与拒绝。API 冒烟使用 `PORT=18093` 创建 `G-API-001`，更新备注，添加 `G-API-001-P002` 后自动满单，`manual-test` 禁用合买返回业务错误，超额参与返回业务错误，dashboard 能返回新计划。浏览器验证 `http://127.0.0.1:5186/` 的“合买配置”页面显示真实计划、可保存计划状态、可添加参与记录 `G202606020001-P003`，控制台无错误；截图保存到 `/tmp/bc-group-buy-smoke.png`。
- 后续动作：提交本阶段代码，归档 Trellis 任务并记录开发日志；下一阶段建议继续推进合买真实投注订单、资金冻结扣款、撤单退款、中奖分账、手机端参与入口或合买机器人真实执行。

## 2026-06-02 20:25:03 HKT

- 完成任务：实现 `06-02-scheduler-configuration-editing` 调度配置后台编辑阶段，新增 `PUT /api/admin/draw-scheduler/config`，让后台可保存常驻调度启用状态、执行周期、未来期号缓冲和封盘提前秒数；“开奖期号与开奖源”页面的“常驻调度”卡片新增配置表单和保存按钮。
- 解决问题：此前常驻调度配置只能通过环境变量初始化，后台只能查看不能修改；本次让 `DrawSchedulerRepository` 支持读取和更新配置，并让已启动的后台循环每轮读取最新配置，`enabled=false` 会跳过自动任务，`futureIssueCount`、`saleCloseLeadSeconds` 和下一轮 `intervalSeconds` 可在当前进程内热生效。本阶段仍不做配置持久化、发布审批、回滚、动态启动/停止后台循环或分布式锁；用户已有的 `admin/vite.config.ts`、`backend/src/main.rs` 端口改动和 `.idea/` 继续保留，不纳入本阶段提交。
- 验证结果：`cargo fmt --check`、`cargo check`、`cargo test`、`npm run build` 均通过；后端测试增加到 83 个，覆盖有效配置更新和无效执行周期拒绝。API 冒烟使用 `PORT=18094` 保存 `enabled=true`、`intervalSeconds=5`、`futureIssueCount=3`、`saleCloseLeadSeconds=20` 后状态接口立即回显，无效 `intervalSeconds=0` 返回业务错误。浏览器验证 `http://127.0.0.1:5187/` 的“常驻调度”配置表单显示最新配置，点击“保存配置”无接口错误，控制台无错误；截图保存到 `/tmp/bc-scheduler-config-smoke.png`。
- 后续动作：提交本阶段代码，归档 Trellis 任务并记录开发日志；下一阶段建议继续推进调度配置持久化、管理员审计、动态启动/停止、失败告警、分布式锁，或转入真实登录鉴权和权限拦截。

## 2026-06-02 21:06:19 HKT

- 完成任务：实现 `06-02-access-maintenance-sidesheet` 用户权限维护侧边栏阶段，将用户管理的“用户维护”、管理员管理的“账号维护”和角色权限的“角色维护”从页面常驻表单改为点击新建或编辑后通过 SideSheet 打开。
- 解决问题：此前用户权限管理页面采用列表与维护表单并排布局，用户维护、账号维护和角色维护会直接显示在页面上，占用列表扫描空间，也不符合用户要求的抽屉式维护方式；本次保留列表主界面，新增“新建用户”“新建账号”“新建角色”入口，并让编辑入口打开对应抽屉。保存用户、账号、角色或删除角色成功后关闭抽屉，切换子模块时自动关闭已打开抽屉。本阶段不修改后端接口、数据模型和权限校验；用户已有的 `admin/vite.config.ts`、`backend/src/main.rs` 端口改动和 `.idea/` 继续保留，不纳入本阶段提交。
- 验证结果：`npm run build` 通过；浏览器验证 `http://127.0.0.1:5188/` 的“用户权限管理”页面中“新建用户”“新建账号”“新建角色”均能打开对应 SideSheet，页面常驻卡片中不再直接显示“用户维护”“账号维护”“角色维护”。控制台仅出现 Vite/React 开发提示和一个资源 404，不影响本次功能；截图保存到 `/tmp/bc-access-sidesheet-smoke.png`。
- 后续动作：提交本阶段代码，归档 Trellis 任务并记录开发日志；下一阶段建议继续推进真实登录鉴权、角色权限拦截、管理员操作审计和用户权限持久化。

## 2026-06-02 21:16:22 HKT

- 完成任务：实现 `06-02-support-chat-component` 客服会话使用 Semi Chat 阶段，将“在线客服”页面的消息记录从手写消息卡片列表改为 Semi UI `Chat` 组件展示。
- 解决问题：此前客服会话消息流是自定义 `div` 卡片列表，不符合用户要求的 `import { Chat } from '@douyinfe/semi-ui';` 组件化会话展示；本次把用户消息映射为 `user`、客服回复映射为 `assistant`、系统消息映射为 `system`，并在标题中保留作者类型、作者名称和消息时间。后台回复输入仍沿用原有业务表单，`Chat` 默认输入区和上传能力已关闭，避免出现重复输入框或 Semi Upload 警告。本阶段不修改后端接口、消息模型、回复保存逻辑或实时聊天能力；用户已有的 `admin/vite.config.ts`、`backend/src/main.rs` 端口改动和 `.idea/` 继续保留，不纳入本阶段提交。
- 验证结果：`npm run build` 通过；引入 `Chat` 后 Vite 仍提示生产 chunk 超过 500 kB，当前主 JS 约 1.58 MB，属于组件依赖体积提示。浏览器验证 `http://127.0.0.1:5189/` 的“在线客服”页面已渲染 `.semi-chat`，用户/客服消息内容可读，Upload 警告已消失；控制台仅剩 Vite/React 开发提示和一个资源 404，不影响本次功能；截图保存到 `/tmp/bc-support-chat-smoke.png`。
- 后续动作：提交本阶段代码，归档 Trellis 任务并记录开发日志；下一阶段建议继续推进客服实时聊天、消息持久化、SLA、自动分配、快捷回复和前端路由级懒加载。

## 2026-06-02 21:20:41 HKT

- 完成任务：实现 `06-02-support-reply-only` 在线客服仅回复用户会话阶段，移除管理后台“在线客服”页面的“新建会话”表单和创建会话逻辑。
- 解决问题：此前后台客服页面仍允许管理员主动新建会话，但用户要求在线客服只需要回复用户过来的信息；本次删除“新建会话”“创建会话”“会话 ID”“绑定用户”“首条消息”等后台创建入口，页面只保留用户会话列表、会话详情、状态维护、客服分配、Semi UI `Chat` 消息记录和后台回复表单。`useSupportConversations` 也不再暴露后台创建会话函数或为创建表单加载用户列表。后端创建会话接口暂时保留，供未来用户端客服入口或测试数据入口使用；用户已有的 `admin/vite.config.ts`、`backend/src/main.rs` 端口改动和 `.idea/` 继续保留，不纳入本阶段提交。
- 验证结果：`npm run build` 通过；浏览器验证 `http://127.0.0.1:5191/` 的“在线客服”页面已显示“用户会话”，不再出现“新建会话”“创建会话”“会话 ID”“首条消息”，`.semi-chat` 和“发送回复”按钮仍正常显示。控制台仅剩 Vite/React 开发提示和一个资源 404，不影响本次功能；截图保存到 `/tmp/bc-support-reply-only-smoke.png`。
- 后续动作：提交本阶段代码，归档 Trellis 任务并记录开发日志；下一阶段建议继续推进用户端发起客服会话入口、实时消息、已读回执、快捷回复和客服转接。

## 2026-06-02 21:30:07 HKT

- 完成任务：实现 `06-02-lottery-console` 彩种控制台实时看板阶段，新增后台“彩种控制台”页面，并把入口加入 dashboard 模块清单、侧边栏和工作台模块卡片。
- 解决问题：此前运营需要分别进入彩种管理和开奖期号页面才能判断每个彩种的当前期号、封盘/开奖时间和开奖结果，缺少一个按彩种扫描的实时总览；本次新增 `useLotteryConsole` hook 并发拉取彩种与开奖期号，页面每秒本地刷新倒计时、每 10 秒轮询服务端数据，按彩种展示销售状态、当前 open/closed 期号、封盘倒计时、开奖倒计时和最近开奖号码。开奖号码继续保持英文逗号分隔格式；用户已有的 `admin/vite.config.ts`、`backend/src/main.rs` 端口改动和 `.idea/` 继续保留，不纳入本阶段提交。
- 验证结果：`npm run build`、`cargo fmt --check`、`cargo check`、`cargo test` 均通过；后端测试 83 个全绿。浏览器验证 `http://127.0.0.1:5192/` 的“彩种控制台”入口可打开，使用 API 创建 `60 秒时时彩` open 期号和 `福彩 3D` 已开奖期号后，页面显示 `CONSOLE-OPEN-20260602212934`、`CONSOLE-DRAWN-20260602212934` 和英文逗号开奖号码 `2,0,3`，倒计时从 `00:00:46` 递减到 `00:00:44`；截图保存到 `/tmp/bc-lottery-console-smoke.png`。
- 后续动作：提交本阶段代码，归档 Trellis 任务并记录开发日志；下一阶段建议继续推进开奖期号持久化、控制台告警、WebSocket/SSE 实时推送或后台真实登录鉴权。

## 2026-06-02 21:42:00 HKT

- 完成任务：实现 `06-02-admin-auth-permission-foundation` 后台登录鉴权与权限拦截基础阶段，新增后台登录页、登录/当前管理员/登出接口、内存 Bearer Token 会话和按角色权限过滤菜单/工作台模块。
- 解决问题：此前管理后台所有 `/api/admin/**` 接口和前端页面都可以直接访问，角色权限只停留在维护数据里，没有参与登录态、菜单入口或 API 拦截；本次让登录成功后前端保存 token 并自动附加到 API 请求，后端中间件按路径映射 `PermissionScope`，缺 token 返回 401、权限不足返回 403，应用外壳显示当前管理员和角色并支持登出。当前仍使用无数据库阶段的演示密码 `admin123`，后续需要替换为密码哈希和持久化凭据；用户已有的 `admin/vite.config.ts`、`backend/src/main.rs` 端口改动和 `.idea/` 继续保留，不纳入本阶段提交。
- 验证结果：`cargo fmt --check`、`cargo check`、`cargo test`、`npm run build` 均通过；后端测试增加到 87 个，覆盖登录成功、锁定管理员拒绝、登出 token 失效和路由权限映射。API 冒烟使用 `PORT=18096` 确认无 token 请求 `/api/admin/dashboard` 返回 401，`admin/admin123` 登录成功并可访问 `/api/admin/auth/me`，`locked_admin/admin123` 返回 403，临时 `role-ops` 管理员可访问 `/api/admin/users` 但访问 `/api/admin/admins` 返回 403。浏览器验证 `http://127.0.0.1:5193/` 未登录显示“管理员登录”，登录后进入系统概览并显示 `admin/超级管理员/登出`，点击登出回到登录页，控制台无 warning/error；截图保存到 `/tmp/bc-auth-login-smoke.png`。
- 后续动作：提交本阶段代码，归档 Trellis 任务并记录开发日志；下一阶段建议推进密码哈希与密码重置、权限数据持久化、按钮级权限、管理员操作审计和 dashboard 敏感数据裁剪。

## 2026-06-03 01:03:51 HKT

- 完成任务：实现 `06-03-dashboard-permission-filtering` dashboard 数据按权限裁剪阶段，新增后端 `dashboard_summary_for_scopes`，让 `/api/admin/dashboard` 根据当前管理员登录会话的 `PermissionScope` 返回允许看到的模块、指标和摘要数据。
- 解决问题：此前 dashboard 虽然需要登录，但为了作为系统概览入口没有绑定单一业务权限，低权限管理员仍可能通过 dashboard 响应看到管理员、角色、财务、机器人、返利等无权限领域摘要；本次保持 `DashboardSummary` 顶层字段不变，对无权限数组返回空数组，对财务、注册配置、邀请返利等对象返回置零或关闭状态，并在模块组和指标层同步过滤。用户已有的 `admin/vite.config.ts`、`backend/src/main.rs` 端口改动和 `.idea/` 继续保留，不纳入本阶段提交。
- 验证结果：`cargo fmt --check`、`cargo check`、`cargo test`、`npm run build` 均通过；后端测试增加到 89 个，覆盖运营 scopes 裁剪和超级管理员全量保留。API 冒烟使用 `PORT=18097` 创建临时 `role-ops` 管理员后确认运营 dashboard 只返回 `users`、`orders`、`lotteries` 指标和用户/订单/彩票模块，管理员、角色、系统设置、财务、客服、机器人、邀请返利模块均不返回，财务金额为 `0`。
- 后续动作：提交本阶段代码，归档 Trellis 任务并记录开发日志；下一阶段建议推进密码哈希、权限持久化、按钮级权限、管理员操作审计，或继续补齐后台剩余真实业务流程。

## 2026-06-03 01:59:30 HKT

- 完成任务：实现 `06-03-admin-password-hash-reset` 管理员密码哈希与重置基础阶段，新增 Argon2id 密码哈希、管理员独立密码哈希存储、管理员保存请求 DTO 和 `PATCH /api/admin/admins/{id}/password` 重置密码接口；管理后台“账号维护” SideSheet 新增初始密码/重置密码输入。
- 解决问题：此前所有后台管理员共用内存全局演示密码 `admin123`，新建账号没有独立密码，也无法在后台维护密码；本次让登录按管理员 ID 校验各自的密码哈希，创建账号可设置初始密码，编辑账号可留空不改密码或填写新密码触发重置。管理员列表、详情、dashboard、auth/me 和登录响应仍只返回 `AdminSummary`，不暴露密码哈希或明文密码；用户已有的 `admin/vite.config.ts`、`backend/src/main.rs` 端口改动和 `.idea/` 继续保留，不纳入本阶段提交。
- 验证结果：`cargo fmt --check`、`cargo check`、`cargo test`、`npm run build` 均通过；后端测试增加到 93 个，覆盖错误密码、锁定账号、新建账号独立密码、重置密码和短密码拒绝。API 冒烟使用 `PORT=18098` 创建 `A-PASS-001/pass_ops`，初始密码可登录，重置后旧密码返回 401，新密码可登录，并确认管理员列表、dashboard 管理员摘要和 auth/me 中没有密码字段。
- 后续动作：提交本阶段代码，归档 Trellis 任务并记录开发日志；下一阶段建议推进用户权限 PostgreSQL 持久化、登录失败锁定、敏感操作审计和密码重置通知。

## 2026-06-03 09:05:33 HKT

- 完成任务：实现 `06-03-api68-draw-source` API68 福彩 3D 开奖源接入阶段，新增后端 API 开奖源服务，应用启动时为 `fc3d` 注入 `api68-fc3d`，并让手动触发开奖和自动开奖任务都复用同一个外部源解析流程。
- 解决问题：此前 `api` 开奖模式仍使用本地生成器，无法按真实第三方 API 拉取开奖结果，也可能在外部结果缺失时生成假号码；本次让 `fc3d` 按 `preDrawIssue` 匹配 API68 响应中的期号，使用 `preDrawCode` 作为开奖号码，并继续统一保存英文逗号分隔格式。API68 未命中期号或请求失败时不回退生成器，手动开奖返回统一错误，自动任务把期号写入 `skippedIssues` 后继续处理其他期号。用户已有的 `admin/vite.config.ts`、`backend/src/main.rs` 端口改动和 `.idea/` 继续保留，不纳入本阶段提交。
- 验证结果：`cargo fmt --check`、`cargo check`、`cargo test`、`npm run build` 均通过；后端测试增加到 101 个，覆盖 API68 响应解析、数字/字符串期号匹配、业务失败、开奖仓储使用 API68、外部源未命中保持期号未开奖、自动任务跳过 API 失败期号。API 冒烟使用 `PORT=18100` 登录后确认 `GET /api/admin/draw-sources` 返回 `api68-fc3d`，创建 `fc3d/2026143` 后触发开奖回填 `3,7,6`，创建 `fc3d/2099999` 后触发开奖返回 404 且期号仍无开奖号码，自动任务对该期写入 `skippedIssues`，同时平台 `ssc60` 期号正常开奖和结算。
- 后续动作：提交本阶段代码，归档 Trellis 任务并记录开发日志；下一阶段建议推进开奖源配置 CRUD、API68 原始响应留痕、失败重试队列、人工复核、排列 3 复用福彩 3D 结果映射和开奖期号持久化。

## 2026-06-03 09:41:11 HKT

- 完成任务：实现 `06-03-draw-source-reuse-config` 开奖源配置与多彩种复用阶段，新增 API 开奖源配置的列表、创建、更新、删除接口，并把默认 API68 来源升级为 `fc3d` 和 `pl3` 共同复用的配置。
- 解决问题：此前 API68 来源仍是硬编码单彩种绑定，后台只能查看不能配置，也无法让排列 3 复用福彩 3D 的 API 结果；本次把 API 来源改为内存配置仓储，按 `reusableForLotteryIds` 绑定多个 API 开奖彩种，保存时校验彩种存在、必须为 API 开奖模式，并禁止同一彩种绑定多个来源以避免开奖歧义。管理后台“开奖期号与开奖源”页面新增“开奖源配置”面板，可维护名称、provider、lotCode、endpoint 和复用彩种；平台生成器仍只读展示。用户已有的 `admin/vite.config.ts`、`backend/src/main.rs` 端口改动和 `.idea/` 继续保留，不纳入本阶段提交。
- 验证结果：`cargo fmt --check`、`cargo check`、`cargo test`、`npm run build` 均通过；后端测试增加到 106 个，覆盖默认 API68 同时绑定 `fc3d/pl3`、重复彩种绑定拒绝、拆分复用彩种配置、非 API 彩种拒绝绑定、`pl3/2026143` 复用 API68 返回 `3,7,6`。API 冒烟使用 `PORT=18101` 登录后确认 `GET /api/admin/draw-sources` 返回 `api68-fc3d` 且复用彩种为 `fc3d/pl3`，重复绑定 `fc3d` 的新来源返回 409，创建 `pl3/2026143` 并开奖回填 `3,7,6`，保存来源为仅 `fc3d` 后再改回 `fc3d/pl3` 均成功。前端浏览器自动化因项目未安装 Playwright 且本轮无可用 browser 工具未执行，已用 `npm run build` 完成类型与生产构建验证。
- 后续动作：提交本阶段代码，归档 Trellis 任务并记录开发日志；下一阶段建议推进开奖源配置 PostgreSQL 持久化、API68 原始响应留痕、失败重试队列、不同彩种期号映射和更多 provider 接入。

## 2026-06-03 09:57:06 HKT

- 完成任务：实现 `06-03-fc3d-issue-generation` 福彩 3D 真实期号生成修复阶段，新增 API68 最新 `preDrawIssue` 解析，并让福彩 3D、排列 3 在预览、单期生成、批量生成和常驻调度补期时使用真实 7 位期号递增。
- 解决问题：此前期号生成服务对所有彩种统一使用开奖时间 `YYYYMMDDHHMMSS`，福彩 3D 会生成 `20260603210015` 这类内部时间戳期号，后续无法匹配 API68 的 `preDrawIssue`；本次改为有 API68 来源时以外部最新期号为基线，例如最新 `2026143` 时生成 `2026144`，本地已有 `2026144` 后继续生成 `2026145`。常驻调度遇到 API 最新期号缺失时只跳过对应彩种并记录原因，不再让整轮补期失败。用户已有的 `admin/vite.config.ts`、`backend/src/main.rs` 端口改动和 `.idea/` 继续保留，不纳入本阶段提交。
- 验证结果：`cargo fmt --check`、`cargo check`、`cargo test` 均通过；后端测试增加到 112 个，覆盖 API68 最新期号解析、`fc3d` 生成 `2026144/2026145`、`pl3` 复用生成 `2026144`、本地已有真实期号继续递增、调度跳过 API 期号生成失败彩种。API 冒烟使用 `PORT=18102` 登录后确认真实 API68 当前最新 `2026143`，`preview-generation` 返回 `2026144`、`2026145`，`generate-next` 先后创建 `2026144` 和 `2026145`，`pl3` 预览返回 `2026144`。
- 后续动作：提交本阶段代码，归档 Trellis 任务并记录开发日志；下一阶段建议推进开奖源配置和最新期号基线 PostgreSQL 持久化、API68 原始响应/生成基线审计、休市日复核和失败重试。

## 2026-06-03 10:21:09 HKT

- 完成任务：实现 `06-03-draw-management-page-ux` 开奖期号与开奖源页面优化阶段，把原先长页面重排为“概览指标 + 期号管理 / 开奖源配置 / 自动任务与调度”三段式工作区。
- 解决问题：此前期号列表、创建期号、开奖执行、开奖源维护、自动任务和调度配置全部平铺，页面首屏拥挤且维护表单长期占用列表扫描空间；本次把创建期号、执行开奖、开奖源维护和调度配置移动到 Semi UI `SideSheet`，主页面保留列表、卡片摘要、状态和操作入口。创建期号表单也不再默认填入旧期号 `20260602001`，避免继续误导福彩 3D 真实期号录入。用户已有的 `admin/vite.config.ts`、`backend/src/main.rs` 端口改动和 `.idea/` 继续保留，不纳入本阶段提交。
- 验证结果：`npm run build` 通过，仅保留既有 chunk size warning；使用 `npm run dev -- --host 127.0.0.1 --port 5196` 启动前端后，`curl -I http://127.0.0.1:5196/` 返回 HTTP 200。当前环境没有可用浏览器检查工具，未执行截图级视觉验证。
- 后续动作：提交本阶段代码，归档 Trellis 任务并记录开发日志；下一阶段建议补期号筛选、状态筛选、异常期号高亮和浏览器级响应式截图验证。

## 2026-06-03 11:01:50 HKT

- 完成任务：实现 `06-03-lottery-console-status-filter` 彩种控制台状态筛选阶段，在“彩种控制台”新增本地状态筛选条。
- 解决问题：此前彩种控制台只能一次性展示所有彩种，运营无法快速聚焦销售开启、已停售、开盘中、待开奖、已开奖或无当前期的彩种；本次新增全部、销售开启、已停售、开盘中、待开奖、已开奖、无当前期筛选项，并在每个筛选项展示匹配数量，筛选结果即时更新卡片列表。用户已有的 `admin/vite.config.ts`、`backend/src/main.rs` 端口改动和 `.idea/` 继续保留，不纳入本阶段提交。
- 验证结果：`npm run build` 通过，仅保留既有 chunk size warning。
- 后续动作：提交本阶段代码，归档 Trellis 任务并记录开发日志；下一阶段建议补按彩种名称搜索、异常期号提示、封盘临近告警和浏览器级截图验证。

## 2026-06-03 11:07:45 HKT

- 完成任务：启动 `06-03-user-management-invite-code` 用户管理显示邀请码阶段，补充任务 PRD 与跨层实现/检查上下文。
- 解决问题：明确用户管理页只需要 `users` 权限，不能额外依赖需要 `rebates` 权限的邀请管理接口；邀请码展示应由后端从邀请关系按邀请人聚合后随用户接口返回，用户维护表单不直接编辑该派生字段。
- 后续动作：完成后端 `inviteCodes` 字段、前端用户表格展示、契约文档更新，并运行后端与前端验证。

## 2026-06-03 11:10:11 HKT

- 完成任务：实现 `06-03-user-management-invite-code` 用户管理显示邀请码阶段，用户列表新增“邀请码”列，并让用户相关接口返回只读 `inviteCodes`。
- 解决问题：此前用户管理只能看到上级代理 ID，无法直接确认代理用户拥有哪些邀请码；本次由 `InviteRepository` 按邀请人聚合邀请码，`/api/admin/users`、用户详情、创建、更新和状态变更响应统一补齐 `inviteCodes`。用户保存时清空并忽略请求中的邀请码数组，避免把邀请关系派生字段写入用户仓储；用户管理页不调用需要 `rebates` 权限的邀请管理接口，低权限运营账号仍可正常查看用户列表。
- 验证结果：`cargo fmt --check`、`cargo check`、`cargo test`、`npm run build` 均通过；后端测试增加到 113 个，覆盖邀请码按邀请人聚合。API 冒烟使用 `PORT=18103` 登录后请求 `/api/admin/users`，确认 `U90001/agent_alpha` 返回 `inviteCodes=["KJHGFDSA","QWERTYPA"]`。前端 dev server `http://127.0.0.1:5197/` 返回 HTTP 200；本轮没有可用浏览器自动化工具，未做截图级页面验证。
- 后续动作：提交本阶段代码，归档 Trellis 任务并记录开发日志；下一阶段建议推进用户管理按邀请码搜索、邀请关系详情跳转、邀请码生成服务和邀请数据持久化。

## 2026-06-03 11:28:12 HKT

- 完成任务：启动 `06-03-06-03-user-code-cn-logs-au5` 全员邀请码、中文日志与澳洲 5 分彩接入阶段，补充任务 PRD 与跨层实现/检查上下文。
- 解决问题：明确上一阶段按邀请关系聚合 `inviteCodes` 不符合“每个用户都有邀请码”的最新业务要求；本阶段改为用户固定 `inviteCode`，代理码可邀请、普通用户码提示无效，同时补后台中文日志和澳洲 5 分彩 API68 来源。
- 后续动作：完成后端模型、邀请校验、开奖源、前端展示、文档更新，并运行后端与前端验证。

## 2026-06-03 11:31:19 HKT

- 完成任务：实现 `06-03-06-03-user-code-cn-logs-au5` 全员邀请码、中文日志与澳洲 5 分彩接入阶段。
- 解决问题：此前用户管理的邀请码来自邀请关系聚合，不能保证每个用户都有自己的邀请码，也会让同一代理因多条邀请关系显示多个码；本次改为每个用户固定单个 `inviteCode`，新建用户自动生成邀请码且校验唯一。邀请关系创建时邀请码必须属于代理用户，普通用户码或不存在的码返回“邀请码无效”，同一个代理码可用于多个不同被邀请人。后台 `tracing`/`panic!` 日志 message 已改为中文。新增 `au5` 澳洲 5 分彩和默认 `api68-au5` 来源，endpoint 为 `https://api.api68.com/CQShiCai/getBaseCQShiCaiList.do`、`lotCode=10010`，并支持 8 位数字 API 期号递增。
- 验证结果：`cargo fmt --check`、`cargo check`、`cargo test`、`npm run build` 均通过；后端测试增加到 117 个，覆盖重复邀请码拒绝、普通用户邀请码无效、代理码复用、澳洲 5 分彩种子彩种、`api68-au5` 默认来源和 8 位 API 期号生成。API 冒烟使用 `PORT=18104` 登录后确认三个种子用户均返回 `inviteCode`，普通用户示例码 `ZXCVBNML` 创建邀请关系返回 `bad request: 邀请码无效`，代理示例码 `KJHGFDSA` 可创建新邀请关系；`GET /api/admin/draw-sources` 返回 `api68-au5`，`GET /api/admin/lotteries/au5` 返回 300 秒 API 彩种；真实 API68 最新期号 `51320851` 回填开奖号码 `7,0,1,3,9`。
- 后续动作：提交本阶段代码，归档 Trellis 任务并记录开发日志；下一阶段建议推进邀请码生成/重置/冻结审计、澳洲 5 分彩开奖源持久化和 API68 原始响应留痕。

## 2026-06-03 11:48:23 HKT

- 完成任务：启动 `06-03-06-03-docker-github-publish` Docker 单镜像打包与 GitHub 上传阶段，补充任务 PRD 与检查上下文。
- 解决问题：明确部署目标为前后端同一个项目镜像，使用 Nginx 服务前端并反向代理后端 `/api`；GitHub 上传当前缺少 remote，需要后续提供远端仓库地址或创建仓库后再推送。
- 后续动作：新增 Dockerfile、Nginx 配置、启动脚本、Compose 和部署说明，验证镜像构建/运行后提交；拿到 GitHub remote 后执行推送。

## 2026-06-03 12:00:54 HKT

- 完成任务：实现 `06-03-06-03-docker-github-publish` Docker 单镜像打包阶段，新增根目录 `Dockerfile`、`.dockerignore`、`docker/nginx.conf`、`docker/entrypoint.sh`、`docker-compose.yml`、中文 `部署说明.md`，并新增 `.trellis/spec/backend/deployment-guidelines.md` 容器部署规范。
- 解决问题：此前项目没有统一容器部署入口，前端、后端需要分别启动；本次改为单镜像多阶段构建，前端使用 Node 构建静态资源，后端使用 Rust 构建 release 二进制，运行时由 Nginx 对外服务前端并反向代理 `/api/` 到同容器后端。入口脚本会按 `BACKEND_PORT` 动态渲染 Nginx 反代端口，并校验端口必须为数字，避免后端端口环境变量与 Nginx 配置不一致。
- 验证结果：`cargo fmt --check`、`cargo check`、`cargo test`、`npm run build` 均通过；`docker build -t bc-platform:latest .` 成功生成 `bc-platform:latest` 镜像，镜像大小约 216MB。临时容器使用 `docker run -d --name bc-platform-smoke -p 18085:80 bc-platform:latest` 启动后状态为 `healthy`，`curl -I http://127.0.0.1:18085/` 返回 200，`curl http://127.0.0.1:18085/api/health` 返回后端健康检查成功；临时容器已清理。
- 后续动作：提交本阶段 Docker 与部署文档改动；当前仓库尚未配置 GitHub remote，需要提供 GitHub 仓库地址或允许创建仓库后再执行 `git push -u origin main`。

## 2026-06-03 12:04:05 HKT

- 完成任务：完成 GitHub 上传阶段，使用已登录的 GitHub 账号 `sydneypoole` 创建私有仓库 `sydneypoole/bc`，配置 `origin` 并推送 `main` 分支。
- 解决问题：此前仓库没有 remote，无法执行上传；本次确认 `origin=https://github.com/sydneypoole/bc.git`，并完成 `main -> origin/main` 的首次推送。
- 后续动作：如后续需要把 Docker image 推到 GitHub Container Registry，可在仓库中继续补 GitHub Actions 和 GHCR 发布配置。

## 2026-06-03 12:10:30 HKT

- 完成任务：启动 `06-03-github-workflow-ci-ghcr` GitHub Actions CI 与 Docker 镜像发布阶段，补充任务 PRD 和部署规范上下文。
- 解决问题：确认仓库缺少 `.github/workflows`，无法在 push/PR 时自动检查，也无法把 Docker 单镜像发布到 GHCR。
- 后续动作：新增 CI workflow，更新架构设计、部署说明和容器部署规范，并在本地完成基础检查后提交推送。

## 2026-06-03 12:12:03 HKT

- 完成任务：实现 `06-03-github-workflow-ci-ghcr` GitHub Actions CI 与 Docker 镜像发布阶段，新增 `.github/workflows/ci.yml`。
- 解决问题：此前 GitHub 仓库没有自动化流水线；本次 workflow 在 `push`、`pull_request` 和手动触发时运行前后端质量检查，并构建 Docker 单镜像。`main` 分支 push 时使用 `GITHUB_TOKEN` 登录 GHCR，推送 `ghcr.io/sydneypoole/bc:latest` 和 `sha-<提交短哈希>` 标签；PR 只构建不推送，避免未合并代码覆盖发布镜像。
- 验证结果：workflow YAML 解析通过；`cargo fmt --check`、`cargo check`、`cargo test`、`npm run build` 均通过；`docker build -t bc-platform:latest .` 通过并命中缓存。前端构建仍保留既有 chunk size warning。
- 后续动作：提交并推送本阶段 workflow；推送后在 GitHub Actions 页面确认 `CI` 工作流通过，并在 GHCR 包页面确认镜像标签生成。

## 2026-06-03 12:24:57 HKT

- 完成任务：优化 GitHub Actions action 版本，按 GitHub API 查询结果升级到 `actions/checkout@v6`、`actions/setup-node@v6`、`actions/cache@v5`、`docker/setup-buildx-action@v4`、`docker/login-action@v4`、`docker/metadata-action@v6`、`docker/build-push-action@v7`，并显式启用 Node.js 24 action runtime。
- 解决问题：第一次云端 CI 已通过并成功发布 GHCR 镜像，但 GitHub 提示旧版 action 运行在 Node.js 20，2026-06-16 后会强制切到 Node.js 24；本次提前升级，降低后续 workflow 警告和运行时兼容风险。
- 后续动作：提交并推送 action 版本升级，重新观察 GitHub Actions 运行状态。

## 2026-06-03 数据库持久化接入

- 完成任务：启动 `06-03-database-persistence` 数据库持久化接入阶段，确认当前只有彩种管理已经有 PostgreSQL 仓储和 migrations，其它后台模块仍是内存仓储。
- 解决问题：此前 `docker compose up --build` 只启动应用容器，不会配置 `DATABASE_URL`，因此即使镜像支持 PostgreSQL，部署仍默认走内存模式；本次把 Compose 改为同时启动 PostgreSQL，并把应用连接到 Compose 内数据库。
- 后续动作：验证 Compose 模式下 PostgreSQL healthcheck、应用健康检查和 `lotteries` 表 migrations；随后提交本阶段改动，并继续规划用户、订单、开奖、资金、权限等模块的 PostgreSQL 持久化。

## 2026-06-03 12:58 HKT 数据库持久化接入验证

- 完成任务：完成 Compose 数据库接入验证，`docker compose up -d --build` 已能启动 PostgreSQL 与同一个前后端应用镜像，`APP_PORT=18081 docker compose up -d --build` 已验证宿主机端口可覆盖。
- 解决问题：本机 `8080` 和 `18080` 已被其它进程占用，固定端口会干扰本地部署验证；本次把 Compose 端口改为 `${APP_PORT:-8080}:80`，默认不变，冲突时可以切换端口。
- 验证结果：`cargo fmt --check`、`cargo check`、`cargo test`、`npm run build` 均通过；Compose 中应用和 PostgreSQL 均为 healthy，`/api/health` 返回成功，PostgreSQL 已生成 `_sqlx_migrations` 和 `lotteries` 表，并能查询到 `au5`、`fc3d`、`pl3`、`ssc60` 等彩种。
- 后续动作：提交并推送本阶段改动；下一阶段继续把用户、订单、开奖期号、开奖源、资金、权限等内存仓储分批迁移到 PostgreSQL。

## 2026-06-03 13:07 HKT 邀请码、中文日志与澳洲 5 分彩采集修正

- 完成任务：启动 `06-03-invite-au5-collection` 修正阶段，针对最新要求复查全员邀请码、普通用户邀请码无效、后台中文日志和澳洲 5 分彩采集接口。
- 解决问题：用户维护 SideSheet 保存用户时此前没有携带 `inviteCode`，编辑已有用户会把原邀请码覆盖为后端自动生成值；邀请管理新增关系仍需手填邀请码，容易填错普通用户码或临时码；开奖源新建表单没有澳洲 5 分彩采集预设。
- 已完成修正：用户维护表单新增邀请码字段并保留原值；邀请管理按所选邀请人自动带出邀请码且只展示代理邀请人；后端日志错误字段改为中文化 `ApiError::log_message()`；开奖源维护新增“澳洲 5 分彩采集”预设，自动填入 `CQShiCai/getBaseCQShiCaiList.do`、`lotCode=10010` 和 `au5`。
- 验证结果：`cargo fmt --check`、`cargo check`、`cargo test`、`npm run build` 均通过；后端测试增加到 119 个，新增覆盖 `ApiError` 中文日志描述；前端构建仍只有既有 chunk size warning。
- 后续动作：提交并推送本阶段改动；后续可继续补邀请码重置审计、开奖源连通性测试和 API68 原始响应留痕。

## 2026-06-03 13:52 HKT 彩种控制台控制开奖号码

- 完成任务：实现 `06-03-lottery-console-manual-draw-control` 彩种控制台控制开奖号码阶段，新增彩种级开奖控制配置和控制台 SideSheet 维护入口。
- 解决问题：此前彩种控制台只能查看倒计时和开奖号码，无法按彩种开启“控制指定号码”；平台开奖仍走本地生成器，API 开奖仍走第三方来源，手动彩种自动任务缺少号码时会跳过。本次新增 `GET/PUT /api/admin/draw-controls`，保存控制号码后由后端统一校验并规范化为英文逗号格式，开奖服务优先使用控制号码覆盖平台/API 来源，自动任务在手动彩种启用控制号码时也能完成开奖、结算和入账。
- 管理后台调整：`useLotteryConsole` 并发加载彩种、期号和控制配置；每个彩种卡片展示“控制开奖/未控制”、控制号码和更新时间；点击“控制”通过 `SideSheet` 开启或关闭控制开奖并保存开奖号码。
- 验证结果：`cargo fmt --check`、`cargo check`、`cargo test`、`npm run build` 均通过；后端测试增加到 123 个，覆盖平台开奖使用控制号码、API 来源被控制号码覆盖、控制号码长度校验和手动彩种自动任务控制开奖。前端构建仍只有既有 chunk size warning。
- 后续动作：提交并推送本阶段改动；下一阶段建议推进开奖控制配置 PostgreSQL 持久化、管理员操作审计、期号级控制队列和高风险控制二次确认。

## 2026-06-03 14:46 HKT 全后台模块数据库持久化

- 完成任务：实现 `06-03-all-modules-database-persistence` 全后台模块数据库持久化阶段，新增 `state_documents` PostgreSQL 状态文档表和 `StateDocumentRepository`，并把用户权限、订单、开奖期号、开奖源、彩种控制台控制号码、资金、合买、邀请、返利、机器人、客服和调度配置/历史接入数据库状态恢复。
- 解决问题：此前 Compose 虽然已有 PostgreSQL，但除彩种和玩法赔率外，其它后台功能仍会在服务重启后丢失数据；本次在保持现有 API 和前端字段不变的前提下，让配置 `DATABASE_URL` 后所有已落地后台模块都能从数据库加载、空库写入种子，并在写操作成功后保存模块状态。
- 技术说明：本阶段采用 JSONB 状态文档作为第一阶段持久化方案；彩种仍使用 `lotteries` 关系表。订单、资金流水、开奖期号、结算批次和管理员权限后续仍需要逐步拆分为独立关系表并补事务、索引、审计和并发保护。
- 验证结果：`cargo fmt --check`、`cargo check`、`cargo test`、`npm run build` 均通过；后端 124 个测试全部成功，新增状态文档仓储测试覆盖种子写入、保存和恢复。前端构建仍只有既有 chunk size warning。
- 后续动作：完成最终质量检查、提交本阶段改动；下一阶段建议推进高风险模块关系表拆分、跨模块事务一致性、管理员操作审计和数据库备份恢复。

## 2026-06-03 15:28 HKT 全业务关系表数据库持久化

- 完成任务：实现 `06-03-relational-business-persistence` 全业务关系表持久化阶段，新增 `BusinessDatabase` 和 `20260603152000_create_business_tables.sql`，把用户权限、订单结算、开奖期号、开奖源、彩种控制台控制号码、资金账户、资金流水、合买、邀请、返利、机器人、客服和调度配置/历史全部迁移到独立业务表。
- 解决问题：上一阶段虽然所有模块已能保存到 PostgreSQL，但使用的是 `state_documents` 单表 JSONB 状态文档，不符合“所有业务都数据库持久化，不使用 state_documents”的要求；本次删除运行时代码中的 `StateDocumentRepository`，应用启动后统一创建 `BusinessDatabase`，各仓储从业务表读取，写操作成功后通过事务保存对应业务表。
- 技术说明：旧 `20260603143000_create_state_documents.sql` 作为历史迁移保留，运行时不再读写 `state_documents`；复杂字段仍按业务表列使用 JSONB 保存当前 API 契约结构，例如角色权限、投注选择、展开投注、中奖匹配和开奖源复用彩种。
- 验证结果：`cargo fmt`、`cargo fmt --check`、`cargo check`、`cargo test`、`npm run build` 均通过；后端 124 个测试全部成功，新增返利策略关系表持久化测试在配置 `BC_TEST_DATABASE_URL` 时验证写入和重新加载恢复。前端构建仍只有既有 chunk size warning。
- 后续动作：提交本阶段关系表迁移改动；下一阶段建议补跨模块数据库事务、管理员操作审计、分页查询、备份恢复和历史 `state_documents` 数据迁移脚本。

## 2026-06-03 15:43 HKT 开奖后自动开盘下一期修复

- 完成任务：启动并修复 `06-03-draw-next-issue-open` 开奖后自动开盘下一期问题，调整常驻调度未来期号缓冲判断。
- 解决问题：此前调度补齐未来期号时把 `closed` 期号也算作未来缓冲；当前期到封盘时间后变为 `closed`，但还没到开奖时间，系统会误以为未来期足够，从而不生成下一期 `open` 期号，导致封盘后没有新期可投注。
- 技术说明：未来缓冲现在只统计同彩种、状态为 `open` 且 `scheduledAt > now` 的期号；`closed` 期号已不可投注，不再占用开盘缓冲。新增测试覆盖当前期封盘后会自动生成下一期 `open` 期号。
- 验证结果：`cargo test scheduler_ -- --nocapture` 已通过，12 个调度测试全部成功；`cargo fmt --check`、`cargo check`、`cargo test`、`npm run build` 均通过，后端 125 个测试全部成功。前端构建仍只有既有 chunk size warning。
- 后续动作：提交本阶段修复改动；后续可继续补调度运行页面中“当前封盘后已开新期”的视觉提示和调度失败告警。

## 2026-06-03 16:02 HKT 后台动态启用开奖调度器

- 完成任务：启动 `06-03-scheduler-backend-dynamic-enable` 并把开奖调度器改为服务启动时由后端常驻启动，后台配置只控制是否执行。
- 解决问题：此前 `spawn_draw_scheduler` 在 `enabled=false` 时直接不创建后台循环，导致管理后台保存“启用”只更新配置，实际没有调度任务在运行，必须依赖环境变量并重启服务才生效。
- 技术说明：`spawn_draw_scheduler` 现在始终创建后台任务；`enabled=false` 时任务每 1 秒读取配置并跳过执行，后台保存 `enabled=true` 后无需重启即可进入自动封盘、开奖、结算和补期流程。
- 验证结果：`cargo test scheduler_ -- --nocapture` 已通过，13 个调度测试全部成功；`cargo fmt --check`、`cargo check`、`cargo test`、`npm run build` 均通过，后端 126 个测试全部成功。前端构建仍只有既有 chunk size warning。
- 后续动作：提交并推送本阶段改动；后续可继续补前端提示文案，明确“保存启用后后台任务会自动生效”。

## 2026-06-03 17:10 HKT 澳洲 5 分彩端到端开奖流程跑通

- 完成任务：启动 `06-03-au5-draw-flow-e2e` 并使用最新代码重新 `APP_PORT=18081 docker compose up -d --build`，完成 Docker 单镜像、PostgreSQL 迁移、后台登录、调度启用、澳洲 5 分彩 API 开奖、订单结算、资金入账和下一期开盘的端到端联调。
- 解决问题：此前本地运行容器仍是旧镜像状态，PostgreSQL 只执行到早期 `lotteries` 迁移，缺少 `draw_issues`、`draw_sources`、`draw_scheduler_config` 等业务表；同时调度配置为 `enabled=false`、`runCount=0`，所以到达开奖时间不会自动拉取 API68 开奖。
- 技术说明：重建最新镜像后 `_sqlx_migrations` 已包含 `20260603143000_create_state_documents` 和 `20260603152000_create_business_tables`；`au5` 彩种为 API 开奖、销售开启，`api68-au5` 绑定 `https://api.api68.com/CQShiCai/getBaseCQShiCaiList.do` 和 `lotCode=10010`。本次使用 API68 最新期号 `51320918`、开奖号码 `9,8,1,3,2` 创建到期测试期号和一笔前 3 直选订单。
- 验证结果：调度器后台开启后，`51320918` 已从 `open` 自动进入 `drawn`，保存开奖号码 `9,8,1,3,2`；测试订单 `O000000000001` 已结算为 `won`，命中 `981`，派奖 `950`；数据库写入结算批次 `S000000000001` 和投注扣款/中奖派奖资金流水；系统自动补出下一期 `51320919`，状态为 `open`。前端首页返回 HTTP 200，`/api/health` 返回成功，调度器最终配置恢复为 `enabled=true, intervalSeconds=60, futureIssueCount=1, saleCloseLeadSeconds=30`。
- 后续动作：真实运营时如果某个旧期号已不在 API68 返回列表中，会继续等待开奖；需要取消旧期号或重新按 API68 当前期号生成，并建议后续补 API68 原始响应留痕、失败重试和调度历史中关键运行记录的长期保留。

## 2026-06-03 21:36:58 HKT 邀请码格式修正

- 完成任务：启动 `06-03-invite-code-letterization` 并将用户邀请码生成与种子默认值统一为 8 位随机大写字母。
- 解决问题：前端/数据库中仍会出现 `USER10001/AGENT10001` 这类旧格式邀请码；当前规则要求必须是随机字母码。此次修正把未填邀请码的用户创建统一走随机字母生成，并把单元测试中的重复邀请码场景改为使用真实种子字母码，确保回归一致。
- 验证结果：补齐测试后计划执行 `cargo fmt --check`、`cargo check`、`cargo test`，确认邀请码重复校验与新规则保持一致，返回长度 8 且仅包含 A-Z。

## 2026-06-03 21:42:00 HKT 后台方法中文注释补齐

- 完成任务：给邀请码相关后台服务方法补充中文功能注释，提高 `access` 与 `invite` 模块可读性。
- 解决问题：运维在对照代码行为时不清楚仓储方法职责，尤其是创建/更新用户、创建/更新邀请关系与数据库持久化入口的处理链路；本次为关键方法补齐“做什么、为何这样做”说明。
- 验证结果：仅为目标文件新增中文注释，不改动业务逻辑；后续建议继续把同样注释标准扩展到其他后端服务文件中的关键入口方法。

## 2026-06-03 22:30:00 HKT 后台方法中文注释全面补齐

- 完成任务：补齐后台所有公开方法（`pub fn` / `pub async fn`）的中文说明注释，覆盖服务层、领域层与路由层的关键入口，使后台代码“每个方法都能看懂”。
- 解决问题：此前项目中大量方法缺少方法级说明，运维和开发人员在排障时无法快速定位某个接口逻辑入口与职责边界；本次统一补齐中文注释，保证团队交接时可直接读代码理解行为。
- 实施内容：对 `backend/src` 下所有公开方法逐一补充中文 doc 注释，保留原有逻辑不变；新增注释采用方法名+用途表述，并补齐模块说明前言。
- 验证结果：`cargo fmt --manifest-path backend/Cargo.toml`、`cargo test -- --nocapture` 均通过（全部 138 条测试通过）；仅保留已存在的警告 `invite.rs:485` 未使用变量。

## 2026-06-03 23:08:00 HKT 后台私有方法补注完成

- 完成任务：在公开方法注释基础上，继续补齐 `backend/src` 中未写注释的私有函数注释，确保服务层、领域层、路由层关键流程中每个函数都能通过中文注释直接判断用途。
- 解决问题：当前排障链路中大量内部 helper 缺少注释，容易在跨文件追踪时“看见函数名但不知道职责”；本次把未注释私有函数补齐为中文行为说明，降低交接和维护门槛。
- 验证结果：`cargo fmt --manifest-path backend/Cargo.toml` 与 `cargo test --manifest-path backend/Cargo.toml -- --nocapture` 全部通过（138/138）；本轮未引入新的编译或测试失败。

## 2026-06-03 23:20:00 HKT 后台注释语义性优化

- 完成任务：对自动补充的私有方法注释进行语义清洗，统一改为“功能含义+作用”表达，避免重复空泛模板。
- 解决问题：先前批量注释中仍有部分“执行 xxx 的具体内部处理逻辑”这类占位语句，影响阅读体验；本次统一改为动词化说明（如“按彩种查找”“校验参数”“更新并持久化”等），让注释更易读。
- 验证结果：再次执行 `cargo fmt --manifest-path backend/Cargo.toml` 与 `cargo test --manifest-path backend/Cargo.toml -- --nocapture`，138 条测试通过且无新增告警（保留既有 `invite.rs:486` 未使用变量提示）。
