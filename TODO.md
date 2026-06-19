# TODO

## 2026-06-19 07:18 HKT 手机端登录页首屏骨架屏

- 完成任务：给手机端登录页面新增首屏骨架屏。
- 解决问题：登录页首次打开时需要等待注册入口配置和可能的品牌配置加载，此前会直接显示空白品牌或表单跳变，用户感知不够稳定。
- 实施内容：新增 `LoginPageSkeleton.vue`，按登录页真实结构模拟 Logo、平台名、标语、账号输入、密码输入、主按钮、分隔线和注册入口；`LoginView.vue` 在注册配置加载中或品牌首次加载中展示骨架屏，加载完成后切换到真实登录/注册表单，登录提交仍沿用原有按钮 loading。

## 2026-06-19 06:52 HKT 管理员登录安全审计日志

- 完成任务：补充管理员后台登录成功和失败的服务端审计日志。
- 解决问题：本地排查管理员登录时，后端此前没有专门记录登录输入账号、请求来源和失败原因，定位前端是否提交账号、请求是否到达后端不够直接。
- 实施内容：`/api/admin/auth/login` 增加请求头审计上下文，优先读取 `CF-Connecting-IP`、`True-Client-IP`、`X-Forwarded-For` 等代理头，再读取 `User-Agent`；登录失败使用 `tracing::error!` 打印账号、IP、UA、密码是否为空、密码长度和失败原因；登录成功使用 `tracing::info!` 打印输入账号、管理员 ID、管理员账号、IP 和 UA；日志仍不记录明文密码、Token 或原始请求体。

## 2026-06-19 05:18 HKT 手机端下注页首屏骨架屏

- 完成任务：给手机端动态下注页添加首屏骨架屏。
- 解决问题：下注页首次进入或切换彩种时，玩法配置、当前期号和最近开奖需要等待接口返回，页面此前容易短暂显示空白或只有加载文字，用户感知不够稳定。
- 实施内容：新增 `BetPageSkeleton.vue`，按下注页真实结构模拟期号卡片、玩法选择、选号区域、倍数区域和底部投注栏；`DynamicBetPage.vue` 在首次加载或路由彩种与当前配置不一致时展示骨架屏，并隐藏真实底部投注栏，普通静默刷新仍保留已渲染内容，只显示“正在刷新玩法...”。

## 2026-06-19 05:12 HKT 手机端首页开奖卡片小球与图片缩小

- 完成任务：收小手机端首页开奖卡片里的彩种 Logo 和开奖号码球。
- 解决问题：首页开奖卡片此前 Logo 和号码球视觉占比偏大，普通分类卡片显得较重，一屏展示彩种数量和轻盈感不够。
- 实施内容：`HomeDrawCard.vue` 中高频精选大卡 Logo 从 `h-8 w-8` 调整为 `h-7 w-7`，二级卡 Logo 从 `h-6 w-6` 调整为 `h-5 w-5`；精选大卡号码球、二级卡号码球、普通分类卡 `group-lottery-card__digit` 和 `group-lottery-card__logo-shell` 同步缩小，小屏媒体查询也同步收敛；架构说明补充首页卡片尺寸口径。

## 2026-06-19 03:37 HKT iOS 真机验证与无签名 IPA 重打包

- 完成任务：使用已连接的 iPad 对手机端 iOS 包进行真机安装启动验证，并在验证通过后重新生成无签名 IPA。
- 解决问题：此前只完成包内结构和品牌资源静态校验，仍需要确认真实 iOS 设备上不会点击后闪退，并确认主屏图标已经切换为后台配置的 Logo。
- 实施内容：同步 `mobile/dist`、`mobile-branding.json`、本地 `app-logo.png` 和 iOS `AppIcon.appiconset` 到临时真机测试工程；使用已有 `arm64/release/libapp.a` 跳过卡住的 Tauri Rust 构建脚本，构建 release 真机 App；通过 `xcrun devicectl` 安装并启动 `com.hongfu.app`；从设备生成 App 图标到 `mobile/src-tauri/gen/apple/build/device-test/hongfu-ios-app-icon.png`；验证通过后重新执行 `pnpm tauri:build:ios-unsigned` 生成最终无签名 IPA。
- 验证结果：设备 `iPad mini (6th generation)` 安装成功，应用列表显示“鼎鸿 / com.hongfu.app / 0.1.0”；启动后 `HongFu`、`WebKit.WebContent`、`WebKit.Networking` 和 `WebKit.GPU` 进程仍在，未生成新的 `HongFu` 或“鼎鸿”崩溃报告；设备生成的 App 图标不是占位图，内容为后台 Logo；最终 IPA 路径为 `mobile/src-tauri/gen/apple/build/DingHong-display-HongFu-internal-unsigned.ipa`，大小 6.8M，解包确认 `Payload/HongFu.app/HongFu`、`CFBundleDisplayName=鼎鸿`、`assets/mobile-branding.json` 和 `assets/app-logo.png` 均正确；`git diff --check` 通过。

## 2026-06-19 03:24 HKT 手机端 IPA 品牌资源同步与远程图片缓存

- 完成任务：让无签名 IPA 打包时同步后台手机端 Logo，并把手机端常见远程图片接入本地缓存。
- 解决问题：iOS IPA 企业签名后桌面图标和 App 内首屏品牌仍可能使用旧资源；同时首页彩种 Logo、全部彩种页 Logo 和 Banner 都是图床网络地址，进入页面时会反复请求同一批图片。
- 实施内容：新增 `mobile/scripts/sync-branding-assets.mjs`，从后台 `GET /api/user/mobile/site-config` 下载 Logo，生成 `app-logo.png`、`logo.svg`、`mobile-branding.json`，并重写临时 iOS `AppIcon.appiconset`；无签名 IPA 脚本默认执行品牌同步；`branding` store 启动时先读取包内 `mobile-branding.json`，再静默刷新后台配置；新增 `CachedRemoteImage` 通用组件，首页彩种卡片、全部彩种页、开奖记录卡片、平台页头 Logo 和首页 Banner 改为使用缓存图片。
- 验证结果：`cd mobile && pnpm build` 通过；品牌同步脚本可生成包内品牌资源和 iOS 多尺寸图标；`cd mobile && pnpm tauri:build:ios-unsigned -- --output src-tauri/gen/apple/build/DingHong-display-HongFu-internal-unsigned.ipa` 通过，生成 6.8M 无签名 IPA；解包确认 `Payload/HongFu.app/HongFu`、`CFBundleDisplayName=鼎鸿`、`CFBundleExecutable=HongFu`，并确认 `assets/mobile-branding.json` 指向包内 `/app-logo.png`；`bash -n mobile/scripts/build-unsigned-ipa.sh`、`node --check mobile/scripts/sync-branding-assets.mjs`、`git diff --check` 均通过。

## 2026-06-19 03:10 HKT 开奖调度慢阶段并发与逐期推送优化

- 完成任务：优化平台开奖和 API 开奖同时较多时的调度慢阶段处理。
- 解决问题：调度慢阶段此前按期号串行写入开奖号码、结算订单、派奖入账，并且等整批慢阶段全部完成后才统一推送开奖结果；当同时开启很多平台开奖彩种时，前面的彩种即使已经处理完成，手机端也可能长时间停留在“开奖中”。
- 实施内容：自动开奖阶段先收集到期候选，再使用最多 8 个并发任务写入开奖号码；订单、资金和合买结算改为走 `OrderRepository::settle_with_payouts` 统一事务入口，避免拆分保存造成半边状态；调度器慢阶段新增单期结算进度回调，每个期号完成开奖和结算后立即推送 `lottery.draw_result` 和余额变化，不再等待整批彩种全部结束。
- 并发边界：只并发“开奖号码写入”这种可独立执行的阶段；订单状态、资金流水、派奖和合买结算仍按顺序通过同一个跨仓储事务提交，避免并发快照覆盖。
- 验证结果：`cargo fmt --manifest-path backend/Cargo.toml`、`cargo fmt --manifest-path backend/Cargo.toml --check`、`cargo check --manifest-path backend/Cargo.toml`、`cargo test --manifest-path backend/Cargo.toml automation_ -- --nocapture`、`cargo test --manifest-path backend/Cargo.toml scheduler_ -- --nocapture`、后端全量 `cargo test --manifest-path backend/Cargo.toml` 均通过（354 个测试成功）。

## 2026-06-19 02:55 HKT 清空手机端默认品牌文案

- 完成任务：调整手机端默认品牌配置，未读取到后台配置前不再显示“鸿福”。
- 解决问题：手机端打包 App 启动时会先渲染本地 `DEFAULT_BRANDING`，此前默认名称为“鸿福”，在后台动态平台名称和 Logo 尚未返回前会短暂显示错误品牌。
- 实施内容：将 `DEFAULT_BRANDING.site_name`、`slogan` 和 `footer_text` 改为空字符串；默认 Logo 改为透明 1 像素占位图，避免空 `src` 触发破图或请求当前页面；同步清空 `mobile/index.html` 的静态标题，避免 JS 接管前短暂显示“鸿福”；提现方式页顶部标题改为读取动态品牌配置，后台 `site-config` 加载成功后仍按接口返回的平台名称、Logo 和介绍覆盖默认值。
- 验证结果：`cd mobile && pnpm tauri:build:ios-unsigned --output src-tauri/gen/apple/build/DingHong-display-HongFu-internal-unsigned.ipa` 通过；解包确认 `Payload/HongFu.app/HongFu`、`CFBundleDisplayName=鼎鸿`、`CFBundleExecutable=HongFu`，并确认 App 内前端资源已无默认“鸿福”“开启您的幸运之门”“传承现代美学”残留；`git diff --check` 通过。

## 2026-06-19 02:48 HKT 修正手机端动态品牌和广告刷新链路

- 完成任务：加强手机端启动和首页进入时的动态品牌、Logo、介绍和广告配置刷新。
- 解决问题：打包 App 内平台名称、Logo、广告标题依赖后端配置，但此前品牌配置只在启动后普通加载一次，首页再次进入不会强制刷新；如果后台刚更新配置或 App 命中旧缓存，容易继续显示默认 Logo 或旧标题。排查同时确认当前打包域名 `https://ad.16888888.live` 的 `/api/user/mobile/site-config` 已返回 `platformName=鼎鸿` 和图床 Logo，但 `/api/user/mobile/advertisements` 返回空数组，因此该域名下手机端轮播广告当前没有可展示数据。
- 实施内容：`branding` Pinia store 增加强制刷新、加载状态、更新时间和“未配置”过滤；App 启动时使用 `force` 读取后台品牌配置；首页挂载时同步强刷品牌配置和手机端广告列表；同步修正架构说明中打包默认后端域名，并补充动态配置依赖当前 `API_BASE` 的前端规范。
- 验证结果：已通过 `curl https://ad.16888888.live/api/user/mobile/site-config` 验证动态平台名和 Logo 可达；`curl https://ad.16888888.live/api/user/mobile/advertisements` 当前返回空数组，后续需要在同一后台域名下新增并启用手机端轮播广告后再验证首页广告展示。

## 2026-06-19 02:30 HKT 确认 iOS 企业签名前后内部名校验规则

- 完成任务：确认无签名 IPA 的正确结构，并记录企业签名后的校验要求。
- 解决问题：用户使用 `DingHong-display-HongFu-internal-unsigned.ipa` 企业签名后仍闪退；真机崩溃日志显示签名后的应用路径再次变为 `/鼎鸿.app/鼎鸿`，并在 `wry::wkwebview::platform_webview_version` 初始化阶段触发 `CFRelease() called with NULL`，说明签名后的内部 `.app` 目录和可执行文件名被改成中文。
- 实施内容：保留“桌面显示名=鼎鸿、内部名=HongFu”的无签名 IPA 生成方式；同步更新架构说明和前端质量规范，明确签名前后都必须校验包结构为 `Payload/HongFu.app/HongFu`，不能让企业签名平台改成 `Payload/鼎鸿.app/鼎鸿`。
- 验证结果：确认 `DingHong-display-HongFu-internal-unsigned.ipa` 的签名前结构是正确方向；后续企业签名后的 IPA 需要重新解包校验内部 `.app` 目录、`CFBundleExecutable` 和实际可执行文件名是否仍为 `HongFu`。

## 2026-06-18 15:26 HKT Cloudflare 注册来源识别修正

- 完成任务：修正 Cloudflare 代理域名下用户注册 IP 和注册地区识别规则。
- 解决问题：Cloudflare 代理后普通 `x-forwarded-for` 可能不再是最可靠的真实用户 IP，后端此前也没有读取 `CF-IPCountry`，导致后台注册来源只显示 IPv6 或无法展示国家/地区。
- 实施内容：注册接口优先读取 `cf-connecting-ip` 和 `true-client-ip`，再读取普通反代头；新增 `CF-IPCountry` 国家或地区识别，常见国家地区代码转为中文；Nginx 反代明确透传 Cloudflare 真实 IP 与国家头；同步更新架构说明和后端接口契约。
- 验证结果：`cargo fmt --manifest-path backend/Cargo.toml` 已执行；`cargo test --manifest-path backend/Cargo.toml registration_ip_parser -- --nocapture`、`cargo test --manifest-path backend/Cargo.toml registration_client_info -- --nocapture`、`cargo test --manifest-path backend/Cargo.toml access_repository_keeps_server_ip_country_registration_location -- --nocapture`、`cargo check --manifest-path backend/Cargo.toml` 和 `git diff --check` 均通过。

## 2026-06-18 15:12 HKT 手机端无签名 IPA 打包脚本

- 完成任务：新增 macOS 无签名 IPA 打包脚本，并在手机端 `package.json` 增加快捷命令。
- 解决问题：直接执行 `pnpm tauri:build:ios` 会因为未配置 Apple Developer Team 在 Xcode 签名阶段失败，临时手动命令又太长，不方便重复生成无签名 IPA。
- 实施内容：新增 `mobile/scripts/build-unsigned-ipa.sh`，脚本会构建手机端前端资源、复制临时 iOS 工程、同步 `mobile/dist` 到 iOS `assets`、在临时工程中跳过 Tauri iOS Rust 构建脚本与 Xcode 签名，并封装 `Payload/*.app` 为无签名 IPA；新增 `pnpm tauri:build:ios-unsigned` 快捷命令。
- 验证结果：`bash -n mobile/scripts/build-unsigned-ipa.sh` 通过；`bash mobile/scripts/build-unsigned-ipa.sh --skip-web-build` 和 `cd mobile && pnpm tauri:build:ios-unsigned -- --skip-web-build` 均成功生成 `/Users/huangkunhuang/Public/程序工程目录/复合工程/bc/mobile/src-tauri/gen/apple/build/鼎鸿-unsigned.ipa`，包内 `CFBundleName` 为“鼎鸿”、`CFBundleIdentifier` 为 `com.hongfu.app`。

## 2026-06-18 12:22 HKT 部署迁移 logo_url 注释顺序修复

- 完成任务：修复新环境部署时 SQLx 执行 `20260603234000_add_all_column_comments.sql` 报 `column "logo_url" of relation "lotteries" does not exist` 的问题。
- 解决问题：早期全量字段注释迁移时间早于 `lotteries.logo_url` 字段创建迁移，却提前执行 `COMMENT ON COLUMN lotteries.logo_url`，导致空库按顺序回放迁移时中断。
- 实施内容：移除 `20260603234000_add_all_column_comments.sql` 中对后续字段 `lotteries.logo_url` 的提前注释，保留 `20260604202000_add_lottery_logo_url.sql` 在创建字段后负责注释；同步更新架构设计和数据库规范，明确字段注释不得早于字段创建迁移。
- 验证结果：`cargo fmt --manifest-path backend/Cargo.toml --check`、`cargo check --manifest-path backend/Cargo.toml` 和 `git diff --check` 均通过；静态复核确认 `20260603234000_add_all_column_comments.sql` 不再提前执行 `COMMENT ON COLUMN lotteries.logo_url`。

## 2026-06-17 00:40 HKT 合买机器人流单前兜底补满

- 完成任务：为合买机器人增加封盘流单前兜底补满能力。
- 解决问题：如果常规分阶段补单窗口被调度延迟、期号已经封盘但还没开奖，用户发起的合买计划仍可能保持未满单并在封盘流单阶段被取消退款。
- 实施内容：新增封盘后开奖前兜底成单口径，只允许已存在合买计划在 `Closed` 且未到 `scheduledAt` 时补建真实订单；新增 `force_fill_user_group_buy_plans_before_refund`，在流单退款前扫描用户发起的未满计划并由已启用合买机器人补满；常驻调度慢阶段和后台手动自动化触发均接入兜底；补充机器人与调度器单元测试，并同步架构说明和 Trellis 后端契约。
- 验证结果：`cargo fmt --manifest-path backend/Cargo.toml -- --check`、`cargo check --manifest-path backend/Cargo.toml`、`cargo test --manifest-path backend/Cargo.toml group_buy_robot -- --nocapture`、`cargo test --manifest-path backend/Cargo.toml scheduler -- --nocapture`、`cargo test --manifest-path backend/Cargo.toml group_buy_flow -- --nocapture`、后端全量 `cargo test --manifest-path backend/Cargo.toml -- --nocapture` 和 `git diff --check` 均通过；后端全量 333 个测试成功。

## 2026-06-16 18:35 HKT 后台资金流水一键清除

- 完成任务：为后台财务管理补充资金流水一键清除能力。
- 解决问题：后台此前只能分页查看资金流水，测试和运营维护时无法像充值、提现、订单和合买计划一样清理历史流水列表。
- 实施内容：后端新增 `DELETE /api/admin/ledger-entries/clear`，资金仓储清理 `ledger_entries` 但保留资金账户余额和下一流水序号；管理后台 API client、`useFinance` 和财务管理“资金流水”标签页新增清理入口；同步更新 OpenAPI、架构说明和 Trellis 前后端规范。
- 验证结果：`cargo fmt --manifest-path backend/Cargo.toml -- --check`、`cargo test --manifest-path backend/Cargo.toml store_clears_ledger_entries_without_changing_balance_or_sequence -- --nocapture`、`cargo test --manifest-path backend/Cargo.toml openapi -- --nocapture`、`cargo check --manifest-path backend/Cargo.toml`、后端全量 `cargo test --manifest-path backend/Cargo.toml -- --nocapture`、管理后台 `npm run build` 和 `git diff --check` 均通过；后端全量 331 个测试成功，管理后台构建仍只有既有 chunk size warning。

## 2026-06-16 09:46 HKT 封盘提前秒数口径纠正

- 完成任务：把封盘时间计算从“开盘后可售秒数”纠正为“开奖前提前秒数”。
- 解决问题：用户确认正确逻辑是 300 秒周期配置 60 秒时，在周期进行到 `300 - 60 = 240` 秒封盘，并在剩余 60 秒显示“开奖中”；上一版把 60 秒理解为开盘后只销售 60 秒，口径错误。
- 实施内容：后端期号生成改为按 `saleClosedAt = scheduledAt - saleCloseLeadSeconds` 计算封盘时间，并保留封盘提前量超过本期周期时按本期开盘时间封盘的保护；调度器继续把已封盘但未开奖的当前期计入未来缓冲，避免封盘后提前开下一期；后台彩种和调度表单文案改为“封盘提前（秒）”；新增数据库注释迁移覆盖字段说明，并同步架构说明与 Trellis 规范。
- 验证结果：`cargo fmt --manifest-path backend/Cargo.toml -- --check`、`cargo test --manifest-path backend/Cargo.toml draw_generation -- --nocapture`、`cargo test --manifest-path backend/Cargo.toml scheduler -- --nocapture`、`cargo check --manifest-path backend/Cargo.toml`、后端全量 `cargo test --manifest-path backend/Cargo.toml -- --nocapture`、管理后台 `npm run build` 和 `git diff --check` 均通过；后端全量 330 个测试成功，管理后台构建仍只有既有 chunk size warning。

## 2026-06-16 08:48 HKT 封盘后等待开奖调度修正（口径已更正）

- 完成任务：修正彩种封盘时间的业务口径和调度开盘时机。
- 解决问题：本条曾把 `saleCloseLeadSeconds` 理解为“开盘后可售秒数”，该口径已在 09:46 HKT 更正为“开奖前封盘提前秒数”；本条仍保留的有效修复是封盘后到开奖前不提前开启下一期。
- 实施内容：允许后端生成已过封盘但未开奖的待开奖期以恢复“开奖中”状态；调度器把 `closed` 且未到开奖时间的当前期计入缓冲，避免封盘后提前生成下一期；本条中关于“开盘后可售秒数”的计算和文案已由 09:46 HKT 修正覆盖。
- 验证结果：`cargo fmt --manifest-path backend/Cargo.toml -- --check`、`cargo test --manifest-path backend/Cargo.toml draw_generation -- --nocapture`、`cargo test --manifest-path backend/Cargo.toml scheduler -- --nocapture`、`cargo test --manifest-path backend/Cargo.toml lottery -- --nocapture`、`cargo check --manifest-path backend/Cargo.toml`、后端全量 `cargo test --manifest-path backend/Cargo.toml -- --nocapture`、管理后台 `npm run build` 和 `git diff --check` 均通过；后端全量 330 个测试成功，管理后台构建仍只有既有 chunk size warning。

## 2026-06-16 08:18 HKT 用户数据读取失败修复

- 完成任务：修复后端启动或访问用户相关接口时可能返回 `Internal("用户数据读取失败")` 的问题。
- 解决问题：`users.created_at` 在 PostgreSQL 中是 `timestamptz`，但访问仓储读取运行时用户快照时直接按 `String` 解码，SQLx 无法完成该类型转换，导致用户数据加载失败。
- 实施内容：用户读取 SQL 将 `created_at` 显式投影为 `YYYY-MM-DD HH:mm:ss` 文本；用户快照保存时显式把文本时间按 `timestamptz` 写入，避免参数类型推断不稳定；同步补充架构说明和数据库规范。
- 验证结果：`cargo fmt --manifest-path backend/Cargo.toml -- --check`、`cargo check --manifest-path backend/Cargo.toml`、`cargo test --manifest-path backend/Cargo.toml access -- --nocapture` 和 `git diff --check` 均通过；使用外部 PostgreSQL 临时启动后端，`GET /api/health` 返回 `ok`，未再出现“用户数据读取失败”。

## 2026-06-16 07:57 HKT 手机端下注页封盘后展示开奖中

- 完成任务：修正手机端下注页封盘时间到达后到下一期开盘前的展示状态。
- 解决问题：此前倒计时已经会显示“开奖中”，但顶部期号卡片仍直接使用后端短暂返回的 `selling` 状态，容易让用户误以为当前期还在销售。
- 实施内容：动态下注页新增展示态归一化，`round.status=selling` 且 `sale_stop_at <= now` 时统一传给期号卡片 `opening`，并保留原有禁用投注、禁用合买和静默轮询下一期逻辑；同步更新架构说明和前端组件规范。
- 验证结果：手机端 `pnpm build` 和 `git diff --check` 均通过。

## 2026-06-16 07:52 HKT 彩种封盘时间动态配置

- 完成任务：为每个彩种新增可动态配置的封盘时间。
- 解决问题：此前自动补期和开奖源同步主要依赖调度器全局 `saleCloseLeadSeconds`，不同彩种无法单独设置封盘时间。
- 实施内容：后端 `LotteryKind` 新增 `saleCloseLeadSeconds`，数据库 `lotteries.sale_close_lead_seconds` 持久化并增加中文字段注释；生成期号默认使用彩种封盘时间，手动生成请求显式传值时仍可临时覆盖；常驻调度、API 彩种补期、开奖源同步和开售后补期改为读取彩种配置；后台彩种新增/编辑 SideSheet 新增“封盘提前（秒）”输入项。
- 验证结果：`cargo fmt --manifest-path backend/Cargo.toml`、`cargo check --manifest-path backend/Cargo.toml`、`cargo test --manifest-path backend/Cargo.toml draw_generation -- --nocapture`、`cargo test --manifest-path backend/Cargo.toml scheduler -- --nocapture`、`cargo test --manifest-path backend/Cargo.toml lottery -- --nocapture`、管理后台 `npm run build` 和 `git diff --check` 均通过；管理后台构建仍只有既有 chunk size warning。

## 2026-06-16 07:35 HKT 平台期号格式支持短序号变量

- 完成任务：扩展平台开奖 `issueFormat`，支持 `{seq1}`、`{seq2}`、`{seq3}` 和原有 `{seq4}`。
- 解决问题：此前平台期号格式只能使用 `{seq4}` 生成 4 位每日递增序号，无法配置 1 位、2 位或 3 位序号规则。
- 实施内容：后端期号模板渲染改为通用序号宽度处理，短序号按开奖日期每日递增并校验上限；已存在期号恢复逻辑按当前模板提取序号，避免重启后重复生成；后台彩种表单说明、架构说明和 Trellis 规范同步补充新变量。
- 验证结果：`cargo fmt --manifest-path backend/Cargo.toml`、`cargo check --manifest-path backend/Cargo.toml`、`cargo test --manifest-path backend/Cargo.toml platform_schedule -- --nocapture`、`cargo test --manifest-path backend/Cargo.toml draw_generation -- --nocapture`、管理后台 `npm run build` 和 `git diff --check` 均通过；管理后台构建仍只有既有 chunk size warning。

## 2026-06-16 07:28 HKT 手机端代理中心展示下级注册与提现

- 完成任务：为手机端代理中心的直属下级列表补充下级注册时间和已通过提现金额展示。
- 解决问题：代理此前只能看到直属下级状态、充值和返利开关，缺少判断下级活跃度所需的注册时间与提现金额。
- 实施内容：后端用户模型持久化读取 `createdAt` 注册时间，邀请中心直属下级响应新增 `registeredAt` 和 `totalWithdrawalMinor`；提现金额按已通过提现订单正向金额按用户汇总；手机端代理中心优先展示 `registeredAt`，并增加“提现 ¥xx”标签；同步更新架构说明和 Trellis 接口/前端规范。
- 验证结果：`cargo fmt --manifest-path backend/Cargo.toml`、`cargo check --manifest-path backend/Cargo.toml`、`cargo test --manifest-path backend/Cargo.toml user_invitation -- --nocapture`、手机端 `pnpm build` 和 `git diff --check` 均通过。

## 2026-06-16 06:13 HKT 合买机器人认购昵称随机化

- 完成任务：优化用户端合买机器人发起和认购时展示的匿名昵称。
- 解决问题：机器人参与合买时此前会显示类似“星河会员”“幸运会员”这类固定模板，用户容易感知为系统生成昵称。
- 实施内容：后端用户端合买 DTO 的机器人昵称池改为多组短中文昵称加数字后缀，例如脱敏后展示为“南风12**”这一类更随机的昵称；机器人发起人和机器人补单参与人都复用该规则，并新增测试保证不再包含真实机器人账号、机器人字样或“会员”模板；同步更新架构说明和后端接口契约。
- 验证结果：`cargo fmt --manifest-path backend/Cargo.toml -- --check`、`cargo test --manifest-path backend/Cargo.toml user_group_buy_plan_masks_robot_initiator_display -- --nocapture`、`cargo test --manifest-path backend/Cargo.toml user_group_buy_plan_returns_masked_participants -- --nocapture`、`cargo check --manifest-path backend/Cargo.toml`、后端全量 `cargo test --manifest-path backend/Cargo.toml -- --nocapture` 和 `git diff --check` 均通过；后端全量 323 个测试成功。

## 2026-06-16 05:32 HKT 彩种按自然时间节点周期开奖

- 完成任务：新增彩种开奖时间类型“时间节点周期”，支持从指定起始时间按自然时钟节点生成开奖时间。
- 解决问题：普通周期会沿用已有期号的秒级偏移，例如历史期号是 `20:18:27` 时下一期继续按 `27` 秒节拍生成；现在 5 分钟节点类彩种可以配置为 `00:00:00 + 300 秒`，让 `00:00` 售卖的期号在 `00:05` 开奖，并持续对齐 `00:10`、`00:15` 等自然节点。
- 实施内容：后端 `DrawSchedule` 增加 `timeNode`，期号生成器新增自然节点计算和偏移期号重新对齐逻辑；彩种保存校验 `intervalSeconds`、`startTime` 和整日整除规则；后台彩种新增/编辑抽屉支持选择“时间节点周期”；dashboard、彩种列表、控制台和手机端聚合接口同步识别该类型；数据库字段注释、架构说明和 Trellis 契约同步更新。
- 验证结果：`cargo fmt --manifest-path backend/Cargo.toml -- --check`、`cargo test --manifest-path backend/Cargo.toml time_node_schedule -- --nocapture`、`cargo test --manifest-path backend/Cargo.toml draw_schedule -- --nocapture`、`cargo check --manifest-path backend/Cargo.toml`、后端全量 `cargo test --manifest-path backend/Cargo.toml -- --nocapture`、管理后台 `npm run build`、手机端 `pnpm build` 和 `git diff --check` 均通过；管理后台构建仍只有既有 chunk size warning。

## 2026-06-16 04:25 HKT 手机端 Header 与底部导航背景统一

- 完成任务：将手机端所有页面顶部 Header 和底部主导航背景统一为首页的淡蓝紫粉渐变。
- 解决问题：部分页面顶部栏和 `mobile-bottom-nav` 仍使用白色或接近白色背景，和首页新视觉不一致。
- 实施内容：在 `mobile/src/index.css` 中新增 `--mobile-app-header-background`、`--mobile-app-header-border` 和 `--mobile-app-header-shadow`，让 `mobile-safe-header`、`mobile-safe-compact-header` 默认使用同款背景；在线客服和聊天大厅自定义顶部栏同步复用这组变量；`LayoutView.vue` 中 `mobile-bottom-nav` 内层导航容器同步使用这组变量；首页 Header 局部样式改为读取全局变量，并同步更新架构说明和前端规范。
- 验证结果：手机端 `pnpm build` 和 `git diff --check` 均通过。

## 2026-06-16 04:01 HKT 手机端首页彩种分类 Tabs

- 完成任务：将手机端首页普通彩种分类改为 Tabs 分类展示。
- 解决问题：原首页把每个彩种分类纵向全部铺开，分类多时页面过长，用户需要连续滚动才能切换分类。
- 实施内容：`HomeView.vue` 新增 `activeGroupTab` 和分类 key 兜底逻辑，使用 `van-tabs/van-tab` 渲染后端返回的彩种分类；每个 Tab 内保留两列 `HomeDrawCard` 彩种卡片；新增胶囊式 Tab 样式，并同步更新架构说明和前端组件规范。
- 验证结果：手机端 `pnpm build` 通过；`git diff --check` 通过。

## 2026-06-16 03:55 HKT 手机端首页 Header 背景统一

- 完成任务：将手机端首页 Header 背景调整为和彩种卡片一致的淡蓝紫粉渐变。
- 解决问题：首页顶部仍是白色半透明背景，和新彩种卡片渐变风格不统一。
- 实施内容：`HomeView.vue` 新增 `home-dashboard-header` 样式，使用 `radial-gradient(circle at 92% 5%, rgba(255, 255, 255, 0.78), transparent 30%), linear-gradient(135deg, #c8f5ff 0%, #d7c8ff 48%, #ffc4d7 100%)`，并保留安全区、毛玻璃和轻投影；同步更新架构说明。
- 验证结果：手机端 `pnpm build` 通过；`git diff --check` 通过。

## 2026-06-16 03:51 HKT 手机端首页彩种卡片背景调淡

- 完成任务：将手机端首页普通彩种卡片背景调淡。
- 解决问题：统一后的蓝紫粉渐变偏鲜艳，卡片整体视觉压迫感略强。
- 实施内容：`HomeDrawCard.vue` 将背景渐变调为更浅的 `#c8f5ff / #d7c8ff / #ffc4d7`，并降低卡片阴影强度；同步更新架构说明。
- 验证结果：手机端 `pnpm build` 通过；`git diff --check` 通过。

## 2026-06-16 03:48 HKT 手机端首页彩种卡片背景统一

- 完成任务：统一手机端首页普通彩种卡片背景风格。
- 解决问题：经典卡和地方卡使用不同渐变，视觉风格不够统一。
- 实施内容：`HomeDrawCard.vue` 将 `.group-lottery-card--classic` 和 `.group-lottery-card--regional` 合并使用同一套背景：`radial-gradient(circle at 92% 5%, rgba(255, 255, 255, 0.68), transparent 30%), linear-gradient(135deg, #7ee7ff 0%, #a78bfa 48%, #ff7aa8 100%)`；同步更新架构说明。
- 验证结果：手机端 `pnpm build` 通过；`git diff --check` 通过。

## 2026-06-16 03:43 HKT 手机端首页彩种卡片鲜艳化

- 完成任务：将手机端首页普通彩种卡片调整得更鲜艳。
- 解决问题：原普通彩种卡片底色偏淡，首页视觉识别度不足。
- 实施内容：`HomeDrawCard.vue` 中经典卡改为红橙暖色渐变，地方卡改为蓝紫粉撞色渐变；开奖号码球改为金色高亮，Logo 容器增加亮面底、边框和投影；同步更新架构说明。
- 验证结果：手机端 `pnpm build` 通过；`git diff --check` 通过。

## 2026-06-16 03:42 HKT 手机端首页彩种 Logo 放大

- 完成任务：将手机端首页普通彩种卡片 `.group-lottery-card__logo-shell` 调整为 `3.2rem`。
- 解决问题：当前彩种 Logo 容器偏小，首页卡片的彩种识别感不足。
- 实施内容：`HomeDrawCard.vue` 中基础样式和小屏覆盖样式都设置 `width: 3.2rem`、`height: 3.2rem`；同步更新架构说明中的普通彩种卡片 Logo 尺寸规则。
- 验证结果：手机端 `pnpm build` 通过；`git diff --check` 通过。

## 2026-06-16 03:37 HKT 手机端首页开奖号码球数字放大

- 完成任务：将手机端首页彩种卡片 `.group-lottery-card__digit` 的数字字号调整为 `0.88rem`。
- 解决问题：开奖号码球放大后，球内数字仍偏小，视觉不够醒目。
- 实施内容：`HomeDrawCard.vue` 中常规样式和小屏覆盖样式统一设置 `font-size: 0.88rem`。
- 验证结果：手机端 `pnpm build` 通过；`git diff --check` 通过。

## 2026-06-16 03:34 HKT 手机端首页开奖号码球放大

- 完成任务：放大手机端首页彩种卡片底部的开奖号码球。
- 解决问题：原开奖号码球偏小，首页卡片里的开奖结果不够醒目。
- 实施内容：`HomeDrawCard.vue` 将常规开奖号码球从 `0.96rem` 调整为 `1.14rem`，小屏覆盖从 `0.9rem` 调整为 `1.04rem`，并同步放大球内数字字号。
- 验证结果：手机端 `pnpm build` 通过；`git diff --check` 通过。

## 2026-06-16 03:30 HKT 手机端首页彩种卡片期号样式调整

- 完成任务：调整手机端首页彩种卡片 `.group-lottery-card__issue` 的颜色和字号。
- 解决问题：原期号颜色偏浅、字号偏小，在彩种卡片中不够清晰。
- 实施内容：`HomeDrawCard.vue` 将期号颜色改为 `#0d0d0dd1`，字号改为 `0.75rem`。
- 验证结果：手机端 `pnpm build` 通过；`git diff --check` 通过。

## 2026-06-16 03:25 HKT 手机端首页彩种卡片标题字号调整

- 完成任务：将手机端首页彩种卡片 `.group-lottery-card__copy h5` 的字体大小调整为 `.96rem`。
- 解决问题：原标题字号偏小，彩种名称在首页卡片中不够醒目。
- 实施内容：`HomeDrawCard.vue` 中普通样式和小屏覆盖样式统一设置为 `font-size: 0.96rem`。
- 验证结果：手机端 `pnpm build` 通过；`git diff --check` 通过。

## 2026-06-16 03:21 HKT 合买计划一键清除跳过未结算

- 完成任务：调整后台合买管理“一键清除合买计划列表”的清理口径，未结算计划不再被清除，也不会导致整次清理失败。
- 解决问题：原逻辑在存在草稿、进行中或已满单未结算计划时直接拒绝清除，运营无法清理已取消或已结算的历史计划。
- 实施内容：后端合买仓储只删除 `Cancelled` 和 `Settled` 计划，自动保留 `Draft`、`Open`、`Filled` 等未结算计划；后台确认文案和成功提示改为说明“未结算计划已保留”；同步更新架构说明、前后端规范。
- 验证结果：后端 `cargo test --manifest-path backend/Cargo.toml group_buy -- --nocapture` 通过，合买相关 43 条测试成功；后台 `npm run build` 通过；`cargo fmt --manifest-path backend/Cargo.toml` 和 `git diff --check` 通过。后台构建仍保留既有的大 chunk 提示。

## 2026-06-16 03:09 HKT 手机端合买自购输入框强化

- 完成任务：强化手机端下注页合买底栏“自购份数”输入框的可见性。
- 解决问题：原自购份数输入和周围文字融合，用户不容易看出中间数字可以点击编辑。
- 实施内容：`UnifiedBetBottomBar.vue` 为自购份数新增独立输入框容器、边框、浅底、阴影和聚焦高亮；同步更新架构说明和前端组件规范。
- 验证结果：手机端 `pnpm build` 通过；`git diff --check` 通过。

## 2026-06-16 03:18 HKT 手机端下注页移除合买每份提示卡

- 完成任务：移除手机端下注页合买模式中部“最低每份 / 固定每份”的黄色提示卡。
- 解决问题：合买自购份数、固定每份和需支付金额已经收敛到底部投注栏和合买摘要中，中部重复提示会增加页面高度并干扰下注操作。
- 实施内容：`DynamicBetPage.vue` 删除重复提示卡，仅在总金额无法按每份金额整除时保留单独错误提示；同步更新架构说明和前端组件规范。
- 验证结果：手机端 `pnpm build` 通过；`git diff --check` 通过。

## 2026-06-16 02:59 HKT 手机端我的记录 Tab 分组兜底修复

- 完成任务：修复手机端“我的记录”切换“我的注单 / 我的合买”时列表看起来没有变化的问题，并确保“合买认购中”只显示在“我的合买”。
- 解决问题：如果接口返回混合数据、旧缓存数据或字段形态是 `order_source/group_buy_pending_plan`，手机端只依赖后端 `view` 参数时可能无法正确区分未成单合买认购，导致两个 Tab 展示相同数据。
- 实施内容：`mobile/src/api/bet.ts` 归一化订单时兼容 `orderSource/order_source/source_name`，并把 `GB-` 开头的特殊记录识别为合买认购；`useBetOrders` 在每个 Tab 收到数据后按 `view` 做本地兜底过滤，`orders` 排除未成单合买，`groupBuy` 只保留未成单合买；切换 Tab 时强制刷新当前 Tab 第 1 页，避免旧缓存继续显示。
- 验证结果：手机端 `pnpm build` 通过；`git diff --check` 通过。

## 2026-06-16 02:57 HKT 手机端合买底栏自购份数输入迁移

- 完成任务：把手机端下注页合买模式的“自购份数”输入迁移到 `bet-bottom-bar`，并让合买底栏样式回到和普通下单更接近的左右两栏结构。
- 解决问题：上一版合买底栏做成独立结算卡片，和普通下注底栏差异过大，而且自购份数仍主要在页面中部编辑，不符合底部直接填写自购份数的操作预期。
- 实施内容：`UnifiedBetBottomBar.vue` 在合买模式下使用普通底栏同一套两栏骨架，左侧提供自购份数输入、总份数、共计注数、方案总额和需付金额，右侧保留红色“投注”主按钮；`DynamicBetPage.vue` 删除页面中部自购份数输入，把 `groupBuySelfShares` 通过 `v-model` 接到底栏，并在输入和失焦时沿用原自动推荐与夹取逻辑；同步修正架构说明和前端规范。
- 验证结果：手机端 `pnpm build` 通过；`git diff --check` 通过。

## 2026-06-16 02:51 HKT 手机端下注页底部投注栏简化

- 完成任务：移除手机端下注页底部“编辑单据”和“加入购物篮/加入购彩篮”入口，并按参考图重排合买模式的 `bet-bottom-bar`。
- 解决问题：原底栏同时提供编辑单据、加入购彩篮和提交按钮，普通下注路径显得复杂；合买模式底栏也没有形成清晰的自购份数、共计金额和投注按钮结构。
- 实施内容：`UnifiedBetBottomBar.vue` 改为单主按钮组件；普通投注只展示已选注数、总金额和“立即投注”；合买模式改为上下两行结算条，上方展示自购份数、确认勾选和共计注数，下方展示方案总额、实际需付金额和“投注”按钮；`DynamicBetPage.vue` 移除购物篮编辑弹层入口和显式加入事件，提交时继续静默把当前草稿转换为待提交单据；同步更新架构说明和前端规范。
- 验证结果：手机端 `pnpm build` 通过；`git diff --check` 通过。

## 2026-06-16 02:36 HKT 手机端我的记录区分注单和合买

- 完成任务：将手机端 `/orders` 页面调整为“我的记录”，通过“我的注单”和“我的合买”两个 Tab 区分真实已下单注单和未成单合买认购。
- 解决问题：此前未满单、未成单的合买认购会和真实下注记录混在同一个列表口径中，用户无法快速区分哪些已经形成投注订单、哪些仍属于合买认购记录。
- 实施内容：后端 `GET /api/user/bet/orders` 新增 `view=orders|groupBuy` 过滤参数；`orders` 返回真实已下单记录，`groupBuy` 返回未成单合买认购；手机端注单 composable 按 Tab 维护独立缓存、页码、加载状态和“加载更多”；页面新增紧凑分段 Tab 和对应空状态文案；OpenAPI 和架构说明同步补充中文契约。
- 验证结果：后端 `cargo fmt --manifest-path backend/Cargo.toml`、`cargo check --manifest-path backend/Cargo.toml` 通过；定向测试 `cargo test --manifest-path backend/Cargo.toml user_bet_order_view_filter_splits_orders_and_group_buy_participations -- --nocapture`、`cargo test --manifest-path backend/Cargo.toml user_visible_bet_orders_include_unformed_group_buy_participation -- --nocapture` 和 `cargo test --manifest-path backend/Cargo.toml openapi_document_contains_core_paths -- --nocapture` 通过；手机端 `pnpm build` 通过；`git diff --check` 通过。

## 2026-06-16 02:29 HKT 后台 APP 更新配置入口独立化

- 完成任务：把后台 APP 安装包上传和更新检查配置从“手机端设置”中拆出为独立“APP更新”标签。
- 解决问题：原配置实际存在，但放在“系统设置 / 手机端设置”下方，入口不够明显，运营容易以为后台没有 APK/IPA 上传与更新检查配置页面。
- 实施内容：系统设置默认打开“APP更新”标签；`mobile_app_*` 配置单独归组到“APP更新”；新增独立 `AppUpdateSettingsPanel` 展示 Android APK、iOS IPA 上传、更新检查开关、版本号、构建号、强制更新和更新说明；手机端展示设置只保留平台名称、Logo、介绍和首页高频配置。
- 验证结果：后台 `npm run build` 和 `git diff --check` 均通过；后台构建仍有既有的大 chunk 提示。

## 2026-06-16 02:25 HKT 控奖合买认购投注内容显示优化

- 完成任务：优化彩种控制台“控制开奖号码”抽屉中“合买认购记录”的投注内容展示。
- 解决问题：合买认购记录此前只展示 `plan.numbers` 原始文本并做截断，运营需要自己判断 `1|2|3`、胆码拖码或大小单双含义，控单扫描效率低。
- 实施内容：管理端新增合买投注文本解析展示工具，按玩法把投注内容转换为中文结构化行；直选展示“第 1 位/第 2 位/第 3 位”，胆拖展示“胆码/拖码”，大小单双展示“十位/个位”的中文属性；解析异常时回退展示“原始内容”并保留悬停原文。
- 验证结果：后台 `npm run build` 和 `git diff --check` 均通过；后台构建仍有既有的大 chunk 提示。

## 2026-06-15 23:45 HKT 手机端我的注单显示未成单合买认购

- 完成任务：让手机端“我的注单”列表显示当前用户已认购但尚未满单成单的合买记录。
- 解决问题：原用户端注单接口只合并了独立下注订单和已经生成真实投注订单的合买单；未满单、未成单的合买计划没有 `orderId`，因此不会出现在“我的注单”，用户看不到自己的认购记录。
- 实施内容：后端 `GET /api/user/bet/orders` 在合并真实订单后，把当前用户参与且尚未生成真实订单的合买计划映射为特殊合买认购记录，并返回计划状态、未成单标记、参与金额和认购份数；手机端订单 API 归一化新增字段，订单卡片和详情页显示“合买认购中”“未成单”“单份金额”和“认购份数”；详情仍可通过合买计划 ID 加载参与人列表；同步更新 OpenAPI 中文说明和 Trellis 后端 API 契约。
- 验证结果：后端 `cargo fmt --manifest-path backend/Cargo.toml --check`、`cargo check --manifest-path backend/Cargo.toml` 通过；后端定向测试 `cargo test --manifest-path backend/Cargo.toml user_visible_bet_orders -- --nocapture` 和 `cargo test --manifest-path backend/Cargo.toml openapi -- --nocapture` 通过；后端全量 `cargo test --manifest-path backend/Cargo.toml` 通过（313 个测试成功）；后台 `npm run build` 和手机端 `npm run build` 通过；`git diff --check` 通过。

## 2026-06-15 23:36 HKT 控奖抽屉显示未成单合买认购记录

- 完成任务：让彩种控制台“控制开奖号码”抽屉在用户下单信息之外，同时展示当前期号的合买认购记录。
- 解决问题：原控奖抽屉只展示已经形成真实投注订单的数据，未成单、未满单的合买计划只有认购记录，没有投注订单，因此控单时看不到这部分用户资金参与情况。
- 实施内容：后端新增 `GET /api/admin/group-buy/plans/by-issue`，按 `lotteryId + issue` 返回仍在流转中的合买计划完整详情和参与人；管理端新增控奖合买认购 hook；控奖抽屉新增“合买认购记录”表格，显示计划、状态、用户、认购时间、玩法、投注内容、金额、份数、占比、进度和真实订单状态；刷新按钮同步刷新订单和认购记录。
- 验证结果：后端 `cargo fmt --check`、`cargo check --manifest-path backend/Cargo.toml` 通过；后台 `npm run build` 通过；后台构建仍保留既有的大 chunk 提示。

## 2026-06-15 23:10 HKT 后台控奖抽屉订单刷新修复

- 完成任务：修复彩种控制台“控制开奖号码”抽屉选择销售中期号后可能看不到最新订单的问题。
- 解决问题：控奖抽屉订单来自控制台当前 `orders` 快照，原来主要依赖 10 秒轮询；用户刚下注或运营刚打开抽屉、切换控制范围和期号时，列表可能还停留在上一轮数据，看起来像缓存未更新。
- 实施内容：打开“控制开奖号码”抽屉时立即触发控制台刷新；切换控制范围、控制期号或指定订单时同步刷新订单数据；用户下单信息区域新增“刷新订单”按钮并显示刷新 loading，方便运营手动拉取最新订单。
- 验证结果：后台 `npm run build` 通过；构建仍保留既有的大 chunk 提示。

## 2026-06-15 21:58 HKT 后台控制开奖号码抽屉排版优化

- 完成任务：按手绘草图调整后台彩种控制台“控制开奖号码”抽屉排版。
- 解决问题：原抽屉表单从上到下堆叠，彩种、控制范围、控制期号、启用开关、开奖号码和用户下单信息的主次关系不够清晰，控单时需要上下扫描。
- 实施内容：抽屉顶部改为三列控制区，依次展示“控制彩种”“控制范围”“控制期号/指定订单”；中部改为“总开关（是/否）”和“开奖号码”双栏，使用 Semi UI `Switch` 表达启用状态；底部保留大面积“用户下单信息”表格，作为控单核对区域。
- 验证结果：后台 `npm run build` 通过；本地后台页面可打开到登录页且浏览器控制台无错误；后台构建仍有既有的大 chunk 提示。

## 2026-06-15 14:09 HKT 手机端合买大厅分类筛选条优化

- 完成任务：优化手机端合买大厅顶部彩种分类筛选条的尺寸和视觉样式。
- 解决问题：原分类按钮高度和内边距偏大，阴影较重，在合买大厅紧凑列表中占用过多竖向空间。
- 实施内容：分类筛选条改为更紧凑的胶囊按钮；按钮高度降为 28px 左右，减小间距和内边距；选中态使用红色渐变和轻阴影，未选态使用浅底细边；横向滚动隐藏滚动条并保留触摸惯性。
- 验证结果：手机端 `pnpm build` 通过。

## 2026-06-15 14:06 HKT 手机端合买页面移除隐藏 Tabs 容器

- 完成任务：移除手机端合买页面中包裹“大厅/我的”的 Vant `Tabs` 结构。
- 解决问题：合买页面虽然隐藏了 Tab 头部，但运行时仍生成 `van-tabs__wrap`、`van-tabs__nav` 和 `van-tabs__line`，造成无用 DOM 和额外布局占位。
- 实施内容：`GroupBuyView.vue` 改为通过 `activeTab` 使用 `v-if/v-else` 直接渲染大厅列表或我的合买列表；保留原有查询参数、返回按钮、数据加载、分页加载和详情弹窗逻辑。
- 验证结果：手机端 `pnpm build` 通过；`GroupBuyView.vue` 已无 `van-tabs`、`group-buy-tabs` 和 `hidden-tab-header` 残留。

## 2026-06-15 13:22 HKT 系统性能与实时链路优化

- 完成任务：对后端高频列表、开奖调度扫描、前端实时刷新、WebSocket 稳定性和聊天渲染做一轮系统性优化。
- 解决问题：
  - 后台期号、合买、资金账户、资金流水、充值、提现和订单列表在路由层先拉全量再分页，数据增长后会拖慢响应。
  - 用户端资金流水、充值、提现和下注页配置也存在不必要的全量读取。
  - 开奖调度每轮扫描所有历史期号，历史 `drawn/cancelled` 数据越多越影响调度节拍。
  - 手机端 WebSocket 固定 3 秒重连，没有退避、心跳超时和页面离屏降级。
  - 后台彩种控制台固定轮询不区分页面可见性，窗口切换时可能重复刷新。
  - 手机端客服长会话会渲染全部消息，历史消息多时页面重绘压力较大。
- 实施内容：
  - 新增后端通用 `PageRequest/ListPage`，统一仓储分页响应口径。
  - `draws`、`group_buys`、`finance`、`orders`、`recharges`、`withdrawals` 仓储新增分页查询入口；PostgreSQL 模式下使用 `COUNT + LIMIT/OFFSET` 下推过滤和分页，内存模式保持兼容。
  - 后台期号、合买、资金账户、资金流水、充值订单、提现申请、投注订单列表切到仓储分页入口。
  - 用户端资金流水、充值订单、提现申请和下注页期号配置减少全量读取；下注页只读取最近一页期号用于当前期、等待开奖和最近开奖计算。
  - 代理返利统计改为只读取返利/返利提现流水、已支付充值和已通过提现；代理返利明细按返利流水在仓储层分页后再补充下级统计。
  - 新增资金流水、充值订单和提现申请的状态/用户/创建时间索引，支撑分页、返利统计和用户记录查询。
  - 开奖自动化和调度补期改为读取 `open/closed` 活跃期号，不再每轮扫描所有历史已开奖/已取消期号。
  - 手机端 WebSocket 增加指数退避、最大重试次数、心跳超时关闭重连，以及页面隐藏时停止重连、回前台再恢复。
  - 后台彩种控制台轮询在页面隐藏时跳过，focus/visibility 刷新增加节流。
  - 手机端客服会话只渲染最近 160 条消息，并展示已折叠历史提示；聊天大厅初始加载也裁剪到最近 100 条。
- 验证结果：后端 `cargo fmt --manifest-path backend/Cargo.toml`、`cargo check --manifest-path backend/Cargo.toml`、全量 `cargo test --manifest-path backend/Cargo.toml` 均通过（312 个测试成功）；后台 `npm run build`、手机端 `pnpm build` 和 `git diff --check` 均通过；后台构建仍保留既有的大 chunk 提示。

## 2026-06-15 13:01 HKT 前端聊天与消息体验优化

- 完成任务：优化手机端在线客服、手机端聊天大厅和后台在线客服的消息交互体验。
- 解决问题：
  - 手机端客服和聊天大厅收到新消息时会直接滚到底部，用户正在查看历史消息时容易被打断。
  - 手机端图片消息点击后直接打开原链接，缺少移动端图片预览体验。
  - 输入法组合输入时按回车可能误触发送，发送中也可能重复点击或重复提交。
  - 后台客服切换用户会话后，上一会话草稿、图片和表情面板可能残留，容易误发到新会话。
- 实施内容：
  - 手机端在线客服和聊天大厅增加“有新消息/N 条新消息”浮动提示，只有用户接近底部或主动点击提示时才滚动到底部。
  - 手机端在线客服图片消息改用 Vant 图片预览，并给消息列表预留底部输入栏安全距离。
  - 手机端在线客服和聊天大厅统一处理回车发送，组合输入、组合键和发送中状态不会误发送。
  - 后台在线客服切换会话时清空草稿、待发图片和表情面板，回复成功后自动回到输入框。
- 验证结果：手机端 `pnpm build` 通过；后台 `npm run build` 通过；`git diff --check` 通过。后台构建仍保留既有的大 chunk 提示。

## 2026-06-15 12:51 HKT 用户端分页优化闭环

- 完成任务：把用户端列表分页能力从后端接口继续接到手机端 API、Pinia 缓存、页面展示和 OpenAPI 文档。
- 解决问题：
  - 上一轮只完成了后端分页能力，手机端仍然按全量列表使用，数据多时页面会越来越重。
  - 注单、充值、提现、资金流水、合买大厅和我的合买缺少“加载更多”入口，用户只能等全量请求完成。
  - OpenAPI 文档没有说明用户端记录接口支持 `page/pageSize`，后续联调容易继续按全量接口使用。
- 实施内容：
  - 手机端 `fetchUserBetOrders`、`fetchRechargeOrders`、`fetchWithdrawalOrders`、`fetchUserLedgerEntries`、`fetchGroupBuyHall`、`fetchMyGroupBuys` 增加分页查询参数。
  - `mobileUserData` 为充值订单、提现申请和资金流水维护当前页、是否还有更多和追加去重逻辑。
  - 注单记录、资金流水、充值页、提现页、合买大厅和我的合买列表增加“加载更多/已加载全部”状态。
  - 合买大厅和我的合买筛选、切换、创建和认购后仍刷新第一页，只有用户点击加载更多时追加下一页。
  - 后端 OpenAPI 对用户端列表接口补充 `page/pageSize` 查询参数说明，并修复注单列表时间排序参数方向。
- 验证结果：手机端 `pnpm build` 通过；后端 `cargo check --bin bc-backend` 通过；后端格式化检查待本次最终复跑确认。

## 2026-06-15 06:02 HKT 用户端记录接口分页与时间排序优化

- 完成任务：为用户端资金流水、注单、合买、充值和提现列表增加可选分页参数，并按时间倒序返回。
- 解决问题：用户端多处列表一直返回全量数据并保持原始顺序，历史订单越多页面越慢，切换到“我的记录”时也缺乏稳定的倒序展示。
- 实施内容：
  - 在 `backend/src/routes/user.rs` 新增 `UserPageQuery`，支持 `page`、`page_size`。
  - 在以下接口接入分页：
    - `GET /api/user/ledger-entries`
    - `GET /api/user/bet/orders`
    - `GET /api/user/group-buy/my`
    - `GET /api/user/recharge/orders`
    - `GET /api/user/withdrawals`
  - `GET /api/user/group-buy/plans` 增加分页参数并按创建时间倒序，兼容不传分页参数的全量返回。
  - 新增统一时间解析与倒序排序工具：`parse_user_timestamp_seconds`、`compare_created_time_desc`，支持 `unix:` 与 `yyyy-mm-dd HH:MM:SS` 两种时间串格式，分页时稳定排序。
- 验证结果：后端 `cargo check --bin bc-backend` 通过。编译不再出现未使用导入告警。

## 2026-06-15 03:46 HKT 彩种卡片隐藏过期控制期号

- 完成任务：彩种控制台卡片上的“开奖控制”摘要不再展示已经过去的控制期号。
- 解决问题：
  - 历史控奖配置如果目标期号已过开奖时间，卡片仍会显示控制号码和旧期号，容易让运营误以为该控制还会生效。
  - 截图中的旧期号控制信息会继续占用卡片摘要位置，影响实时扫描。
- 实施内容：
  - `LotteryConsolePage.tsx` 新增“可展示控奖配置”判断。
  - 指定期号或指定订单所在期号必须仍处于 `open/closed`，并且开奖时间未过去，卡片才显示控制号码和目标信息。
  - 目标期号不存在、已开奖、已取消或已过期开奖时间时，卡片按“未启用”展示，不再显示旧号码、旧期号和旧更新时间。
  - 同步更新前端组件规范和架构说明。
- 验证结果：管理端 `npm run build` 通过；`git diff --check` 通过。后台构建仍保留既有的大 chunk 提示。

## 2026-06-15 03:43 HKT 控制开奖号码期号过滤已取消期号

- 完成任务：彩种控制台“控制开奖号码”的“控制期号”下拉过滤已取消期号。
- 解决问题：
  - “控制期号”此前只排除了已开奖期号，已取消期号仍可能出现在可选项中。
  - 如果历史配置或接口绕过前端提交已取消期号，控奖配置可能残留在无效目标上。
- 实施内容：
  - 管理后台 `LotteryConsolePage.tsx` 的控制期号候选只保留 `open/closed` 状态。
  - 已取消目标期号和已开奖目标期号一样，自动取消“启用控制开奖”并展示“指定期号已结束”提示。
  - 后端 `normalize_admin_draw_control_target()` 对已取消指定期号兜底归一化为关闭控制。
  - 新增后端定向测试覆盖已取消指定期号自动关闭控奖。
  - 同步更新前端规范、后端 API 契约和架构说明。
- 验证结果：后端 `cargo test --manifest-path backend/Cargo.toml draw_control_issue_target_disables_when_issue -- --nocapture` 通过，2 条定向测试成功；管理端 `npm run build` 通过；`cargo fmt --manifest-path backend/Cargo.toml --check` 和 `git diff --check` 均通过。后台构建仍保留既有的大 chunk 提示。

## 2026-06-15 02:48 HKT 合买计划列表一键清除入口命名

- 完成任务：合买管理的一键清除入口统一改为“合买计划列表”口径。
- 解决问题：
  - 原按钮文案为“清除合买记录”，不够直观，运营不一定能看出这是清除合买计划列表。
- 实施内容：
  - 合买管理页按钮文案改为“一键清除合买计划列表”。
  - 二次确认、成功提示和失败提示统一使用“合买计划列表/合买计划”描述。
  - 同步更新前端规范、后端 API 契约和架构说明。
- 验证结果：管理端 `npm run build` 通过；`git diff --check` 通过。后台构建仍保留既有的大 chunk 提示。

## 2026-06-15 02:21 HKT 手机端首页顶部淡红氛围优化

- 完成任务：手机端首页最上方增加淡红色氛围层。
- 解决问题：
  - 首页顶部此前主要是白色 Header 和内容区，视觉上偏平，缺少一点品牌红色过渡。
  - 需要增强美观但不能影响 Banner、钱包和彩种卡片点击。
- 实施内容：
  - `HomeView.vue` 根容器增加不拦截交互的淡红线性渐变顶罩。
  - 首页 Header 调整为更柔和的半透明白底，并增加淡红色底部分隔线。
  - 主内容层级提升，确保卡片和 Banner 始终位于氛围层之上。
  - 同步更新前端规范和架构说明。
- 验证结果：手机端 `pnpm build` 通过；`git diff --check` 通过。

## 2026-06-15 01:55 HKT 控制开奖号码期号过滤已结束期号

- 完成任务：彩种控制台“控制开奖号码”抽屉的“控制期号”下拉过滤已开奖和已取消期号。
- 解决问题：
  - “控制期号”此前直接展示当前彩种全部期号，已经开奖或已取消的期号也会出现在可选项中，容易让运营误选无效目标。
  - 没有销售中期号时，默认期号可能回退到最近已开奖或已取消期号。
- 实施内容：
  - `LotteryConsolePage.tsx` 新增控制期号候选过滤逻辑，只保留 `open/closed` 期号。
  - “控制期号”下拉和默认控制期号共用候选过滤结果，避免渲染和默认值口径不一致。
  - 同步更新前端规范和架构说明。
- 验证结果：管理端 `npm run build` 通过；`git diff --check` 通过。后台构建仍保留既有的大 chunk 提示。

## 2026-06-15 01:53 HKT 管理后台合买计划列表清理

- 完成任务：管理后台合买管理新增“清除合买计划列表”能力。
- 解决问题：
  - 合买计划和参与记录会随着测试、机器人和运营操作不断累积，后台缺少一键清理入口。
  - 直接删除未完成合买会导致扣款、退款或派奖失去业务追溯，需要在后端统一限制。
- 实施内容：
  - 后端新增 `DELETE /api/admin/group-buy/plans/clear`，复用 `deletedCount` 清理响应格式。
  - 合买仓储新增清理方法，已取消或已结算记录可以清空，草稿、进行中或已满单未结算计划会返回中文业务错误。
  - 管理端合买页面新增清理按钮，点击后中文二次确认，成功后关闭详情抽屉、回到第一页并刷新 dashboard。
  - 同步更新后端 API 契约、前端组件规范和架构说明。
- 验证结果：后端 `cargo test --manifest-path backend/Cargo.toml group_buy -- --nocapture` 通过，合买相关 38 条测试成功；管理端 `npm run build` 通过；`cargo fmt --manifest-path backend/Cargo.toml --check` 和 `git diff --check` 均通过。

## 2026-06-15 01:33 HKT 手机端下注底部栏适配合买自购状态

- 完成任务：手机端下注页底部固定操作栏适配合买自购份数状态。
- 解决问题：
  - 开启合买后，底部栏仍只展示普通投注口径的“已选/共”，用户看不到自购份数和实际需支付金额。
  - 合买底部栏内容增加后，如果页面底部预留空间不变，容易遮挡自购份数输入区。
- 实施内容：
  - `UnifiedBetBottomBar.vue` 新增合买模式摘要，展示方案总额、自购份数和需支付金额。
  - `DynamicBetPage.vue` 向底部栏传入自购份数、总份数和预计支付金额。
  - 合买模式下下注页主内容增加底部 padding，避免固定栏遮挡自购设置。
  - 同步更新前端规范和架构说明。
- 验证结果：手机端 `pnpm build`、`pnpm test` 和 `git diff --check` 均通过；当前手机端测试脚本显示 0 个测试用例。

## 2026-06-14 23:51 HKT 手机端下注页受限全选随机抽取

- 完成任务：手机端下注页位置选号存在最大数量限制时，“全”按钮改为随机抽取允许数量的号码。
- 解决问题：
  - 玩法配置最多选择 7 个号码时，点击“全”此前会固定选择号码池前 7 个，用户每次都得到 `0,1,2,3,4,5,6`。
  - 胆拖玩法全选胆码也存在固定取前缀的问题。
- 实施内容：
  - `positionLimits.ts` 新增随机抽取和随机受限裁剪工具。
  - `DynamicBetPage.vue` 的“全”按钮改用随机受限裁剪；没有上限的位置仍选中全部号码。
  - 胆拖玩法全选胆码改为随机抽取，同时继续排除对侧已选号码。
  - “大/小/单/双”预设按钮保持原有预设裁剪行为，不引入随机。
  - 同步更新前端规范和架构说明。
- 验证结果：手机端 `pnpm build`、`pnpm test` 和 `git diff --check` 均通过；当前手机端测试脚本显示 0 个测试用例。

## 2026-06-14 23:33 HKT 手机端合买方案详情显示参与人

- 完成任务：手机端合买大厅和“我的合买”的方案详情支持查看参与人列表。
- 解决问题：
  - 方案详情此前只展示金额、进度和投注信息，用户无法在方案页核对已有认购人和自己的参与记录。
  - 大厅列表不适合直接携带参与人明细，需要继续通过详情接口懒加载完整数据。
- 实施内容：
  - `GroupBuyView.vue` 的详情弹层新增“参与人/认购明细”区块。
  - 参与人列表展示脱敏名、参与时间、认购金额和份数，当前用户使用“我”标记并高亮。
  - 无参与人数据时显示中文空状态。
  - 同步更新前端规范和架构说明。
- 验证结果：手机端 `pnpm build`、`pnpm test` 和 `git diff --check` 均通过；当前手机端测试脚本显示 0 个测试用例。

## 2026-06-14 22:10 HKT 彩种控制台已开奖指定期号自动关闭控奖

- 完成任务：彩种控制台控奖抽屉在“指定期号”模式下选择已开奖期号时，自动取消“启用控制开奖”勾选。
- 解决问题：
  - 控制范围选择“指定期号”后，如果控制期号已经开奖，继续保持启用控制没有实际意义，还容易让运营误以为控奖仍会生效。
  - 只靠前端取消勾选不够稳妥，直接请求保存接口仍可能提交已开奖期号的启用控制。
- 实施内容：
  - 管理后台 `LotteryConsolePage.tsx` 增加期号状态归一化，选中已开奖期号时自动关闭启用控制，并展示中文 warning 提示。
  - 控奖抽屉打开后轮询刷新到目标期号已开奖时，同步刷新当前彩种数据并取消启用控制。
  - 后端 `normalize_admin_draw_control_target()` 在指定期号已开奖时兜底把控制配置归一化为关闭状态。
  - 新增后端定向测试覆盖已开奖指定期号会自动关闭控奖。
  - 同步更新前端规范和架构说明。
- 验证结果：`cargo fmt --manifest-path backend/Cargo.toml --check`、`cargo test --manifest-path backend/Cargo.toml draw_control_issue_target_disables_when_issue_drawn -- --nocapture`、`cargo check --manifest-path backend/Cargo.toml`、后台 `npm run build` 和 `git diff --check` 均通过；后台构建仍保留既有的大 chunk 提示。

## 2026-06-14 21:57 HKT 手机端合买注单详情显示参与人列表

- 完成任务：手机端注单详情在合买下单场景中展示参与人列表。
- 解决问题：
  - 用户查看合买注单详情时，只能看到自己的参与金额，无法核对整单有哪些参与记录。
  - 直接把参与人塞进注单列表会增加列表接口负担，也可能暴露完整用户名和机器人账号。
- 实施内容：
  - 后端 `/api/user/bet/orders` 对合买订单返回 `groupBuyPlanId`，供手机端详情懒加载合买计划详情。
  - 后端 `UserGroupBuyPlan` 增加 `participants` 脱敏参与人摘要，只返回展示名、认购金额、份数、是否本人和参与时间。
  - 手机端合买计划 API 归一化 `participants` 字段。
  - 手机端打开合买注单详情时通过 `groupBuyPlanId` 拉取合买详情，并在 `OrderDetailSheet.vue` 中展示参与人列表。
  - 同步更新前端规范、后端 API 契约和架构说明。
- 验证结果：`cargo fmt --manifest-path backend/Cargo.toml --check`、`cargo check --manifest-path backend/Cargo.toml`、合买注单和参与人脱敏定向测试、手机端 `pnpm build`、手机端 `pnpm test` 和 `git diff --check` 均通过；当前手机端测试脚本显示 0 个测试用例。

## 2026-06-14 21:46 HKT 手机端注单详情移除投注内容

- 完成任务：移除手机端注单详情中的“投注内容”展示。
- 解决问题：
  - 注单详情中投注内容号码/属性展示占用空间，用户当前不需要在详情弹层里重复查看投注内容。
  - 详情弹层仍保留注单信息、开奖号码、匹配项和订单元信息，避免影响开奖核对和订单追踪。
- 实施内容：
  - `OrderDetailSheet.vue` 删除“投注内容”面板中的号码球、大小单双属性标签和“查看更多”逻辑。
  - 原面板改为“注单信息”，保留玩法名称、注单类型、赔率、单注金额、注数、倍数、金额和结算金额。
  - `useBetOrders()` 不再为详情弹层计算 `selectedOrderNumbers`，`HistoryView.vue` 不再传递该 prop。
  - 清理详情弹层中已废弃的投注内容样式。
  - 同步更新前端规范和架构说明。
- 验证结果：手机端 `pnpm build`、`pnpm test` 和 `git diff --check` 均通过；当前手机端测试脚本显示 0 个测试用例。

## 2026-06-14 07:52 HKT 手机端点击响应体验优化

- 完成任务：优化手机端按钮点击、底部导航切换和常用页面首次打开的响应体验。
- 解决问题：
  - 顶层路由使用完整路径作为 key，导致手机端主框架在子页面切换时被重复卸载重建，底部导航、WebSocket 和未读状态会跟着重建，体感上像按钮响应慢。
  - 部分可点击元素没有全局触摸行为约束，移动端 WebView 可能出现默认点击等待、高亮和滚动边界干扰。
  - 懒加载页面首次点击进入时需要临时加载 chunk，常用入口可能出现短暂等待。
  - 应用启动时等待站点配置接口返回后才挂载页面，接口慢时会拖慢首屏进入。
- 实施内容：
  - 顶层 `App.vue` 改为按根路由保持 Shell key，登录页和主布局切换时才重建外壳，主布局内部子页面切换不再重建整个 `LayoutView`。
  - 移除顶层路由 `out-in` 等待模式，并把页面切换动画从 `180ms` 缩短到 `96ms`。
  - 在全局 CSS 中为按钮、链接、Vant 常用点击控件增加 `touch-action: manipulation`、去除移动端 tap 高亮，并保留输入控件正常编辑行为。
  - 应用启动只等待本地登录态恢复，站点 Logo、平台名称和介绍文案改为页面挂载后异步刷新。
  - 新增常用页面 chunk 分批预加载，应用挂载后优先在浏览器空闲时间预热首页、合买、聊天、开奖、我的、充值、客服、提现和代理中心等入口。
  - 底部导航点击当前页面时直接忽略，避免无效路由跳转。
- 验证结果：手机端 `pnpm build`、`pnpm test` 和 `git diff --check` 均通过；当前手机端测试脚本显示 0 个测试用例。

## 2026-06-14 07:11 HKT 控制开奖号码下单信息精简与玩法中文化

- 完成任务：优化彩种控制台“控制开奖号码”抽屉中的用户下单信息展示。
- 解决问题：
  - 用户下单信息的“下注信息”列会额外显示展开注码 Tag，对控奖扫描来说信息过重。
  - “玩法”列直接显示玩法编码，不便于运营快速识别玩法。
- 实施内容：
  - 控制开奖号码表格中的 `OrderBetInfo` 关闭展开注码展示，不再渲染 `mt-2 flex max-w-[320px] flex-wrap gap-1` 对应的 Tag 区域。
  - 彩种控制台加载玩法目录，并在用户下单信息“玩法”列展示中文玩法名称。
  - 抽取共享 `formatPlayRuleLabel()`，订单管理和控奖表格统一使用同一套玩法中文标签逻辑。
- 验证结果：后台 `npm run build` 和 `git diff --check` 均通过；后台构建仍保留既有的大 chunk 提示。

## 2026-06-14 07:07 HKT 控制开奖号码默认指定销售期号

- 完成任务：调整彩种控制台“控制开奖号码”抽屉的默认控制范围和期号。
- 解决问题：
  - 新打开控奖抽屉时默认控制范围仍偏向整彩种，容易让运营误触后续所有开奖。
  - 控制期号没有自动聚焦销售中期号，运营需要手动从期号列表里选择当前开售期。
- 实施内容：
  - 控制开奖号码表单默认 `targetScope` 改为“指定期号”。
  - 打开控奖抽屉时，如果当前没有启用中的历史控奖配置，控制期号自动取销售中的 `open` 期号。
  - 切换到“指定期号”或“指定订单所在期号”时，统一优先按销售中期号匹配期号和待开奖订单。
  - 已启用的历史控奖配置继续按原范围和期号展示，避免覆盖正在生效的控制策略。
- 验证结果：后台 `npm run build` 和 `git diff --check` 均通过；后台构建仍保留既有的大 chunk 提示。

## 2026-06-14 07:04 HKT 代理返利详情显示用户总充值

- 完成任务：代理返利统计和详情新增用户总充值展示。
- 解决问题：
  - 代理返利详情此前只能看到返利金额和下级总提现，运营无法直接核对下级用户累计充值贡献。
  - 下级返利记录只展示单笔充值金额，无法看到该下级用户历史总充值。
- 实施内容：
  - 后端 `AgentRebateSummary` 新增 `directInviteeRechargeMinor`，按直属下级已入账充值订单汇总。
  - 后端 `AgentRebateRecord` 新增 `inviteeTotalRechargeMinor`，按返利明细中的下级用户汇总已入账充值。
  - 返利统计列表新增“下级总充值”列。
  - 代理返利详情顶部新增“下级总充值”标签和汇总项。
  - 下级返利记录表新增“用户总充值”列。
  - OpenAPI 描述和架构说明同步更新。
- 验证结果：`cargo fmt --manifest-path backend/Cargo.toml --check`、`cargo test --manifest-path backend/Cargo.toml agent_rebate -- --nocapture`、`cargo check --manifest-path backend/Cargo.toml`、后台 `npm run build` 和 `git diff --check` 均通过；后台构建仍保留既有的大 chunk 提示。

## 2026-06-14 06:23 HKT 控制开奖号码显示合买订单

- 完成任务：彩种控制台“控制开奖号码”的用户下单信息支持显示合买订单。
- 解决问题：
  - 控奖抽屉复用后台订单接口时没有显式包含机器人数据，合买机器人补单参与后生成或关联的合买订单可能被过滤掉。
  - 用户下单信息表格没有“来源”列，运营即使看到订单也不能直接区分独立下单和合买下单。
- 实施内容：
  - 彩种控制台拉取订单时传入 `includeRobotData=true`，控单场景使用完整订单口径。
  - 控制开奖号码 `SideSheet` 用户下单信息表格新增“来源”列。
  - 使用中文标签展示“独立下单 / 合买下单”，合买订单使用醒目的标签颜色。
  - 同步更新架构说明。
- 验证结果：后台 `npm run build` 和 `git diff --check` 均通过；后台构建仍保留既有的大 chunk 提示。

## 2026-06-14 06:19 HKT 手机端客服图片不展示用户文件名

- 完成任务：手机端在线客服用户发送图片后不再显示图片文件名。
- 解决问题：
  - 用户发送图片时前端把本地 `file.name` 写入客服消息内容，图片下方会多显示类似 `2.png` 的文件名。
  - 历史用户图片消息如果已经保存文件名，重新打开会话仍会把文件名渲染出来。
- 实施内容：
  - 用户发送客服图片时，消息 `content` 改为空字符串，不再上传本地文件名。
  - 图片消息渲染时，用户自己发送的图片不展示 `content`。
  - 客服发送的图片说明继续保留，避免影响客服备注说明。
  - 同步更新架构说明。
- 验证结果：手机端 `pnpm build` 和 `git diff --check` 均通过。

## 2026-06-14 06:16 HKT 控制开奖号码下单时间展示

- 完成任务：彩种控制台“控制开奖号码”的用户下单信息新增下单时间展示。
- 解决问题：
  - 控单时只能看到订单、用户、期号、玩法和金额，缺少下单时间，运营无法直接按订单时序核对控单目标。
  - 历史订单时间可能存在 `unix:` 标签，不能在后台页面原样展示。
- 实施内容：
  - 控制开奖号码 `SideSheet` 的用户下单信息表格新增“下单时间”列。
  - 复用后台统一 `formatDateTime` 时间格式化函数，展示为 `YYYY-MM-DD HH:mm:ss`。
  - 同步更新架构说明。
- 验证结果：后台 `npm run build` 和 `git diff --check` 均通过；后台构建仍保留既有的大 chunk 提示。

## 2026-06-14 06:11 HKT 彩种控制台筛选优化

- 完成任务：彩种控制台默认筛选销售开启彩种，并支持按彩种名称搜索。
- 解决问题：
  - 控制台默认展示全部彩种，停售彩种和无关彩种会干扰运营查看实时开盘、封盘和开奖状态。
  - 彩种数量增长后，运营只能通过状态筛选和页面扫描找目标彩种，缺少名称搜索入口。
- 实施内容：
  - 将彩种控制台 `statusFilter` 默认值改为 `saleEnabled`。
  - 状态筛选卡片新增“搜索彩种名称”输入框。
  - 名称搜索与状态筛选叠加生效，筛选按钮计数会按当前名称搜索结果重新计算。
  - 同步更新架构说明。
- 验证结果：后台 `npm run build` 和 `git diff --check` 均通过；后台构建仍保留既有的大 chunk 提示。

## 2026-06-14 06:05 HKT 用户维护用户名不可编辑

- 完成任务：用户维护中已有用户的用户名改为只读。
- 解决问题：
  - 用户名是登录标识和历史业务审计的重要显示口径，后台维护时可编辑会导致订单、资金流水、客服记录等历史核对口径变化。
  - 只在前端禁用不够安全，直接调用更新接口仍可能改名。
- 实施内容：
  - 后台用户维护 `SideSheet` 编辑已有用户时禁用“用户名”输入框，并提示“用户名创建后不可编辑”。
  - 新建用户时用户名输入框保持可编辑。
  - 后端 `AccessStore::update_user()` 强制保留原用户名，同时继续保留原余额、邀请码和头像。
  - 更新后端单元测试，覆盖传入不同用户名时仍返回原用户名。
  - 同步更新架构说明。
- 验证结果：`cargo test --manifest-path backend/Cargo.toml access_repository_update_preserves_username_balance_and_invite_code -- --nocapture`、`cargo fmt --manifest-path backend/Cargo.toml --check`、`cargo check --manifest-path backend/Cargo.toml`、后台 `npm run build` 和 `git diff --check` 均通过；后台构建仍保留既有的大 chunk 提示。

## 2026-06-14 05:58 HKT 用户列表支持删除用户

- 完成任务：后台用户列表新增删除用户能力。
- 解决问题：
  - 用户管理只能停用或锁定账号，无法清理测试账号、重复账号或确认不再使用的账号。
  - 直接硬删用户可能误删有余额或仍作为上级代理的账号，导致财务和代理关系断链。
- 实施内容：
  - 后端新增 `DELETE /api/admin/users/{id}`，删除用户基础资料并清理用户密码哈希、会话、重置码和提现方式。
  - 删除前校验资金账户余额和冻结金额必须为 0；仍有下级用户引用该用户为上级代理时拒绝删除。
  - 历史订单、资金流水、充值和提现记录不随用户删除，继续保留用户 ID 作为审计线索。
  - 后台用户列表操作列新增“删除”危险按钮，点击前弹出确认提示，删除成功后刷新用户列表和系统概览。
  - OpenAPI 文档同步登记用户删除接口。
  - 同步更新架构说明。
- 验证结果：`cargo test --manifest-path backend/Cargo.toml access_repository_deletes_user_and_access_artifacts -- --nocapture`、`cargo test --manifest-path backend/Cargo.toml access_repository_rejects_delete_user_with_direct_invitees -- --nocapture`、`cargo test --manifest-path backend/Cargo.toml openapi_document_contains_core_paths -- --nocapture`、`cargo fmt --manifest-path backend/Cargo.toml --check`、`cargo check --manifest-path backend/Cargo.toml`、后台 `npm run build` 和 `git diff --check` 均通过；后台构建仍保留既有的大 chunk 提示。

## 2026-06-14 05:51 HKT 用户列表停用和锁定职责拆分

- 完成任务：用户列表操作列去掉单独“锁定”快捷操作，把“停用/锁定”的使用场景拆开。
- 解决问题：
  - 用户列表同时展示“停用”和“锁定”按钮，两者都会禁止用户登录，运营扫描列表时容易认为是重复功能。
  - 后端此前对停用和锁定用户统一返回“用户账号未激活”，用户端无法知道账号是运营停用还是安全锁定。
- 实施内容：
  - 用户列表快捷操作只保留“停用”和“启用/解除锁定”，锁定状态只在用户维护 `SideSheet` 的状态下拉中维护。
  - 用户维护状态字段增加说明：停用用于运营主动禁用账号，锁定用于安全异常冻结账号，两种状态都会禁止登录。
  - 后端用户登录和 token 恢复时区分停用、锁定错误文案，分别返回“用户账号已停用”和“用户账号已锁定”。
  - 新增后端测试覆盖停用登录、锁定登录和锁定会话恢复三种拒绝场景。
  - 同步更新架构说明。
- 验证结果：`cargo test --manifest-path backend/Cargo.toml access_repository_distinguishes_suspended_and_locked_user_login_errors -- --nocapture`、`cargo fmt --manifest-path backend/Cargo.toml --check`、`cargo check --manifest-path backend/Cargo.toml`、后台 `npm run build` 和 `git diff --check` 均通过；后台构建仍保留既有的大 chunk 提示。

## 2026-06-14 05:41 HKT 代理返利详情显示下级总提现

- 完成任务：代理返利详情抽屉中明确显示直属下级总提现金额。
- 解决问题：
  - 详情顶部原汇总文案为“下级提现”，运营不容易确认这是下级已通过提现的总额；查看时需要从明细表格再人工理解。
- 实施内容：
  - 代理返利详情标题右侧新增“下级总提现”金额标签。
  - 顶部金额汇总项从“下级提现”改为“下级总提现”。
  - 同步更新架构说明。
- 验证结果：后台 `npm run build` 和 `git diff --check` 均通过；后台构建仍保留既有的大 chunk 提示。

## 2026-06-14 05:39 HKT 订单合买认购详情抽屉宽度调整

- 完成任务：订单列表中的“合买认购详情” `SideSheet` 宽度调整为 80%。
- 解决问题：
  - 原固定 720px 宽度在展示合买计划、投注内容和认购记录表格时空间偏窄，运营查看金额、份数和用户信息不够舒展。
- 实施内容：
  - 将 `OrderManagementPage.tsx` 中合买认购详情抽屉 `width={720}` 改为 `width="80%"`。
  - 同步更新架构说明。
- 验证结果：后台 `npm run build` 和 `git diff --check` 均通过；后台构建仍保留既有的大 chunk 提示。

## 2026-06-14 04:53 HKT 订单列表查看合买认购详情

- 完成任务：订单管理列表中，合买下单的订单支持直接查看对应合买计划的认购详情。
- 解决问题：
  - 运营在订单列表看到合买总单后，无法直接确认这个合买订单由哪些用户认购、各自认购金额和占比，需要跳转到合买管理再人工按订单号查找。
- 实施内容：
  - 后端新增 `GET /api/admin/orders/{id}/group-buy-plan`，通过投注订单号反查合买计划，并返回计划详情和参与记录。
  - 该接口会拒绝独立下单订单，并在合买订单缺少认购记录时返回明确错误。
  - OpenAPI 文档同步登记订单合买认购详情接口。
  - 后台订单列表仅对“合买下单”显示“认购详情”按钮。
  - 点击后打开 `SideSheet`，上方展示订单、计划、彩种、期号、玩法、进度和投注内容，下方展示全部认购用户、用户 ID、金额、份数、占比、认购时间和备注。
  - 同步更新架构说明。
- 验证结果：`cargo fmt --manifest-path backend/Cargo.toml --check`、后台 `npm run build`、`cargo check --manifest-path backend/Cargo.toml`、`cargo test --manifest-path backend/Cargo.toml group_buy -- --nocapture`、`cargo test --manifest-path backend/Cargo.toml openapi_document_contains_core_paths -- --nocapture` 和 `git diff --check` 均通过；后台构建仍保留既有的大 chunk 提示。

## 2026-06-14 04:44 HKT 注册可选 QQ 联系方式

- 完成任务：用户注册支持选填 QQ 联系方式，并在后台用户维护中展示和编辑。
- 解决问题：
  - 手机端注册此前没有联系方式字段，客服或财务后续核对用户时只能依赖用户名、邮箱或聊天记录。
- 实施内容：
  - 后端用户资料新增 `contactQq/contact_qq` 字段，新增迁移 `20260614043000_add_user_contact_qq.sql`。
  - 用户注册接口支持可选 `contactQq`，后端校验填写时必须为 5-12 位数字。
  - 手机端注册表单新增 QQ 输入框，并和邀请码放在同一行，避免撑高登录注册页。
  - 手机端注册密码提示和前端校验同步为至少 8 位，与后端规则一致。
  - 后台用户列表展示用户 QQ，用户维护 `SideSheet` 增加“联系方式 QQ”输入框。
  - 同步更新架构说明。
- 验证结果：`cargo check --manifest-path backend/Cargo.toml`、`cargo test --manifest-path backend/Cargo.toml access_repository_registers_user_by_username_or_email -- --nocapture`、手机端 `npm run build`、后台 `npm run build`、`cargo fmt --manifest-path backend/Cargo.toml --check` 和 `git diff --check` 均通过；后台构建仍保留既有的大 chunk 提示。

## 2026-06-14 04:32 HKT 后台用户锁定筛选与密码重置

- 完成任务：用户管理支持按账号状态筛选锁定用户，并在用户维护抽屉中重置普通用户登录密码。
- 解决问题：
  - 运营此前无法直接筛选出锁定状态用户，只能通过排序或人工扫描列表定位异常账号。
  - 用户维护抽屉没有普通用户密码重置入口，后台只能依赖用户端忘记密码流程处理登录密码问题。
- 实施内容：
  - 后端 `GET /api/admin/users` 增加 `status` 查询参数，支持 `active/suspended/locked` 过滤。
  - 后端新增 `PATCH /api/admin/users/{id}/password`，复用用户密码规则和 Argon2 哈希写入 `user_password_hashes`。
  - 后台用户列表增加状态筛选下拉框和“锁定”快捷按钮。
  - 后台用户维护 `SideSheet` 增加“重置密码/初始密码”输入框，留空不修改，填写后保存会重置会员登录密码。
  - 同步更新架构说明。
- 验证结果：`cargo fmt --manifest-path backend/Cargo.toml`、`cargo check --manifest-path backend/Cargo.toml`、`cargo test --manifest-path backend/Cargo.toml access_repository_supports_admin_reset_user_password -- --nocapture`、`cargo test --manifest-path backend/Cargo.toml user_list_status_filter_keeps_only_locked_users -- --nocapture`、后台 `npm run build` 和 `git diff --check` 均通过；后台构建仍保留既有的大 chunk 提示。

## 2026-06-11 23:22 HKT 修复体彩排列5新增后的种子彩种数量测试

- 完成任务：修复 CI 中 `repository_uses_seeded_memory_lotteries` 失败。
- 解决问题：
  - 新增 `pl5` 体彩排列5后，内存种子彩种总数从 22 增加到 23，但旧测试仍断言 22，导致全量后端测试失败。
- 实施内容：
  - 将内存种子彩种数量断言更新为 23。
  - 增加 `pl5` 存在性断言，避免测试只检查数量而没有覆盖新增彩种。
- 验证结果：`cargo fmt --manifest-path backend/Cargo.toml --check`、`cargo test --manifest-path backend/Cargo.toml repository_uses_seeded_memory_lotteries -- --nocapture` 和 `cargo test --manifest-path backend/Cargo.toml -- --nocapture` 均通过，后端全量测试 292 个通过。

## 2026-06-11 22:56 HKT 体彩排列3/排列5 API68 接入

- 完成任务：新增体彩排列3独立开奖源，并接入体彩排列5彩种和开奖源。
- 解决问题：
  - `pl3` 过去复用 `api68-fc3d` 的 `lotCode=10041`，无法使用用户提供的体彩排列3独立接口。
  - 系统此前没有 `pl5` 体彩排列5彩种和默认开奖源。
- 实施内容：
  - `api68-fc3d` 默认来源收窄为只绑定 `fc3d`。
  - 新增 `api68-pl3`，endpoint 为 `https://api.api68.com/QuanGuoCai/getLotteryInfo1.do`，`lotCode=10043`，绑定 `pl3`。
  - 新增 `pl5` 体彩排列5默认彩种，号码类型为 `fiveDigit`，默认每日 `21:00:15` 开奖、停售、关闭合买。
  - 新增 `api68-pl5`，endpoint 为 `https://api.api68.com/QuanGuoCai/getLotteryInfo.do`，`lotCode=10044`，绑定 `pl5`。
  - 新增迁移修正旧库中的 `pl3` 绑定，并补齐 `api68-pl3/api68-pl5` 开奖源。
  - 同步更新架构说明和后端 API 契约。
- 验证结果：`cargo fmt --manifest-path backend/Cargo.toml --check`、`cargo check --manifest-path backend/Cargo.toml`、`cargo test --manifest-path backend/Cargo.toml draw_api -- --nocapture`、`cargo test --manifest-path backend/Cargo.toml seeded_lotteries_include_requested_api68_lotteries -- --nocapture` 和 `git diff --check` 均通过；本机没有 `psql` 命令，未执行迁移 SQL 的事务回滚试跑。

## 2026-06-11 22:41 HKT 手机端合买大厅发起人头像展示

- 完成任务：合买大厅列表卡片左侧改为显示发起人头像。
- 解决问题：
  - 合买大厅此前使用本地生成的彩票图标，用户无法直观看到合买发起人。
  - 直接展示机器人头像可能暴露机器人身份，需要继续保持前台匿名规则。
- 实施内容：
  - 后端用户端合买响应新增 `initiatorAvatarUrl`，普通用户计划按 `initiatorUserId` 读取用户头像。
  - 机器人合买计划不返回真实头像，继续只返回脱敏发起人展示名。
  - 手机端合买 API 类型和适配层接入 `initiatorAvatarUrl`。
  - 手机端合买大厅卡片使用公共缓存头像组件展示圆形发起人头像，无头像时显示脱敏名首字。
  - 同步更新架构说明和前后端规范。
- 验证结果：`cargo fmt --manifest-path backend/Cargo.toml --check`、`cargo check --manifest-path backend/Cargo.toml`、`cargo test --manifest-path backend/Cargo.toml user_group_buy_plan -- --nocapture`、`cd mobile && npm run build` 和 `git diff --check` 均通过。

## 2026-06-11 22:00 HKT 手机端客服未读 Badge 提示

- 完成任务：手机端客服回复未读提示从小红点升级为数字 Badge。
- 解决问题：
  - 客服有新回复时，手机端底部“我的”和个人中心“在线客服”此前只显示小红点，用户无法直观看到未读数量。
  - 在线客服多会话标签也只显示红点，多个未读会话不够醒目。
- 实施内容：
  - 手机端注册 Vant `Badge` 组件，用于未读数量展示。
  - 底部导航“我的”图标右上角改为显示客服未读数量 Badge，超过 99 显示 `99+`。
  - 个人中心“在线客服”设置项右侧显示客服未读数量 Badge，并继续保留“有新消息”文案。
  - 在线客服多会话标签使用数字 Badge 标记每个会话的 `userUnreadCount`。
  - 同步更新架构说明和前端规范，要求客服未读提示使用后端 `userUnreadCount` 和 Vant Badge。
- 验证结果：`cd mobile && npm run build` 和 `git diff --check` 均通过。

## 2026-06-11 21:53 HKT 后台在线客服已解决会话删除

- 完成任务：后台在线客服支持删除已解决的用户会话。
- 解决问题：
  - 客服会话处理到“已解决”后，后台只能继续保留在列表中，无法直接清理已完成会话。
  - 多个后台窗口同时查看在线客服时，某个窗口删除会话后其它窗口可能继续看到旧会话。
- 实施内容：
  - 后端新增 `DELETE /api/admin/support/conversations/{id}`，只允许删除 `resolved` 状态会话。
  - 客服仓储新增已解决会话删除逻辑，删除后同步持久化，并保留处理中、等待用户和已关闭状态的删除保护。
  - 新增 `support.conversation_deleted` 实时事件，后台和用户连接收到后可移除本地会话。
  - 管理后台在线客服详情中，只有当前会话状态为“已解决”时显示“删除会话”按钮，点击后直接删除并刷新工作台。
  - 同步更新 OpenAPI、架构说明、前端组件规范和后端 API 契约。
- 验证结果：`cargo fmt --manifest-path backend/Cargo.toml --check`、`cargo check --manifest-path backend/Cargo.toml`、`cargo test --manifest-path backend/Cargo.toml support -- --nocapture`、`cargo test --manifest-path backend/Cargo.toml openapi -- --nocapture`、`cd admin && npm run build`、`cd mobile && npm run build` 和 `git diff --check` 均通过。

## 2026-06-11 21:44 HKT 后台在线客服未读优先排序

- 完成任务：优化后台在线客服会话列表排序和未读标记。
- 解决问题：
  - 后台客服会话此前按会话 ID 排序，最新用户消息不会自动排到前面。
  - WebSocket 实时更新时会话被追加到列表末尾，导致客服需要手动查找最新未读。
  - 未读数只显示普通数字，不够醒目。
- 实施内容：
  - 后端客服仓储 `list()` 和 `list_for_user()` 改为按“未读优先、最近消息/更新时间倒序、会话 ID 倒序”排序。
  - 后台 `useSupportConversations` 在初始加载和实时 upsert 后复用相同排序，保证新消息推送后列表立即重排。
  - 在线客服表格未读行增加浅红底，主题旁显示红点，未读列使用 Semi UI `Badge` 展示未读数量；已读显示“已读”标签。
  - 新增客服仓储排序测试，覆盖未读且最近的会话排在最前。
  - 同步更新架构说明、前端组件规范和后端 API 契约。
- 验证结果：`cargo fmt --manifest-path backend/Cargo.toml --check`、`cargo check --manifest-path backend/Cargo.toml`、`cargo test --manifest-path backend/Cargo.toml support_repository -- --nocapture`、`cd admin && npm run build` 和 `git diff --check` 均通过。

## 2026-06-11 20:59 HKT 后台表格列宽拖拽

- 完成任务：后台所有原生表格支持通过拖动表头右侧边缘调整列宽。
- 解决问题：
  - 订单、财务、用户、合买、计奖派奖等运营表格字段较多时，列宽固定，运营无法临时放大用户名、期号、下注信息、说明等关键列。
  - 每个页面单独写拖拽逻辑会造成维护成本高且行为不一致。
- 实施内容：
  - 新增 `useResizableAdminTables` 全局 hook，自动扫描后台页面和 SideSheet 中的 `<table>`。
  - 表头右侧增加列宽拖拽热区，拖动后按列同步设置表头和表体单元格宽度。
  - hook 使用 `MutationObserver` 处理页面切换、抽屉打开和动态列表更新，不需要各页面逐个接入。
  - 全局 CSS 增加列宽拖拽手柄、拖拽中光标和禁止选中文本样式。
  - 前端组件规范和架构说明同步记录后台表格列宽拖拽约定。
- 验证结果：`cd admin && npm run build` 和 `git diff --check` 均通过；后台构建仅保留既有 Vite chunk 体积提示。

## 2026-06-11 20:52 HKT 平台开奖期号格式可配置

- 完成任务：新增平台开奖彩种的期号生成格式配置能力。
- 解决问题：
  - 平台开奖此前统一按开奖时间生成 `yyyyMMddHHmmss` 期号，运营无法按彩种配置不同的期号格式。
  - 后台彩种新增/编辑没有期号格式入口，无法满足平台开奖彩种差异化规则。
- 实施内容：
  - `lotteries` 表新增 `issue_format` 字段，默认 `{yyyy}{MM}{dd}{HH}{mm}{ss}`，并补充中文字段注释。
  - 后端彩种模型、仓储 SQL、种子彩种、测试夹具和期号生成器接入 `issueFormat`。
  - 平台开奖模式使用配置模板生成期号；API 开奖仍按开奖源最新期号顺延，不受平台模板影响。
  - 期号模板支持 `{yyyy}`、`{yy}`、`{MM}`、`{dd}`、`{HH}`、`{mm}`、`{ss}`、`{date}`、`{time}`、`{timestamp}`，并校验非法变量、空结果和过长结果。
  - 后台彩种新增/编辑 SideSheet 在“平台开奖”时显示“平台期号格式”输入框。
- 验证结果：`cargo fmt --manifest-path backend/Cargo.toml --check`、`cargo check --manifest-path backend/Cargo.toml`、`cargo test --manifest-path backend/Cargo.toml draw_generation -- --nocapture`、`cd admin && npm run build` 和 `git diff --check` 均通过；后台构建仅保留既有 Vite chunk 体积提示。

## 2026-06-11 20:40 HKT 订单列表 unix 时间展示转换

- 完成任务：把后台订单列表中可能出现的 `unix:秒` 原始时间标签转换为可读时间。
- 解决问题：
  - 订单列表“开奖”列直接渲染 `settledAt`，当后端返回 `unix:1781102946` 时会原样展示内部时间编码。
  - 订单列表此前没有展示创建时间，运营核对订单顺序时缺少直接时间信息。
- 实施内容：
  - 订单列表订单列新增创建时间，使用公共 `formatDateTime` 处理 `createdAt`。
  - “开奖”列的结算时间改为使用公共 `formatDateTime` 处理 `settledAt`。
  - 前端组件规范和架构说明同步补充订单创建/结算时间的展示规则。
- 验证结果：`cd admin && npm run build` 和 `git diff --check` 均通过；后台构建仅保留既有 Vite chunk 体积提示。

## 2026-06-11 04:24 HKT 侧边栏注册配置入口更名

- 完成任务：把后台侧边栏里的“用户注册”入口更名为“系统配置”。
- 解决问题：原名称容易让运营误以为该入口是用户端注册页面，而实际功能是维护注册方式、邮箱注册和邀请码要求。
- 实施内容：
  - 后端 dashboard 模块 `registration` 的展示名改为“系统配置”，说明改为“注册方式与邀请要求”。
  - 管理后台侧边栏 `registration` 图标改为设置图标。
  - 模块 key、权限映射和注册配置接口保持不变。
  - 同步更新架构说明和前端组件规范。
- 验证结果：`cargo check --manifest-path backend/Cargo.toml`、`cd admin && npm run build` 和 `git diff --check` 均通过；后台构建仅保留既有 Vite chunk 体积提示。

## 2026-06-11 04:18 HKT 机器人配置删除能力修复

- 完成任务：修复后台机器人配置无法删除普通机器人配置的问题。
- 解决问题：
  - 旧实现为了保护合买机器人和购彩机器人，把机器人删除接口和前端删除入口全部移除，导致后台新建或测试类普通机器人配置也无法删除。
  - 页面说明仍写着“机器人配置不能删除”，和当前运营需要不一致。
- 实施内容：
  - 后端机器人响应新增 `deletable` 字段，由仓储根据机器人 ID 计算删除权限。
  - 新增 `DELETE /api/admin/robots/{id}`，普通机器人配置可删除，核心内置机器人返回“内置机器人配置不能删除，请改为暂停或禁用”。
  - 管理后台 API client 和 `useRobots` 接入删除能力，机器人列表与编辑抽屉新增删除按钮。
  - 后台页面删除普通机器人后关闭对应抽屉、刷新工作台；核心内置机器人删除按钮禁用并保留暂停/禁用能力。
  - 同步更新 OpenAPI、架构说明、后端 API 契约和前端组件规范。
- 验证结果：`cargo fmt --manifest-path backend/Cargo.toml --check`、`cargo check --manifest-path backend/Cargo.toml`、`cargo test --manifest-path backend/Cargo.toml robot -- --nocapture`、`cargo test --manifest-path backend/Cargo.toml openapi -- --nocapture`、`cd admin && npm run build` 和 `git diff --check` 均通过；后台构建仅保留既有 Vite chunk 体积提示。

## 2026-06-11 04:09 HKT 财务时间展示转换

- 完成任务：把资金流水等财务记录中的 `unix:秒` 原始时间标签转换为可读时间展示。
- 解决问题：
  - 后台财务管理资金流水直接渲染 `createdAt`，当历史数据或接口返回 `unix:1781104790` 时，运营会看到内部时间编码。
  - 同一财务页的充值、提现时间字段也存在直接展示原始值的风险。
  - 手机端公共时间解析没有识别 `unix:秒`，我的账户资金流水等页面可能继续露出原始标签。
- 实施内容：
  - 后台公共格式化工具新增财务时间格式化能力，支持标准日期、`unix:秒` 和秒级时间戳，统一展示为北京时间 `YYYY-MM-DD HH:mm:ss`。
  - 财务管理充值订单、提现管理和资金流水时间字段统一调用 `formatDateTime`。
  - 手机端 `parseChinaDateTime` 支持 `unix:秒` 与秒级时间戳，复用 `formatDateTime` 的资金页面同步受益。
  - 同步更新架构说明和前端组件规范。
- 验证结果：`cd admin && npm run build`、`cd mobile && npm run build` 和 `git diff --check` 均通过；后台构建仅保留既有 Vite chunk 体积提示。

## 2026-06-10 21:02 HKT 手机端下注页封盘后轮询修复

- 完成任务：修复手机端下注页封盘时间到达后长期停在“开奖中”且不进入下一期的问题。
- 解决问题：
  - 后端下注页配置此前只按期号状态 `open` 返回 `round.status=selling`，没有排除已经超过 `sale_closed_at` 的旧期号。
  - 前端倒计时会根据过期封盘时间显示“开奖中”，但轮询逻辑只在接口状态为 `opening` 时启动，导致页面显示与刷新状态脱节。
- 实施内容：
  - 后端下注页配置新增当前时间判断，只有 `open` 且 `sale_closed_at` 未过的期号才返回 `selling`。
  - 已过封盘时间但仍为 `open` 的期号改为 `opening` 候选返回，保留期号展示并触发手机端开盘轮询。
  - 手机端动态下注页把 `selling + sale_stop_at 已过` 也视为需要轮询下一期，并在本地封盘后禁用加入购彩篮和提交按钮。
  - 新增后端测试覆盖过期 `open` 期号进入 `opening` 状态，以及存在下一期可售期时优先展示下一期。
- 验证结果：`cargo fmt --manifest-path backend/Cargo.toml --check`、`cargo check --manifest-path backend/Cargo.toml`、`cargo test --manifest-path backend/Cargo.toml mobile_bet -- --nocapture`、完整 `cargo test --manifest-path backend/Cargo.toml -- --nocapture`、手机端 `npm run build`、手机端 `npm run test` 和 `git diff --check` 均通过；后端完整测试 278 条通过，手机端测试脚本当前显示 0 个测试用例。

## 2026-06-10 20:32 HKT 开奖调度快慢阶段拆分

- 完成任务：把开奖调度拆成封盘补期快阶段和开奖结算慢阶段，并修复周期彩种晚调度时的期号节拍漂移。
- 解决问题：调度器原先一轮内串行执行封盘、API开奖、平台开奖、结算、机器人和补期，且整轮结束后才推送 WebSocket；当 API 或机器人耗时较长时，60 秒平台彩种的新期开奖事件会被拖到只剩几十秒甚至十几秒才到用户端。
- 实施内容：
  - `run_draw_automation` 拆成 `close_due_draw_issues` 与 `draw_due_issues` 两个阶段，原入口保留并合并两个阶段结果。
  - 调度器先执行封盘、补齐未来期号并立即推送 `lottery.issue_closed`、`lottery.issue_opened`。
  - 开奖结果、资金余额变更和机器人订单推送改为慢阶段完成后发送。
  - 调度后台线程改成固定节拍追赶，不再每轮执行完成后额外 sleep 一个完整周期。
  - 周期彩种补期以最新本地开奖时间对齐固定周期；调度晚到时仍保持原开奖节拍。
  - 新增测试覆盖晚调度仍生成 `20:19:27` 这类固定节拍下一期，以及 WebSocket 事件顺序先开盘后开奖。
- 验证结果：`cargo fmt --check`、`cargo check`、完整 `cargo test` 和 `git diff --check` 均通过；后端完整测试 276 条通过，新增覆盖晚调度仍保持固定节拍和 WebSocket 先开盘后开奖的事件顺序。

## 2026-06-10 20:02 HKT API开奖源延迟可配置

- 完成任务：为 API 开奖彩种新增“API开奖延迟秒数”配置，并让自动开奖调度按延迟后的时间请求第三方开奖源。
- 解决问题：部分第三方 API 在官方开奖时间到达后不会立刻返回开奖号码，平台如果马上请求会拿不到结果，导致彩种在前端长时间显示“开奖中”并反复等待调度。
- 实施内容：
  - `lotteries` 表新增 `api_draw_delay_seconds` 字段、非负约束和字段中文注释。
  - `LotteryKind`、彩种仓储 SQL、内置彩种种子和测试夹具补齐新字段。
  - 自动开奖调度封盘逻辑保持不变，只在 API 彩种开奖候选判断时叠加延迟秒数；未到延迟时间时不会请求开奖源。
  - 后台彩种新增/编辑抽屉在开奖模式为 API 接口时显示延迟秒数输入框，并提交为非负整数。
  - 新增自动化测试覆盖“先封盘不开奖、延迟到点后读取 API 并开奖”的流程。
- 验证结果：`cargo fmt --check`、`cargo check`、完整 `cargo test`、管理后台 `npm run build` 和 `git diff --check` 均通过；后端完整测试 274 条通过，后台构建仅保留既有 Vite chunk 体积提示。

## 2026-06-10 17:54 HKT 开奖调度 API 读取并发化

- 完成任务：把开奖调度中的外部 API 最新期号和开奖号码读取改为并发预取。
- 解决问题：调度器此前一轮内部按期号串行请求 API，某个彩种或旧期号接口慢时会拖住整轮调度，导致其它彩种也长时间停留在“等待调度”或“等待开奖源”。
- 实施内容：
  - `run_draw_automation` 先收集本轮到期开奖候选，再并发预取 API 最新期号，用于旧期号重试上限判断。
  - 对未被旧期上限跳过、且没有后台控奖号码的 API 期号，并发预取开奖号码。
  - 开奖写库、订单结算、派奖入账和合买状态更新仍按原期号顺序串行执行，避免重复开奖或重复派奖。
  - `DrawRepository` 新增预取开奖号码读取入口和“使用已预取 API 号码开奖”的内部入口；后台控奖号码仍优先于预取 API 号码。
  - 同步更新架构说明和后端 API 契约规范。
- 验证结果：`cargo fmt --manifest-path backend/Cargo.toml`、`cargo fmt --manifest-path backend/Cargo.toml --check`、`cargo check --manifest-path backend/Cargo.toml`、`cargo test --manifest-path backend/Cargo.toml automation_ -- --nocapture`、`cargo test --manifest-path backend/Cargo.toml prefetched -- --nocapture`、完整 `cargo test --manifest-path backend/Cargo.toml -- --nocapture` 和 `git diff --check` 均通过；后端完整测试 273 条通过。

## 2026-06-10 04:02 HKT 彩种控制台立即同步开奖源

- 完成任务：后台彩种控制台新增 API 彩种“立即同步开奖源”按钮，并补齐后端手动校准接口。
- 解决问题：API 彩种本地待开奖期号和外部开奖源偏移时，运营此前只能等待调度或手动处理期号，缺少直接按开奖源校准当前可销售期的入口。
- 实施内容：
  - 新增 `POST /api/admin/lotteries/{id}/sync-draw-source`，按当前绑定 API 开奖源和调度封盘提前量计算目标期号。
  - 同步时目标期不存在则生成，目标期已存在且未开奖则更新为 `open` 并校准开奖/封盘时间。
  - 同彩种其它 `open/closed` 旧期如果没有待开奖订单会自动取消；如果存在待开奖订单则保留到结果中，避免静默影响资金链路。
  - 后台彩种控制台 API 彩种卡片新增“立即同步”按钮，执行中显示 loading，成功后用中文 Toast 展示同步结果并刷新控制台。
  - 同步更新 OpenAPI、架构说明、后端 API 契约规范和前端组件规范。
- 验证结果：`cargo fmt --manifest-path backend/Cargo.toml --check`、`cargo check --manifest-path backend/Cargo.toml`、`cargo test --manifest-path backend/Cargo.toml sync_api_draw_source -- --nocapture`、`cargo test --manifest-path backend/Cargo.toml kj_txffc_source -- --nocapture`、完整 `cargo test --manifest-path backend/Cargo.toml -- --nocapture`、管理后台 `npm run build` 和 `git diff --check` 均通过；后端完整测试 272 条通过，后台构建仅保留既有 Vite chunk 体积提示。

## 2026-06-10 02:10 HKT API开奖旧期号停止过度重试

- 完成任务：给 API 开奖旧期号增加“距离最新期号超过 5 期则停止重试”的自动开奖规则。
- 解决问题：API 期号已经明显落后开奖源最新期号时，每轮调度仍继续请求旧期号开奖号码，多个旧期号叠加第三方接口等待会拖慢同一轮调度，导致本地平台开奖彩种到点后长时间显示“等待调度”。
- 实施内容：`run_draw_automation` 在 API 期号开奖前读取并缓存同彩种最新 API 期号；当前期号和最新期号都是纯数字且差距超过 5 期时，直接写入 `skippedIssues` 并提示“停止重试旧期号”，不再请求旧期号开奖号码；刚好相差 5 期仍保持原开奖源请求逻辑。同步更新架构说明和后端 API 契约规范。
- 验证结果：`cargo fmt --manifest-path backend/Cargo.toml --check`、`cargo check --manifest-path backend/Cargo.toml`、`cargo test --manifest-path backend/Cargo.toml automation_ -- --nocapture`、完整 `cargo test --manifest-path backend/Cargo.toml -- --nocapture` 和 `git diff --check` 均通过；后端完整测试 270 条通过，新增覆盖超过 5 期停止重试和刚好 5 期继续重试。

## 2026-06-10 01:53 HKT 后台资金流水和用户列展示补齐

- 完成任务：补齐后台资金流水类型展示，并统一多个后台表格的用户列展示。
- 解决问题：
  - 资金流水前端类型和中文映射缺少 `redPacketDebit`、`redPacketCredit`，红包相关流水会出现类型无法显示。
  - 财务管理“资金流水”只显示用户 ID，运营需要额外查询用户名。
  - 首页最近订单、计奖派奖明细和彩种控制台控单表格仍存在只显示用户 ID 的位置。
- 实施内容：
  - 后台资金流水接口返回 `username`，前端资金流水用户列改为“用户名 + 用户 ID”。
  - 资金流水类型补充“红包支出”和“红包入账”的中文文案与颜色。
  - 后台首页最近订单摘要补充用户名，并在首页表格展示用户名和用户 ID。
  - 计奖派奖结算明细接口和前端表格补充用户名。
  - 彩种控制台目标订单下拉和用户下单信息表改为展示用户名和用户 ID。
  - 合买计划参与记录用户列改为显示参与用户 ID，不再误显示参与记录 ID。
  - 同步更新 OpenAPI、架构说明和前端组件规范。
- 验证结果：`cargo fmt --manifest-path backend/Cargo.toml --check`、`cargo check --manifest-path backend/Cargo.toml`、`cd admin && npm run build` 和 `git diff --check` 均通过；后台构建仅保留既有 Vite chunk 体积提示。

## 2026-06-10 01:23 HKT 后台合买详情与参与记录改为 SideSheet

- 完成任务：将后台合买计划的“计划详情”和“参与记录”从页面常驻区域改为点击“查看详情”后通过 Semi UI `SideSheet` 打开。
- 解决问题：
  - 合买管理页此前在列表右侧或下方保留详情维护区，会挤占列表扫描空间。
  - 旧结构中参与记录和计划详情的展示顺序与当前需求不一致。
- 实施内容：
  - 合买列表继续保留最右侧“查看详情”按钮，点击后先打开详情抽屉再加载计划详情。
  - 主页面移除常驻详情提示区和详情卡片，只保留统计、筛选、分页列表、新增入口。
  - 详情抽屉上半部分展示计划详情，下半部分展示参与记录，并保留保存计划状态和添加参与记录能力。
  - 当前查看计划行使用高亮，详情加载中在抽屉内显示 loading，加载失败在抽屉内展示错误提示。
  - 同步更新架构说明和前端组件规范。
- 验证结果：`cd admin && npm run build` 和 `git diff --check` 均通过；后台构建仅保留既有 Vite chunk 体积提示。

## 2026-06-10 00:54 HKT 后台新增合买计划改为 SideSheet

- 完成任务：将后台合买管理的“新增合买计划”表单改为点击按钮后通过 Semi UI `SideSheet` 打开。
- 解决问题：
  - 合买管理页此前把新增计划表单常驻在列表下方，页面首屏会被维护表单挤占，不利于运营扫描合买计划。
  - 创建计划和查看计划详情都在同一页展开，容易让运营误以为创建后会自动进入详情维护。
- 实施内容：
  - 合买管理顶部新增“新增合买计划”按钮。
  - 原新增计划字段、期号/玩法联动、金额校验和创建逻辑保持不变，承载方式改为 `SideSheet`。
  - 创建成功后自动关闭抽屉，刷新工作台数据，并继续要求通过“查看详情”打开参与记录和计划详情。
  - 同步更新架构说明和前端组件规范。
- 验证结果：`cd admin && npm run build` 和 `git diff --check` 均通过；后台构建仅保留既有 Vite chunk 体积提示。

## 2026-06-10 00:35 HKT 后台合买计划详情按操作按钮打开

- 完成任务：调整后台合买计划列表的详情打开方式。
- 解决问题：
  - 合买管理页此前会在加载列表时自动选中并加载第一条计划详情，右侧直接显示“参与记录”和“计划详情”。
  - 计划 ID 本身承担打开详情交互，不符合“最右侧操作列点击查看详情才显示维护区”的使用习惯。
- 实施内容：
  - `useGroupBuyPlans` 取消初次加载自动拉取第一条计划详情，创建计划后也只更新列表。
  - 合买计划列表新增最右侧“操作”列，并提供“查看详情”按钮。
  - 计划 ID 改为普通文本，点击详情按钮后才调用 `loadPlan()`。
  - 未选择计划时右侧只展示提示；选择计划后才显示“参与记录”和“计划详情”。
  - 同步更新架构说明和前端组件规范。
- 验证结果：`cd admin && npm run build` 和 `git diff --check` 均通过；后台构建仅保留既有 Vite chunk 体积提示。

## 2026-06-10 00:18 HKT 后台订单列表字段与下注信息展示优化

- 完成任务：优化后台订单管理列表展示字段。
- 解决问题：
  - 订单列表原先把期号放在订单号下方小字中，不够醒目。
  - 用户列只展示用户 ID，运营无法直接看到用户名。
  - 下注信息会展示展开注码 Tag，列表中出现 `mt-2 flex max-w-[320px] flex-wrap gap-1` 对应的一组标签，影响扫描。
- 实施内容：
  - 后台订单列表、详情、创建和取消接口返回 `username` 字段。
  - 订单管理表格新增独立“期号”列，用户列改为“用户名 + 用户 ID”。
  - `OrderBetInfo` 新增 `showExpandedBets` 开关，订单管理列表关闭展开注码 Tag，仅保留选号结构。
  - 同步更新 OpenAPI 描述、架构说明和前端组件规范。
- 验证结果：`cargo fmt --manifest-path backend/Cargo.toml --check`、`cargo check --manifest-path backend/Cargo.toml`、`cd admin && npm run build` 和 `git diff --check` 均通过；后台构建仅保留既有 Vite chunk 体积提示。

## 2026-06-09 23:22 HKT 在线客服关闭会话隐藏与首页彩种卡片优化

- 完成任务：隐藏后台和手机端在线客服已关闭会话，并优化首页分类彩种卡片展示结构。
- 解决问题：
  - 用户端客服会话列表此前会继续展示 `status=closed` 的已关闭会话，用户从在线客服入口进入时仍可能看到不可继续处理的历史会话。
  - 后台在线客服保留“已关闭”筛选入口，实时推送或保存状态后仍可能让已关闭会话出现在运营列表中。
  - 首页分类彩种卡片原先按“头像 + 名称 + 状态 + 开奖号”纵向堆叠，两列布局下视觉拥挤，和彩种 logo 的识别感不够强。
- 实施内容：
  - `mobile/src/api/user.ts` 新增客服会话状态归一化和可见性判断，用户端客服列表过滤已关闭会话。
  - `SupportView` 和 `supportUnread` 缓存统一使用可见会话口径；路由指向已关闭会话时自动落到仍可见会话或空状态。
  - 后台 `useSupportConversations` 统一过滤已关闭会话，并从在线客服状态 Tabs 移除“已关闭”入口。
  - `HomeDrawCard` 的普通分类卡改为左文案、右 logo 的横向浅色卡片，保留状态、期号和紧凑开奖号码。
  - 普通分类卡中的期号从状态标签同行拆到独立行展示，并取消单行截断，避免状态标签挤占期号显示空间。
  - 普通分类卡的开奖号码行移到卡片底部，脱离左侧文案区域，改用整张卡片宽度展示；号码行保持单行不换行，号码位数较多时只在号码行内部横向滚动，不再撑高卡片。
  - 同步更新架构说明和前端组件规范。
- 验证结果：`cd mobile && npm run build`、`cd mobile && npm run test`、`cd admin && npm run build` 和 `git diff --check` 均通过；手机端测试脚本当前显示 0 个测试用例，后台构建仅保留既有 Vite chunk 体积提示。

## 2026-06-09 23:02 HKT 手机端客服图片消息兼容修复

- 完成任务：修复手机端在线客服看不到后台客服图片消息的问题。
- 解决问题：
  - 手机端客服页面虽然有图片渲染分支，但用户端客服接口直接返回原始会话数据，遇到 `message_type/image_url` 或历史图片链接在 `content` 中的消息时，页面无法识别为图片消息。
  - 实时事件到达后页面会重新拉取会话详情，因此修复应放在 API 适配层，保证列表、详情、发送和已读返回都使用同一口径。
- 实施内容：
  - `mobile/src/api/user.ts` 新增客服会话和消息归一化函数。
  - 客服消息兼容 `messageType/imageUrl`、`message_type/image_url`，并对 `Image/Text` 这类大小写差异做归一化。
  - 图片链接存在时强制归为 `image` 类型；历史数据若把图片链接放在 `content`，也会提取到 `imageUrl`。
  - 同步更新架构说明和前端组件规范。
- 验证结果：`cd mobile && npm run build`、`cd mobile && npm run test` 和 `git diff --check` 均通过；手机端测试脚本当前显示 0 个测试用例。

## 2026-06-09 22:40 HKT 后台合买计划过滤机器人发起计划

- 完成任务：后台合买计划列表新增非机器人发起计划筛选能力。
- 解决问题：
  - 合买机器人发起的计划和真实用户计划混在同一个列表里，运营查看用户发起合买时需要手动辨认发起人。
  - 只靠前端过滤会影响分页总数和页码，因此需要后端先过滤再分页。
- 实施内容：
  - 后端 `GET /api/admin/group-buy/plans` 支持 `includeRobotData` 查询参数，默认隐藏机器人发起计划。
  - 机器人判断使用发起人 `initiatorUserId`，机器人只是参与人补单的普通用户计划仍会展示。
  - 后台合买管理新增“显示机器人数据”开关，切换时重置到第 1 页并重新拉取列表。
  - 同步更新 OpenAPI 描述、架构说明、后端接口契约和前端组件规范。
- 验证结果：`cargo fmt --manifest-path backend/Cargo.toml --check`、`cargo test --manifest-path backend/Cargo.toml group_buy_plan_filter_hides_robot_initiator_by_default -- --nocapture`、`cargo check --manifest-path backend/Cargo.toml`、`cd admin && npm run build` 和 `git diff --check` 均通过；后台构建仅保留既有 Vite chunk 体积提示。

## 2026-06-09 22:16 HKT 后台合买计划详情与参与记录顺序调整

- 完成任务：将后台合买管理中选中计划后的“参与记录”和“计划详情”展示顺序对换。
- 解决问题：
  - 运营查看合买计划时更需要先核对参与用户、金额、份数和备注，原先“计划详情”在前会让参与信息需要向下查找。
- 实施内容：
  - 将 `GroupBuyManagementPage` 的“参与记录”卡片移动到“计划详情”卡片之前。
  - 保留原有参与记录添加、计划状态保存和空状态提示逻辑。
  - 同步更新架构说明和前端组件规范。
- 验证结果：`cd admin && npm run build` 和 `git diff --check` 均通过；后台构建仅保留既有 Vite chunk 体积提示。

## 2026-06-09 22:35 HKT 合买计划列表按期号倒序

- 完成任务：将合买计划列表统一改为最新期号在最前面。
- 解决问题：
  - 后端合买仓储此前依赖 `BTreeMap` 的计划 ID 顺序，后台合买列表、手机端合买大厅和我的合买不一定优先展示最新期号。
  - 后台分页会先按原始顺序切片，如果排序不在后端完成，最新期号可能被排到后续页面。
- 实施内容：
  - `GroupBuyStore::list()` 和 `GroupBuyStore::list_details()` 统一按期号倒序返回。
  - 期号相同的计划继续按创建时间和计划 ID 倒序稳定排列。
  - 增加后端测试覆盖摘要列表和详情列表的期号倒序。
  - 同步更新架构说明、后端接口契约和手机端组件规范。
- 验证结果：`cargo fmt --manifest-path backend/Cargo.toml --check`、`cargo test --manifest-path backend/Cargo.toml group_buy_repository_lists_latest_issue_first -- --nocapture`、`cargo check --manifest-path backend/Cargo.toml` 和 `git diff --check` 均通过。

## 2026-06-09 22:20 HKT 订单管理创建投注单改为 SideSheet

- 完成任务：将后台订单管理的“创建投注单”表单改为点击按钮后通过 Semi UI `SideSheet` 打开。
- 解决问题：
  - 订单管理原先采用列表加右侧常驻创建表单的两栏布局，创建区域长期占用列表扫描空间。
  - 空订单提示还提示“先在右侧创建”，与后台其它维护表单使用 `SideSheet` 的规范不一致。
- 实施内容：
  - 订单管理顶部新增“创建投注单”按钮。
  - 订单列表恢复为全宽展示，空状态改为提示点击上方按钮创建投注单。
  - 创建投注单表单移动到 `SideSheet` 中，成功创建后自动关闭抽屉并在列表上方展示最近创建摘要。
  - 保留原有玩法、期号、金额校验和创建订单逻辑。
- 验证结果：`cd admin && npm run build` 和 `git diff --check` 均通过；后台构建仅保留既有 Vite chunk 体积提示。

## 2026-06-09 22:05 HKT 后台用户列表默认降序

- 完成任务：将后台用户维护列表默认排序方向改为降序。
- 解决问题：
  - 用户维护页此前默认按用户 ID 升序展示，运营进入页面后优先看到旧用户，不符合优先查看新用户或编号靠后用户的使用习惯。
- 实施内容：
  - 后端 `GET /api/admin/users` 未传 `sortDirection` 或传空值时默认使用 `desc`。
  - 后台用户维护页排序方向初始值改为“降序”，排序方向下拉也把“降序”放在第一项。
  - `fetchUsers()` 这类数组拉取场景默认按用户 ID 降序，保证用户下拉和列表默认口径一致。
  - 同步更新架构说明和后端接口契约。
- 验证结果：`cd admin && npm run build`、`cd backend && cargo check`、`cd backend && cargo test user_list_sort -- --nocapture`、`cd backend && cargo test admin_users_documents_pagination_and_sort_query_parameters -- --nocapture` 均通过；后台构建仅保留既有 Vite chunk 体积提示。

## 2026-06-09 21:45 HKT 手机端在线客服未读红点

- 完成任务：为手机端在线客服增加未读红点提醒。
- 解决问题：
  - 原有 `unreadCount` 只表示后台客服侧未读，客服回复用户时会清零，不能用于用户端提醒。
  - 个人中心“在线客服”旁边此前是固定绿色点，无法表达是否真的有未读客服回复。
- 实施内容：
  - 后端客服会话新增 `userUnreadCount`，客服回复时递增，用户回复或打开会话标记已读时清零。
  - 新增 `POST /api/user/support/conversations/{id}/read`，仅允许当前登录用户清理自己的客服会话未读。
  - 新增数据库迁移 `20260609213000_add_support_user_unread_count.sql`，给 `support_conversations` 增加用户侧未读字段并补中文字段注释。
  - 手机端新增 `supportUnread` Pinia 缓存，个人中心“在线客服”、底部“我的”和多会话标签根据未读状态显示小红点。
  - 手机端收到客服 WebSocket 事件后静默刷新未读状态，打开某个客服会话后只清理该会话未读。
- 验证结果：`cd backend && cargo fmt --check`、`cd backend && cargo check`、`cd backend && cargo test`、`cd mobile && npm run build`、`cd admin && npm run build` 和 `git diff --check` 均通过；后台构建仅保留既有 Vite chunk 体积提示。

## 2026-06-09 21:20 HKT 后台客服消息记录位置对调

- 完成任务：调整后台在线客服“消息记录”中用户消息和客服消息的左右位置。
- 解决问题：
  - 后台客服页面使用 Semi UI `Chat` 时，原先把用户消息映射为 `user` 角色、客服消息映射为 `assistant` 角色，导致后台视角下用户和客服气泡位置与期望相反。
- 实施内容：
  - 将后台客服消息记录中的客服消息映射到右侧角色，用户消息映射到左侧角色。
  - 头像文字和颜色改为依据真实消息作者展示，避免位置对调后“用户/客服”头像显示错乱。
  - 同步更新架构说明和前端组件规范。
- 验证结果：`cd admin && npm run build` 与 `git diff --check` 均通过；后台构建仅保留既有 Vite chunk 体积提示。

## 2026-06-09 18:15 HKT 客服新消息 Telegram 提醒配置

- 完成任务：为用户发来的新客服消息增加可配置的 Telegram 提醒能力。
- 解决问题：
  - 后台客服虽然已有 WebSocket 实时消息，但客服人员离开后台页面时无法通过外部渠道收到新消息提醒。
  - 提醒配置需要放在后台系统设置中，避免继续使用环境变量或硬编码 Token。
- 实施内容：
  - 新增后端 `support_notification` 服务，读取系统设置中的 Telegram 开关、Bot Token 和 Chat ID。
  - 用户创建客服直充会话或继续回复客服会话后，后端在消息落库并推送实时事件后异步发送 Telegram 文本提醒。
  - Telegram 请求失败、超时或配置缺失只记录中文 warning，不影响用户发送客服消息。
  - 系统设置新增 `support_telegram_notification_enabled`、`support_telegram_bot_token` 和 `support_telegram_chat_id` 三个配置项。
  - 后台系统设置新增“通知设置”分组，Telegram 开关使用 Semi UI `Select` 下拉配置。
  - 同步更新架构说明和后端接口契约。
- 验证结果：`cargo fmt --manifest-path backend/Cargo.toml --check`、`cargo check --manifest-path backend/Cargo.toml`、`cargo test --manifest-path backend/Cargo.toml support_telegram -- --nocapture`、`cd admin && npm run build` 和 `git diff --check` 均通过；后台构建仅保留既有 Vite chunk 体积提示。

## 2026-06-09 17:20 HKT 合买注单个人派奖展示修复

- 完成任务：修复手机端合买注单中奖金额显示为整单派奖金额的问题。
- 解决问题：
  - 财务结算已经按合买参与比例给每个参与人写入 `payoutCredit` 资金流水，但用户注单接口此前只返回真实合买订单的整单 `payoutMinor`。
  - 手机端归一化后把整单奖金当作当前用户奖金展示，导致认购金额不同的参与人看到相同中奖金额。
- 实施内容：
  - 后端 `GET /api/user/bet/orders` 为合买订单新增 `participationPayoutMinor`，优先读取当前用户真实派奖流水。
  - 历史缺失派奖流水时，后端按合买参与比例兜底计算，并保持最后一名参与人承接余数的财务规则。
  - 手机端注单归一化新增个人派奖字段，合买订单展示中奖金额时优先使用 `participationPayoutMinor`。
  - 同步更新架构说明、后端接口契约和手机端组件规范。
- 验证结果：`cargo fmt --manifest-path backend/Cargo.toml --check`、`cargo check --manifest-path backend/Cargo.toml`、`cargo test --manifest-path backend/Cargo.toml user_visible_bet_orders -- --nocapture`、`cargo test --manifest-path backend/Cargo.toml store_credits_group_buy_settlement_by_participant_share -- --nocapture`、`cd mobile && npm run build`、`cd mobile && npm run test` 和 `git diff --check` 均通过；手机端测试脚本当前显示 0 个测试用例。

## 2026-06-09 16:42 HKT 后台客服会话列表显示用户 ID

- 完成任务：让后台在线客服“用户会话”列表展示用户 ID。
- 解决问题：
  - 客服列表此前只显示用户名，处理同名用户、充值核验或账户排查时不够明确。
- 实施内容：
  - 在用户列中保留用户名，并在用户名下方显示 `userId`。
  - 用户 ID 使用小号等宽文本展示，避免新增宽列影响会话列表扫描。
- 验证结果：`npm run build`（admin）和 `git diff --check` 均通过；后台构建仅保留既有 Vite chunk 体积提示。

## 2026-06-09 16:40 HKT 后台客服回车发送回复

- 完成任务：为后台在线客服回复框增加回车发送能力。
- 解决问题：
  - 客服回复用户消息时必须点击“发送回复”，高频沟通效率不够顺手。
  - 普通回车发送如果不处理输入法组合态，中文选词时可能误发消息。
- 实施内容：
  - 后台客服回复输入框新增键盘事件处理，`Enter` 触发发送。
  - `Shift+Enter`、`Ctrl+Enter`、`Alt+Enter`、`Meta+Enter` 保留默认输入行为，方便客服换行或使用系统组合键。
  - 发送前检查 `nativeEvent.isComposing`，中文输入法组合输入期间不触发发送。
- 验证结果：`npm run build`（admin）和 `git diff --check` 均通过；后台构建仅保留既有 Vite chunk 体积提示。

## 2026-06-09 17:30 HKT 后台用户列表分页和排序

- 完成任务：为后台用户维护列表增加分页能力，并支持按配置的排序规则查询。
- 解决问题：
  - 用户维护此前一次性拉取全部用户，用户量增长后页面加载、渲染和扫描效率会下降。
  - 用户列表只能按后端默认顺序展示，运营无法按余额、状态、类型等字段快速排序排查。
- 实施内容：
  - 后端 `GET /api/admin/users` 改为返回分页结构 `items/totalCount/page/pageSize/totalPages`。
  - 用户列表支持 `page`、`pageSize`、`sortBy`、`sortDirection` 查询参数，并对白名单排序字段做校验。
  - 管理后台用户维护页接入公共 `PageControls`，新增排序字段和升降序下拉框，切换排序规则时回到第 1 页。
  - `fetchUsers()` 保留数组返回形态，内部从分页响应中提取 `items`，避免影响邀请、合买等用户下拉场景。
- 验证结果：`npm run build`（admin）、`cargo fmt --manifest-path backend/Cargo.toml`、`cargo check --manifest-path backend/Cargo.toml`、`cargo test --manifest-path backend/Cargo.toml user_list_sorting_runs_before_pagination -- --nocapture`、`cargo test --manifest-path backend/Cargo.toml admin_users_documents_pagination_and_sort_query_parameters -- --nocapture` 和 `git diff --check` 均通过；后台构建仅保留既有 Vite chunk 体积提示。

## 2026-06-09 17:00 HKT 后台客服图片回复和状态筛选

- 完成任务：增强后台在线客服，支持图片回复和按状态区分用户会话。
- 解决问题：
  - 客服此前只能发送文本，处理充值凭证、截图说明时无法直接给用户发送图片。
  - 后台会话列表只展示全部会话，客服无法按处理中、等待用户、已解决、已关闭快速区分处理队列。
- 实施内容：
  - 后端客服消息新增 `messageType` 和 `imageUrl`，并新增数据库迁移保存消息类型和图片链接。
  - 后台客服回复接口支持 `messageType=image` 图片回复，图片链接必须是 `http/https` 地址，说明文字可选。
  - 管理后台在线客服列表新增状态 Tabs，按全部、处理中、等待用户、已解决、已关闭筛选会话。
  - 管理后台回复区接入图床上传，图片上传成功后展示预览，发送后在 Semi UI `Chat` 中展示图片消息。
  - 手机端客服会话支持渲染后台发送的图片消息。
- 验证结果：`npm run build`（admin）、`npm run build`（mobile）、`cargo fmt --manifest-path backend/Cargo.toml --check`、`cargo check --manifest-path backend/Cargo.toml`、`cargo test --manifest-path backend/Cargo.toml` 和 `git diff --check` 均通过；后端 257 个测试全部通过，前端构建仅保留既有 Vite chunk 体积提示。

## 2026-06-09 15:58 HKT 后台金额输入统一改用元

- 完成任务：把管理后台仍要求运营输入“分”的金额表单统一改为输入“元”。
- 解决问题：
  - 调账、订单创建、合买计划、合买参与、彩种合买配置和系统充值上下限原本存在直接填写分的入口，容易把 10 元误填成 10 分。
  - 后端订单、账务、充值、提现和合买仍以最小货币单位保存，需要在前端提交前完成可靠换算。
- 实施内容：
  - 新增 `admin/src/utils/moneyInput.ts`，统一处理“元”输入和最小货币单位之间的换算。
  - 财务管理手动调账改为“调账金额（元）”，支持正数补款和负数扣减。
  - 订单管理单注金额、合买管理计划总金额、发起人认购金额和参与金额全部改为元输入。
  - 彩种管理合买配置里的每份最低金额、参与最低金额改为元输入。
  - 系统设置里的用户单笔充值最小金额、最大金额改为元展示和元输入，保存时仍写回最小货币单位。
  - 输入格式错误、金额小于等于 0 或调账金额为 `0` 时用中文提示阻止提交。
- 验证结果：`npm run build`（admin）、`cargo fmt --manifest-path backend/Cargo.toml --check`、`cargo check --manifest-path backend/Cargo.toml`、`cargo test --manifest-path backend/Cargo.toml` 和 `git diff --check` 均通过；后端 256 个测试全部通过，后台构建仅保留既有 Vite chunk 体积提示。

## 2026-06-09 01:08 HKT 手机端 iOS 模拟器打包

- 完成任务：初始化手机端 Tauri iOS 工程，并成功打出 iOS Simulator 可运行的 `HongFu.app`。
- 解决问题：
  - 手机端此前只有 Android、桌面应用和 DMG 打包脚本，没有 iOS 初始化和构建入口。
  - 本机缺少 iOS Rust 编译目标和 iOS Simulator 运行时，无法直接执行 iOS 构建。
  - 真机/IPA 构建需要 Apple Developer Team 和 provisioning profile，当前 Xcode 未配置开发团队签名。
- 实施内容：
  - 执行 Tauri iOS 初始化，生成 `mobile/src-tauri/gen/apple/` Xcode 工程和 `iOS-schema.json`。
  - 安装 iOS Rust targets，并通过 Xcode 安装 iOS 26.3.1 Simulator runtime。
  - 手机端 `package.json` 新增 `tauri:ios:init`、`tauri:build:ios-sim` 和 `tauri:build:ios` 脚本。
  - 生成模拟器构建产物：`mobile/src-tauri/gen/apple/build/arm64-sim/HongFu.app`。
  - 生成模拟器压缩包：`mobile/src-tauri/gen/apple/build/arm64-sim/HongFu-ios-simulator-arm64.zip`。
- 验证结果：`cd mobile && npm run build` 通过；`cd mobile && npm run tauri -- ios build --ci --target aarch64-sim` 通过；真机/IPA 构建执行到 Xcode 签名阶段失败，错误为未配置开发团队，后续需要在 Xcode 中为 `com.hongfu.app` 配置 Apple Developer Team 后再构建真机包。

## 2026-06-08 16:10 HKT 机器人配置禁止删除

- 完成任务：把合买机器人和购彩机器人调整为不能删除的系统配置，只能通过暂停或禁用停止执行。
- 解决问题：
  - 原后台机器人配置 SideSheet 中存在“删除”按钮，后端 `DELETE /api/admin/robots/{id}` 也会真正删除仓储中的机器人配置。
  - 机器人配置关联调度、资金流水、合买计划和运营排查链路，误删后会造成自动化执行配置缺失。
- 实施内容：
  - 后端不再注册 `DELETE /api/admin/robots/{id}`，机器人配置没有业务删除接口。
  - 移除 `RobotRepository` 删除写入口，避免后端内部继续暴露机器人删除能力。
  - 管理后台移除机器人表单里的“删除”按钮，页面说明改为只能暂停或禁用。
  - 前端 `useRobots` 和 API client 移除删除调用。
  - 架构说明、OpenAPI 说明、后端 API 契约和前端组件规范同步记录机器人禁止删除规则。
- 验证结果：`cargo fmt --manifest-path backend/Cargo.toml --check`、`cargo check --manifest-path backend/Cargo.toml`、`cargo test --manifest-path backend/Cargo.toml openapi_document_contains_core_paths -- --nocapture`、`cargo test --manifest-path backend/Cargo.toml -- --nocapture`、`cd admin && npm run build` 和 `git diff --check` 均通过；管理后台构建仅保留既有 Vite chunk 体积提示。

## 2026-06-08 15:21 HKT 手机端 Tauri 头像缓存

- 完成任务：为手机端个人中心和聊天大厅头像新增 Tauri 兼容的图片缓存能力，减少同一个头像链接反复请求图床。
- 解决问题：
  - 个人中心和聊天大厅此前直接渲染远程头像 URL，页面重新进入、历史消息重新渲染或 WebView 缓存失效时会再次请求图片。
  - 手机端后续要打包为 Tauri App，不能只依赖网页环境的普通 HTTP 缓存或 Service Worker。
- 实施内容：
  - 新增 `avatarImageCache` 工具，以头像 URL 为 key 做内存缓存和本地 data URL 缓存，并限制缓存有效期、单项大小和缓存数量。
  - 新增 `CachedAvatarImage` 公共头像组件，缓存命中时直接显示 data URL，失败时回退原始 URL，再失败则显示用户名首字。
  - 个人中心头像和聊天大厅头像统一切换为公共缓存组件。
  - Tauri Rust 侧新增 `cache_avatar_image` 命令，下载 `http/https` 头像、校验 `image/*` 类型和 1MB 大小限制后返回 data URL。
  - 架构说明和前端组件规范同步记录“手机端头像必须通过公共缓存组件展示”的规则。
- 验证结果：`cd mobile && npm run build`、`cd mobile && npm test`、`cargo fmt --manifest-path mobile/src-tauri/Cargo.toml --check`、`cargo check --manifest-path mobile/src-tauri/Cargo.toml`、`cargo test --manifest-path mobile/src-tauri/Cargo.toml` 和 `git diff --check` 均通过。

## 2026-06-07 22:48 HKT 玩法按位置配置最大选号数量

- 完成任务：为每个彩种的每个玩法增加按位置配置最大选号数量的能力。
- 解决问题：
  - 原手机端只有全局 `maxSelectPerPosition` 兜底，不能表达“前 3 直选第一位最多 7 个数字，第二位和第三位不限制”这种精细配置。
  - 只在手机端限制会被直接调用下单 API 绕过，需要后端订单报价同步校验。
- 实施内容：
  - 后端 `LotteryPlayConfig` 新增 `positionSelectLimits`，保存彩种时校验位置 key 属于当前玩法且上限必须大于 0。
  - 后端订单报价和创建时按配置校验每个位置的选号数量，超过上限返回“{位置}最多选择 N 个号码”。
  - 管理后台“玩法配置与赔率”表新增“位置选号上限”列，每个玩法按位置显示输入框，留空表示不限制。
  - 手机端下注页读取 `positionSelectLimits`，选号按钮、全选和快捷选择都按具体位置限制。
  - 架构说明、后端 API 契约和前端组件规范同步记录按位置选号上限规则。
- 验证结果：`cargo fmt --manifest-path backend/Cargo.toml --check`、`cargo check --manifest-path backend/Cargo.toml`、`cargo test --manifest-path backend/Cargo.toml store_rejects_position_select_limit_overflow -- --nocapture`、`cargo test --manifest-path backend/Cargo.toml mobile_bet_page_config_returns_position_select_limits -- --nocapture`、`cd admin && npm run build`、`cd mobile && npm run build` 和 `git diff --check` 均通过。

## 2026-06-07 22:01 HKT 手机端合买认购金额输入修复

- 完成任务：修复手机端合买详情弹层“认购金额”输入框删除键无法正常清空的问题。
- 解决问题：
  - 页面此前监听 `joinAmountInput` 并在每次输入变化后立即调用金额归一化。
  - 用户按删除键把输入框删空时，输入值会立刻被最低认购金额回填，看起来像删除键失效。
- 实施内容：
  - 移除认购金额输入框的逐键强制归一化监听。
  - 新增 `commitJoinAmountInput`，在失焦、回车、确认认购、快捷金额和加减按钮场景再校正金额。
  - 输入框补充 `inputmode="decimal"`，保留移动端数字输入体验。
  - 架构说明和前端组件规范同步记录资金类金额输入不能逐键强制格式化的规则。
- 验证结果：`cd mobile && npm run build` 和 `git diff --check` 均通过。

## 2026-06-07 21:50 HKT 手机端合买大厅发起人脱敏展示

- 完成任务：优化手机端合买大厅发起人展示，用户端不再完整显示发起人昵称，并提升发起人名称字号。
- 解决问题：
  - 合买大厅此前直接展示普通用户完整昵称，用户要求只保留前后字符，中间用 `*` 替代。
  - 原卡片里的发起人名称继承辅助信息小字号，视觉权重不足。
- 实施内容：
  - 后端用户端合买 DTO 转换统一对 `initiatorDisplay` 做脱敏，普通用户和机器人计划都只展示首尾字符，中间使用 `*`。
  - 保留后台真实 `initiatorUserId/initiatorUsername`、资金流水和审计链路，避免影响运营排查。
  - 手机端合买大厅卡片发起人名称改为更大字号、加粗和单行截断。
  - 架构说明、后端 API 契约和前端组件规范同步记录新的脱敏展示规则。
- 验证结果：`cargo fmt --manifest-path backend/Cargo.toml --check`、`cargo check --manifest-path backend/Cargo.toml`、`cargo test --manifest-path backend/Cargo.toml user_group_buy_plan_masks -- --nocapture`、`cargo test --manifest-path backend/Cargo.toml mask_group_buy_initiator_display_handles_edge_cases -- --nocapture`、`cd mobile && npm run build` 和 `git diff --check` 均通过。

## 2026-06-06 02:55 HKT 后台计奖派奖分页

- 完成任务：为后台“计奖派奖”的结算批次列表补充分页能力。
- 解决问题：
  - 结算批次此前一次性拉取并展示全部记录，历史开奖结算增多后页面加载和扫描会变慢。
  - 计奖派奖页面没有复用后台公共分页控件，和订单、财务、合买列表的分页交互不一致。
- 实施内容：
  - 后端 `GET /api/admin/settlements` 改为返回分页结构 `items/totalCount/page/pageSize/totalPages`，支持 `page` 和 `pageSize`。
  - 管理后台 `fetchSettlements` 和 `useSettlements` 改为消费分页结构。
  - 计奖派奖页面新增每页条数、上一页和下一页控件，并在执行新结算后回到第 1 页。
  - OpenAPI 说明、前端规范和架构说明同步记录结算批次分页契约。
- 验证结果：`cargo fmt --manifest-path backend/Cargo.toml --check`、`cargo check --manifest-path backend/Cargo.toml`、`cargo test --manifest-path backend/Cargo.toml -- --nocapture`、`cd admin && npm run build` 和 `git diff --check` 均通过；管理后台构建仅保留既有 Vite chunk 体积提示。

## 2026-06-06 02:50 HKT 后台订单管理分页

- 完成任务：为后台订单管理补充分页能力，并统一后台分页控件。
- 解决问题：
  - 订单管理此前一次性拉取并展示全部订单，订单增长后页面加载、扫描和控单效率都会下降。
  - 财务管理和合买管理各自维护相同分页控件，后续新增分页列表容易出现样式和行为不一致。
- 实施内容：
  - 后端 `GET /api/admin/orders` 改为返回分页结构 `items/totalCount/page/pageSize/totalPages`，支持 `page`、`pageSize` 和 `includeRobotData`。
  - 管理后台 `useOrders` 改为消费分页结构，订单管理页面新增每页条数、上一页和下一页控件。
  - 抽取公共 `PageControls` 组件，财务管理、合买管理和订单管理统一复用。
  - OpenAPI 说明、前后端规范和架构说明同步记录订单分页契约。
- 验证结果：`cargo fmt --manifest-path backend/Cargo.toml --check`、`cargo check --manifest-path backend/Cargo.toml`、`cargo test --manifest-path backend/Cargo.toml -- --nocapture`、`cd admin && npm run build` 和 `git diff --check` 均通过；管理后台构建仅保留既有 Vite chunk 体积提示。

## 2026-06-06 02:42 HKT 手机端下注提示去重

- 完成任务：修复手机端下注时同时出现“已加入购彩篮”和“下注成功”两个提示的问题。
- 解决问题：
  - 用户直接点击立即投注或发起合买时，页面内部会先把当前草稿加入购彩篮，导致先弹出“已加入购彩篮”，随后又弹出最终提交成功提示。
- 实施内容：
  - `addDraftToCart` 增加 `silent` 选项，保留校验失败提示，成功入篮提示可按调用场景关闭。
  - 普通投注和发起合买的内部入篮调用改为静默，最终只保留下注或合买提交成功提示。
  - 用户主动点击“加入购彩篮”按钮时仍显示“已加入购彩篮”，避免影响正常购物篮反馈。
  - 前端组件规范和架构说明同步记录下注页提示去重规则。
- 验证结果：`cd mobile && npm run build` 和 `git diff --check` 均通过。

## 2026-06-06 02:39 HKT 手机端下注成功自动返回首页

- 完成任务：调整手机端下注页成功后的跳转流程。
- 解决问题：
  - 用户普通投注或发起合买成功后仍停留在下注页，可能继续看到原页面并产生重复操作的误解。
- 实施内容：
  - 普通投注成功后清空本地购彩篮，并使用 `router.replace({ name: 'Home' })` 自动回到首页。
  - 发起合买成功后清空本地购彩篮、关闭合买模式，并自动回到首页。
  - 失败路径保持不跳转，继续刷新余额和期号状态，方便用户修正后重试。
  - 前端组件规范和架构说明同步记录下注成功返回首页的流程要求。
- 验证结果：`cd mobile && npm run build` 和 `git diff --check` 均通过。

## 2026-06-06 02:31 HKT 手机端确认认购按钮可读性修复

- 完成任务：修复手机端合买详情弹层中“确认认购”按钮颜色不清晰的问题。
- 解决问题：
  - 原按钮仅依赖 `lacquer-gradient` 和文字颜色类，Vant 按钮内部样式可能导致文字颜色不够明确，用户难以看清“确认认购”。
- 实施内容：
  - 将按钮改为 Vant `type="primary"` 并增加专用 `group-buy-join-button` 样式。
  - 显式覆盖按钮背景、文字、加载状态和禁用状态颜色，保证主操作按钮在深红背景下保持白字高对比。
- 验证结果：`cd mobile && npm run build` 和 `git diff --check` 均通过。

## 2026-06-06 01:00 HKT 后台订单下注信息展示优化

- 完成任务：补齐后台订单管理和彩种控制台控单流程中的用户下注信息展示。
- 解决问题：
  - 订单管理列表原来只展示订单号、用户、彩种、玩法、金额和状态，看不到用户具体投注内容。
  - 彩种控制台控单 SideSheet 的用户下单信息表格看不到下注选择和展开注码，运营无法依据某一单的投注内容进行控单。
- 实施内容：
  - 新增后台通用 `OrderBetInfo` 组件，展示订单 `selection` 和 `expandedBets`。
  - 新增下注信息格式化工具，把直选位置、普通选号、胆码拖码、大小单双翻译成中文可读内容。
  - 订单管理列表新增“下注信息”列，最近创建订单区域也展示完整下注详情。
  - 彩种控制台控单 SideSheet 的目标订单下拉项加入下注摘要，用户下单信息表格新增“下注信息”列。
  - 前端组件规范和架构说明同步记录后台控单必须展示下注信息的约束。
- 验证结果：`cd admin && npm run build`、`git diff --check` 均通过；本地启动管理后台 dev server 后登录页可正常加载且无前端错误，因当前没有登录后端会话未进入内部订单页做真实数据截图。

## 2026-06-06 01:05 HKT 手机端合买大厅列表密度优化

- 完成任务：优化手机端合买大厅计划列表卡片结构，让首屏可以显示更多合买计划。
- 解决问题：
  - 原合买计划卡片包含大图标区、发起人头像区、两列大金额块和整行大按钮，单项高度过高，一屏最多只能看到约 3 个计划。
  - 用户浏览合买大厅时需要频繁滚动，计划扫描效率低。
- 实施内容：
  - 大厅计划卡片改为紧凑信息行，保留彩种、期号、玩法、发起人、总额、单份、进度和剩余份数。
  - 参与入口压缩为右侧小状态标签，整卡仍可点击打开详情。
  - 分类筛选 chip 降低高度和横向内边距，减少首屏空间占用。
  - 架构说明和前端组件规范同步记录“首屏至少 6 个合买计划”的规则。
- 验证结果：`cd mobile && npm run build` 和 `git diff --check` 均通过；本地 dev server 可启动，自动截图验证因当前环境未安装 `playwright` 未执行。

## 2026-06-06 00:47 HKT 手机端下注页合买自购份数自动匹配

- 完成任务：优化手机端下注页合买模式的自购份数默认值，让页面根据方案金额、固定每份金额和发起人最低自购比例自动填入最适配数量。
- 解决问题：
  - 原下注页自购份数固定从 1 份开始，后台配置最低自购比例较高时，用户需要提交失败后才知道份数不足。
  - 用户修改投注内容、倍数或购彩篮后，方案金额变化不会主动调整自购份数，容易造成预计支付与最低自购规则不匹配。
- 实施内容：
  - 新增 `calculateRecommendedSelfShares`，按总金额、每份金额和最低自购比例计算推荐自购份数；无最低比例时默认至少自购 1 份。
  - 下注页合买模式开启时自动填入推荐份数，投注金额变化时自动跟随；用户手动填更高份数时保留，低于最低份数或超过总份数时自动校正。
  - 自购份数提示文案显示当前最低自购比例和自动匹配/建议份数。
  - 架构说明和前端组件规范同步记录本次规则。
- 验证结果：`cd mobile && npm run build` 和 `git diff --check` 均通过。

## 2026-06-06 00:05 HKT 彩种控制台订单查看与控制范围

- 完成任务：增强后台彩种控制台，让运营可以查看彩种下单信息，并把开奖号码控制从“整彩种长期控制”扩展为“整彩种 / 指定期号 / 指定订单所在期号”三种范围。
- 解决问题：
  - 原控制台无法直接查看用户在当前彩种和当前期的下单情况，运营需要跳到订单管理页面交叉核对。
  - 原控制开奖号码只按彩种整体生效，开启后容易影响后续所有期号，无法只控制某一期或某个订单所在期。
  - “控制一单”如果不绑定期号，会和“一期一个开奖结果”的规则冲突；本次明确为选择目标订单并控制该订单所在期号。
- 实施内容：
  - 后端开奖控制请求和响应新增 `targetScope`、`targetIssue`、`targetOrderId`，保存时校验目标期号或目标订单，并在订单范围下自动补齐订单期号。
  - `draw_controls` 增加控制范围、目标期号、目标订单字段，数据库迁移补齐中文字段注释和范围约束。
  - 开奖服务按当前期号判断控制范围是否命中，整彩种继续兼容旧行为，指定期号和订单范围只命中目标期号。
  - 管理后台彩种控制台新增本期下注摘要，控制 SideSheet 展示用户下单信息，并支持用 Semi UI `Select` 选择控制范围和目标订单。
  - 架构说明、OpenAPI 说明、数据库规范和前端组件规范同步记录本次规则。
- 验证结果：`cargo fmt --manifest-path backend/Cargo.toml --check`、`cargo check --manifest-path backend/Cargo.toml`、`cargo test --manifest-path backend/Cargo.toml -- --nocapture`、`cd admin && npm run build` 和 `git diff --check` 均通过；后端全量 225 条测试成功，后台构建仅保留既有 Vite chunk 体积提示。

## 2026-06-05 21:53 HKT 手机端我的注单显示合买订单

- 完成任务：修正用户端 `GET /api/user/bet/orders` 的注单归属口径，让“我的注单”能显示当前用户参与且已经满单成单的合买订单。
- 解决问题：合买满单生成的真实投注订单归属发起人，原接口只按 `order.userId == 当前用户` 过滤，导致普通参与人无法在“我的注单”看到自己的合买下单记录。
- 实施内容：后端注单列表合并本人独立下注订单，以及本人在合买 `participants` 中且计划 `orderId` 指向真实投注订单的 `orderSource=groupBuy` 订单；未满单的合买计划仍由“我的合买”展示，不混入注单列表。
- 验证结果：`cargo fmt --manifest-path backend/Cargo.toml --check`、`cargo check --manifest-path backend/Cargo.toml`、`cargo test --manifest-path backend/Cargo.toml -- --nocapture`、`cd mobile && npm run build` 和 `git diff --check` 均通过；后端全量 221 条测试成功，新增覆盖合买参与人也能看到已成单合买注单。

## 2026-06-05 19:51 HKT 手机端首页高频极速模块视觉优化

- 完成任务：优化手机端首页“高频极速”模块的主推卡、二级卡和区块标题展示。
- 解决问题：
  - 截图中二级卡片的“可下注”状态被挤成逐字竖排，标题、倒计时和状态在两列小卡内互相抢空间。
  - 二级卡底部大“进入”按钮占用高度，导致模块视觉层级偏重、扫描效率低。
  - 主推卡的开奖号码、和值和投注按钮分散，信息成组感不足。
- 具体实现：
  - `HomeDrawCard.vue` 新增 `countdownDisplay`，把倒计时拆成“计时/封盘/状态”短标签和值。
  - 主推卡新增浅色开奖结果区域，把号码球和值合并展示，并压缩卡片间距。
  - 二级卡改为整卡点击，底部大按钮替换为小“进入”标签，状态、倒计时、入口标签全部禁止换行。
  - 小屏下自动缩小高频极速号码球，继续兼容 3 位和 5 位开奖号码正圆展示。
  - 首页高频极速标题条改为更完整的模块头部，保留后台配置标题。
  - 前端组件规范和架构说明同步记录高频极速二级卡片扫描规则。
- 验证记录：
  - `cd mobile && npm run build` 通过。
  - `git diff --check` 通过。
  - 本地启动后端和手机端 dev server，临时打开高频极速配置验证接口返回；由于当前浏览器工具无法写入手机端登录态，未能完成登录后首页截图验证。
  - 临时打开的高频极速配置已恢复，临时注册的预览用户已从外部 PostgreSQL 清理。
- 后续动作：后续如果仍希望进一步强化该模块，可考虑把二级卡改为横向滑动列表或增加后台排序权重。

## 2026-06-05 19:40 HKT 平台开奖彩种开售补期修复

- 完成任务：修复平台开奖彩种从停售切换为开售后不会立即自动补齐期号的问题。
- 解决问题：
  - 后台开售补期此前只判断 `DrawMode::Api`，平台开奖彩种开售后没有立刻生成未来 `open` 期号。
  - 运营刚开售平台彩种时需要等待调度下一轮；如果调度未启用或本轮未触发，就表现为彩种不会自己更新期号。
  - 开售补出的期号此前没有从该入口返回给实时事件发布流程，前端不容易立即感知新期开盘。
- 具体实现：
  - 新增 `should_align_draw_issue_plan_after_sale_on`，把开售即时补期范围扩展为 `api` 和 `platform` 开奖模式。
  - `align_draw_issue_plan_after_sale_on` 返回本次生成的期号列表，已满足未来期号缓冲时返回空列表。
  - 彩种开售补出的期号会发布 `lottery.issue_opened` 实时事件，让后台和手机端可立即刷新当前期号。
  - 增加回归测试覆盖 API/平台/手动三种开奖模式判断，以及平台开奖彩种开售后生成未来期号。
- 验证记录：
  - `cargo fmt --manifest-path backend/Cargo.toml` 通过。
  - `cargo test --manifest-path backend/Cargo.toml sale_on_alignment -- --nocapture` 通过。
  - `cargo fmt --manifest-path backend/Cargo.toml --check` 通过。
  - `cargo check --manifest-path backend/Cargo.toml` 通过。
  - `cargo test --manifest-path backend/Cargo.toml -- --nocapture` 通过，211 条后端测试全部成功。
  - `git diff --check` 通过。
- 后续动作：手动开奖彩种仍由运营维护期号；如后续也希望手动彩种开售即自动开盘，需要先确认手动开奖的号码来源和运营流程。

## 2026-06-05 19:24 HKT 手机端首页高频极速配置与倒计时修复

- 完成任务：优化手机端首页“高频极速”模块，新增后台配置开关和彩种选择，并修复开奖时间后倒计时长期停在“开奖中”的问题。
- 解决问题：
  - 高频极速此前按开奖周期自动展示，默认就是开启状态，后台无法控制是否展示或展示哪些彩种。
  - 首页彩种卡片还会展示合买标签和合买入口，不符合首页只展示投注入口的要求。
  - 手机端首页只消费 `draw_result`，没有消费 `issue_opened` / `issue_closed`，到达开奖时间后如果新期号没有被页面拉取，会长期显示“开奖中”。
- 具体实现：
  - 后端新增 `mobile_home_featured_enabled`、`mobile_home_featured_title`、`mobile_home_featured_lottery_codes` 三个系统设置，种子和迁移默认关闭高频极速。
  - `/api/lottery/home` 读取系统设置后，只在开关开启且配置彩种命中销售中彩种时返回高频极速推荐区，并按后台配置顺序返回。
  - 管理后台系统设置的手机端设置面板新增高频极速开关、标题和 Semi UI 彩种多选配置。
  - 手机端首页高频极速按 `settings.featuredEnabled` 和后端返回彩种展示，所有首页卡片移除合买标签、合买按钮和合买大厅入口。
  - 手机端首页新增 `issue_opened` / `issue_closed` 实时事件消费，并在开奖时间已过时每 5 秒最多静默刷新一次首页，避免倒计时卡死。
  - Trellis 前后端规范和 OpenAPI 说明同步记录本次规则。
- 验证记录：
  - `cargo fmt --manifest-path backend/Cargo.toml` 通过。
  - `cargo check --manifest-path backend/Cargo.toml` 通过。
  - `cargo test --manifest-path backend/Cargo.toml -- --nocapture` 通过，209 条后端测试全部成功。
  - `cd admin && npm run build` 通过；Vite 仍提示后台主 chunk 超过 500 kB，这是既有构建体积提示。
  - `cd mobile && npm run build` 通过。
  - `git diff --check` 通过。
- 后续动作：后续如需要更精细的首页运营位，可把高频极速配置从系统设置字符串扩展为独立排序表。

## 2026-06-05 19:08 HKT 手机端客服账号名展示修正

- 完成任务：修正手机端在线客服页面中后台管理员账号名的用户可见展示。
- 解决问题：
  - 客服接入状态此前会显示“客服 admin 已接入”，把后台管理员账号直接暴露给用户。
  - 客服消息气泡此前优先展示后台消息的 `authorName`，当管理员账号为 `admin` 时用户会看到“admin”。
- 具体实现：
  - 客服接入状态改为统一显示“客服已接入”，不拼接管理员账号名。
  - 后台消息气泡作者统一显示“客服”，用户消息仍显示“我”，系统消息仍显示“系统”。
  - 前端组件规范补充手机端客服聊天不得直接展示后台管理员账号名的规则。
- 验证记录：
  - `cd mobile && npm run build` 通过。
  - `git diff --check` 通过。
- 后续动作：后续如果客服页增加客服头像、昵称或在线状态，也需要使用前台展示名，不直接使用后台登录账号。

## 2026-06-05 19:03 HKT 手机端充值页金额输入类型修复

- 完成任务：修复手机端充值页面 `trim is not a function` 运行时错误。
- 解决问题：
  - 充值页金额解析函数此前假设输入值一定是字符串，直接调用 `value.trim()`。
  - 浏览器原生 `type="number"` 输入或快捷金额回填在运行时可能产生数字值，页面计算充值金额时会崩溃。
- 具体实现：
  - `amount` 状态类型调整为 `string | number`，覆盖输入框和快捷金额两种来源。
  - `amountMinorFromYuan` 参数调整为 `unknown`，先通过 `String(value ?? '').trim()` 归一化，再校验两位小数并转换为最小货币单位。
  - 前端组件规范补充手机端充值页金额输入归一化规则，后续同类资金输入不能直接对未知值调用字符串方法。
- 验证记录：
  - `cd mobile && npm run build` 通过。
  - `git diff --check` 通过。
- 后续动作：后续继续审查充值、提现、财务调整等金额输入，必要时抽取统一金额解析工具。

## 2026-06-05 18:19 HKT 彩种控制台模块紧凑化

- 完成任务：优化彩种控制台中每个彩种模块过大的问题，让一屏可以扫描更多彩种状态。
- 解决问题：
  - 原彩种卡片最多 3 列展示，单卡内部存在当前期号、最近开奖、开奖控制等多个大块区域，运营查看 20 多个彩种时需要频繁滚动。
  - 原倒计时和期号说明文案较长，在紧凑布局下容易撑高模块或截断得不自然。
- 具体实现：
  - 彩种卡片改为紧凑布局，Semi `Card` 单独缩小 `bodyStyle` 内边距，不影响其它页面卡片。
  - 桌面端彩种列表改为最多 4 列展示，提高控制台信息密度。
  - 当前期号和最近开奖改为并排信息格，开奖控制压缩到底部操作行。
  - 倒计时文案从“封盘倒计时 / 开奖倒计时”压缩为“封盘 / 开奖”，无当前期文案压缩为“暂无当前期”。
  - 开奖时间显示使用 `formatTimePoint` 转为本地时分秒，避免长日期把卡片撑高。
  - 前端组件规范同步沉淀彩种控制台监控卡片需要保持紧凑的规则。
- 验证记录：
  - `cd admin && npm run build` 通过；Vite 仍提示后台主 chunk 超过 500 kB，这是既有构建体积提示。
  - `git diff --check` 通过。
  - 本地启动后端 `cargo run` 和后台前端 `npm run dev -- --host 127.0.0.1 --port 5188`，浏览器登录后进入彩种控制台，确认 4 列紧凑卡片、倒计时、期号、最近开奖和“控制”按钮均正常显示。
- 后续动作：后续如果彩种数量继续增加，可以再增加“列表密度切换”或“表格模式”。

## 2026-06-05 18:06 HKT 客服直充实时流程审查修复

- 完成任务：审查客服直充实时聊天流程，并修复会话状态、充值页刷新和用户回复状态恢复问题。
- 解决问题：
  - 后台只修改客服会话状态、优先级或分配客服时，此前不会通过 WebSocket 通知用户端，手机端客服页无法实时显示“客服已接入”。
  - 用户在“等待用户”状态的会话中补充消息后，后端仍保留 `pending` 状态，客服后台容易误判为还在等待用户。
  - 客服确认充值入账后，充值页虽然会收到用户实时事件，但页面没有监听 `recharge_changed` 和 `balance_changed`，订单状态和余额不会实时刷新。
- 具体实现：
  - 后端新增 `support.conversation_updated` 实时事件，后台更新会话状态、优先级或分配客服后同步推送给会话所属用户和后台客服连接。
  - 后台实时事件归一化和客服 hook 支持 `support.conversation_updated`，收到后 upsert 会话。
  - 手机端实时事件归一化和客服页支持 `support_conversation_updated`，收到后重新拉取会话详情。
  - `SupportRepository::user_reply` 在会话处于 `pending/resolved/closed` 时自动恢复为 `open`，并补充回归测试。
  - 手机端充值页接收 `wsMessage`，在 `recharge_changed` 时刷新充值订单，在 `balance_changed` 时刷新用户余额。
  - Trellis 后端契约和架构设计同步记录本次审查发现的流程规则。
- 验证记录：
  - `cargo fmt --manifest-path backend/Cargo.toml --check` 通过。
  - `cargo test --manifest-path backend/Cargo.toml support_repository_reopens_pending_conversation_when_user_replies -- --nocapture` 通过。
  - `cargo test --manifest-path backend/Cargo.toml support_conversation_updated_event_contains_conversation -- --nocapture` 通过。
  - `cargo check --manifest-path backend/Cargo.toml` 通过。
  - `cargo test --manifest-path backend/Cargo.toml` 通过，207 条后端测试全部成功。
  - `cd admin && npm run build` 通过；Vite 仍提示后台主 chunk 超过 500 kB，这是既有构建体积提示。
  - `cd mobile && npm run build` 通过。
  - `git diff --check` 通过。
- 后续动作：后续可继续补消息已读回执和后台实时连接状态提示。

## 2026-06-05 17:58 HKT 客服直充 WebSocket 实时聊天

- 完成任务：把客服直充会话接入 WebSocket 实时聊天，用户和后台客服发送消息后对方可以实时刷新。
- 解决问题：
  - 客服直充此前只通过 HTTP 创建会话和手动查询消息，用户或客服发消息后另一端无法实时看到。
  - 现有用户实时通道只有开奖、余额、订单、充值和提现事件，缺少客服消息事件。
  - 后台没有可供浏览器使用的实时连接入口，不能实时接收用户发来的客服直充消息。
- 具体实现：
  - 后端实时事件中心新增后台受众和 `publish_admin`，普通用户和匿名连接不会收到后台私有事件。
  - 新增 `support.message_created` 事件，携带 `conversationId`、`userId`、完整会话和最新消息。
  - 新增后台 `/api/admin/realtime?token=<管理员登录 token>` WebSocket 入口，校验管理员 token 和客服权限。
  - 客服直充创建会话、用户发送客服消息、后台回复客服消息后，都会把消息事件推送给会话所属用户和后台客服连接。
  - 管理后台客服 hook 建立后台实时连接，收到消息事件后 upsert 会话。
  - 手机端实时事件归一化新增 `support_message_created`，客服页收到事件后重新拉取会话详情。
  - OpenAPI、Trellis 后端契约和架构设计同步记录客服直充实时聊天规则。
- 验证记录：
  - `cargo fmt --manifest-path backend/Cargo.toml --check` 通过。
  - `cargo check --manifest-path backend/Cargo.toml` 通过。
  - `cargo test --manifest-path backend/Cargo.toml realtime -- --nocapture` 通过，4 条实时事件定向测试全部成功。
  - `cargo test --manifest-path backend/Cargo.toml` 通过，205 条后端测试全部成功。
  - `cd admin && npm run build` 通过；Vite 仍提示后台主 chunk 超过 500 kB，这是既有构建体积提示。
  - `cd mobile && npm run build` 通过。
- 后续动作：后续继续补消息已读回执、客服在线状态、输入中状态和文件消息。

## 2026-06-05 17:43 HKT 合买机器人前台展示伪装修正

- 完成任务：隐藏合买机器人发起计划在手机端的机器人痕迹，并保证不同机器人计划显示不同发起人。
- 解决问题：
  - 机器人自动发起的合买计划此前会把真实机器人账号 `agent_alpha` 作为用户端 `initiatorDisplay` 返回，用户可以看出是机器人单。
  - 机器人计划标题此前包含机器人配置名称，历史已落库计划在手机端详情或列表中仍可能暴露“机器人”来源。
  - 只修改真实发起人会影响资金、补单和派奖追溯，因此需要把真实业务账户和用户端展示身份拆开。
- 具体实现：
  - 用户端合买 DTO 转换识别 `G-ROBOT-` 开头计划，发起人展示名改为按计划 ID 稳定生成的普通会员展示名。
  - 不同机器人计划使用不同展示名；同一计划刷新后展示名保持稳定。
  - 用户端机器人计划标题统一返回“彩种 第期号期合买”，兼容历史标题已经带机器人名称的旧数据。
  - 机器人新建合买计划时标题改为通用合买标题，真实 `initiator_user_id`、资金流水和成单账户保持不变。
  - Trellis 后端契约和架构设计同步记录用户端不得暴露机器人账号、机器人名称和机器人标题的规则。
- 验证记录：
  - `cargo fmt --manifest-path backend/Cargo.toml --check` 通过。
  - `cargo check --manifest-path backend/Cargo.toml` 通过。
  - `cargo test --manifest-path backend/Cargo.toml user_group_buy_plan_masks_robot_initiator_display -- --nocapture` 通过。
  - `cargo test --manifest-path backend/Cargo.toml` 通过，204 条后端测试全部成功。
- 后续动作：如果后续用户端增加参与人列表或头像，也要复用同一套机器人前台脱敏规则。

## 2026-06-05 17:36 HKT 注单来源与下注购彩篮提示修正

- 完成任务：修正注单记录来源展示和下注页加入购彩篮交互。
- 解决问题：
  - 注单记录此前无法稳定区分独立下单和合买满单生成的真实投注订单。
  - 手机端下注页按钮使用“加入组合/加入篮子”文案，容易让用户误解为跨彩种玩法组合。
  - 下注页失败提示仍读取旧系统 `detail` 字段，当前后端统一信封返回 `message` 时可能只显示兜底提示。
  - 购彩篮虽然路由切换时会清空，但加入和提交动作缺少同彩种、同期号的边界校验。
- 具体实现：
  - 后端订单领域新增 `orderSource`，普通订单为 `direct`，合买成单订单为 `groupBuy`。
  - 新增迁移 `20260605173000_add_order_source.sql`，为 `orders.order_source` 设置默认值并补充中文注释。
  - 订单仓储 PostgreSQL 读写、订单摘要、用户注单列表和后台订单类型同步返回订单来源。
  - 合买满单成单链路调用 `create_with_source(..., OrderSource::GroupBuy)`，普通下单仍走默认 `direct`。
  - 手机端注单归一化保留 `orderSource/order_source/is_group_buy`，卡片和详情展示“独立下单/合买下单”。
  - 动态下注页按钮和购物篮弹层统一使用“加入购彩篮/提交购彩篮”，加入和提交时校验只能同一彩种、同一期号。
  - 下注页错误提示优先读取 `response.data.message`，兼容旧 `detail` 字段。
- 验证记录：
  - `cargo fmt --manifest-path backend/Cargo.toml --check` 通过。
  - `cargo check --manifest-path backend/Cargo.toml` 通过。
  - `cargo test --manifest-path backend/Cargo.toml` 通过，202 条后端测试全部成功。
  - `cd admin && npm run build` 通过；Vite 仍提示后台主 chunk 体积超过 500 kB，这是既有构建体积提示。
  - `cd mobile && npm run build` 通过。
- 后续动作：如后续需要把注单记录按来源筛选，可在 `/api/user/bet/orders` 和后台订单列表继续增加 `orderSource` 筛选参数。

## 2026-06-05 17:26 HKT 合买机器人分阶段补单修正

- 完成任务：修正合买机器人补单策略，让机器人不能创建后一次性满单，而是在临近封盘时按节奏分阶段补单。
- 解决问题：
  - 机器人此前会按剩余金额一次性补满合买计划，不符合“快开奖时再逐步满单”的业务要求。
  - 旧机器人补单参与记录 ID 固定，无法支持同一计划多次追加机器人参与记录。
  - 机器人自动发起的默认合买金额可能太小，容易导致只有一次有效补单空间。
- 具体实现：
  - `run_group_buy_robots` 解析本轮执行时间，并把时间传入补单决策。
  - 补单窗口限定为封盘前 90 秒：90-61 秒目标进度 40%，60-31 秒目标进度 60%，30-16 秒目标进度 80%，最后 15 秒目标进度 100%。
  - 未进入补单窗口或当前进度已达到阶段目标时，只写入中文跳过原因，不追加参与记录、不成单。
  - 每次机器人补单生成递增参与记录 ID，支持同一合买计划多次节奏补单。
  - 阶段补单未满时只写入本轮合买扣款流水；最终满单后才复用合买成单链路生成真实投注订单。
  - 机器人自动发起金额至少支持多个有效参与动作，避免默认金额过小导致无法分阶段。
  - Trellis 后端契约和架构设计同步记录补单窗口、阶段比例和验证要求。
- 验证记录：
  - `cargo fmt --manifest-path backend/Cargo.toml --check` 通过。
  - `cargo check --manifest-path backend/Cargo.toml` 通过。
  - `cargo test --manifest-path backend/Cargo.toml` 通过，201 条后端测试全部成功。
- 后续动作：后续继续补机器人执行历史持久化、风控限额、失败重试和跨仓储事务幂等保护。

## 2026-06-05 16:40 HKT 全项目审查首轮修复

- 完成任务：继续审查后端、管理后台、手机端和部署配置，先修复审查中发现的明确问题。
- 解决问题：
  - 后端系统设置种子中存在真实图床授权 token 默认值，属于敏感信息泄露风险。
  - 已经写入 PostgreSQL 的历史图床授权值可能来自该默认种子，需要在迁移中清空，避免继续使用泄露值。
  - 手机端 `npm test` 引用了一串不存在的 `.test.mjs` 文件，导致测试命令直接失败，无法作为质量门禁。
- 具体实现：
  - `image_bed_authorization_token` 种子默认值改为空字符串，说明改为必须由后台手动配置。
  - 新增迁移 `20260605164000_clear_seeded_image_bed_token.sql`，清空历史默认写入的图床授权值，并补充敏感配置不得写入真实密钥的 SQL 注释。
  - 手机端 `test` 脚本改为 `node --test` 自动发现测试文件；当前没有测试文件时输出 `0 tests` 并正常结束。
  - Trellis 数据库规范新增“系统设置敏感配置默认值”场景，约束后续 token、key、secret、password 等配置不能写入真实默认值。
- 验证记录：
  - `cargo fmt --manifest-path backend/Cargo.toml --check` 通过。
  - `cargo check --manifest-path backend/Cargo.toml` 通过。
  - `cargo test --manifest-path backend/Cargo.toml` 通过，201 条后端测试全部成功。
  - `cd admin && npm run build` 通过；Vite 仍提示后台主 chunk 体积超过 500 kB，这是既有构建体积提示。
  - `cd mobile && npm run build` 通过。
  - 修复前 `cd mobile && npm test` 失败，原因是脚本引用的测试文件不存在；修复后 `cd mobile && npm test` 通过，当前输出 `0 tests`。
- 后续动作：继续审查前端 `any` 使用、资金跨仓储事务、后台硬编码开奖源预设、系统设置敏感值展示和容器 WebSocket 发布后验证。

## 2026-06-05 16:10 HKT 容器 WebSocket 开奖推送修复

- 完成任务：排查手机端 WebSocket 没有推送开奖信息的问题，并修复单镜像容器里的 Nginx 代理配置。
- 解决问题：
  - 后端开奖调度、后台手动开奖和自动开奖链路已经会发布 `lottery.draw_result`，手机端也已经连接 `/api/user/realtime` 并监听开奖事件。
  - 容器内 Nginx 只按普通 HTTP 代理 `/api/`，没有转发 WebSocket `Upgrade` 和 `Connection` 头，导致打包部署后实时连接无法正确升级或长时间保持。
- 具体实现：
  - `docker/nginx.conf` 新增 `$connection_upgrade` 映射。
  - `/api/` 代理新增 `Upgrade`、`Connection` 请求头转发。
  - `/api/` 代理新增 `proxy_read_timeout` 和 `proxy_send_timeout`，覆盖后端实时心跳间隔，避免开奖推送连接被代理层提前断开。
  - Trellis 容器部署规范和架构设计同步记录 WebSocket 代理要求。
- 验证记录：
  - 已确认后端发布事件、用户端实时路由、手机端 `useWebSocket` 和页面 `wsMessage` 传递链路字段一致。
  - 本次按本地联调规则未执行 Docker 镜像打包；需要发布镜像时再做容器级 WebSocket 升级验证。
- 后续动作：部署新镜像后，在手机端保持首页打开并执行一次手动开奖或等待调度开奖，确认收到 `lottery.draw_result` 后最近开奖刷新。

## 2026-06-05 15:48 HKT 合买机器人真实执行

- 完成任务：完善合买机器人，让已启用的合买机器人可以在开盘期间自动发起合买，并按当前分阶段补单规则辅助自身计划和同彩种当前期的非机器人未满单计划生成真实投注订单。
- 解决问题：
  - 机器人此前只维护配置，后台启用后不会真实创建合买计划，也不会辅助满单。
  - 常驻开奖调度器每轮执行时没有调用机器人，导致“机器人配置”和真实业务链路脱节。
  - 管理后台缺少手动执行入口，联调时只能观察配置，无法看到本轮机器人为什么执行或跳过。
- 具体实现：
  - 新增 `services/group_buy_robot.rs`，按启用的 `groupBuy` 机器人、绑定彩种、当前 open 期号和已启用玩法生成确定性合买计划。
  - 机器人使用系统账户 `U90001` 出资，计划 ID 按“机器人 ID + 彩种 ID + 期号”生成，保证同一期重复调度不会重复发起。
  - 机器人创建计划后写入发起人 `groupBuyDebit`，后续按临近封盘阶段追加认购，满单后复用 `group_buy_flow` 创建真实投注订单并回写 `orderId`；该补单节奏已在 2026-06-05 17:26 修正为封盘前 90 秒内分阶段执行。
  - 机器人会扫描同彩种当前期由用户或后台发起的 `draft/open` 未满单计划，按临近封盘阶段辅助补单，最终满单后成单。
  - 执行过程遵守彩种开售、合买开关、期号 open、封盘时间、玩法启用、注数报价和余额校验；不满足条件时返回中文跳过原因。
  - 常驻开奖调度器在补齐未来期号后执行合买机器人，并推送机器人产生的余额和订单实时事件。
  - 管理后台新增“立即执行”按钮和本轮结果摘要，展示新增合买、满单、订单、扣款金额和跳过项。
  - OpenAPI、架构设计和 Trellis 后端契约同步记录合买机器人执行接口与调度关系。
- 验证记录：
  - `cd backend && cargo fmt && cargo check` 通过。
  - `cd backend && cargo fmt && cargo test` 通过，201 条后端测试全部成功。
  - `cd admin && npm run build` 通过；Vite 仍提示后台主 chunk 体积超过 500 kB，这是既有构建体积提示。
- 后续动作：继续补机器人风控限额、失败重试、执行历史持久化、管理员操作审计、跨仓储事务和购彩机器人真实执行。

## 2026-06-05 15:30 HKT 合买满单成单与结算分账

- 完成任务：完善合买后续核心流程，让合买计划可以按当前后台规则满单成单、封盘流单退款、开奖中奖分账，并同步后台和手机端展示。
- 解决问题：
  - 合买此前只做参与扣款，满单后没有真实投注订单，后续开奖结算无法识别合买方案。
  - 未满员计划到封盘后不会自动流单退款，用户资金会停留在合买认购扣款状态。
  - 中奖后缺少按参与份额给发起人和参与人分账的财务流水。
  - 后台合买页面缺少期号、玩法、投注内容和成单订单号配置展示，手机端仍保留旧号码格式提示。
- 具体实现：
  - 新增 `group_buy_flow` 编排服务，把合买投注文本解析为当前订单引擎的 `PlaySelection`，支持直选、组合、胆拖和大小单双输入格式。
  - 合买计划新增 `orderId` 持久化字段，满员后创建一张真实投注订单并回写订单号；真实订单不重复执行普通订单扣款。
  - 自动化封盘时取消未满员合买计划，并按参与记录写入幂等 `groupBuyRefund` 流水。
  - 开奖结算识别合买真实订单，中奖金额按参与金额比例拆分为参与人的 `payoutCredit`，并把合买计划标记为已结算。
  - 后台合买管理补充期号、玩法、标题、投注内容、订单号字段，财务流水补充合买认购和合买退款中文类型。
  - 手机端合买适配 `orderId` 展示，移除旧系统福彩 3D 纯数字校验，输入提示改为当前后端规则。
  - Trellis 后端契约、前端规范和架构设计同步记录合买真实成单、流单退款和分账规则。
- 验证记录：
  - `cargo fmt --manifest-path backend/Cargo.toml` 通过。
  - `cargo check --manifest-path backend/Cargo.toml` 通过。
  - `cargo test --manifest-path backend/Cargo.toml` 通过，198 条后端测试全部成功。
  - `cd admin && npm run build` 通过；Vite 仍提示后台主 chunk 体积超过 500 kB，这是既有构建体积提示。
  - `cd mobile && npm run build` 通过。
- 后续动作：合买机器人真实发起和补满已在 2026-06-05 15:48 实现；后续继续完善机器人风控限额、并发事务保护、执行历史、管理员操作审计和失败补偿。

## 2026-06-05 14:51 HKT 用户端合买真实参与流程

- 完成任务：完善合买功能，让手机端可以查看合买大厅、发起合买、参与合买和查看我的合买。
- 解决问题：
  - 手机端合买仍请求旧 `/group-buys/*` 路径，当前后端没有这些旧接口。
  - 合买计划缺少期号、玩法和投注内容字段，无法和当前彩种、期号、玩法配置对应。
  - 后台创建计划和追加参与人此前只写参与记录，没有真实扣减用户资金。
- 具体实现：
  - 后端合买计划补充 `issue`、`ruleCode`、`title`、`numbers` 字段，并新增数据库迁移。
  - 用户端新增 `/api/user/group-buy/*` 接口，覆盖列表、详情、发起、参与、我的合买和发起选项。
  - 发起和参与合买时校验彩种销售状态、合买开关、开放期号、启用玩法、投注内容、金额配置和可用余额。
  - 发起人自购和参与人认购都会写入 `groupBuyDebit` 资金流水，并推送用户余额变化实时事件。
  - 手机端合买 API 适配层切换到当前接口，并把金额在页面字符串和后端最小货币单位之间转换。
  - OpenAPI、Trellis 规格、前端组件规范和架构设计同步记录当前合买接口与后续边界。
- 验证记录：
  - `cargo fmt --manifest-path backend/Cargo.toml --check` 通过。
  - `cargo check --manifest-path backend/Cargo.toml` 通过。
  - `cargo test --manifest-path backend/Cargo.toml` 通过，192 条后端测试全部成功；仍有既有 `LotteryCategory` 未使用导入 warning。
  - `cd mobile && npm run build` 通过。
  - `cd admin && npm run build` 通过；Vite 仍提示后台主 chunk 体积超过 500 kB，这是既有构建体积提示。
- 后续动作：合买满单真实投注订单、流单退款和中奖分账已在 2026-06-05 15:30 继续实现；后续继续推进合买机器人执行、事务一致性和审计补偿。

## 2026-06-05 14:29 HKT 手机端实时事件接口重构

- 完成任务：重构手机端与后端的实时通信链路，移除旧系统 `/ws/lottery` 残留路径。
- 解决问题：
  - 手机端此前只尝试连接旧 WebSocket 路径，后端没有对应路由，实时开奖推送实际无法跑通。
  - 手机端页面直接依赖旧推送字段，不适合当前系统后续扩展封盘、开盘、余额、订单、充值、提现事件。
  - 用户资产事件如果简单广播会有隐私风险，需要按用户 token 过滤后只推送给本人。
- 具体实现：
  - 后端新增 `services/realtime.rs`，提供 `RealtimeHub`、公开/用户受众过滤和统一事件信封。
  - 用户侧新增 `GET /api/user/realtime` WebSocket 接口；匿名连接接收公开彩种事件，带 `token` 查询参数时可接收本人私有事件。
  - 开奖调度器、后台手动开奖、封盘、生成期号、自动结算、订单扣款/退款、充值确认、提现申请和提现审核都接入实时事件发布。
  - 手机端 `useWebSocket.ts` 改为连接 `/api/user/realtime`，并在登录 token 变化时自动重连。
  - 新增 `mobile/src/types/realtime.ts`，把后端 `lottery.draw_result`、`lottery.issue_opened`、`user.balance_changed` 等事件归一化为页面本地事件。
  - 手机端下注页监听开奖、封盘和开盘事件，命中当前彩种后刷新下注页配置。
  - 架构说明和 Trellis 规格同步记录新实时接口契约，后续不再使用旧 `/ws/lottery`。
- 验证记录：
  - `cd backend && cargo fmt --check` 通过。
  - `cd backend && cargo check` 通过。
  - `cd backend && cargo test` 通过，191 条后端测试全部成功；仍有既有 `LotteryCategory` 未使用导入 warning。
  - `cd mobile && npm run build` 通过。
  - `cd mobile && npm test` 未通过，原因是 `package.json` 中列出的 `.test.mjs` 文件当前不存在，命令在查找测试文件阶段失败。

## 2026-06-05 13:39 HKT 手机端注单详情按玩法展示匹配项

- 完成任务：优化手机端注单详情中的匹配项展示。
- 解决问题：注单详情原来只展示投注内容和开奖号码，没有依据直选、直选组合、组三、组六、胆拖、大小单双等玩法展示具体命中项。
- 具体实现：
  - `lotteryFormat.ts` 新增注单匹配项格式化逻辑，读取后端 `matchedBets`，按玩法输出中文标签、命中值和开奖窗口说明。
  - `OrderDetailSheet.vue` 新增“匹配项”面板，并在投注内容中高亮命中的投注组合。
  - `mobile/src/api/bet.ts` 在订单归一化时保留 `matched_bets`、`expanded_bets`，并让大小单双不再被当成直选展开。
  - 展示逻辑完全按当前 `/api/user/bet/orders` 契约适配，不再为旧系统订单字段做兜底。
  - 前端组件规范和架构说明同步记录注单详情匹配项展示规则。
- 验证记录：
  - `npx tsx -e "<调用 orderMatchItems/orderBetNumbers 的直选、组六、大小单双样例>"` 通过。
  - `cd mobile && npm run build` 通过。

## 2026-06-05 13:10 HKT 首页高频极速移除合买大厅按钮

- 完成任务：移除手机端首页“高频极速”推荐大卡底部的“合买大厅”按钮。
- 解决问题：首页高频极速模块主操作过多，用户要求不再展示合买大厅入口。
- 具体实现：
  - `HomeDrawCard.vue` 在 `featured` 卡片分支中只保留“立即投注”按钮。
  - 前端组件规范补充高频极速推荐大卡不展示“合买大厅”按钮的规则。
- 验证记录：
  - `cd mobile && npm run build` 通过。

## 2026-06-05 13:09 HKT 开奖结果状态中文显示

- 完成任务：修复手机端开奖结果状态 `drawn` 原样显示的问题。
- 解决问题：开奖历史或复用通用状态格式化的页面在没有开奖时间兜底时，可能直接展示接口状态值 `drawn`。
- 具体实现：
  - `lotteryFormat.ts` 将通用状态映射补充为 `drawn -> 已开奖`，并兼容 `pendingDraw -> 待开奖`。
  - 前端组件规范补充开奖历史和注单状态不能直接渲染接口状态值的约束。
- 验证记录：
  - `cd mobile && npm run build` 通过。

## 2026-06-05 13:06 HKT 手机端增加倍数按钮颜色修正

- 完成任务：修正手机端下注页增加倍数按钮的 `+` 颜色。
- 解决问题：增加倍数按钮中的 `+` 可能被全局按钮样式覆盖，显示不够清晰。
- 具体实现：
  - `DynamicBetPage.vue` 为增加倍数按钮和内部 `+` 文本强制设置白色文本样式。
- 验证记录：
  - `cd mobile && npm run build` 通过。

## 2026-06-05 13:04 HKT 手机端下注页倍数输入框优化

- 完成任务：优化手机端下注页的投注倍数输入控件。
- 解决问题：原倍数区域只有单个大号数字输入框，移动端直接输入不够顺手，也缺少快速微调入口。
- 具体实现：
  - `DynamicBetPage.vue` 将倍数输入改为 `- / 数字输入 / +` 的步进控件。
  - 输入框改为数字键盘输入，只保留数字字符，失焦或回车时自动夹到玩法允许的倍数范围。
  - 加减按钮根据最小倍数和最大倍数自动禁用，并继续保留原有滑块联动。
- 验证记录：
  - `cd mobile && npm run build` 通过。
  - 本地浏览器打开手机端应用可达；下注页受登录态和路由保护影响，未完成真实下注页视觉确认。

## 2026-06-05 13:00 HKT 手机端合买最低自购文案调整

- 完成任务：调整手机端下注页合买模式中的发起人最低自购提示文案。
- 解决问题：原文案显示为“发起人最低自购 0 份（10%，0% 表示不限制）”，容易让用户误解为最低自购份数为 0。
- 具体实现：
  - `DynamicBetPage.vue` 新增最低自购比例展示文本，去掉末尾无效小数。
  - 文案改为“发起人最低自购10%”这类百分比展示，不再显示“份”和“0% 表示不限制”。
- 验证记录：
  - `cd mobile && npm run build` 通过。

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
- 解决问题：这是 2026-06-03 的历史修复记录，当时目标是避免开奖后没有下一期；当前 2026-06-16 规则已更正为封盘后等待开奖，到开奖点才开启下一期。
- 技术说明：历史实现曾只统计 `open` 期号；当前调度口径已改为 `open` 或 `closed` 且 `scheduledAt > now` 的待处理期号占用未来缓冲，避免封盘后提前开下一期。
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

## 2026-06-05 14:02 HKT 手机端我的账户资金流水

- 完成任务：在手机端“我的账户”新增“资金流水”入口，并新增独立资金流水页面。
- 解决问题：此前用户只能在充值、提现等局部页面看到部分记录，缺少一个统一查看本人资金变动的地方；本次接入当前系统的 `GET /api/user/ledger-entries`，展示登录用户自己的充值入账、投注扣款、派奖、退款、提现冻结/打款/驳回和财务调整流水。
- 实施内容：补充手机端资金流水类型和请求方法；新增 `/ledger` 路由；“我的账户”列表增加资金流水入口；资金流水页展示当前余额、入账合计、支出合计、流水笔数、变动后余额和创建时间，不展示关联单号。
- 约束说明：资金流水页只调用用户侧当前接口，不调用后台全量资金流水接口，不兼容旧系统字段。
- 验证结果：`npm run build` 通过，浏览器访问 `/ledger` 未登录时正常跳转登录页且无控制台错误；`git diff --check` 通过。`npm test` 当前失败，原因是 `mobile/package.json` 中列出的测试文件在 `mobile/` 目录下不存在或路径未配置正确，本次未改动该既有测试脚本。

## 2026-06-05 14:08 HKT 手机端资金流水隐藏关联单号

- 完成任务：按最新要求移除手机端资金流水列表中的“关联单号”展示。
- 解决问题：资金流水条目底部此前会把后端 `referenceId` 渲染为“关联单号”，手机端用户不需要看到该内部关联编号；本次保留接口字段和数据类型，只在页面展示层隐藏。
- 文档同步：更新架构说明和 TODO 中的资金流水展示范围，明确手机端展示余额、金额、描述、类型和时间，不展示关联单号。
- 验证结果：`npm run build` 通过，`git diff --check` 通过；`npm test` 仍因 `mobile/package.json` 中列出的测试文件在 `mobile/` 目录下不存在或路径未配置正确而失败，本次未改动该既有测试脚本。

## 2026-06-05 14:08 HKT 手机端首页统计卡片默认隐藏

- 完成任务：把手机端首页接口的 `settings.statsEnabled` 默认值从开启调整为关闭。
- 解决问题：手机端首页统计卡片由 `settings.statsEnabled` 控制，前端本地兜底已经是关闭，但后端首页聚合接口默认返回开启，导致首页默认展示“今日中奖人数”和“累计派奖金额”统计卡片；本次改为默认不显示。
- 实施内容：更新 `build_mobile_lottery_home` 默认配置，新增单元测试断言 `stats_enabled=false`，并同步 API 契约说明。
- 验证结果：`cargo fmt --manifest-path backend/Cargo.toml --check`、`cargo check --manifest-path backend/Cargo.toml`、`cargo test --manifest-path backend/Cargo.toml mobile_home -- --nocapture`、`cargo test --manifest-path backend/Cargo.toml -- --nocapture` 和 `git diff --check` 均通过；后端全量 188 个测试通过，保留既有测试导入警告。

## 2026-06-05 14:14 HKT 手机端大小单双投注内容显示修正

- 完成任务：修正手机端注单列表和注单详情中大小单双投注内容的展示方式。
- 解决问题：此前大小单双订单的 `numbers` 文本会被普通投注号码逻辑拆分，详情页可能显示为“十位:大”“小”这类普通号码球，位置和属性关系不清晰；本次新增投注内容格式化逻辑，把 `big/small/odd/even` 翻译成“大/小/单/双”，并按位置展示。
- 实施内容：新增订单投注内容文本和分组格式化方法；注单列表显示“十位：大、小；个位：单、双”；注单详情对大小单双使用属性标签，不再使用普通号码球。
- 验证结果：`npm run build` 和 `git diff --check` 通过；`npm test` 仍因 `mobile/package.json` 中列出的测试文件在 `mobile/` 目录下不存在或路径未配置正确而失败，本次未改动该既有测试脚本。

## 2026-06-05 20:00 HKT 手机端邀请中心接入新后台

- 完成任务：为用户端新增 `GET /api/user/invitations/summary` 邀请中心汇总接口，并把手机端 `InvitationCenterView.vue` 从旧 `/auth/invitations/summary` 切换到当前后台接口。
- 解决问题：旧页面仍使用 snake_case 字段和旧接口，无法读取当前系统的代理邀请码、邀请策略、注册代理关系和资金流水；通过邀请码注册的直属用户也可能因为没有后台邀请记录而不显示。
- 实施内容：后端邀请中心合并后台邀请记录与用户 `agentId` 代理关系，普通用户返回 `canInvite=false`，代理用户按返利策略判断是否可复制邀请码；直属充值按 `rechargeCredit` 正向资金流水汇总；手机端展示返利模式、默认返利比例、直属人数、有效下级、直属充值、已付返利和直属用户状态。
- 验证结果：`cargo fmt --manifest-path backend/Cargo.toml --check`、`cargo check --manifest-path backend/Cargo.toml`、`cargo test --manifest-path backend/Cargo.toml -- --nocapture`、`npm run build` 和 `git diff --check` 均已通过；本地内存后端 `GET /api/user/invitations/summary` 真实请求验证通过，代理 `agent_alpha` 可邀请并返回 2 个直属用户，普通用户 `demo_user` 返回 `canInvite=false` 且直属列表为空；浏览器烟测确认 `/invitation-center` 路由可加载且无控制台错误。

## 2026-06-05 21:13 HKT 合买机器人按玩法随机选号

- 完成任务：修正合买机器人自动发起合买计划的投注内容生成逻辑。
- 解决问题：此前机器人会长期使用 `1|2|3`、`1,2,3` 这类固定样例投注内容，用户容易看出机器人计划规律，也不能体现不同玩法的真实选号格式。
- 实施内容：新增按机器人、彩种、期号、玩法和玩法顺序派生的确定性随机选号器；直选、直选组合、组选、胆拖、大小单双分别生成对应合法格式，生成后继续经过合买选号解析和订单报价校验。
- 验证结果：`cargo fmt --manifest-path backend/Cargo.toml --check`、`cargo check --manifest-path backend/Cargo.toml`、`cargo test --manifest-path backend/Cargo.toml -- --nocapture` 和 `git diff --check` 均通过；全量后端 216 个测试成功，其中新增覆盖所有玩法机器人选号可解析可展开，以及直选投注内容会随期号变化、不再固定为 `1|2|3`。

## 2026-06-05 21:33 HKT 机器人账户自动授信与后台过滤

- 完成任务：为合买机器人账户新增自动授信/自动补余额，并在后台订单管理、财务管理中新增“显示机器人数据”开关。
- 解决问题：机器人账户余额不足时会导致合买计划创建或临近封盘补单失败；同时订单列表、资金账户、资金流水和财务总览默认混入机器人数据，会影响运营查看真实用户业务。
- 实施内容：机器人发起合买计划和分阶段补单前先检查 `U90001` 可用余额，余额不足时写入正向 `manualAdjustment` 流水“机器人账户自动授信补余额”，再继续真实扣款；后台财务总览、资金账户、资金流水和订单列表新增 `includeRobotData` 查询口径，默认过滤机器人数据，前端通过 Semi UI `Switch` 控制是否显示。
- 验证结果：`cargo fmt --manifest-path backend/Cargo.toml`、`cargo check --manifest-path backend/Cargo.toml`、`cargo test --manifest-path backend/Cargo.toml -- --nocapture`、`npm run build`（admin）和 `git diff --check` 均通过；后端全量 219 条测试成功，新增覆盖机器人账户自动授信和后台机器人数据默认过滤；本地接口烟测确认资金账户和订单默认不含 `U90001`，传入 `includeRobotData=true` 后包含机器人数据，财务总览金额随过滤口径变化。

## 2026-06-05 21:45 HKT 后台合买计划列表分页

- 完成任务：为后台合买计划列表新增分页能力。
- 解决问题：合买管理页此前一次性请求全部合买计划，计划数量增加后会拖慢页面加载和运营扫描。
- 实施内容：`GET /api/admin/group-buy/plans` 支持 `page/pageSize` 并返回 `items/totalCount/page/pageSize/totalPages`；后台 API client、`useGroupBuyPlans` 和合买管理页面接入分页参数；列表顶部增加每页条数、上一页和下一页控件。
- 验证结果：`cargo fmt --manifest-path backend/Cargo.toml`、`cargo check --manifest-path backend/Cargo.toml`、`cargo test --manifest-path backend/Cargo.toml -- --nocapture`、`npm run build`（admin）和 `git diff --check` 均通过；后端全量 220 条测试成功，新增覆盖后台分页结构的当前页切片、总数和总页数。

## 2026-06-06 03:03 HKT 充值返利真实入账修复

- 完成任务：补齐充值成功后给上级代理发放返利的真实资金链路，新增 `rechargeRebateCredit` 资金流水类型，并接入彩虹易支付回调和后台客服直充确认。
- 解决问题：此前系统只有返利策略配置、邀请关系和邀请中心摘要，充值确认只给充值用户写 `rechargeCredit`，没有给上级代理写返利流水，导致“充值返利给上级代理”看起来完全没有作用；本次按后台邀请记录或注册 `agentId` 解析上级代理，并用 `recharge-rebate:{充值单号}:{代理用户 ID}` 幂等引用避免重复回调重复发放。
- 实施内容：后端新增返利发放服务逻辑，优先使用 `status=active` 且 `rebateEnabled=true` 的人工邀请记录；被邀请人已有人工邀请记录但记录禁用时不回退 `agentId`；无人工记录时使用注册代理关系；返利接收方必须是有效代理用户。邀请中心 `totalPaidCommissionMinor` 改为统计真实正向 `rechargeRebateCredit` 流水；后台和手机端资金流水补充“充值返利”展示；数据库迁移更新 `ledger_entries.kind` 中文注释；同步更新架构说明和 API 契约。
- 验证结果：`cargo fmt --manifest-path backend/Cargo.toml --check`、`cargo check --manifest-path backend/Cargo.toml`、`cargo test --manifest-path backend/Cargo.toml -- --nocapture`、管理后台 `npm run build`、手机端 `npm run build` 和 `git diff --check` 均通过；后端全量 230 条测试成功。使用本地 PostgreSQL 连接启动后端烟测时迁移和仓储初始化未出现版本错误，随后立即停止服务；期间仅出现已有腾讯分分彩历史期号开奖源缺期警告，与本次返利修复无关。

## 2026-06-06 13:09 HKT 系统逻辑漏洞审查与充值返利幂等修复

- 完成任务：审查资金、充值返利、提现、批量下注、合买机器人、后台权限和实时事件等关键流程，并修复充值返利幂等引用规则。
- 解决问题：此前 `rechargeRebateCredit` 使用 `recharge-rebate:{充值单号}:{代理用户 ID}` 作为幂等引用；如果同一充值单首次返利后，后台邀请关系或注册代理关系发生变化，后续支付通知或人工确认再次触发时可能给新代理再发一笔返利。本次改为 `recharge-rebate:{充值单号}`，确保同一充值订单无论代理关系如何变化都只能生成一笔返利流水。
- 实施内容：更新 `FinanceRepository::credit_recharge_rebate` 和 `FinanceStore::credit_recharge_rebate` 注释与引用生成逻辑，新增“代理变化后重复触发不重复发放”的后端单元测试；同步更新 `架构设计.md`、`.trellis/spec/backend/api-contracts.md` 和 `.trellis/spec/backend/database-guidelines.md` 的资金幂等约定。
- 验证结果：`cargo fmt --manifest-path backend/Cargo.toml`、`cargo check --manifest-path backend/Cargo.toml` 和 `cargo test --manifest-path backend/Cargo.toml -- --nocapture` 均通过；后端全量 231 条测试成功。审查中仍发现跨仓储事务、批量下注失败补偿、后台高危操作权限和合买机器人身份隐藏等后续风险，需要下一阶段继续修复。

## 2026-06-06 13:42 HKT 资金与订单跨仓储事务修复

- 完成任务：为直接下注、批量下注、订单取消退款、开奖结算派奖、充值入账和提现冻结/审核新增跨仓储事务协调。
- 解决问题：此前订单、资金、充值、提现和合买结算按仓储分步保存，可能出现订单已创建但扣款失败、批量下注返回失败但部分订单已生效、提现冻结成功但提现单未保存、结算订单已标记中奖但派奖流水未入账等状态漂移。本次改为先在仓储快照上完成业务变更，PostgreSQL 模式下使用同一个 SQLx 事务保存相关业务表，提交成功后再替换运行时内存快照。
- 实施内容：`OrderRepository` 新增 `create_with_debit/create_many_with_debit/cancel_with_refund/settle_with_payouts`；用户端批量下注和后台订单创建、取消、结算切换到事务入口；`RechargeRepository` 确认入账与 `FinanceStore::credit_recharge` 同事务保存；`WithdrawalRepository` 提现申请、通过、驳回与资金冻结/打款/解冻同事务保存；各仓储抽出 `save_*_store_in_transaction` 内部函数。
- 验证结果：`cargo fmt --manifest-path backend/Cargo.toml --check`、`cargo check --manifest-path backend/Cargo.toml` 和 `cargo test --manifest-path backend/Cargo.toml -- --nocapture` 均通过；后端全量 234 条测试成功，新增覆盖批量下注失败不产生部分订单和扣款、订单取消退款、开奖结算派奖。
- 后续动作：合买发起、合买认购、后台追加合买参与人和机器人补单仍需要继续收敛到统一事务协调入口，当前保留既有补偿删除逻辑。

## 2026-06-06 15:17 HKT 后台支付方式开关配置

- 完成任务：在后台系统设置的“充值设置”Tab 新增“支付方式开关”面板，可独立配置彩虹易支付、客服直充以及彩虹易支付下的支付宝/微信充值方式。
- 解决问题：此前支付方式只能在普通配置项里维护，运营不容易看出哪些充值方式实际开启；如果彩虹易支付总开关开启但支付方式为空，用户端还可能回退到默认支付宝，导致后台开关和实际下单入口不一致。
- 实施内容：管理后台隐藏被面板接管的 `recharge_rainbow_epay_enabled`、`recharge_customer_service_enabled`、`recharge_rainbow_epay_pay_types` 普通项，改用清晰的开关和多选控件维护；后端在充值配置返回和下单校验中要求彩虹易支付必须至少开启一个支付方式；手机端充值页只展示真正可用的充值渠道，不再为缺失 `payTypes` 的彩虹易支付默认补支付宝。
- 验证结果：`cargo fmt --manifest-path backend/Cargo.toml --check`、`cargo check --manifest-path backend/Cargo.toml`、`cargo test --manifest-path backend/Cargo.toml -- --nocapture`、管理后台 `npm run build`、手机端 `npm run build` 和 `git diff --check` 均通过；后端全量 236 条测试成功，新增覆盖彩虹易支付开启但支付方式为空时用户端配置不可用、下单返回错误；后台 dev server 浏览器烟测通过，页面标题正常且无控制台错误。

## 2026-06-06 15:46 HKT 后台客服聊天表情面板

- 完成任务：为后台在线客服回复区接入 `@emoji-mart/react` 和 `@emoji-mart/data`，客服可以打开表情面板并把表情插入回复内容。
- 解决问题：此前后台客服回复只能输入纯文本，无法从聊天界面直接选择表情；如果静态导入完整表情数据，也会把后台主包撑大。
- 实施内容：后台新增 emoji-mart 依赖；`SupportManagementPage` 增加“表情”按钮和 Semi UI `Popover` 弹层；点击表情后按当前光标位置插入原生 emoji，并恢复输入焦点；表情选择器、表情数据和中文语言包都通过动态 `import()` 懒加载，保持首屏主包相对轻量。
- 验证结果：管理后台 `npm run build` 通过，构建输出中 emoji-mart 相关代码已拆为独立异步 chunk；`git diff --check` 通过；后台 dev server 浏览器烟测通过，页面标题正常且无控制台错误。

## 2026-06-06 15:58 HKT 手机端客服聊天表情面板

- 完成任务：为手机端在线客服输入栏增加表情按钮和 emoji-mart 表情面板，让用户也可以在客服会话中发送表情。
- 解决问题：此前只有后台客服侧支持表情，用户手机端仍只能输入纯文本，客服直充聊天体验不对称；同时手机端是 Vue，不能直接使用 React 版 `@emoji-mart/react`。
- 实施内容：手机端新增 `emoji-mart` 和 `@emoji-mart/data` 依赖，并生成 `pnpm-lock.yaml` 记录当前 pnpm 依赖树；`SupportView.vue` 通过动态 `import()` 加载原生 `Picker`、表情数据和中文语言包；表情面板挂在输入栏上方，选中后按当前输入框光标位置插入原生 emoji；`LucideIcon` 新增 `mood` 图标映射。
- 验证结果：手机端 `npm run build` 通过，构建输出中 `emoji-mart` 与表情数据已拆为独立异步 chunk；`git diff --check` 通过；手机端 dev server 浏览器烟测通过，未登录访问 `/support` 正常跳转登录页且无浏览器控制台错误；烟测时因本地后端未启动出现 Vite 代理日志，与本次表情面板改动无关。

## 2026-06-06 16:25 HKT 手机端客服表情面板重渲染错误修复

- 完成任务：修复手机端在线客服在收到 WebSocket 实时消息后可能触发的 `Cannot read properties of null (reading 'emitsOptions')` 运行时错误。
- 解决问题：此前 `emoji-mart` 原生 `Picker` 被 `replaceChildren` 注入到 Vue 模板管理的 DOM 子树里；实时消息更新 `LayoutView` 的 `router-view` props 后会让客服页重渲染，手动改过的子节点可能破坏 Vue patch 状态。
- 实施内容：表情面板改用 `Teleport` 渲染到 `body`，原生 `Picker` 只挂载到无 Vue 子节点的空宿主容器，并在父节点不一致时复用 `appendChild`；同步更新前端组件规范和架构说明。
- 验证结果：手机端 `npm run build` 和 `git diff --check` 通过；本地 dev server 配合临时 mock 后端完成登录、客服会话加载、WebSocket 推送客服消息和打开表情面板烟测，浏览器控制台无 `emitsOptions` 或空对象读取错误。

## 2026-06-06 16:32 HKT 后台客服表情弹窗重复打开修复

- 完成任务：修复后台在线客服表情弹窗第一次打开后，关闭再点击“表情”无法再次正常打开的问题。
- 解决问题：此前 Semi UI `Popover` 和 `emoji-mart` Picker 同时处理外部点击关闭；Picker 内部监听在弹窗关闭后可能影响下一次按钮点击，导致第二次打开瞬间又关闭。
- 实施内容：后台客服表情弹窗改为由 Popover 统一处理外部点击关闭，设置 `keepDOM` 复用 Picker 实例，并移除 Picker 的 `onClickOutside`；同步更新前端组件规范和架构说明。
- 验证结果：管理后台 `npm run build` 和 `git diff --check` 通过；本地 dev server 配合临时 mock 后端完成后台登录、进入在线客服、第一次打开表情弹窗、关闭、第二次再次打开的浏览器烟测，关闭后弹窗尺寸归零，第二次打开后 Picker 和 Popover 尺寸恢复正常，浏览器控制台无错误。

## 2026-06-06 23:06 HKT 手机端客服表情弹窗重复打开修复

- 完成任务：修复手机端 `/support` 表情弹窗第一次打开后，关闭再点击“表情”无法再次正常打开的问题。
- 解决问题：此前手机端虽然已经把 `emoji-mart` 原生 Picker 放到 `Teleport` 面板里，但仍给 Picker 传入 `onClickOutside`；Picker 内部监听与 Vue 遮罩关闭逻辑叠加，可能导致第二次点击按钮时刚打开就被关掉。
- 实施内容：移除手机端原生 Picker 的 `onClickOutside`，外部点击关闭统一由 Vue `Teleport` 遮罩处理，选中表情后仍由 `insertEmoji` 插入并关闭弹窗；同步更新前端组件规范和架构说明。
- 验证结果：手机端 `npm run build`、`npm test` 和 `git diff --check` 通过；本地 dev server 配合临时 mock 后端完成登录、进入 `/support`、第一次打开表情弹窗、点击遮罩关闭、第二次再次打开的浏览器烟测，关闭后 Picker 尺寸归零，第二次打开后 Picker 和遮罩尺寸恢复正常，浏览器控制台无错误。

## 2026-06-07 01:09 HKT 手机端公共聊天大厅

- 完成任务：新增手机端公共聊天大厅，所有登录用户都可以进入同一大厅发送和查看消息。
- 解决问题：此前手机端只有一对一客服会话，缺少面向所有会员的公共聊天场景；如果复用客服会话会把客服工单和公共聊天混在一起，也无法把消息广播给所有在线用户。
- 实施内容：后端新增 `chat_hall_messages` 数据表、领域模型、仓储、`GET/POST /api/user/chat-hall/messages` 接口和 `chat_hall.message_created` 公开实时事件；手机端新增聊天大厅 API 类型、实时事件归一化、`/chat-hall` 页面和“我的账户”入口；架构说明和 Trellis 规范同步记录接口契约。
- 验证结果：`cargo fmt --manifest-path backend/Cargo.toml --check`、`cargo check --manifest-path backend/Cargo.toml`、`cargo test --manifest-path backend/Cargo.toml chat_hall -- --nocapture`、`cargo test --manifest-path backend/Cargo.toml realtime -- --nocapture`、`cargo test --manifest-path backend/Cargo.toml openapi -- --nocapture`、`cargo test --manifest-path backend/Cargo.toml -- --nocapture`、手机端 `npm run build`、手机端 `npm test` 和 `git diff --check` 均通过；本地内存后端配合手机端 dev server 完成浏览器烟测，`demo_user` 登录后进入 `/chat-hall` 可发送消息，`agent_alpha` 通过接口发送的大厅消息可通过 WebSocket 实时追加，浏览器控制台无错误。

## 2026-06-07 15:40 HKT 手机端聊天大厅表情面板

- 完成任务：为手机端 `/chat-hall` 聊天大厅输入栏增加表情按钮和 emoji-mart 表情面板。
- 解决问题：聊天大厅此前只能手动输入或粘贴 emoji，没有表情选择器；用户在聊天大厅无法像在线客服一样直接选择表情。
- 实施内容：`ChatHallView.vue` 复用手机端客服页稳定的表情方案，动态加载 `emoji-mart` 原生 `Picker`、`@emoji-mart/data` 和中文语言包；表情面板通过 `Teleport` 挂到 `body`，由 Vue 遮罩关闭，不传 `onClickOutside`；选中表情后按当前输入框光标位置插入并恢复焦点。
- 验证结果：手机端 `npm run build`、手机端 `npm test` 和 `git diff --check` 均通过；代码检查确认聊天大厅未使用 `onClickOutside` 或 `replaceChildren`。本轮浏览器烟测未完成，后续如需继续验证可本地打开 `/chat-hall` 检查表情面板重复打开和插入效果。

## 2026-06-07 16:19 HKT 手机端底部导航聊天大厅入口

- 完成任务：在手机端 `mobile-bottom-nav` 新增“聊天”入口，点击后进入 `/chat-hall` 公共聊天大厅。
- 解决问题：聊天大厅此前只能从“我的账户”进入，底部主导航没有直达入口；如果直接显示底部导航，原固定输入栏和表情面板会被导航栏遮挡。
- 实施内容：`LayoutView.vue` 新增聊天导航项和 `/chat-hall` 激活态，并取消聊天大厅隐藏底部导航；`ChatHallView.vue` 为底部导航定义预留空间，将输入栏和表情面板抬到导航栏上方，同时增加消息列表底部滚动留白；架构说明和前端组件规范同步记录该布局约束。
- 验证结果：手机端 `npm run build`、手机端 `npm test` 和 `git diff --check` 均通过。

## 2026-06-07 16:51 HKT 聊天大厅标题栏按钮移除

- 完成任务：移除手机端聊天大厅标题栏里的返回按钮和刷新按钮。
- 解决问题：用户要求聊天大厅不再展示 `chat-hall__icon-button` 返回和刷新入口，顶部保留更简洁的标题说明即可。
- 实施内容：`ChatHallView.vue` 删除返回/刷新按钮、清理不再使用的 `useRouter`、按钮样式和刷新旋转动画，并把标题栏改为居中标题布局；架构说明和前端组件规范同步记录聊天大厅顶部展示规则。
- 验证结果：手机端 `npm run build`、手机端 `npm test` 和 `git diff --check` 均通过；代码检查确认聊天大厅已无 `chat-hall__icon-button`、`useRouter` 和刷新旋转动画残留。

## 2026-06-07 17:33 HKT 聊天大厅红包与合买分享

- 完成任务：优化手机端聊天大厅底部输入区和底部导航的视觉关系，并新增发送红包、领取红包、发送自己的合买计划能力。
- 解决问题：截图中输入栏和底部导航像两个割裂浮层，聊天室只能发送文本，不能承载红包和合买计划分享。
- 实施内容：后端聊天消息新增 `messageType/payload`，新增红包表、红包领取表、红包扣款/入账流水类型和三个用户端接口；红包发送和领取与资金快照同事务保存；合买分享只允许分享当前用户发起或参与过的计划。手机端聊天大厅新增“+”附件菜单、红包弹窗、合买计划选择弹窗、红包卡片、合买进度卡片，并把同 ID 实时消息改成替换更新；合买大厅支持通过 `plan_id` 自动打开计划详情。
- 验证结果：`cargo fmt --manifest-path backend/Cargo.toml --check`、`cargo check --manifest-path backend/Cargo.toml`、`cargo test --manifest-path backend/Cargo.toml chat_hall -- --nocapture`、`cargo test --manifest-path backend/Cargo.toml openapi -- --nocapture`、`cargo test --manifest-path backend/Cargo.toml realtime -- --nocapture`、`cargo test --manifest-path backend/Cargo.toml -- --nocapture`、手机端 `npm run build`、手机端 `npm test` 和 `git diff --check` 均通过。浏览器烟测尝试打开本地手机端页面时内置浏览器连接超时，未完成截图验证。

## 2026-06-07 17:41 HKT 聊天大厅副标题移除

- 完成任务：删除手机端聊天大厅标题下方副标题文案。
- 解决问题：用户要求聊天大厅不再展示顶部副标题，顶部区域需要更简洁。
- 实施内容：`ChatHallView.vue` 移除副标题节点和对应样式，并收紧顶部栏高度与消息列表顶部留白；同步更新架构说明和前端组件规范，记录聊天大厅顶部只保留标题。
- 验证结果：手机端 `npm run build`、手机端 `npm test` 和 `git diff --check` 均通过；全局搜索确认原副标题文案已不再出现在页面代码、架构说明和前端规范中。

## 2026-06-07 17:55 HKT 资金流水保存失败修复

- 完成任务：修复用户端出现 `internal error: 资金流水数据保存失败` 时资金快照并发保存不稳定的问题。
- 解决问题：资金仓储使用快照式保存，多个请求同时保存 `ledger_entries` 时可能在删除和重插之间互相冲突；历史数据库如果 `finance_runtime.next_sequence` 落后，也可能生成重复流水编号。
- 实施内容：`save_finance_store_in_transaction` 保存前锁定 `ledger_entries`、`financial_accounts`、`finance_runtime` 三张资金表，并在资金流水插入失败时记录具体数据库错误和流水上下文；启动加载资金仓储时按已有 `L...` 流水编号校正 `next_sequence`；新增迁移补充红包资金流水类型的中文字段注释；同步更新架构说明和数据库规范。
- 验证结果：`cargo fmt --manifest-path backend/Cargo.toml --check`、`cargo check --manifest-path backend/Cargo.toml`、`cargo test --manifest-path backend/Cargo.toml finance -- --nocapture`、`cargo test --manifest-path backend/Cargo.toml chat_hall -- --nocapture`、`cargo test --manifest-path backend/Cargo.toml -- --nocapture` 和 `git diff --check` 均通过；数据库只读检查确认当前 `finance_runtime.next_sequence` 与现有最大资金流水编号一致，且已存在红包扣款和红包入账流水类型。

## 2026-06-07 19:12 HKT 后端中文注释补齐

- 完成任务：补齐后端代码中缺少中文说明的核心声明和接口处理函数。
- 解决问题：后端领域模型、路由处理函数、仓储入口和数据库持久化入口有大量声明缺少中文注释，后续维护时需要反复阅读实现才能理解用途。
- 实施内容：为 `backend/src/domain` 的公开模型、`backend/src/routes` 的非测试接口处理函数、`backend/src/services` 的仓储/Store/load/save 核心入口，以及 `app/error/response/main` 主入口补充中文注释；同步更新后端质量规范，要求后续新增公开模型、路由处理函数、仓储入口和持久化方法必须写中文用途说明。
- 验证结果：后端注释扫描确认公开模型、路由处理函数、核心仓储/Store/load/save 入口缺口为 0；`cargo fmt --manifest-path backend/Cargo.toml --check`、`cargo check --manifest-path backend/Cargo.toml`、`cargo test --manifest-path backend/Cargo.toml -- --nocapture` 和 `git diff --check` 均通过。

## 2026-06-07 23:13 HKT 手机端开奖结果号码球尺寸优化

- 完成任务：缩小手机端“开奖结果”列表和单彩种全部开奖弹层里的开奖号码球。
- 解决问题：此前 `ResultBalls` 默认号码球为 40px，在开奖列表卡片里视觉偏大，5 位开奖结果会占用过多空间。
- 实施内容：将 `ResultBalls` 默认号码球调整为 32px，小屏调整为 30px，并同步收紧间距、字号、阴影和字距；同步更新架构说明和前端组件规范。
- 验证结果：手机端 `npm run build`、手机端 `npm test` 和 `git diff --check` 均通过；当前手机端测试命令显示 0 个测试用例。

## 2026-06-07 23:44 HKT 合买认购失败问题修复

- 完成任务：排查并修复合买计划偶发无法认购的问题。
- 解决问题：参与人最低认购金额和剩余金额存在冲突，计划可能留下低于最低认购金额的小尾巴，导致后续用户既不能按最低金额认购，也不能超过剩余金额；手机端也没有拿到参与人最低认购金额，默认金额可能低于后台配置。
- 实施内容：用户端合买计划响应新增 `participantMinAmountMinor`；后端允许最后尾单低于最低认购金额时一次性全包，同时拒绝普通认购后留下不可认购小尾巴；手机端认购金额按最低认购、单份金额和剩余金额自动归一化，并优先展示后端统一错误 `message`；同步更新架构说明和前后端规范。
- 验证结果：`cargo fmt --manifest-path backend/Cargo.toml --check`、`cargo check --manifest-path backend/Cargo.toml`、`cargo test --manifest-path backend/Cargo.toml group_buy -- --nocapture`、`cargo test --manifest-path backend/Cargo.toml -- --nocapture`、手机端 `npm run build`、手机端 `npm test` 和 `git diff --check` 均通过；手机端测试命令当前显示 0 个测试用例。

## 2026-06-08 00:13 HKT 手机端用户头像设置

- 完成任务：补齐登录用户设置头像能力，手机端“我的账户”可点击头像上传图片。
- 解决问题：此前用户资料没有头像字段，手机端也没有用户自己的头像上传入口；如果直接复用后台图床上传接口，会把普通用户头像能力绑到后台权限上。
- 实施内容：`users` 表新增 `avatar_url` 字段和中文字段注释；后端 `UserSummary` 新增 `avatarUrl`，并新增 `PUT /api/user/avatar`、`POST /api/user/avatar/upload` 用户端受保护接口；上传接口读取系统图床配置透传图片并自动保存返回链接。手机端新增头像 API 封装，个人中心头像区域支持上传成功后刷新页面资料和 Pinia 登录态；同步更新架构说明、OpenAPI 和前后端规范。
- 验证结果：`cargo fmt --manifest-path backend/Cargo.toml --check`、`cargo check --manifest-path backend/Cargo.toml`、`cargo test --manifest-path backend/Cargo.toml access_repository -- --nocapture`、`cargo test --manifest-path backend/Cargo.toml openapi -- --nocapture`、`cargo test --manifest-path backend/Cargo.toml -- --nocapture`、后台 `npm run build`、手机端 `npm run build`、手机端 `npm test` 和 `git diff --check` 均通过；完整后端测试 250 条通过，手机端测试命令当前显示 0 个测试用例。

## 2026-06-08 00:39 HKT 手机端头像点击与圆形展示修复

- 完成任务：修复手机端“我的账户”点击头像没有反应的问题，并把头像展示改成圆形。
- 解决问题：此前头像上传依赖 `van-uploader` 自定义插槽包裹按钮，移动端点击头像区域可能没有稳定触发文件选择；头像容器使用圆角矩形，不符合圆形头像要求。
- 实施内容：头像上传触发改为原生 `input[type="file"]` 与 `label for` 绑定，点击头像本体即可打开文件选择器；头像容器、上传中遮罩和相机角标改为圆形视觉；同步更新架构说明和前端组件规范。
- 验证结果：手机端 `npm run build`、手机端 `npm test` 和 `git diff --check` 均通过；当前环境未安装 Playwright，无法自动触发文件选择器烟测，但源码检查确认头像 `label for="profile-avatar-input"` 已绑定原生文件输入。

## 2026-06-08 00:43 HKT 手机端下注页最近开奖号码防溢出

- 完成任务：缩小手机端下注页顶部“上期开奖”的开奖号码球，并防止号码跑到手机屏幕外面。
- 解决问题：此前 `BetRoundInfoCard` 最近开奖号码使用固定 `28px` 号码球且不换行，部分窄屏手机上 5 位开奖号码会和期号文本互相挤压，导致号码区域横向溢出。
- 实施内容：最近开奖卡片改为 scoped CSS 响应式布局，号码球使用 `clamp(20px-24px)`，号码容器允许换行并限制最大宽度；330px 以下极窄屏自动把开奖文本和号码球拆为上下两行；同步更新架构说明和前端组件规范。
- 验证结果：手机端 `npm run build`、手机端 `npm test` 和 `git diff --check` 均通过；手机端测试命令当前显示 0 个测试用例。

## 2026-06-08 01:49 HKT 聊天大厅头像展示修复

- 完成任务：修复用户上传头像后，聊天大厅仍显示文字头像的问题。
- 解决问题：用户头像只保存到了 `users.avatar_url`，聊天大厅消息模型、历史接口和实时事件都没有 `avatarUrl` 字段，手机端也只渲染用户名首字。
- 实施内容：`chat_hall_messages` 新增 `avatar_url` 字段和中文字段注释；`ChatHallMessage` 新增 `avatarUrl`，文本、红包、合买计划分享消息创建时写入当前头像；用户更新头像后同步刷新该用户聊天大厅历史消息头像；历史加载时用用户表当前头像兜底旧消息；手机端聊天大厅头像改为图片优先、加载失败回退文字头像；同步更新架构说明和前后端规范。
- 验证结果：`cargo fmt --manifest-path backend/Cargo.toml --check`、`cargo check --manifest-path backend/Cargo.toml`、`cargo test --manifest-path backend/Cargo.toml chat_hall -- --nocapture`、`cargo test --manifest-path backend/Cargo.toml realtime -- --nocapture`、`cargo test --manifest-path backend/Cargo.toml -- --nocapture`、手机端 `npm run build`、手机端 `npm test` 和 `git diff --check` 均通过；完整后端测试 251 条通过，手机端测试命令当前显示 0 个测试用例。

## 2026-06-08 02:03 HKT 手机端错误提示中文化

- 完成任务：把手机端展示给用户的错误提示统一改成中文。
- 解决问题：手机端公共错误函数会把后端 `bad request:`、`not found:`、`internal error:`、`financial account ... not found`、Axios `Network Error` 等英文内容原样弹出；旧下注组合和动态下注页还有直接读取 `response.data.detail/message` 的分支。
- 实施内容：新增 `mobile/src/utils/errorMessage.ts`，统一翻译后端错误前缀、常见英文业务错误、网络错误、超时和 HTTP 状态码；`mobile/src/api/user.ts` 的 `unwrapApiData/errorMessage` 接入统一工具；旧下注组合和动态下注页统一改用 `errorMessage`；用户端鉴权和会话解析的高频英文错误源文案改为中文；同步更新架构说明和前端规范。
- 验证结果：`cargo fmt --manifest-path backend/Cargo.toml --check`、`cargo check --manifest-path backend/Cargo.toml`、`cargo test --manifest-path backend/Cargo.toml access_repository -- --nocapture`、`cargo test --manifest-path backend/Cargo.toml -- --nocapture`、手机端 `npm run build`、手机端 `npm test` 和 `git diff --check` 均通过；完整后端测试 251 条通过，手机端测试命令当前显示 0 个测试用例；搜索确认移动端只有统一错误工具内部读取后端 `message/detail`。

## 2026-06-08 02:15 HKT 合买参与人数据保存失败修复

- 完成任务：修复用户端认购合买时返回 `internal error: 合买参与人数据保存失败` 的问题。
- 解决问题：合买仓储使用快照式保存，多个认购或结算回写请求并发时可能出现旧快照晚于新快照落库；落库失败时内存状态也可能已经变化；参与记录编号原先主要依赖毫秒时间，极端高并发下存在主键碰撞风险。
- 实施内容：合买仓储新增写操作锁，创建、认购、回滚、取消未满单、满单关联订单和结算回写全部串行化；落库失败时恢复内存快照；数据库保存前锁定 `group_buy_participants` 和 `group_buy_plans`，参与人保存失败日志输出真实数据库错误和关键上下文；用户端参与记录编号增加纳秒时间、参与序号和 64 位随机后缀。
- 验证结果：`cargo fmt --manifest-path backend/Cargo.toml --check`、`cargo check --manifest-path backend/Cargo.toml`、`cargo test --manifest-path backend/Cargo.toml group_buy -- --nocapture`、`cargo test --manifest-path backend/Cargo.toml -- --nocapture` 均通过；完整后端测试 252 条通过。

## 2026-06-08 14:48 HKT 合买注单显示参与金额

- 完成任务：让手机端注单记录中的合买订单显示当前用户实际参与金额。
- 解决问题：合买满单生成的真实投注订单金额是整单总额，参与人查看“我的注单”时如果直接展示 `amountMinor`，会误以为自己投注了整单金额。
- 实施内容：用户端 `GET /api/user/bet/orders` 对合买订单新增 `participationAmountMinor`，按当前用户在合买参与记录中的金额累加；手机端注单卡片和详情页对合买订单显示“参与金额”，并优先展示该字段；同步更新架构说明、后端 API 契约和前端组件规范。
- 验证结果：`cargo fmt --manifest-path backend/Cargo.toml --check`、`cargo check --manifest-path backend/Cargo.toml`、`cargo test --manifest-path backend/Cargo.toml user_visible_bet_orders_include_participated_group_buy_order -- --nocapture`、`cargo test --manifest-path backend/Cargo.toml -- --nocapture`、手机端 `npm run build`、手机端 `npm test` 和 `git diff --check` 均通过；完整后端测试 252 条通过，手机端测试命令当前显示 0 个测试用例。

## 2026-06-08 16:07 HKT 手机端下注提交 Loading

- 完成任务：给手机端下注页补充普通投注、提交购彩篮和发起合买过程中的页面级 loading。
- 解决问题：此前普通投注提交期间没有明确等待反馈，底部加入、编辑和提交入口仍可能被重复触发；合买也只有按钮文案变化，用户在网络慢时容易误以为没有响应。
- 实施内容：`DynamicBetPage.vue` 新增普通投注提交态和统一提交遮罩，提交期间显示“正在提交投注”或“正在发布合买”；`UnifiedBetBottomBar.vue` 新增 `submitting` 入参并在提交期间禁用加入购彩篮、编辑单据和提交按钮；同步更新架构说明和前端规范。
- 验证结果：手机端 `npm run build`、手机端 `npm test` 和 `git diff --check` 均通过；当前手机端测试命令显示 0 个测试用例。

## 2026-06-08 19:34 HKT 手机端 APK 启动闪退排查修复

- 完成任务：排查并修复手机端打包 APK 安装后启动异常的主要风险点。
- 解决问题：Tauri v2 APK 原先没有 `src-tauri/capabilities`，主窗口缺少 `store`、核心 IPC 和剪贴板文本能力声明；Android 构建目录里还存在指向旧工程路径的多 ABI 原生库断链，重新构建前可能让 APK 产物状态不稳定。
- 实施内容：新增 `mobile/src-tauri/capabilities/default.json`，授予主窗口 `core:default`、`store:default` 和文本剪贴板权限；手机端 `bootstrap()` 增加启动异常兜底错误页；执行 Android APK 构建刷新 `gen/schemas/capabilities.json`，并确认四个 ABI 原生库都来自当前 `bc/mobile/src-tauri/target` 路径。
- 验证结果：手机端 `npm run build`、`mobile/src-tauri cargo check`、`npm test`、`npx tauri android build --apk --ci` 均通过；新生成的 `app-universal-release-unsigned.apk` 包含 `arm64-v8a`、`armeabi-v7a`、`x86`、`x86_64` 四个原生库，构建输出的各 Android 目标 `capabilities.json` 均包含 `mobile-main` 能力。

## 2026-06-08 19:49 HKT 手机端 APK 构建脚本修正

- 完成任务：修正 `pnpm tauri:build:app` 找不到 APK 的问题。
- 解决问题：原脚本执行 `tauri build --bundles app`，产物是 macOS 桌面 `.app`，不会生成 Android APK，导致按脚本名打包后找不到 APK。
- 实施内容：`tauri:build:app` 和 `tauri:build:apk` 改为执行 `tauri android build --apk --debug --ci`，用于生成可安装调试 APK；新增 `tauri:build:apk:release` 用于 release APK 构建，新增 `tauri:build:desktop-app` 保留 macOS `.app` 打包能力；同步更新架构说明和前端质量规范。
- 验证结果：`pnpm build`、`pnpm test`、`mobile/src-tauri cargo check` 和 `pnpm tauri:build:app` 均通过；已确认 APK 生成在 `mobile/src-tauri/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk`，文件大小约 501M。

## 2026-06-08 20:15 HKT 手机端 APK 默认构建体积修正

- 完成任务：修正 `pnpm tauri:build:app` 默认生成 debug 大包的问题。
- 解决问题：debug APK 会打入四个未瘦身 ABI 原生库，导致包体约 501M；此前用户正常打包约 27M，对应的是 release APK。
- 实施内容：`tauri:build:app` 和 `tauri:build:apk` 改为执行 `tauri android build --apk --ci`，默认生成 release APK；新增 `tauri:build:apk:debug` 专门用于需要调试时生成 debug APK；同步更新架构说明和前端质量规范。
- 验证结果：`pnpm tauri:build:app` 已通过，生成 `mobile/src-tauri/gen/android/app/build/outputs/apk/universal/release/app-universal-release-unsigned.apk`；当前 universal release APK 文件大小约 42M，内部四个 release 原生库约 7.1M 到 11M。

## 2026-06-08 20:24 HKT 手机端 release APK 闪退风险修复

- 完成任务：处理手机端 release APK 打包后仍闪退的高概率构建风险，并尝试使用安卓虚拟机验证。
- 解决问题：当前 release 构建开启 R8/ProGuard 混淆裁剪，但 Tauri Android 桥接和插件没有完整 keep 规则，存在 release 启动闪退风险；同时原 release APK 为 unsigned，不能作为直接安装验证包。
- 实施内容：关闭 Android release 构建的 `isMinifyEnabled`，并让本地 release APK 使用 Android debug keystore 签名；正式发布前仍需替换正式签名。尝试启动本机已有 AVD，但两个虚拟机均提示缺失系统镜像，`sdkmanager` 重新安装镜像时卡在远端 manifest 下载，暂时无法完成虚拟机实测。
- 验证结果：`pnpm tauri:build:app` 已通过，生成 `mobile/src-tauri/gen/android/app/build/outputs/apk/universal/release/app-universal-release.apk`；`apksigner verify --verbose` 显示 `Verifies`，当前文件大小约 44M。

## 2026-06-08 21:28 HKT 手机端 Tauri 插件配置闪退修复

- 完成任务：修复手机端 release APK 在真机启动时因 Tauri store 插件配置解析失败导致的闪退。
- 解决问题：真机 logcat 显示 `PluginInitialization("store", "invalid type: map, expected unit")`，根因是 `tauri.conf.json` 中写了 `"store": {}` 空对象配置；当前 store 插件在 Android 侧期望无配置单元，空对象会被当成错误 map 解析。
- 实施内容：移除 `tauri.conf.json` 里的 `plugins.store` 和 `plugins.clipboard-manager` 空对象配置；插件继续通过 Rust `.plugin(...)` 注册，权限继续由 `capabilities/default.json` 控制；Tauri 原生入口保留中文 logcat 诊断钩子并补充中文注释；同步更新架构说明和前端质量规范。
- 验证结果：`mobile/src-tauri cargo check`、`pnpm tauri:build:app` 和 `apksigner verify --verbose` 均通过；已安装到真机 `V2301A` 并启动，8 秒后 `pidof com.hongfu.app` 仍有进程，logcat 没有 `PluginInitialization`、panic 或 SIGABRT，真机截屏确认应用进入登录页。

## 2026-06-08 21:36 HKT 手机端 APK HTTP 域名解析修复

- 完成任务：修复 APK 内 HTTP/WS 请求没有打到真实后端域名的问题。
- 解决问题：域名虽然存在于构建产物，但原判断把所有 `http` 协议都当成普通浏览器同源；Tauri Android 本地页面使用 `http://tauri.localhost` 时，手机端会走相对 `/api`，导致请求打到 `tauri.localhost/api`。
- 实施内容：`mobile/src/api/http.ts` 新增 API base 解析函数，优先使用 `VITE_API_BASE_URL` / `VITE_API_BASE`，识别 `tauri.localhost`、`tauri:`、`asset:` 后回落到 `https://bc.hippo-web3.cc.cd`；普通浏览器开发环境继续使用相对 `/api` 走 Vite 代理；同步更新架构说明和前端质量规范。
- 验证结果：`pnpm build` 通过且产物包含 `tauri.localhost` 判断和打包域名；`pnpm tauri:build:app` 通过；`apksigner verify --verbose` 通过；真机安装并清理应用数据后启动，登录页成功加载后端站点配置，显示图床 logo 和“祝您理性购彩、好运常伴。”，logcat 未发现启动崩溃或网络异常。

## 2026-06-08 21:50 HKT 手机端首页重复加载优化

- 完成任务：优化手机端首页缓存，避免每次通过底部导航返回首页都重新请求并显示加载态。
- 解决问题：`HomeView` 原先在每次 `onMounted` 时无条件请求用户余额、首页聚合接口和广告接口；底部导航切换回来会重新挂载页面，因此首页会重复网络请求和重新加载。
- 实施内容：新增 `mobile/src/stores/homepage.ts`，用 Pinia 缓存首页聚合数据和手机端广告；首页聚合缓存 30 秒，广告缓存 5 分钟；同类请求做去重；倒计时超过开奖时间或收到开奖 WebSocket 推送时仍通过 `force + silent` 绕过缓存静默刷新；后续余额统一迁移到用户数据缓存；同步更新架构说明和前端状态规范。
- 验证结果：手机端 `pnpm build`、`pnpm test` 和 `git diff --check` 均通过；当前测试命令显示 0 个测试用例。

## 2026-06-08 22:39 HKT 手机端登录注册一屏展示优化

- 完成任务：把手机端登录页和注册页调整为一个屏幕内完整展示。
- 解决问题：原页面品牌区、表单卡片和页脚间距偏大，注册态在小屏手机需要拖动；同时 `100dvh` 在本地浏览器验证时比实际视口多 8px，会造成轻微可滚动。
- 实施内容：`LoginView` 改为根节点满高布局，移除登录页页脚，收紧 Logo、标题、表单卡片、输入框和按钮尺寸；全局清除 `body` 默认外边距并让 `html/body/#app` 高度等于视口；同步更新架构说明和前端组件规范。
- 验证结果：本地浏览器按 `390x844` 和 `360x667` 验证登录态、注册态，`scrollHeight` 均等于 `innerHeight`，垂直溢出为 0；后续已继续执行手机端构建、测试和空白差异检查。

## 2026-06-08 22:55 HKT 手机端顶部安全区修复

- 完成任务：修复 APK 真机里顶部 Header 被 Android 状态栏遮挡的问题。
- 解决问题：首页、开奖、合买、我的、充值、提现等页面的顶部栏使用 `fixed top-0` 或 `sticky top-0`，Tauri Android 下会顶进状态栏，导致 Logo、站点名和钱包与时间/信号图标重叠。
- 实施内容：viewport 增加 `viewport-fit=cover`；新增公共 `mobile-safe-*` 安全区样式；品牌 Header、紧凑 Header、固定 Header 主内容偏移以及聊天/客服 scoped 顶部栏统一接入安全区变量；同步更新架构说明和前端组件规范。
- 验证结果：手机端 `pnpm build`、`pnpm test`、`git diff --check`、`pnpm tauri:build:app` 和 APK 签名校验均通过；真机 `V2301A` 覆盖安装并冷启动后无崩溃日志，首页截图确认 Header 已避开状态栏。

## 2026-06-08 23:16 HKT 手机端弹窗尺寸收紧

- 完成任务：收紧手机端底部弹层、详情弹层、聊天弹窗和表情面板尺寸。
- 解决问题：多个弹窗使用 80-90vh 或第三方默认 435px 高度，真机上视觉过大，容易遮住过多页面上下文。
- 实施内容：全局 Vant 弹窗圆角和对话框宽度收紧；下注购彩篮、玩法选择、开奖历史、注单详情、合买发起/详情、提现方式编辑、聊天红包/合买选择弹窗统一降低最大高度；客服和聊天大厅 `emoji-mart` 表情面板改为动态宽度、较小按钮和 300px 高度。
- 验证结果：手机端 `pnpm build`、`pnpm test`、`git diff --check`、`pnpm tauri:build:app` 和 APK 签名校验均通过；真机 `V2301A` 覆盖安装并冷启动后应用保持运行，logcat 未发现应用崩溃或前端运行时错误。

## 2026-06-08 23:30 HKT 手机端用户数据缓存统一

- 完成任务：把手机端高频用户数据统一放入 Pinia 缓存，并让页面按需刷新。
- 解决问题：首页、全部彩种、开奖页、我的账户、充值、提现、资金流水、安全中心、合买和下注页分别请求用户资料或资金列表，切换页面时容易重复 HTTP 请求和出现加载闪烁。
- 实施内容：新增 `mobileUserData` store，缓存当前用户资料、充值配置、充值订单、提现方式、提现申请和资金流水；余额统一读取 `profile.balance`；提交下注、合买、充值、提现、邮箱/密码和头像修改后强制刷新或写回缓存；`homepage` store 不再保留独立余额缓存。
- 验证结果：手机端 `pnpm build`、`pnpm test` 和 `git diff --check` 均通过；当前测试脚本显示 0 个测试用例。

## 2026-06-08 23:58 HKT 默认封盘提前秒数调整

- 完成任务：把开奖期号生成默认封盘提前量从 30 秒调整为 1 秒。
- 解决问题：未显式传入 `saleCloseLeadSeconds` 时仍会按旧的 30 秒默认值生成封盘时间，容易让 5 分钟彩种首页可下注倒计时看起来少 30 秒。
- 实施内容：后端默认常量 `DEFAULT_SALE_CLOSE_LEAD_SECONDS` 改为 1；后台手动创建期号和调度配置空状态默认值同步改为 1 秒；同步更新架构说明和后端 API 契约规范。已有数据库里的调度配置不会被常量自动覆盖，需要后台保存配置后生效。
- 验证结果：`cargo fmt --check`、`cargo check`、`cargo test`、管理后台 `npm run build` 和 `git diff --check` 均通过；后端测试 252 个用例全部通过。

## 2026-06-09 00:11 HKT 后台充值导出与记录清理

- 完成任务：后台新增用户充值记录导出、一键清除充值记录、一键清除提现记录、一键清除用户投注记录。
- 解决问题：后台此前只能分页查看充值、提现和投注记录，缺少导出留档与测试/维护场景下的批量清理能力。
- 实施内容：后端新增充值 CSV 导出接口和三类清理接口；充值清理不回滚余额和资金流水；提现存在待审核申请时拒绝清理；投注存在待开奖订单时拒绝清理；投注清理同步清除计奖派奖批次并保留流水号。后台财务管理页签新增导出/清理按钮，订单管理新增清除投注记录按钮，所有操作使用中文确认和中文结果提示。
- 验证结果：`cargo fmt --check`、`cargo check`、`cargo test clear`、完整 `cargo test`、管理后台 `npm run build` 和 `git diff --check` 均通过；新增清理测试 4 条通过，完整后端测试 256 条通过。管理后台构建仍有既有的大 chunk 提示，但构建成功。

## 2026-06-10 18:26 HKT 彩种销售状态开关优化

- 完成任务：后台彩种列表的“销售/停售”改为使用 Semi UI `Switch` 开关。
- 解决问题：原彩种列表使用普通按钮切换销售状态，视觉上不够像二元开关，容易让运营误判是状态展示而不是可操作控制。
- 实施内容：彩种列表销售列改为 `Switch` 加中文状态标签；切换时只让当前彩种开关显示加载状态；新增/编辑彩种侧栏里的销售状态也统一改为 `Switch`。
- 验证结果：管理后台 `npm run build` 和 `git diff --check` 均通过；构建仍有既有的大 chunk 提示。

## 2026-06-10 18:34 HKT 彩种列表销售状态分类与分页

- 完成任务：后台彩种列表新增按销售状态分类的 Tabs，并接入公共分页条。
- 解决问题：彩种数量增长后，销售中和已停售彩种混在同一长列表中不便扫描；原列表没有分页，运营维护时需要一次性浏览全部彩种。
- 实施内容：彩种列表增加“全部、销售中、已停售”Tabs；每个 Tab 显示对应数量；列表复用 `PageControls` 支持每页 10、20、50、100 条；切换 Tab 或每页条数时自动回到第 1 页；无数据时显示中文空状态。
- 验证结果：管理后台 `npm run build` 和 `git diff --check` 均通过；构建仍有既有的大 chunk 提示。

## 2026-06-10 18:37 HKT 开奖调度跳过明细展示收敛

- 完成任务：移除开奖管理中调度结果的逐条黄色跳过明细展示。
- 解决问题：停售彩种和历史期号被调度跳过时，页面会展开大量“彩种已停售，跳过自动任务”提示，影响运营查看真正的调度结果。
- 实施内容：手动自动任务结果增加“跳过 X 项”汇总；调度历史保留跳过数量指标；移除 `skippedIssues` 和 `skippedLotteries` 的逐条黄色列表；错误信息红色提示继续保留。
- 验证结果：管理后台 `npm run build` 和 `git diff --check` 均通过；构建仍有既有的大 chunk 提示。

## 2026-06-10 19:15 HKT 合买模块命名调整

- 完成任务：把后台运营入口中的“合买配置”改为“合买管理”。
- 解决问题：当前模块已经负责合买计划、认购进度、参与记录和状态维护，不只是彩种参数配置，继续叫“合买配置”容易让运营误解入口职责。
- 实施内容：后台合买页面标题改为“合买管理”；后台首页主要功能组描述改为“彩种、开奖、玩法与合买管理”；后台首页 `group-buy` 模块名称改为“合买管理”，说明文案改为“合买计划、认购进度和参与记录”。
- 验证结果：管理后台 `npm run build`、后端 `cargo fmt --check && cargo check` 和 `git diff --check` 均通过；构建仍有既有的大 chunk 提示。

## 2026-06-10 19:17 HKT Docker 单镜像日志输出收敛

- 完成任务：打包成 Docker 镜像后，容器日志只保留后端服务输出为主，Nginx 日志不再进入 Docker 日志。
- 解决问题：官方 Nginx 镜像默认把访问日志输出到 stdout、错误日志输出到 stderr，部署后 `docker logs` 容易被静态资源、健康检查和代理请求刷屏，不便观察后端业务日志。
- 实施内容：`docker/nginx.conf` 增加 `access_log off;` 并把 `error_log` 指向 `/dev/null`；同步更新容器部署规范、部署说明和架构说明。
- 验证结果：`sh -n docker/entrypoint.sh` 和 `git diff --check` 均通过；当前本机 Docker daemon 未运行，`docker run ... nginx -t` 与 `docker build -t bc-platform:latest .` 暂无法执行，报错为无法连接 Docker daemon。

## 2026-06-10 21:53 HKT 下注页开奖中卡住修复

- 完成任务：修复手机端下注页到达封盘/开奖时间后可能一直停在“开奖中”，不自动进入下一期的问题，并用 `魔力分分彩`、`腾讯分分彩` 在本地服务联调验证。
- 解决问题：根因不是单点前端显示问题，而是多个链路叠加：下注页配置曾把已过封盘时间的 `open` 期继续当成 `selling`；首页当前期曾在历史 `closed` 期中取最早旧期；常驻调度快路径曾被 API 补期、合买流单退款、资金/期号全量持久化等慢操作拖慢。
- 实施内容：下注页和首页当前期选择都按“未封盘可售期优先、最新待开奖期次之、最近已开奖兜底”处理；开奖期号创建、封盘、开奖、取消改为单条 `draw_issues` upsert；调度器拆成开盘快阶段和后台慢阶段，快阶段只处理封盘、补期和开盘推送，慢阶段再处理开奖结算、流单退款和机器人；慢阶段未结束时不阻塞下一轮快阶段。
- 验证结果：本地后端连接 `postgres://root:***@192.168.2.3:15432/postgres` 后启动，`/api/lottery/home` 在 `21:46 -> 21:47` 跨期后返回 `ssc60=20260610214759/selling`、`txffc=202606101308/selling`；登录临时用户后，`/api/user/bet/page-config/ssc60` 返回 `20260610215259/selling`，`/api/user/bet/page-config/txffc` 返回 `202606101313/selling`。后端 `cargo check`、自动化流单退款单测和调度器测试已通过，后续继续跑完整测试与手机端构建。

## 2026-06-10 22:46 HKT 后台内存缓存刷新按钮

- 完成任务：后台系统设置新增“刷新内存缓存”维护按钮，后端新增 `POST /api/admin/system-settings/cache/reload` 接口。
- 解决问题：手动清空或直接修改 PostgreSQL 业务表后，运行中的后端快照型仓储仍保留启动时加载的内存数据，后台页面会继续看到旧数据，甚至后续写入可能把旧快照重新保存回数据库。
- 实施内容：用户权限、广告、聊天大厅、开奖期号与控制、资金、合买、邀请、订单、返利、充值、机器人、调度、客服、提现等仓储新增数据库重载入口；彩种配置标记为数据库直读；系统设置页按钮会确认后执行刷新并显示已刷新模块、数据库直读模块和跳过模块。
- 验证结果：已完成代码接入，后续继续执行后端格式化、检查、测试和后台构建验证。

## 2026-06-11 00:52 HKT 合买计划列表显示创建时间

- 完成任务：后台合买计划列表新增“创建时间”列。
- 解决问题：合买列表此前只显示计划、彩种期号、成单、进度和状态，运营无法在列表层直接核对计划生成时间。
- 实施内容：后端 `GroupBuyPlanSummary` 摘要增加 `createdAt` 字段；后台类型、列表本地摘要转换和表格列同步展示创建时间；接口契约和前端规范同步记录。
- 验证结果：已完成代码接入，后续继续执行后端格式化、检查、测试和后台构建验证。

## 2026-06-11 01:05 HKT 合买管理移除统计卡片

- 完成任务：移除后台合买管理顶部四个统计卡片。
- 解决问题：合买管理主页面需要优先服务列表扫描，`grid gap-3 sm:grid-cols-2 xl:grid-cols-4` 统计区占用页面空间且当前不需要。
- 实施内容：删除合买计划、进行中、已满单、已认购四个 `MetricCard`，并清理对应的 `totals` 计算和 `groupBuyTotals` 辅助函数。
- 验证结果：已完成代码接入，后续继续执行后台构建和空白差异检查。

## 2026-06-11 01:10 HKT 财务管理列表时间倒序

- 完成任务：财务管理的充值订单、提现申请和资金流水列表统一改为按创建时间倒序展示。
- 解决问题：后台财务列表分页前没有显式保证时间倒序，充值和提现记录可能受仓储或数据库读取顺序影响，导致第一页不一定是最新业务记录。
- 实施内容：后端财务列表接口在分页前执行统一时间排序；同一秒内按业务编号倒序兜底；排序兼容标准时间和历史 `unix:秒` 标签；充值记录 CSV 导出同步保持最新在前。
- 验证结果：后端 `cargo fmt --check`、`cargo check`、财务排序定向单测和 `git diff --check` 均通过。

## 2026-06-11 01:34 HKT 后台用户列表上级代理显示用户名

- 完成任务：后台用户维护列表的“上级代理”列新增代理用户名展示。
- 解决问题：用户列表此前只显示 `agentId`，运营需要额外记忆或查询代理 ID 才能识别上级代理。
- 实施内容：后端后台用户响应新增 `agentUsername` 派生字段，列表、详情、创建、更新和状态变更响应统一返回；后台类型和用户表格同步展示代理用户名，并保留代理 ID。
- 验证结果：后端 `cargo fmt --check`、`cargo check`、上级代理用户名定向单测、后台 `npm run build` 和 `git diff --check` 均通过。

## 2026-06-11 04:00 HKT 资金账户最新用户优先

- 完成任务：财务管理“资金账户”列表改为按用户编号倒序展示。
- 解决问题：资金账户分页此前依赖仓储返回顺序，第一页可能优先展示旧用户，不符合财务人员优先查看最新用户的习惯。
- 实施内容：后端 `GET /api/admin/financial-accounts` 在分页前按用户编号倒序排序；机器人过滤仍先执行，过滤后的账户再排序分页；补充后端排序单测和接口契约说明。
- 验证结果：后端 `cargo fmt --check`、`cargo check`、资金账户排序定向单测和 `git diff --check` 均通过。

## 2026-06-11 04:05 HKT 用户权限管理移除统计卡片

- 完成任务：移除用户权限管理顶部四个统计卡片。
- 解决问题：用户管理、管理员管理和角色权限主页面需要优先服务列表扫描，`grid gap-3 sm:grid-cols-2 xl:grid-cols-4` 统计区占用页面空间且当前不需要。
- 实施内容：删除 `AccessManagementPage` 中用户总数、活跃用户、后台账号、角色数量四个 `MetricCard`；清理 `totals` 派生值和 `accessTotals` 辅助函数；同步更新前端组件规范和架构说明。
- 验证结果：后台 `npm run build` 和 `git diff --check` 均通过。

## 2026-06-12 16:09 HKT 手机端注单记录下注时间展示

- 完成任务：手机端注单记录列表补强下注时间展示。
- 解决问题：用户在注单记录页需要直接核对每笔下注时间，不应只能进入注单详情后查看投注时间。
- 实施内容：注单卡片在期号下方显示“下注时间”；订单接口适配层兼容 `createdAt` 和 `created_at`；注单详情投注时间同步使用兜底字段。
- 验证结果：手机端 `pnpm build` 和 `git diff --check` 均通过。

## 2026-06-12 18:16 HKT 用户列表关联查看注单与流水

- 完成任务：后台用户列表新增“注单”和“流水”两个快捷入口。
- 解决问题：运营查看某个用户的投注订单或资金流水时，需要先记住用户 ID 再切换到订单/财务页面，操作链路过长。
- 实施内容：后台订单和资金流水接口新增 `userId` 查询过滤；用户列表操作列新增“注单”“流水”按钮；点击后分别跳转订单管理或财务管理资金流水 Tab，并带上当前用户筛选标签；目标页面支持清除筛选恢复全量列表。
- 验证结果：后端 `cargo fmt --check`、用户过滤/分页相关定向单测、后台 `npm run build` 和 `git diff --check` 均通过；后台构建仍有既有的大 chunk 提示。

## 2026-06-12 19:30 HKT 资金账户行内手动调账

- 完成任务：后台财务管理的手动调账入口改到资金账户列表每行最右侧，通过“调账”按钮打开 `SideSheet`。
- 解决问题：原来的手动调账卡片常驻在账户列表右侧，并允许直接手填用户 ID，容易占用列表扫描空间，也增加误调到其他用户的风险。
- 实施内容：资金账户表格新增“操作”列；调账抽屉展示当前账户用户、用户 ID、可用余额和冻结余额；用户 ID 输入框改为禁用，只能对当前行账户调账；调账成功后自动关闭抽屉并刷新汇总。
- 验证结果：后台 `npm run build` 和 `git diff --check` 均通过；后台构建仍有既有的大 chunk 提示。

## 2026-06-12 19:53 HKT 后台代理返利统计与提现处理

- 完成任务：后台返利模块升级为“返利管理”，新增代理返利统计、下级返利明细查看和代理返利提现处理能力。
- 解决问题：此前后台只能维护邀请关系和返利策略，无法按代理统计已产生返利，也无法查看代理下级每一笔返利记录，更缺少对已入账返利金额进行提现处理的专用入口。
- 实施内容：后端新增返利统计、代理返利明细和返利提现处理接口；资金流水新增 `agentRebateWithdrawal` 类型；后台返利页使用 `Tabs` 区分“返利统计”和“策略配置”；代理详情通过 `SideSheet` 展示汇总、提现处理表单和明细分页表格；OpenAPI、数据库字段注释、后台/手机端资金流水中文文案同步更新。
- 验证结果：后端 `cargo check`、`cargo fmt --check`、代理返利统计定向测试、资金返利提现定向测试、后台 `npm run build`、手机端 `pnpm build` 和 `git diff --check` 均通过；后台构建仍有既有的大 chunk 提示。

## 2026-06-12 22:02 HKT 用户合买机器人兜底满单

- 完成任务：修复用户发起合买不能稳定百分百满单的问题。
- 解决问题：常驻调度器此前先封盘、再慢阶段退款/开奖，最后才执行合买机器人；如果机器人没有刚好在最后 15 秒补满，用户发起的合买可能先被封盘流单退款，机器人后续没有机会补单。
- 实施内容：调度器每轮封盘前先执行一次合买机器人兜底；慢阶段顺序改为机器人先尝试补单，再处理封盘流单退款和开奖结算；用户发起的非机器人合买进入兜底策略，封盘前最终窗口或封盘点到开奖前会直接补齐剩余金额；机器人自己发起的合买仍保持分阶段节奏，不会变成创建后一次性满单。
- 验证结果：`cargo fmt --check`、`cargo check`、`cargo test -- --nocapture`、`scheduler_fills_user_group_buy_before_closing_issue`、`robot_run_fills_existing_non_robot_group_buy_plan_with_rhythm` 和 `git diff --check` 均通过；后端全量 296 个测试成功。

## 2026-06-12 22:05 HKT 系统概览移除最近订单

- 完成任务：后台系统概览页面移除“最近订单”区块。
- 解决问题：系统概览首屏需要保持轻量扫描，最近订单明细已经由“订单管理”承载，继续在概览页展示会占用页面空间。
- 实施内容：删除系统概览中的最近订单卡片和表格展示，清理不再使用的赔率格式化引用；后端 `recentOrders` 字段暂保留用于接口兼容，不在页面渲染。
- 验证结果：后台 `npm run build` 和 `git diff --check` 均通过；后台构建仍有既有的大 chunk 提示。

## 2026-06-13 00:11 HKT 手机端首页彩种图片缩小

- 完成任务：进一步缩小手机端首页彩种卡片的彩种图片尺寸。
- 解决问题：首页彩种图片在主推卡、二级卡和普通分类卡中仍偏大，会挤占名称、状态、期号和开奖号码的扫描空间。
- 实施内容：主推彩种图片从约 48px 收到约 40px；高频极速二级卡从约 32px 收到约 28px；普通分类卡 logo 容器从约 50px 收到约 42px，小屏下约 38px。
- 验证结果：手机端 `pnpm build` 和 `git diff --check` 均通过。

## 2026-06-13 00:28 HKT 手机端首页彩种图片再次缩小

- 完成任务：按反馈把手机端首页彩种图片再缩小一档。
- 解决问题：第一次缩小后，首页彩种图片在部分卡片里仍略抢占视觉空间，需要继续给彩种名称、状态和开奖号码让位。
- 实施内容：主推彩种图片从约 40px 收到约 36px；高频极速二级卡从约 28px 收到约 24px；普通分类卡 logo 容器从约 42px 收到约 36px，小屏下约 32px。
- 验证结果：手机端 `pnpm build` 和 `git diff --check` 均通过。

## 2026-06-13 00:31 HKT 手机端首页期号文本压缩

- 完成任务：缩小手机端首页普通分类彩种卡片的期号文本，并强制单行展示。
- 解决问题：`group-lottery-card__issue` 在期号较长时可能换行，占用卡片高度并影响开奖号码区域扫描。
- 实施内容：期号文本字号从 `0.62rem` 调整为 `0.56rem`，增加单行、省略号和最大宽度约束。
- 验证结果：手机端 `pnpm build` 和 `git diff --check` 均通过。

## 2026-06-13 00:39 HKT 合买大厅发起人脱敏规则调整

- 完成任务：调整用户端合买计划发起人展示名脱敏规则。
- 解决问题：原规则保留首尾字符，中间打星；现在需要类似“爱情819281”展示为“爱情81****”，即保留前 4 个字符、后续全部替换为星号。
- 实施内容：后端 `mask_group_buy_initiator_display` 改为前 4 字符保留规则；长度不超过 4 的昵称保持原样，空昵称仍显示“会员”；同步更新普通用户合买和边界测试。
- 验证结果：后端 `cargo fmt --check`、`cargo check`、`cargo test group_buy_initiator -- --nocapture`、`cargo test user_group_buy_plan_masks_normal_initiator_display -- --nocapture`，手机端 `pnpm build`，以及 `git diff --check` 均通过。

## 2026-06-13 01:45 HKT 手机端首页倒计时改为距开奖

- 完成任务：把手机端首页彩种卡片主倒计时从“距封盘”调整为“距开奖”。
- 解决问题：澳洲幸运5等 5 分钟 API 彩种在开奖结果延迟返回后，首页按封盘时间显示会看起来只剩 4:30-4:40，容易误解为周期被缩短。
- 实施内容：首页倒计时优先使用 `nextDrawTime`，展示标签为“开奖”；`draw_result` 推送不再覆盖已有 `issue_opened` 写入的下一期官方开奖时间；同步更新前端规范和架构说明。
- 验证结果：手机端 `pnpm build` 和 `git diff --check` 均通过。

## 2026-06-13 03:36 HKT 手机端首页彩种卡片紧凑化

- 完成任务：进一步压缩手机端首页彩种卡片和分组间距。
- 解决问题：首页彩种卡片高度、内边距和区块间距偏大，单屏可见彩种数量不足。
- 实施内容：主推卡减少内边距、结果面板、按钮和号码球尺寸；高频二级卡降低最小高度；普通分类卡收紧 padding、logo、标题、期号、号码球、阴影和内部间距；首页主内容、推荐区和分组区外部间距同步收紧；前端组件规范和架构说明同步记录紧凑卡片要求。
- 验证结果：手机端 `pnpm build` 和 `git diff --check` 均通过。

## 2026-06-13 04:14 HKT 平台开奖默认期号改为日期加 4 位序号

- 完成任务：把系统平台开奖默认期号格式改为 `yyyyMMdd` 加 4 位每日递增序号。
- 解决问题：原默认 `{yyyy}{MM}{dd}{HH}{mm}{ss}` 会直接使用开奖时间作为期号，不符合现在要求的 `202606130001` 这类默认期号规则。
- 实施内容：后端默认 `issueFormat` 改为 `{date}{seq4}`；期号生成器新增 `{seq4}` 变量并按开奖日期每日递增；数据库迁移更新 `lotteries.issue_format` 默认值并把老默认配置迁移为新默认；后台彩种表单提示、架构说明和 Trellis 规范同步更新。
- 验证结果：后端 `cargo fmt --check`、`cargo check`、`cargo test draw_generation -- --nocapture`、后台 `npm run build`、手机端 `pnpm build` 和 `git diff --check` 均通过；后台构建仍有既有的大 chunk 提示。

## 2026-06-13 05:03 HKT 修复调度器测试期号断言

- 完成任务：修复 CI 中调度器相关测试仍断言旧平台期号格式的问题。
- 解决问题：平台开奖默认期号已改成 `{date}{seq4}` 后，`scheduler_opens_next_issue_after_current_issue_closes` 和 `scheduler_runs_due_automation_before_generating_future_issues` 仍期待旧的 `yyyyMMddHHmmss` 期号，导致后端全量测试失败。
- 实施内容：将调度器测试里的平台开奖默认期号样例和断言更新为 `202606020001`、`202606020002`，继续覆盖调度器先执行到期开奖/封盘，再补齐未来开盘期号的行为。
- 验证结果：两个失败的 scheduler 定向测试、后端 `cargo fmt --check`、`cargo check`、全量 `cargo test -- --nocapture` 和 `git diff --check` 均通过；后端全量 296 个测试成功。

## 2026-06-13 07:18 HKT 手机端 Notify 磨砂玻璃样式优化

- 完成任务：优化手机端 Vant `Notify` 全局提示样式。
- 解决问题：原来的 `van-notify` 是贴顶的实心红色横条，视觉偏重，也容易和顶部安全区、Header 贴得太近。
- 实施内容：将通知改为安全区下方的浮层圆角卡片；增加半透明渐变、磨砂模糊、细边框、高光层和柔和阴影；成功、警告、错误通知保留轻微色彩差异。
- 验证结果：手机端 `pnpm build` 和 `git diff --check` 均通过；本地静态视觉预览因内置浏览器安全策略拦截 `data:` 页面，未绕过该限制。

## 2026-06-13 07:22 HKT 手机端首页普通彩种卡片 padding 调整

- 完成任务：把 `.group-lottery-card` 的 padding 调整为 `0.28rem 0.58rem`。
- 解决问题：普通分类彩种卡片仍需要进一步收紧内边距，让首页同屏展示更多彩种。
- 实施内容：基础样式和小屏媒体查询中的 `.group-lottery-card` padding 统一为上下 `0.28rem`、左右 `0.58rem`。
- 验证结果：手机端 `pnpm build` 和 `git diff --check` 均通过。

## 2026-06-13 22:24 HKT 手机端代理中心与后台代理审核

- 完成任务：新增手机端代理中心申请能力，并在后台返利管理中提供代理申请审核。
- 解决问题：此前只有代理能使用邀请中心，普通玩家没有正式申请成为代理的入口，后台也没有审核普通玩家升级为代理的闭环。
- 实施内容：后端新增代理申请领域模型、仓储、数据库表和用户端/后台接口；后台“返利管理”新增“代理申请”Tab 和审核 `SideSheet`；手机端 `/agent-center` 支持普通玩家提交申请、查看待审核/驳回状态，代理继续查看邀请码和直属下级返利数据；OpenAPI 和架构说明同步更新。
- 验证结果：后端 `cargo fmt`、`cargo fmt --check`、`cargo check`、`cargo test agent_application -- --nocapture`、`cargo test openapi_document_contains_core_paths -- --nocapture` 和全量 `cargo test -- --nocapture` 均通过（299 个测试成功）；后台 `npm run build`、手机端 `pnpm build` 和 `git diff --check` 均通过；后台构建仍有既有的大 chunk 提示。

## 2026-06-13 22:44 HKT 后台用户列表排序栏不换行

- 完成任务：调整后台用户列表排序工具栏的布局。
- 解决问题：用户列表中排序字段、排序方向等控件使用 `flex-wrap` 时会在窄宽度下换行，影响运营扫描和操作连贯性。
- 实施内容：将排序栏内部容器改为 `flex-nowrap`，增加 `whitespace-nowrap` 和横向溢出滚动，保证控件保持单行展示。
- 验证结果：后台 `npm run build` 和 `git diff --check` 均通过；后台构建仍有既有的大 chunk 提示。

## 2026-06-13 22:33 HKT 手机端首页普通彩种卡片参考草图改版

- 完成任务：按用户提供的手绘结构调整手机端首页普通分类彩种卡片。
- 解决问题：普通分类彩种卡片需要更接近“左侧彩种信息、右侧 Logo、底部开奖号码”的扫描结构，减少信息堆叠带来的拥挤感。
- 实施内容：`HomeDrawCard.vue` 普通卡片改为 CSS Grid 两列布局；彩种名和状态同行、期号单独一行、Logo 右侧靠下、开奖号码左下单行滚动展示；主推卡和高频极速二级卡不变。
- 验证结果：手机端 `pnpm build` 通过，`git diff --check` 通过；尝试用内置浏览器打开本地 Vite 预览时浏览器插件返回不可用，未完成可视截图验证。

## 2026-06-14 00:09 HKT 手机端开奖页与我的注单入口拆分

- 完成任务：把手机端“我的注单”入口移动到“我的”页面，并让“开奖”页只展示最新开奖。
- 解决问题：原来“开奖”页用 Tab 同时承载“开奖结果”和“我的注单”，个人订单入口放在开奖页内不够清晰。
- 实施内容：`/history` 改为只渲染最新开奖；`/orders` 继续渲染我的注单但底部导航高亮“我的”；个人中心账户功能区新增“我的注单”入口；移除已废弃的 `HistoryTabs` 组件。
- 验证结果：手机端 `pnpm build`、`pnpm test` 和 `git diff --check` 均通过；当前手机端测试脚本显示 0 个测试用例。

## 2026-06-14 00:23 HKT 管理端侧边栏常用功能优先

- 完成任务：按运营常用顺序重排管理端侧边栏。
- 解决问题：原侧边栏按后端模块分组展示，高频入口分散在不同分组里，客服、财务、用户、合买、订单和开奖处理时需要来回查找。
- 实施内容：后台导航组装时新增常用模块排序规则，依次展示在线客服、财务管理、用户管理、合买管理、订单管理、彩种控制台、计奖派奖、邀请管理、返利管理；未列入常用的模块统一放入“不常用”分组，并自动加两位序号。
- 验证结果：后台 `npm run build` 和 `git diff --check` 均通过；后台构建仍有既有的大 chunk 提示。

## 2026-06-14 00:42 HKT 手机端在线客服发送图片

- 完成任务：补齐手机端在线客服发送图片能力。
- 解决问题：后台客服可以发送图片，手机端此前只能查看图片和发送文字，用户无法在客服会话中上传充值凭证或问题截图。
- 实施内容：后端用户客服回复支持 `messageType=image`、`imageUrl` 和可选说明；新增 `/api/user/support/images/upload` 图床代理上传接口；手机端 `/support` 输入栏新增图片按钮，选择图片后上传并自动发送图片消息，发送过程中禁用输入和按钮；同步把客服回复空内容校验文案改为中文提示。
- 验证结果：`cargo fmt --manifest-path backend/Cargo.toml --check`、`cargo check --manifest-path backend/Cargo.toml`、`cargo test --manifest-path backend/Cargo.toml support_repository_allows_user_image_reply -- --nocapture`、`cargo test --manifest-path backend/Cargo.toml support_repository_allows_user_to_continue_owned_conversation -- --nocapture`、`cargo test --manifest-path backend/Cargo.toml openapi_document_contains_core_paths -- --nocapture`、手机端 `pnpm build`、手机端 `pnpm test` 和 `git diff --check` 均通过；当前手机端测试脚本显示 0 个测试用例。

## 2026-06-14 01:10 HKT 后台详情抽屉与彩种控制开关优化

- 完成任务：优化合买计划详情、控制开奖号码和代理返利详情抽屉，并增加彩种开奖号码控制开关。
- 解决问题：原三个 `SideSheet` 宽度偏窄，表格和明细内容横向拥挤；所有彩种默认都展示控制按钮，无法区分不需要控开的彩种；代理返利详情缺少下级提现汇总。
- 实施内容：三个详情 `SideSheet` 宽度改为 `80%`；彩种配置新增 `drawControlEnabled`/`draw_control_enabled` 并持久化到数据库，后台彩种表单可配置，控制台按该开关隐藏“控制”按钮；后端拒绝未开启控制的彩种启用控制号码，自动开奖也不会用历史控制号覆盖；代理返利统计和明细增加下级已通过提现金额展示；同步更新 OpenAPI 路径描述。
- 验证结果：`cargo fmt --manifest-path backend/Cargo.toml`、`cargo check --manifest-path backend/Cargo.toml`、`cargo test --manifest-path backend/Cargo.toml agent_rebate -- --nocapture`、`cargo test --manifest-path backend/Cargo.toml repository_draws_api_issue_with_prefetched_number_without_refetching_source -- --nocapture`、`cargo test --manifest-path backend/Cargo.toml repository_save_draw_control -- --nocapture`、`cargo test --manifest-path backend/Cargo.toml openapi_document_contains_core_paths -- --nocapture`、后台 `npm run build` 和 `git diff --check` 均通过；后台构建仍有既有的大 chunk 提示。

## 2026-06-14 04:01 HKT 手机端注单记录独立页面壳

- 完成任务：调整手机端 `/orders` 注单记录页的页面外壳。
- 解决问题：注单记录从“我的”进入后仍显示底部导航，页面层级和代理中心等子页不一致，列表可用高度也被底部导航占用。
- 实施内容：`LayoutView.vue` 对 `/orders` 隐藏底部导航；`HistoryView.vue` 在订单模式下使用代理中心同款紧凑安全区 Header，提供返回、标题“我的注单”和刷新按钮；开奖模式继续保留原品牌 Header。
- 验证结果：手机端 `pnpm build` 和 `git diff --check` 均通过；尝试用内置浏览器打开 `/orders` 做可视验证时，因本地页面需要登录态且浏览器安全策略禁止通过 `javascript:` 注入 localStorage，未完成可视截图验证。

## 2026-06-14 04:06 HKT 手机端首页彩种卡片标题字号调优

- 完成任务：调整手机端首页普通彩种卡片标题字号。
- 解决问题：`group-lottery-card__copy h5` 标题字号仍偏大，在紧凑卡片中容易挤占状态和期号空间。
- 实施内容：将 `mobile/src/components/lottery/HomeDrawCard.vue` 中 `.group-lottery-card__copy h5` 的默认字号和小屏覆盖字号统一调整为 `0.66rem`。
- 验证结果：手机端 `pnpm build` 和 `git diff --check` 均通过。

## 2026-06-14 04:11 HKT 手机端注单卡片展示开奖号码

- 完成任务：在手机端 `/orders` 注单记录卡片中展示开奖号码。
- 解决问题：用户此前只能进入注单详情查看开奖号码，列表页无法直接核对每张注单的开奖结果。
- 实施内容：`BetOrderCard.vue` 复用 `orderDrawNumbers`，在投注号码下方新增“开奖号码”区域；有开奖号码时显示紧凑圆形号码球，待开奖显示“待开奖”，缺失数据显示“暂无开奖数据”；`api/bet.ts` 兼容 `drawNumber`、`draw_number`、`draw_result` 和 `result` 字段并统一归一化。
- 验证结果：手机端 `pnpm build` 和 `git diff --check` 均通过。

## 2026-06-15 23:56 HKT 合买机器人开奖前补满策略

- 完成任务：为合买机器人新增“开奖前补满”策略，可在后台配置距离开奖多少秒时一次性补满。
- 解决问题：原来只有阶段性满单策略，无法满足“平时不分阶段跟单，只在开奖前指定时间补满”的运营需求。
- 实施内容：后端机器人配置新增 `groupBuyFillStrategy` 和 `groupBuyFillBeforeDrawSeconds`；数据库新增对应持久化字段和中文注释；合买机器人执行逻辑新增 `beforeDraw` 策略，进入开奖前配置窗口后直接补满；后台机器人配置列表和 `SideSheet` 支持查看、编辑补满策略与秒数；同步更新接口契约和架构说明。
- 验证结果：后端 `cargo fmt --manifest-path backend/Cargo.toml`、`cargo check --manifest-path backend/Cargo.toml`、`cargo test --manifest-path backend/Cargo.toml group_buy_robot -- --nocapture`、`cargo test --manifest-path backend/Cargo.toml robot_repository -- --nocapture`、`cargo test --manifest-path backend/Cargo.toml openapi_document_contains_core_paths -- --nocapture`、后端全量 `cargo test --manifest-path backend/Cargo.toml`、后台 `npm run build` 和 `git diff --check` 均通过；后台构建仍有既有的大 chunk 提示。

## 2026-06-16 00:00 HKT 手机端普通彩种卡片状态标签与图片调整

- 完成任务：去掉手机端首页普通彩种卡片中的状态 pill，并放大彩种图片。
- 解决问题：普通卡片标题行里的 `lottery-state-pill group-lottery-card__state` 占用横向空间，影响彩种名称扫描；图片偏小，彩种识别度不够。
- 实施内容：移除普通卡片标题行状态标签和对应 `.group-lottery-card__state` 样式；普通卡片 Logo 容器从 `2.05rem` 放大到 `2.38rem`，小屏从 `1.7rem` 放大到 `2rem`；主推卡和高频极速二级卡不变。
- 验证结果：手机端 `pnpm build` 和 `git diff --check` 均通过。

## 2026-06-16 01:57 HKT 后台 APP 安装包上传与手机端更新检查

- 完成任务：新增后台 Android APK、iOS IPA 安装包上传和更新策略配置，并接入手机端启动更新检查。
- 解决问题：此前后台只能配置手机端 Logo 和介绍，无法维护 APP 安装包下载地址、最新版本、强制更新和更新说明；手机端启动后也没有统一的更新检查入口。
- 实施内容：后端系统设置新增 Android/iOS 更新配置种子和迁移；新增 `/api/admin/app-packages/upload` 安装包上传接口与 `/api/user/mobile/app-update` 公开检查接口；OpenAPI 同步记录；后台“手机端设置”新增 APP 更新配置区块，支持上传安装包并回填链接；手机端启动后异步检查更新并按强制/可选策略展示中文更新弹窗。
- 验证结果：后端 `cargo fmt --manifest-path backend/Cargo.toml`、`cargo fmt --manifest-path backend/Cargo.toml --check`、`cargo check --manifest-path backend/Cargo.toml`、`cargo test --manifest-path backend/Cargo.toml mobile_app_update -- --nocapture`、`cargo test --manifest-path backend/Cargo.toml version_compare_handles_equal_and_newer_versions -- --nocapture`、`cargo test --manifest-path backend/Cargo.toml openapi_document_contains_core_paths -- --nocapture` 和全量 `cargo test --manifest-path backend/Cargo.toml` 均通过（319 个测试成功）；后台 `npm run build`、手机端 `pnpm build`、手机端 `pnpm test` 和 `git diff --check` 均通过；后台构建仍有既有的大 chunk 提示，手机端测试脚本当前显示 0 个测试用例。

## 2026-06-16 04:19 HKT 手机端首页分类 Tabs 均分布局

- 完成任务：调整手机端首页彩种分类 Tabs 的导航分布。
- 解决问题：首页分类 Tabs 使用默认导航布局时，多个分类在胶囊容器内的横向分布不够均匀。
- 实施内容：将 `HomeView.vue` 中 `.home-category-tabs :deep(.van-tabs__nav)` 设置为全宽 `flex` 布局，并使用 `justify-content: space-around` 让分类项横向均分。
- 验证结果：手机端 `pnpm build` 和 `git diff --check` 均通过。

## 2026-06-17 01:04 HKT 用户注册来源与注册地审计

- 完成任务：用户注册时记录注册 IP、粗粒度注册地和来源，并在后台用户列表/用户维护中展示。
- 解决问题：此前用户注册只保存账号、邮箱、QQ、邀请码等资料，后台无法审计用户注册来源，也无法区分注册来源来自请求 IP、客户端定位或未知来源。
- 实施内容：后端 `UserSummary` 增加 `registrationLocation`；注册接口从常见反代请求头提取客户端 IP；访问控制仓储创建、读取、保存用户时持久化注册地字段；数据库新增用户注册来源字段和中文注释；手机端注册提交时尝试获取定位授权并上报粗粒度来源；后台用户列表新增“注册地”列，用户维护抽屉新增只读注册来源信息。
- 验证结果：后端 `cargo fmt --manifest-path backend/Cargo.toml --check`、`cargo check --manifest-path backend/Cargo.toml`、`cargo test --manifest-path backend/Cargo.toml access_repository_registers_user_by_username_or_email -- --nocapture` 和后端全量 `cargo test --manifest-path backend/Cargo.toml` 均通过（333 个测试成功）；后台 `npm run build`、手机端 `pnpm build`、手机端 `pnpm test` 和 `git diff --check` 均通过；后台构建仍有既有的大 chunk 提示，手机端测试脚本当前显示 0 个测试用例。

## 2026-06-17 01:22 HKT 系统设置一键清除聊天大厅消息

- 完成任务：把聊天大厅历史消息清理入口放到后台系统设置中，提供一键清除能力，不展示聊天大厅列表。
- 解决问题：运营只需要清空公共聊天大厅展示记录，后台此前没有对应维护按钮；如果只手动清库，还可能与后端内存快照不一致。
- 实施内容：后端新增 `DELETE /api/admin/system-settings/chat-hall/messages/clear`，清空聊天大厅消息、红包展示记录和红包领取展示记录，保留后续消息序号且不回滚资金流水；清空成功后广播 `chat_hall.messages_cleared` 实时事件；后台系统设置页新增确认、loading 和清除条数反馈；手机端聊天大厅收到清空事件后同步清空本地消息和未读提示；OpenAPI 和架构说明同步更新。
- 验证结果：`cargo fmt --manifest-path backend/Cargo.toml --check`、`cargo check --manifest-path backend/Cargo.toml`、`cargo test --manifest-path backend/Cargo.toml chat_hall -- --nocapture`、`cargo test --manifest-path backend/Cargo.toml openapi_document_contains_core_paths -- --nocapture`、后台 `npm run build`、手机端 `pnpm build`、手机端 `pnpm test` 和 `git diff --check` 均通过；后台构建仍有既有的大 chunk 提示，手机端测试脚本当前显示 0 个测试用例。

## 2026-06-17 01:37 HKT 聊天大厅红包领取记录查看

- 完成任务：聊天大厅红包支持查看谁抢了红包。
- 解决问题：红包卡片此前只显示领取进度，用户无法查看具体领取人、领取金额和领取时间；不可领取时按钮虽然显示“查看”，但整个卡片被禁用，实际无法打开详情。
- 实施内容：后端新增 `GET /api/user/chat-hall/red-packets/{id}/claims`，返回红包总额、已领进度和领取记录；聊天大厅仓储新增只读查询并补充单元测试；手机端新增领取记录 API、红包领取记录底部弹窗，显示领取用户名、金额和时间；领取成功后本机记录已领取状态，再次点击同一红包会打开领取记录；OpenAPI、Trellis 契约、前端规范和架构说明同步更新。
- 验证结果：`cargo fmt --manifest-path backend/Cargo.toml --check`、`cargo check --manifest-path backend/Cargo.toml`、`cargo test --manifest-path backend/Cargo.toml chat_hall -- --nocapture`、`cargo test --manifest-path backend/Cargo.toml openapi_document_contains_core_paths -- --nocapture`、手机端 `pnpm build`、手机端 `pnpm test` 和 `git diff --check` 均通过；手机端测试脚本当前显示 0 个测试用例。

## 2026-06-17 02:00 HKT 手机端代理中心下级投注画像

- 完成任务：手机端代理中心直属下级列表新增购买信息，代理可以看到下级买过的彩种、玩法、投注金额和最近购买记录。
- 解决问题：代理中心此前只展示下级注册时间、状态、充值和提现，无法判断下级实际玩了什么彩种、买了什么玩法、购买了多少金额；合买认购中但未满单的记录也容易被漏掉。
- 实施内容：后端 `GET /api/user/invitations/summary` 的 `directUsers` 新增 `totalBetAmountMinor`、`betLotterySummaries`、`betPlaySummaries` 和 `latestBet`；普通独立下注按未取消注单汇总，合买按参与人认购记录汇总且跳过已取消计划；玩法名复用 `play_rule_summaries()` 中文标签；直属充值统计改为一次性按资金流水汇总；手机端 `InvitationCenterView.vue` 展示投注总额、最近投注和主要玩法汇总；同步更新 Trellis 接口/前端规范和架构说明。
- 验证结果：`cargo fmt --manifest-path backend/Cargo.toml`、`cargo fmt --manifest-path backend/Cargo.toml --check`、`cargo check --manifest-path backend/Cargo.toml`、`cargo test --manifest-path backend/Cargo.toml user_invitation -- --nocapture`、`cargo test --manifest-path backend/Cargo.toml openapi_document_contains_core_paths -- --nocapture`、手机端 `pnpm build`、手机端 `pnpm test` 和 `git diff --check` 均通过；手机端测试脚本当前显示 0 个测试用例。

## 2026-06-17 02:19 HKT 河内5分彩 BB 开奖源接入

- 完成任务：新增“河内5分彩”彩种，并接入 BB 开奖接口 `gameCodeList=VIFFC5`。
- 解决问题：系统此前没有河内5分彩，也没有能解析 BB 开奖 `newest/last/next` 响应结构的开奖源供应商，无法通过该接口同步期号和开奖号码。
- 实施内容：后端 `DrawSourceProvider` 新增 `bbKaijiang`；新增 `bb-hn5` 默认开奖源和 `hn5` 默认 API 彩种；BB 开奖解析器按 `last` 作为最近已开奖锚点、`newest` 作为下一期参考，按期号开奖时要求对应快照存在非空 `openNumber`；后台开奖源维护新增“BB开奖”供应商和“河内5分彩采集”预设；数据库迁移 `20260617021000_add_hanoi5_bb_source.sql` 为已有库补默认开奖源并更新字段注释；架构说明同步记录接口字段口径。
- 验证结果：`cargo fmt --manifest-path backend/Cargo.toml`、`cargo fmt --manifest-path backend/Cargo.toml --check`、`cargo check --manifest-path backend/Cargo.toml`、`cargo test --manifest-path backend/Cargo.toml draw_api -- --nocapture`、`cargo test --manifest-path backend/Cargo.toml seeded_lotteries_include_requested_api68_lotteries -- --nocapture`、`cargo test --manifest-path backend/Cargo.toml openapi_document_contains_core_paths -- --nocapture`、后台 `npm run build` 和 `git diff --check` 均通过；后台构建仍有既有的大 chunk 提示。

## 2026-06-17 03:33 HKT 印尼5分彩开奖源接入

- 完成任务：新增“印尼5分彩”彩种，并接入印尼开奖接口 `https://draw.indonesia-lottery.org/others/draw.php`。
- 解决问题：系统此前没有印尼5分彩，也没有能解析印尼接口 `latest_origin/latest_num/history/next_num/next_time` 响应结构的开奖源供应商，无法通过该接口同步期号和开奖号码。
- 实施内容：后端 `DrawSourceProvider` 新增 `indonesiaLottery`；新增 `indonesia-id5` 默认开奖源和 `id5` 默认 API 彩种；解析器把 `20260617-042`、`20260617-42` 归一为系统数字期号 `20260617042`，使用 `latest_num` 和 `history.result` 返回开奖号码，使用 `next_num` 与 `next_time` 作为下一期锚点；后台开奖源维护新增“印尼开奖”供应商和“印尼5分彩采集”预设；数据库迁移 `20260617023000_add_indonesia5_draw_source.sql` 为已有库补默认开奖源并更新字段注释；架构说明同步记录接口字段口径。
- 验证结果：`cargo fmt --manifest-path backend/Cargo.toml`、`cargo fmt --manifest-path backend/Cargo.toml --check`、`cargo check --manifest-path backend/Cargo.toml`、`cargo test --manifest-path backend/Cargo.toml draw_api -- --nocapture`、`cargo test --manifest-path backend/Cargo.toml seeded_lotteries_include_requested_api68_lotteries -- --nocapture`、`cargo test --manifest-path backend/Cargo.toml openapi_document_contains_core_paths -- --nocapture`、后台 `npm run build` 和 `git diff --check` 均通过；后台构建仍有既有的大 chunk 提示。

## 2026-06-17 04:03 HKT 默认彩种数量测试修复

- 完成任务：修复后端全量测试中 `repository_uses_seeded_memory_lotteries` 的默认彩种数量断言。
- 解决问题：新增河内5分彩和印尼5分彩后，内存种子彩种数量从旧值增加，但测试仍硬编码为 `23`，导致 CI 报 `left: 25 right: 23`。
- 实施内容：将测试断言改为对齐 `seed_lotteries().len()`，让仓储列表数量跟默认种子单一事实来源保持一致，后续新增彩种不再需要同步维护魔法数字。
- 验证结果：`cargo fmt --manifest-path backend/Cargo.toml --check`、`cargo check --manifest-path backend/Cargo.toml`、`cargo test --manifest-path backend/Cargo.toml repository_uses_seeded_memory_lotteries -- --nocapture`、后端全量 `cargo test --manifest-path backend/Cargo.toml` 均通过（345 个测试成功）。

## 2026-06-18 10:18 HKT 时间节点周期跨零点期号归属修正

- 完成任务：修正时间节点周期开奖的跨零点业务口径，明确节点同时完成上一期开奖并开启下一注。
- 解决问题：`00:00:00` 节点不应该被理解为当天第一期开奖时间；正确逻辑是 `23:55:00` 开盘的前一天最后一注在次日 `00:00:00` 开奖，同时 `00:00:00` 开启当天第一期下注，该期在 `00:05:00` 开奖。
- 实施内容：后端期号生成在 `timeNode` 排期下继续按严格晚于基线的下一个自然节点作为 `scheduledAt`，但期号模板和 `{seqN}` 每日序号改为按本期的开盘节点归属；新增跨零点测试覆盖 `23:55 -> 00:00` 仍归属前一天，以及 `00:00 -> 00:05` 归属当天第一期；架构说明和 Trellis 契约同步更新。
- 验证结果：`cargo fmt --manifest-path backend/Cargo.toml --check`、`cargo check --manifest-path backend/Cargo.toml`、`cargo test --manifest-path backend/Cargo.toml draw_generation -- --nocapture`、`cargo test --manifest-path backend/Cargo.toml scheduler -- --nocapture`、后端全量 `cargo test --manifest-path backend/Cargo.toml` 均通过（347 个测试成功）。全量测试初次运行时发现本地未提交的 API68 PKS 默认地址多出 `17` 前缀导致开奖源测试失败，已恢复为正确地址后重新验证通过。

## 2026-06-18 06:15 HKT API 开奖源抓取快照与注册地来源修正

- 完成任务：API 开奖源每次读取最新期号或开奖号码时保存抓取快照；手机端注册不再用浏览器语言/时区推断注册地。
- 解决问题：API 源只把解析结果用于调度，缺少原始抓取记录，后续难以对比第三方期号和本地期号；注册地此前会把浏览器语言、系统地区或时区当成定位来源，导致后台显示与真实 IP 不匹配。
- 实施内容：新增 `api_draw_source_snapshots` 数据表和中文字段注释；统一 API68、KJAPI、BB 开奖、印尼开奖的请求-解析-快照保存流程，成功、HTTP 异常和解析失败都会保存快照；后端注册 IP 解析支持 `Forwarded` 头和带端口地址；访问控制仓储清洗 `source=client` 或未知来源的地区字段；手机端注册页移除浏览器语言/时区定位上报；架构说明和 Trellis 规范同步更新。
- 验证结果：`cargo fmt --manifest-path backend/Cargo.toml`、`cargo fmt --manifest-path backend/Cargo.toml --check`、`cargo check --manifest-path backend/Cargo.toml`、`cargo test --manifest-path backend/Cargo.toml draw_api -- --nocapture`、`cargo test --manifest-path backend/Cargo.toml registration_location -- --nocapture`、`cargo test --manifest-path backend/Cargo.toml registration_ip_parser_handles_proxy_headers -- --nocapture`、后端全量 `cargo test --manifest-path backend/Cargo.toml` 均通过（350 个测试成功）；手机端 `pnpm build`、`pnpm test` 和 `git diff --check` 均通过，手机端测试脚本当前显示 0 个测试用例。
- 外部库备注：尝试使用指定 PostgreSQL 启动后端烟测，迁移阶段未输出错误，但测试库当前启用的开奖调度在启动早期反复写入历史并报“开奖调度历史数据保存失败”，未等到监听日志；该问题属于既有调度历史保存链路，未作为本次 API 快照和注册来源修正的通过项。

## 2026-06-18 07:36 HKT 澳洲幸运5开奖源初始化地址统一

- 完成任务：按确认结果把澳洲幸运5 `api68-au5` 默认开奖源统一为 `https://api.api68.com/CQShiCai/getBaseCQShiCai.do`。
- 解决问题：早期 SQL 迁移会在空库中先插入 `getBaseCQShiCaiList.do`，而当前代码和后台预设使用 `getBaseCQShiCai.do`；新机器部署时可能出现数据库初始化地址和当前系统预设不一致。
- 实施内容：新增前向迁移 `20260618114500_update_au5_draw_source_endpoint.sql`，通过 `id=api68-au5`、`provider=api68`、`lot_code=10010` 精准更新 endpoint，并只把旧默认名称规整为“API68 澳洲幸运5”；同步更新架构说明。
- 验证结果：`git diff --check` 通过。

## 2026-06-18 08:14 HKT 后端中文注释全量治理

- 完成任务：给后端代码补齐中文注释，覆盖公开领域模型字段、公开常量、公开函数、仓储方法、数据库持久化入口、内部 helper 和测试 helper。
- 解决问题：后端仍残留部分字段无注释、私有方法缺少用途说明，以及“具体内部流程”“说明 xxx 的业务用途”等模板化注释，维护时难以快速判断业务边界。
- 实施内容：为 `backend/src/domain` 全部公开字段补充中文字段含义；为 `backend/src/services`、`backend/src/routes`、`app/error/response/main` 中函数和方法补充中文用途说明；重点清洗开奖源解析、开奖调度、资金、充值、客服、彩种、用户访问控制、合买机器人等关键链路注释；同步更新架构说明。
- 验证结果：后端注释扫描确认函数、公开字段、公开常量均已有中文注释，模板化占位注释为 0；`cargo fmt --manifest-path backend/Cargo.toml` 和 `cargo check --manifest-path backend/Cargo.toml` 均通过，后续继续执行测试编译验证。

## 2026-06-18 10:27 HKT API 开奖源采集快照后台可视化

- 完成任务：给已经落库的 API 开奖源采集快照补上后台查看和一键清除入口。
- 解决问题：此前 `api_draw_source_snapshots` 已经保存第三方期号、开奖号码和原始响应，但后台“开奖期号与开奖源”页面没有任何入口查看，运营无法确认 API 抓取到的数据是否和本地期号、调度结果一致；采集快照持续增长后也缺少后台维护清理入口。
- 实施内容：后端新增 `GET /api/admin/draw-source-snapshots`，支持按彩种、开奖源、采集用途、成功状态和期号筛选，并使用数据库分页倒序读取；新增 `DELETE /api/admin/draw-source-snapshots/clear`，只清除 API 采集快照审计记录并返回 `deletedCount`；新增采集快照领域分页模型、仓储查询/清理入口和 OpenAPI 文档；管理后台新增“采集快照”分段，列表展示采集摘要并可通过 `SideSheet` 查看 endpoint、错误信息、原始 JSON 和原始文本，同时提供中文确认的一键清除按钮；同步更新架构说明。
- 验证结果：管理后台 `npm run build` 通过；`cargo fmt --manifest-path backend/Cargo.toml`、`cargo fmt --manifest-path backend/Cargo.toml --check`、`cargo check --manifest-path backend/Cargo.toml`、`cargo test --manifest-path backend/Cargo.toml draw_api -- --nocapture`、`cargo test --manifest-path backend/Cargo.toml openapi_document_contains_core_paths -- --nocapture`、后端全量 `cargo test --manifest-path backend/Cargo.toml`（350 个测试成功）和 `git diff --check` 均通过。

## 2026-06-18 14:05 HKT 修复 Tauri 图标格式导致的宏展开失败

- 完成任务：修复手机端 Tauri 编译时报 `tauri::generate_context!()` proc macro panic 的问题。
- 解决问题：`mobile/src-tauri/icons/icon.png` 文件名是 PNG，但图像内容缺少 RGBA alpha 通道，Tauri 编译期读取图标时报 `icon ... is not RGBA`，导致宏展开失败。
- 实施内容：保留现有图标画面内容，将图标重新编码为真正的 RGBA PNG，避免 Tauri 编译期图标解析失败。
- 验证结果：`cd mobile/src-tauri && cargo check` 通过，`generate_context!()` 不再 panic。

## 2026-06-19 01:12 HKT 修复 iOS 真机启动闪退

- 完成任务：修复 Tauri iOS 真机安装后点击 App 立即闪退的问题。
- 解决问题：iPad 崩溃报告显示主线程在 `wry::wkwebview::platform_webview_version` 初始化 WKWebView 时触发 `CFRelease() called with NULL`，崩溃路径中 `.app` 和可执行文件名均为中文“鼎鸿”；iOS 26.5 下该组合会在 Wry 查询 WebKit 版本阶段触发原生崩溃。
- 实施内容：将 Tauri 内部 `productName`、iOS `PRODUCT_NAME` 和可执行文件名统一改为英文 `HongFu`，并通过 `CFBundleDisplayName=鼎鸿` 保留桌面显示名；同步更新 iOS 架构说明，明确后续原生产物名必须使用英文内部名。
- 验证结果：使用已连接 iPad 真机构建并安装成功，产物路径变为 `HongFu.app/HongFu`，设备进程列表显示 `HongFu` 和 WebKit 子进程持续运行，未再生成新的“鼎鸿/HongFu”崩溃报告。
