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
- 后台订单、财务等运营列表如果支持“显示机器人数据”开关，默认必须关闭；开关文案使用“显示机器人数据”，打开后才把 `includeRobotData=true` 传给后端。
- 后台机器人配置页面不能提供删除入口；合买机器人和购彩机器人只允许通过状态切换暂停或禁用，避免误删系统自动化配置。
- 客服会话消息流应使用 Semi UI `Chat` 组件承载，不手写消息气泡列表；当后台页面只需要展示历史消息、回复输入由业务表单承担时，需要设置 `renderInputArea={() => null}` 和 `enableUpload={false}`，避免出现重复输入区或默认上传控件警告。后台客服回复区如接入表情，使用 `@emoji-mart/react` 与 `@emoji-mart/data`，并通过动态 `import()` 懒加载选择器、数据和中文语言包；选中后插入到当前 `textarea` 光标位置，不能把完整表情数据静态打进后台首屏主包。表情面板由 Semi UI `Popover` 负责显示与外部点击关闭，`Popover` 需要设置 `keepDOM` 复用 `emoji-mart` Picker 实例，并通过 `onClickOutSide` 关闭；不要同时给 Picker 传 `onClickOutside`，避免 Picker 内部监听在弹窗关闭后残留，导致第二次点击“表情”刚打开就被关掉。
- 手机端彩票卡片展示开奖号码时，不能按卡片变体写死 3 位；必须优先使用真实 `latestResult.length`，再结合后端 `resultCount` 决定展示数量，兼容 3 位和 5 位彩种。没有开奖结果时才按该数量用期号尾号或 `?` 补位。
- 手机端首页“高频极速”推荐区的开奖号码必须使用固定正圆号码球，样式需要同时约束 `width`、`height`、`aspect-ratio: 1 / 1` 和 `border-radius: 9999px`，不要只依赖文字内容或内边距撑开形状。
- 手机端首页“高频极速”推荐区默认由后台 `mobile_home_featured_enabled` 关闭；开启后只展示 `mobile_home_featured_lottery_codes` 配置的销售中彩种，不能按开奖周期自动兜底展示。
- 手机端首页“高频极速”二级彩种卡片必须保持可扫描：状态标签、倒计时和值/入口标签都要 `white-space: nowrap`，不能因为两列宽度不足把“可下注”挤成逐字竖排；二级卡优先整卡点击加小入口标签，不使用占满底部的大按钮撑高卡片。
- 手机端首页所有彩种卡片都不展示合买标签、合买按钮或合买大厅入口；合买功能只在合买大厅、下注页合买模式和我的合买等专用页面展示。
- 手机端首页倒计时到达开奖时间后必须触发静默刷新，且需要消费 `issue_opened` / `issue_closed` 实时事件更新当前期号和封盘状态，避免长期停留在“开奖中”。
- 手机端开奖历史和注单状态展示必须通过中文状态映射处理，不能把 `drawn`、`pendingDraw` 等接口状态值直接渲染给用户。
- 手机端“开奖结果”列表和单彩种全部开奖弹层的号码球必须保持紧凑，默认直径控制在 32px 左右，小屏控制在 30px 左右；不要复用首页主推大卡的大号码球尺寸，避免开奖列表被号码区域撑高。
- 手机端充值页必须以 `GET /api/user/recharge/config` 返回的后台充值配置为准，只展示已开启且真正可用的 `rainbowEpay` 和 `customerService` 渠道；彩虹易支付必须使用后端 `payTypes`，当 `payTypes` 为空时不能展示在线支付入口，也不能默认补 `alipay`。客服直充创建订单后跳转绑定的客服会话。不要再调用旧 `/payment/*` 接口，也不要在后端未配置时展示 USDT 或快捷充值模式。
- 手机端充值页属于高频资金操作页，需要直接展示充值渠道卡片、余额摘要、快捷金额和底部固定提交栏；快捷金额必须按后台 `minAmountMinor/maxAmountMinor` 过滤，最近订单中可继续处理的彩虹易支付订单应提供“继续支付”，客服直充订单应提供“联系客服”。
- 手机端充值页金额输入属于浏览器原生 `type="number"` 输入，运行时可能返回字符串或数字；金额解析函数不能直接调用 `value.trim()`，必须先用 `String(value ?? '').trim()` 归一化，再按两位小数转换为最小货币单位。
- 手机端资金类金额输入不能在每个按键后立即强制格式化或夹到最小金额，否则用户按删除键、选择文本或粘贴时会被立刻回填；应允许编辑态为空或部分小数，失焦、回车、快捷金额、加减按钮或提交时再做最终归一化。
- 手机端所有展示给用户的错误提示都必须走 `mobile/src/utils/errorMessage.ts` 的 `errorMessage` 或 `userFacingErrorMessage`；不得直接把后端 `message/detail`、Axios `err.message`、`Network Error` 或 `Request failed with status code ...` 原样传给 `showToast/showNotify`，避免出现英文错误提示。
- 手机端“我的账户”资金流水必须使用当前系统的 `GET /api/user/ledger-entries` 当前用户接口，只展示登录用户自己的流水；页面不得调用后台全量资金流水接口，也不需要为旧系统字段做兼容。流水条目不展示后端 `referenceId` 关联单号，避免把内部关联编号暴露给普通用户。
- 手机端“我的账户”头像设置必须使用用户端 `POST /api/user/avatar/upload` 或 `PUT /api/user/avatar`，不能复用后台 `/api/admin/image-bed/upload`；上传成功后需要同步刷新页面资料和 Pinia 登录态，保证返回个人中心或重新打开应用时继续展示最新 `avatarUrl`。头像点击上传优先使用原生 `input[type="file"]` 与 `label for` 绑定，避免自定义上传插槽在移动端点击不触发；头像视觉必须同时固定宽高并使用 `border-radius: 9999px` 保持正圆。
- 手机端头像图片展示必须通过公共 `CachedAvatarImage` 和 `avatarImageCache` 缓存，优先读取内存和本地 data URL 缓存；Tauri 打包场景可通过 Rust 命令下载远程头像并转为 data URL，避免个人中心和聊天大厅对同一个图床头像地址反复请求。缓存失败时才能回退原始 URL。
- 手机端邀请中心必须使用当前系统的 `GET /api/user/invitations/summary` 当前用户接口，不再请求旧 `/auth/invitations/summary`；页面消费 `canInvite`、`invitationCode`、`directUsers` 等 `camelCase` 字段，普通用户只展示邀请码标识和无可用邀请权限提示，不允许自行把普通用户邀请码当成有效邀请入口。
- 手机端下注页必须使用 `/api/user/bet/page-config/{lottery_id}`、`POST /api/user/bet/orders` 和 `GET /api/user/bet/orders`，不再调用旧 `/api/bet/*`。提交时前端只负责把位置宫格、胆拖、直选组合和大小单双转换成后端 `selection`，订单金额仍由后端按玩法展开注数和单注金额计算。
- 手机端下注页顶部的最近开奖号码球必须比开奖历史页更紧凑，默认直径控制在 20-24px，号码容器必须 `flex-wrap` 且设置最大宽度；360px 以下不能使用固定不换行的一排大球，极窄屏需要允许“上期开奖”和号码分成上下两行，避免 5 位开奖号码跑出屏幕。
- 手机端下注页读取玩法 `positionSelectLimits` 时必须按 `positionKey` 精准限制对应位置的选号数量；未配置的位置不限制。不要只用全局 `maxSelectPerPosition` 套到所有位置，例如前 3 直选只配置 `first=7` 时，第二位和第三位仍应保持不限制。
- 手机端下注页普通投注或发起合买成功后必须清空本地购彩篮，并用 `router.replace({ name: 'Home' })` 自动返回首页；接口失败时才停留在下注页并刷新余额、期号状态，方便用户继续处理。
- 手机端下注页进入普通投注、提交购彩篮或发起合买的接口请求后，必须显示页面级 loading 遮罩，并禁用加入购彩篮、编辑单据和提交按钮，直到接口完成和必要的余额/期号刷新结束；不能只改按钮文案，避免用户误以为没有响应或重复点击产生重复下单。
- 手机端下注页用户直接点击“立即投注”“提交购彩篮”或“发起合买”时，如果页面内部需要先把当前草稿加入购彩篮，该内部入篮动作必须静默执行，不显示“已加入购彩篮”；只有用户主动点击“加入购彩篮”按钮时才显示入篮成功提示，最终提交成功只保留下注或合买成功提示。
- 手机端合买详情的认购金额必须同时使用后端 `participantMinAmountMinor` 和 `shareAmountMinor` 校正：默认金额不能低于参与人最低认购金额，金额必须按完整份额取整；如果剩余金额低于参与人最低认购金额，应允许并提示用户直接全包尾单。合买模块错误提示必须优先展示统一响应信封里的 `message`。
- 手机端实时事件必须通过 `GET /api/user/realtime` WebSocket 接口接入，不再使用旧 `/ws/lottery`；页面组件不得直接依赖后端原始事件信封，必须先通过 `mobile/src/types/realtime.ts` 归一化为 `draw_result`、`issue_opened`、`balance_changed`、`chat_hall_message_created` 等本地事件后再消费。
- 手机端公共聊天大厅必须使用当前系统 `GET/POST /api/user/chat-hall/messages`、`POST /api/user/chat-hall/red-packets`、`POST /api/user/chat-hall/red-packets/{id}/claim`、`POST /api/user/chat-hall/group-buy-plans` 和 `chat_hall_message_created` 实时事件；大厅是所有登录用户的公共聊天流，不得复用或写入客服会话接口，断线重连后通过 HTTP 拉取最近历史并在 WS 事件到达时按消息 ID 替换或追加。聊天大厅消息头像必须优先渲染后端 `avatarUrl` 图片，图片为空或加载失败时再显示用户名首字文字头像。聊天大厅输入栏接入表情时沿用手机端客服的 `emoji-mart` 原生 `Picker`、动态加载、`Teleport` 面板、Vue 遮罩关闭和不传 `onClickOutside` 的规则，避免第二次打不开或实时消息重渲染错误。聊天大厅加入 `mobile-bottom-nav` 后，页面必须为底部导航预留空间，底部输入区应是居中的圆角操作条，红包和合买计划通过“+”附件菜单进入，输入栏、表情面板和消息列表底部不能被导航遮挡。聊天大厅顶部只保留标题，不展示副标题、返回或刷新图标按钮。聊天消息必须按 `messageType` 渲染：文本用普通气泡，红包用可领取红包卡片，合买计划用带进度的计划卡片。
- 手机端客服聊天属于用户可见前台页面，不能直接展示后台管理员账号名；客服接入状态统一显示“客服已接入”，后台消息气泡作者统一显示“客服”。手机端是 Vue，客服输入栏接入表情时使用 `emoji-mart` 原生 `Picker` 与 `@emoji-mart/data`，不要使用 `@emoji-mart/react`；选择器、表情数据和中文语言包必须动态加载，选中后插入到当前输入框光标位置。原生 `Picker` 必须挂载到 Vue 不管理子节点的空容器中，优先放入 `Teleport` 面板；不要在 Vue 模板子树里对宿主节点调用 `replaceChildren`，否则 WebSocket 消息触发页面重渲染时可能破坏 Vue patch 状态并出现 `emitsOptions` 运行时错误。手机端表情面板的外部点击关闭由 Vue `Teleport` 遮罩负责，不要给原生 Picker 传 `onClickOutside`，避免 Picker 内部监听在弹窗关闭后影响第二次点击“表情”打开。
- 手机端下注页的投注倍数输入应使用“减少按钮 + 数字输入 + 增加按钮”的步进控件，并保留滑块联动；输入框只接收数字，失焦或回车时必须夹到当前玩法允许的最小/最大倍数范围。
- 手机端下注页开启合买模式时，自购份数需要按方案总额、固定每份金额和发起人最低自购比例自动填入最适配份数：有最低比例时填最低所需份数，无最低比例时至少填 1 份；用户手动填更高份数时保留，但低于最低份数或超过总份数时必须自动校正。
- 手机端注单列表和注单详情展示“投注内容”时，大小单双不能按普通号码拆球展示；必须把 `selection.bigSmallOddEven` 或订单 `numbers` 中的 `big/small/odd/even` 翻译成中文，并按位置显示为“十位：大、小；个位：单、双”这类属性格式。
- 手机端注单详情必须读取当前 `/api/user/bet/orders` 契约中的 `matchedBets`，并按玩法展示匹配项：直选显示按位命中，直选组合显示排列命中，组三/组六显示顺序无关命中，胆拖显示胆码拖码组成命中，大小单双显示十位/个位属性命中；前端只能做展示翻译，不能自行覆盖后端中奖判定，也不需要为旧系统订单字段做兼容。
- 手机端注单列表和注单详情展示合买订单金额时，必须优先使用 `/api/user/bet/orders` 返回的 `participationAmountMinor`，标题显示为“参与金额”；普通独立订单继续显示“下注金额”。不要直接把合买真实订单的 `amountMinor` 当成当前用户实际参与金额。
- 手机端合买大厅、发起合买、参与合买和我的合买必须使用当前 `/api/user/group-buy/*` 接口；金额在 API 适配层转换为最小货币单位，页面不提交用户 ID、不自行扣款、不再调用旧 `/group-buys/*` 路径。
- 手机端合买大厅计划列表必须优先服务批量扫描，同一首屏至少能看到 6 个合买计划；大厅卡片应使用紧凑信息行展示彩种、期号、玩法、发起人、总额、单份、进度和小入口，不使用大头像区、两列大金额块或整行大按钮撑高卡片。
- 手机端合买大厅发起人只展示后端 `initiatorDisplay` 脱敏值，不能展示完整昵称或自行拼接 `initiatorUsername`；发起人名称在紧凑卡片中应比辅助标签更醒目，使用更大字号、加粗和单行截断。
- 手机端合买详情里的“确认认购”等关键 Vant 主按钮必须显式保证高对比：使用 `type="primary"` 或专用类覆盖按钮背景，同时用 `:deep(.van-button__text)` 处理内部文字颜色，不能只依赖外层 Tailwind `text-white`。
- 手机端发起合买的投注内容输入必须按当前后端规则展示示例：直选 `1|2|3`、组合 `1,2,3`、胆拖 `1|2,3,4`、大小单双 `tens:big|ones:odd`；不要再兼容旧系统福彩 3D 纯数字格式，校验结果以后端返回为准。
- 后台彩种控制台属于监控扫描页，彩种模块必须保持紧凑：桌面端优先 4 列展示，单卡内边距应小于普通详情卡；期号、最近开奖和开奖控制不能各自占用大块竖向面板，长日期时间应转成 `HH:mm:ss` 或更短的中文状态，避免一屏可见彩种数量下降。
- 后台彩种控制台的控制 SideSheet 必须同时展示相关用户下单信息，默认聚焦当前期或当前控制范围；控制范围必须用 Semi UI `Select` 明确区分“整个彩种”“指定期号”“指定订单所在期号”，订单范围只能选择待开奖订单，并把订单期号同步为控制期号。
- 后台订单管理和彩种控制台控单 SideSheet 必须直接展示用户下注信息：`selection` 要翻译成中文位置、选号、胆码拖码或大小单双，`expandedBets` 要展示为展开注码标签；不能只展示订单号、用户、期号和金额，否则运营无法依据下注内容进行控单。
- 后台订单、财务、合买、计奖派奖等运营列表只要数据会持续增长，就必须使用分页结构和公共 `PageControls`，页面顶部显示总数、每页条数、上一页和下一页；切换“显示机器人数据”等影响列表口径的开关时应回到第 1 页。

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
