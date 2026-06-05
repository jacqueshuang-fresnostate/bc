//! OpenAPI 文档路由，统一暴露接口规范 JSON 和 Swagger UI 页面

use axum::{response::Html, routing::get, Json, Router};
use serde_json::{json, Map, Value};

use crate::app::AppState;

/// 组装 OpenAPI 文档路由，文档入口保持公开，便于本地联调和前端类型核对。
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/openapi.json", get(openapi_json))
        .route("/docs", get(swagger_ui))
}

/// 返回 OpenAPI 规范 JSON；这里不能包统一 API 信封，否则 Swagger UI 无法直接读取。
async fn openapi_json() -> Json<Value> {
    Json(openapi_document())
}

/// 返回 Swagger UI 页面，页面会读取同服务下的 `/api/openapi.json`。
async fn swagger_ui() -> Html<&'static str> {
    Html(SWAGGER_UI_HTML)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AuthMode {
    None,
    Admin,
    User,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RequestBodyKind {
    None,
    Form,
    Json,
    Multipart,
}

#[derive(Debug, Clone, Copy)]
struct RouteDoc {
    method: &'static str,
    path: &'static str,
    tag: &'static str,
    summary: &'static str,
    description: &'static str,
    auth: AuthMode,
    request_body: RequestBodyKind,
}

const SWAGGER_UI_HTML: &str = r##"<!doctype html>
<html lang="zh-CN">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>彩票管理后台 OpenAPI 文档</title>
    <link rel="stylesheet" href="https://unpkg.com/swagger-ui-dist@5/swagger-ui.css" />
    <style>
      body { margin: 0; background: #f7f8fa; }
      #swagger-ui { max-width: 1440px; margin: 0 auto; }
    </style>
  </head>
  <body>
    <div id="swagger-ui"></div>
    <script src="https://unpkg.com/swagger-ui-dist@5/swagger-ui-bundle.js"></script>
    <script>
      window.ui = SwaggerUIBundle({
        url: "/api/openapi.json",
        dom_id: "#swagger-ui",
        deepLinking: true,
        displayRequestDuration: true,
        persistAuthorization: true
      });
    </script>
  </body>
</html>
"##;

/// 接口文档的单一事实来源；现有业务路由新增时应在这里同步补充中文说明。
const ROUTE_DOCS: &[RouteDoc] = &[
    doc(
        "get",
        "/health",
        "公共接口",
        "健康检查",
        "返回服务名、运行状态和后端版本。",
        AuthMode::None,
        RequestBodyKind::None,
    ),
    doc(
        "post",
        "/admin/auth/login",
        "管理员认证",
        "管理员登录",
        "使用管理员账号密码登录后台并返回访问令牌。",
        AuthMode::None,
        RequestBodyKind::Json,
    ),
    doc(
        "get",
        "/admin/auth/me",
        "管理员认证",
        "读取当前管理员",
        "根据 Bearer Token 返回当前管理员资料和权限。",
        AuthMode::Admin,
        RequestBodyKind::None,
    ),
    doc(
        "post",
        "/admin/auth/logout",
        "管理员认证",
        "管理员退出登录",
        "清理当前管理员令牌。",
        AuthMode::Admin,
        RequestBodyKind::None,
    ),
    doc(
        "get",
        "/admin/dashboard",
        "管理后台概览",
        "读取工作台概览",
        "按当前管理员权限裁剪后返回工作台指标、模块和业务摘要。",
        AuthMode::Admin,
        RequestBodyKind::None,
    ),
    doc(
        "get",
        "/admin/users",
        "用户管理",
        "用户列表",
        "返回后台用户列表。",
        AuthMode::Admin,
        RequestBodyKind::None,
    ),
    doc(
        "post",
        "/admin/users",
        "用户管理",
        "新增用户",
        "创建后台用户记录，空邀请码由后端自动生成。",
        AuthMode::Admin,
        RequestBodyKind::Json,
    ),
    doc(
        "get",
        "/admin/users/{id}",
        "用户管理",
        "用户详情",
        "按用户 ID 返回单个用户详情。",
        AuthMode::Admin,
        RequestBodyKind::None,
    ),
    doc(
        "put",
        "/admin/users/{id}",
        "用户管理",
        "更新用户",
        "更新用户基础资料、类型、状态、代理和邀请码。",
        AuthMode::Admin,
        RequestBodyKind::Json,
    ),
    doc(
        "patch",
        "/admin/users/{id}/status",
        "用户管理",
        "修改用户状态",
        "快速启用、锁定或禁用用户。",
        AuthMode::Admin,
        RequestBodyKind::Json,
    ),
    doc(
        "get",
        "/admin/admins",
        "管理员管理",
        "管理员列表",
        "返回后台管理员账号列表。",
        AuthMode::Admin,
        RequestBodyKind::None,
    ),
    doc(
        "post",
        "/admin/admins",
        "管理员管理",
        "新增管理员",
        "创建管理员账号并保存独立密码。",
        AuthMode::Admin,
        RequestBodyKind::Json,
    ),
    doc(
        "get",
        "/admin/admins/{id}",
        "管理员管理",
        "管理员详情",
        "按管理员 ID 返回账号详情。",
        AuthMode::Admin,
        RequestBodyKind::None,
    ),
    doc(
        "put",
        "/admin/admins/{id}",
        "管理员管理",
        "更新管理员",
        "更新管理员账号、角色和状态。",
        AuthMode::Admin,
        RequestBodyKind::Json,
    ),
    doc(
        "patch",
        "/admin/admins/{id}/password",
        "管理员管理",
        "重置管理员密码",
        "为指定管理员设置新密码。",
        AuthMode::Admin,
        RequestBodyKind::Json,
    ),
    doc(
        "patch",
        "/admin/admins/{id}/status",
        "管理员管理",
        "修改管理员状态",
        "快速启用、锁定或停用管理员。",
        AuthMode::Admin,
        RequestBodyKind::Json,
    ),
    doc(
        "get",
        "/admin/roles",
        "角色权限",
        "角色列表",
        "返回后台角色和权限范围。",
        AuthMode::Admin,
        RequestBodyKind::None,
    ),
    doc(
        "post",
        "/admin/roles",
        "角色权限",
        "新增角色",
        "创建一个后台角色并配置权限范围。",
        AuthMode::Admin,
        RequestBodyKind::Json,
    ),
    doc(
        "get",
        "/admin/roles/{id}",
        "角色权限",
        "角色详情",
        "按角色 ID 返回角色详情。",
        AuthMode::Admin,
        RequestBodyKind::None,
    ),
    doc(
        "put",
        "/admin/roles/{id}",
        "角色权限",
        "更新角色",
        "更新角色名称和权限范围。",
        AuthMode::Admin,
        RequestBodyKind::Json,
    ),
    doc(
        "delete",
        "/admin/roles/{id}",
        "角色权限",
        "删除角色",
        "删除未被管理员账号占用的角色。",
        AuthMode::Admin,
        RequestBodyKind::None,
    ),
    doc(
        "get",
        "/admin/system-settings",
        "系统设置",
        "系统设置列表",
        "返回所有后台系统配置项。",
        AuthMode::Admin,
        RequestBodyKind::None,
    ),
    doc(
        "patch",
        "/admin/system-settings/{key}",
        "系统设置",
        "修改系统设置",
        "按配置键更新单个系统设置值。",
        AuthMode::Admin,
        RequestBodyKind::Json,
    ),
    doc(
        "post",
        "/admin/image-bed/upload",
        "图床配置",
        "上传图片到图床",
        "通过后台保存的图床配置代理上传文件并返回图片链接。",
        AuthMode::Admin,
        RequestBodyKind::Multipart,
    ),
    doc(
        "get",
        "/admin/advertisements",
        "广告管理",
        "广告列表",
        "返回后台维护的手机端轮播广告配置。",
        AuthMode::Admin,
        RequestBodyKind::None,
    ),
    doc(
        "post",
        "/admin/advertisements",
        "广告管理",
        "新增广告",
        "创建手机端轮播广告，广告 ID 留空时由后端自动生成。",
        AuthMode::Admin,
        RequestBodyKind::Json,
    ),
    doc(
        "get",
        "/admin/advertisements/{id}",
        "广告管理",
        "广告详情",
        "按广告 ID 返回单条广告配置。",
        AuthMode::Admin,
        RequestBodyKind::None,
    ),
    doc(
        "put",
        "/admin/advertisements/{id}",
        "广告管理",
        "更新广告",
        "更新广告图片、跳转链接、排序、状态和展示时间。",
        AuthMode::Admin,
        RequestBodyKind::Json,
    ),
    doc(
        "delete",
        "/admin/advertisements/{id}",
        "广告管理",
        "删除广告",
        "删除指定广告配置。",
        AuthMode::Admin,
        RequestBodyKind::None,
    ),
    doc(
        "get",
        "/admin/registration",
        "用户管理",
        "读取注册配置",
        "返回用户名注册、邮箱注册和代理邀请码要求。",
        AuthMode::Admin,
        RequestBodyKind::None,
    ),
    doc(
        "put",
        "/admin/registration",
        "用户管理",
        "更新注册配置",
        "维护用户注册开关和邀请码策略。",
        AuthMode::Admin,
        RequestBodyKind::Json,
    ),
    doc(
        "get",
        "/admin/finance-overview",
        "财务管理",
        "财务总览",
        "返回平台总余额、冻结提现金额、今日充值和今日派奖指标。",
        AuthMode::Admin,
        RequestBodyKind::None,
    ),
    doc(
        "get",
        "/admin/financial-accounts",
        "财务管理",
        "资金账户列表",
        "分页返回用户资金账户摘要，并包含用户 ID 与用户名。",
        AuthMode::Admin,
        RequestBodyKind::None,
    ),
    doc(
        "get",
        "/admin/ledger-entries",
        "财务管理",
        "资金流水列表",
        "分页返回后台可查看的资金流水。",
        AuthMode::Admin,
        RequestBodyKind::None,
    ),
    doc(
        "get",
        "/admin/recharge-orders",
        "财务管理",
        "充值订单列表",
        "分页返回彩虹易支付和客服直充的充值订单。",
        AuthMode::Admin,
        RequestBodyKind::None,
    ),
    doc(
        "post",
        "/admin/recharge-orders/{id}/confirm",
        "财务管理",
        "确认客服直充入账",
        "后台确认客服直充订单已收款，写入充值流水并增加用户余额。",
        AuthMode::Admin,
        RequestBodyKind::Json,
    ),
    doc(
        "get",
        "/admin/withdrawal-orders",
        "财务管理",
        "提现申请列表",
        "分页返回用户提现申请，供后台财务审核。",
        AuthMode::Admin,
        RequestBodyKind::None,
    ),
    doc(
        "post",
        "/admin/withdrawal-orders/{id}/approve",
        "财务管理",
        "通过提现申请",
        "后台通过提现申请，扣减冻结余额并写入提现打款流水。",
        AuthMode::Admin,
        RequestBodyKind::None,
    ),
    doc(
        "post",
        "/admin/withdrawal-orders/{id}/reject",
        "财务管理",
        "驳回提现申请",
        "后台驳回提现申请，解冻余额并写入提现驳回解冻流水。",
        AuthMode::Admin,
        RequestBodyKind::None,
    ),
    doc(
        "post",
        "/admin/financial-adjustments",
        "财务管理",
        "手工资金调整",
        "管理员手工增加或扣减用户余额并写入流水。",
        AuthMode::Admin,
        RequestBodyKind::Json,
    ),
    doc(
        "get",
        "/admin/group-buy/plans",
        "合买管理",
        "合买计划列表",
        "返回合买计划列表。",
        AuthMode::Admin,
        RequestBodyKind::None,
    ),
    doc(
        "post",
        "/admin/group-buy/plans",
        "合买管理",
        "新增合买计划",
        "创建合买计划和发起人份额；满员时自动生成真实投注订单。",
        AuthMode::Admin,
        RequestBodyKind::Json,
    ),
    doc(
        "get",
        "/admin/group-buy/plans/{id}",
        "合买管理",
        "合买计划详情",
        "按计划 ID 返回合买详情。",
        AuthMode::Admin,
        RequestBodyKind::None,
    ),
    doc(
        "put",
        "/admin/group-buy/plans/{id}",
        "合买管理",
        "更新合买计划",
        "更新合买计划状态和说明；取消时按参与记录退款。",
        AuthMode::Admin,
        RequestBodyKind::Json,
    ),
    doc(
        "post",
        "/admin/group-buy/plans/{id}/participants",
        "合买管理",
        "添加合买参与人",
        "为合买计划添加参与人份额；满员时自动生成真实投注订单。",
        AuthMode::Admin,
        RequestBodyKind::Json,
    ),
    doc(
        "get",
        "/admin/invitations",
        "邀请返利",
        "邀请关系列表",
        "返回代理邀请关系列表。",
        AuthMode::Admin,
        RequestBodyKind::None,
    ),
    doc(
        "post",
        "/admin/invitations",
        "邀请返利",
        "新增邀请关系",
        "创建代理与下级用户的邀请关系。",
        AuthMode::Admin,
        RequestBodyKind::Json,
    ),
    doc(
        "get",
        "/admin/invitations/{id}",
        "邀请返利",
        "邀请关系详情",
        "按邀请关系 ID 返回详情。",
        AuthMode::Admin,
        RequestBodyKind::None,
    ),
    doc(
        "put",
        "/admin/invitations/{id}",
        "邀请返利",
        "更新邀请关系",
        "更新邀请关系状态、返利开关和备注。",
        AuthMode::Admin,
        RequestBodyKind::Json,
    ),
    doc(
        "get",
        "/admin/invite-policy",
        "邀请返利",
        "读取返利策略",
        "返回默认邀请和充值返利策略。",
        AuthMode::Admin,
        RequestBodyKind::None,
    ),
    doc(
        "put",
        "/admin/invite-policy",
        "邀请返利",
        "更新返利策略",
        "维护代理邀请策略和返利基点。",
        AuthMode::Admin,
        RequestBodyKind::Json,
    ),
    doc(
        "get",
        "/admin/realtime",
        "在线客服",
        "后台实时事件",
        "后台通过 WebSocket 接收客服消息等实时事件，浏览器连接时使用 token 查询参数鉴权。",
        AuthMode::Admin,
        RequestBodyKind::None,
    ),
    doc(
        "get",
        "/admin/support/conversations",
        "在线客服",
        "客服会话列表",
        "返回用户客服会话列表。",
        AuthMode::Admin,
        RequestBodyKind::None,
    ),
    doc(
        "post",
        "/admin/support/conversations",
        "在线客服",
        "新增客服会话",
        "创建一条客服会话。",
        AuthMode::Admin,
        RequestBodyKind::Json,
    ),
    doc(
        "get",
        "/admin/support/conversations/{id}",
        "在线客服",
        "客服会话详情",
        "按会话 ID 返回消息和状态。",
        AuthMode::Admin,
        RequestBodyKind::None,
    ),
    doc(
        "put",
        "/admin/support/conversations/{id}",
        "在线客服",
        "更新客服会话",
        "更新客服会话状态、优先级或分配管理员。",
        AuthMode::Admin,
        RequestBodyKind::Json,
    ),
    doc(
        "post",
        "/admin/support/conversations/{id}/messages",
        "在线客服",
        "回复客服消息",
        "管理员回复用户发来的客服消息。",
        AuthMode::Admin,
        RequestBodyKind::Json,
    ),
    doc(
        "get",
        "/admin/robots",
        "机器人配置",
        "机器人列表",
        "返回合买机器人和购彩机器人配置。",
        AuthMode::Admin,
        RequestBodyKind::None,
    ),
    doc(
        "post",
        "/admin/robots",
        "机器人配置",
        "新增机器人",
        "创建机器人配置。",
        AuthMode::Admin,
        RequestBodyKind::Json,
    ),
    doc(
        "post",
        "/admin/robots/run",
        "机器人配置",
        "执行合买机器人",
        "立即执行已启用的合买机器人，返回创建合买、满单、订单、流水和跳过原因。",
        AuthMode::Admin,
        RequestBodyKind::None,
    ),
    doc(
        "get",
        "/admin/robots/{id}",
        "机器人配置",
        "机器人详情",
        "按机器人 ID 返回配置详情。",
        AuthMode::Admin,
        RequestBodyKind::None,
    ),
    doc(
        "put",
        "/admin/robots/{id}",
        "机器人配置",
        "更新机器人",
        "更新机器人名称、类型、状态和彩种范围。",
        AuthMode::Admin,
        RequestBodyKind::Json,
    ),
    doc(
        "delete",
        "/admin/robots/{id}",
        "机器人配置",
        "删除机器人",
        "删除机器人配置。",
        AuthMode::Admin,
        RequestBodyKind::None,
    ),
    doc(
        "patch",
        "/admin/robots/{id}/status",
        "机器人配置",
        "修改机器人状态",
        "快速启用或停用机器人。",
        AuthMode::Admin,
        RequestBodyKind::Json,
    ),
    doc(
        "get",
        "/admin/lotteries",
        "彩种管理",
        "彩种列表",
        "返回所有彩种、玩法赔率、分类和 Logo。",
        AuthMode::Admin,
        RequestBodyKind::None,
    ),
    doc(
        "post",
        "/admin/lotteries",
        "彩种管理",
        "新增彩种",
        "创建彩种并按号码类型补齐玩法配置。",
        AuthMode::Admin,
        RequestBodyKind::Json,
    ),
    doc(
        "get",
        "/admin/lotteries/{id}",
        "彩种管理",
        "彩种详情",
        "按彩种 ID 返回单个彩种详情。",
        AuthMode::Admin,
        RequestBodyKind::None,
    ),
    doc(
        "put",
        "/admin/lotteries/{id}",
        "彩种管理",
        "更新彩种",
        "更新彩种基础信息、开奖模式、分类、玩法和合买配置。",
        AuthMode::Admin,
        RequestBodyKind::Json,
    ),
    doc(
        "delete",
        "/admin/lotteries/{id}",
        "彩种管理",
        "删除彩种",
        "删除指定彩种配置。",
        AuthMode::Admin,
        RequestBodyKind::None,
    ),
    doc(
        "patch",
        "/admin/lotteries/{id}/sale",
        "彩种管理",
        "修改彩种销售状态",
        "快速开启或停止彩种销售。",
        AuthMode::Admin,
        RequestBodyKind::Json,
    ),
    doc(
        "get",
        "/admin/lottery-categories",
        "彩种管理",
        "彩种分类列表",
        "返回可维护的彩种分类配置。",
        AuthMode::Admin,
        RequestBodyKind::None,
    ),
    doc(
        "post",
        "/admin/lottery-categories",
        "彩种管理",
        "新增彩种分类",
        "创建彩种分类编码和名称。",
        AuthMode::Admin,
        RequestBodyKind::Json,
    ),
    doc(
        "put",
        "/admin/lottery-categories/{code}",
        "彩种管理",
        "更新彩种分类",
        "更新彩种分类名称和排序。",
        AuthMode::Admin,
        RequestBodyKind::Json,
    ),
    doc(
        "delete",
        "/admin/lottery-categories/{code}",
        "彩种管理",
        "删除彩种分类",
        "删除未被彩种占用的分类。",
        AuthMode::Admin,
        RequestBodyKind::None,
    ),
    doc(
        "get",
        "/admin/draw-sources",
        "开奖源与调度",
        "开奖源列表",
        "返回 API 和静态开奖源配置。",
        AuthMode::Admin,
        RequestBodyKind::None,
    ),
    doc(
        "post",
        "/admin/draw-sources",
        "开奖源与调度",
        "新增开奖源",
        "创建开奖源并绑定可复用彩种。",
        AuthMode::Admin,
        RequestBodyKind::Json,
    ),
    doc(
        "put",
        "/admin/draw-sources/{id}",
        "开奖源与调度",
        "更新开奖源",
        "更新开奖源供应商、地址、编码和绑定彩种。",
        AuthMode::Admin,
        RequestBodyKind::Json,
    ),
    doc(
        "delete",
        "/admin/draw-sources/{id}",
        "开奖源与调度",
        "删除开奖源",
        "删除指定开奖源配置。",
        AuthMode::Admin,
        RequestBodyKind::None,
    ),
    doc(
        "get",
        "/admin/draw-controls",
        "开奖源与调度",
        "控制号码列表",
        "返回每个彩种的控制开奖号码状态。",
        AuthMode::Admin,
        RequestBodyKind::None,
    ),
    doc(
        "get",
        "/admin/draw-controls/{lottery_id}",
        "开奖源与调度",
        "控制号码详情",
        "按彩种 ID 返回控制开奖号码配置。",
        AuthMode::Admin,
        RequestBodyKind::None,
    ),
    doc(
        "put",
        "/admin/draw-controls/{lottery_id}",
        "开奖源与调度",
        "保存控制号码",
        "保存彩种控制开奖号码开关和号码。",
        AuthMode::Admin,
        RequestBodyKind::Json,
    ),
    doc(
        "get",
        "/admin/draw-issues",
        "开奖期号",
        "期号列表",
        "分页返回开奖期号，支持后台筛选。",
        AuthMode::Admin,
        RequestBodyKind::None,
    ),
    doc(
        "post",
        "/admin/draw-issues",
        "开奖期号",
        "创建期号",
        "手工创建单个开奖期号。",
        AuthMode::Admin,
        RequestBodyKind::Json,
    ),
    doc(
        "post",
        "/admin/draw-issues/generate-next",
        "开奖期号",
        "生成下一期",
        "按彩种配置生成下一期开奖期号。",
        AuthMode::Admin,
        RequestBodyKind::Json,
    ),
    doc(
        "post",
        "/admin/draw-issues/preview-generation",
        "开奖期号",
        "预览期号生成",
        "预览下一期生成结果但不落库。",
        AuthMode::Admin,
        RequestBodyKind::Json,
    ),
    doc(
        "post",
        "/admin/draw-issues/generate-batch",
        "开奖期号",
        "批量生成期号",
        "按彩种配置批量生成未来期号。",
        AuthMode::Admin,
        RequestBodyKind::Json,
    ),
    doc(
        "get",
        "/admin/draw-issues/{id}",
        "开奖期号",
        "期号详情",
        "按期号记录 ID 返回详情。",
        AuthMode::Admin,
        RequestBodyKind::None,
    ),
    doc(
        "patch",
        "/admin/draw-issues/{id}/close",
        "开奖期号",
        "封盘期号",
        "把 open 期号改为 closed。",
        AuthMode::Admin,
        RequestBodyKind::None,
    ),
    doc(
        "patch",
        "/admin/draw-issues/{id}/draw",
        "开奖期号",
        "执行开奖",
        "按手动号码、控制号码、平台号码或 API 源完成开奖。",
        AuthMode::Admin,
        RequestBodyKind::Json,
    ),
    doc(
        "patch",
        "/admin/draw-issues/{id}/cancel",
        "开奖期号",
        "取消期号",
        "取消尚未完成开奖的期号。",
        AuthMode::Admin,
        RequestBodyKind::None,
    ),
    doc(
        "get",
        "/admin/draw-scheduler/status",
        "开奖源与调度",
        "读取调度状态",
        "返回开奖调度配置、运行状态和最近执行记录。",
        AuthMode::Admin,
        RequestBodyKind::None,
    ),
    doc(
        "put",
        "/admin/draw-scheduler/config",
        "开奖源与调度",
        "更新调度配置",
        "修改开奖调度开关、执行周期和生成缓冲。",
        AuthMode::Admin,
        RequestBodyKind::Json,
    ),
    doc(
        "post",
        "/admin/draw-automation/run",
        "开奖源与调度",
        "手动执行开奖自动化",
        "立即执行封盘、开奖、结算和补期流程。",
        AuthMode::Admin,
        RequestBodyKind::Json,
    ),
    doc(
        "get",
        "/admin/play-rules",
        "玩法规则",
        "玩法规则列表",
        "返回 3 位和 5 位玩法规则目录。",
        AuthMode::Admin,
        RequestBodyKind::None,
    ),
    doc(
        "post",
        "/admin/play-rules/evaluate",
        "玩法规则",
        "评估玩法选号",
        "计算注数、展开投注并判断是否命中开奖号码。",
        AuthMode::Admin,
        RequestBodyKind::Json,
    ),
    doc(
        "get",
        "/admin/orders",
        "订单与结算",
        "订单列表",
        "返回后台订单列表。",
        AuthMode::Admin,
        RequestBodyKind::None,
    ),
    doc(
        "post",
        "/admin/orders",
        "订单与结算",
        "创建订单",
        "按用户、彩种、期号和选号创建投注订单。",
        AuthMode::Admin,
        RequestBodyKind::Json,
    ),
    doc(
        "get",
        "/admin/orders/{id}",
        "订单与结算",
        "订单详情",
        "按订单 ID 返回订单详情。",
        AuthMode::Admin,
        RequestBodyKind::None,
    ),
    doc(
        "patch",
        "/admin/orders/{id}/cancel",
        "订单与结算",
        "取消订单",
        "取消未结算的投注订单并退款。",
        AuthMode::Admin,
        RequestBodyKind::None,
    ),
    doc(
        "get",
        "/admin/settlements",
        "订单与结算",
        "结算批次列表",
        "返回开奖结算批次列表。",
        AuthMode::Admin,
        RequestBodyKind::None,
    ),
    doc(
        "get",
        "/admin/settlements/{id}",
        "订单与结算",
        "结算批次详情",
        "按结算批次 ID 返回详情。",
        AuthMode::Admin,
        RequestBodyKind::None,
    ),
    doc(
        "post",
        "/admin/settlements/draw-issues/{id}",
        "订单与结算",
        "结算指定期号",
        "对已开奖期号执行订单结算和派奖。",
        AuthMode::Admin,
        RequestBodyKind::None,
    ),
    doc(
        "get",
        "/lottery/home",
        "用户端内容",
        "手机端首页彩种",
        "返回所有销售中的彩种，按后台彩种分类分组，并附带当前期号和最近开奖号码。",
        AuthMode::None,
        RequestBodyKind::None,
    ),
    doc(
        "get",
        "/lottery/groups",
        "用户端内容",
        "手机端彩种分组",
        "返回手机端开奖历史页可筛选的销售中彩种分组。",
        AuthMode::None,
        RequestBodyKind::None,
    ),
    doc(
        "get",
        "/lottery/history/latest",
        "用户端内容",
        "手机端最新开奖",
        "返回每个销售中彩种最近一期已开奖数据，支持按彩种或分组筛选。",
        AuthMode::None,
        RequestBodyKind::None,
    ),
    doc(
        "get",
        "/lottery/history",
        "用户端内容",
        "手机端开奖历史",
        "返回单彩种或筛选范围内的已开奖历史。",
        AuthMode::None,
        RequestBodyKind::None,
    ),
    doc(
        "get",
        "/user/mobile/advertisements",
        "用户端内容",
        "手机端轮播广告",
        "返回当前启用且在展示时间内的手机端轮播广告。",
        AuthMode::None,
        RequestBodyKind::None,
    ),
    doc(
        "get",
        "/user/mobile/site-config",
        "用户端内容",
        "手机端站点配置",
        "返回手机端平台名称、Logo 图片链接和站点介绍。",
        AuthMode::None,
        RequestBodyKind::None,
    ),
    doc(
        "get",
        "/user/register-options",
        "用户端账户",
        "注册配置",
        "返回手机端注册页需要展示的用户名注册、邮箱注册和邀请码策略。",
        AuthMode::None,
        RequestBodyKind::None,
    ),
    doc(
        "post",
        "/user/register",
        "用户端账户",
        "用户注册",
        "支持用户名注册或邮箱注册，并按策略处理代理邀请码。",
        AuthMode::None,
        RequestBodyKind::Json,
    ),
    doc(
        "post",
        "/user/login",
        "用户端账户",
        "用户登录",
        "支持用户名或邮箱登录并返回用户访问令牌。",
        AuthMode::None,
        RequestBodyKind::Json,
    ),
    doc(
        "post",
        "/user/forgot-password",
        "用户端账户",
        "申请忘记密码",
        "按用户名或邮箱生成重置密码令牌。",
        AuthMode::None,
        RequestBodyKind::Json,
    ),
    doc(
        "post",
        "/user/reset-password",
        "用户端账户",
        "重置用户密码",
        "使用重置令牌设置用户新密码。",
        AuthMode::None,
        RequestBodyKind::Json,
    ),
    doc(
        "get",
        "/user/me",
        "用户端账户",
        "读取当前用户",
        "根据用户 Bearer Token 返回当前用户资料。",
        AuthMode::User,
        RequestBodyKind::None,
    ),
    doc(
        "post",
        "/user/logout",
        "用户端账户",
        "用户退出登录",
        "清理当前用户令牌。",
        AuthMode::User,
        RequestBodyKind::None,
    ),
    doc(
        "post",
        "/user/bind-email",
        "用户端账户",
        "绑定邮箱",
        "为当前用户绑定邮箱，后续可用邮箱登录。",
        AuthMode::User,
        RequestBodyKind::Json,
    ),
    doc(
        "post",
        "/user/password/change",
        "用户端账户",
        "修改用户密码",
        "当前用户通过旧密码修改新密码。",
        AuthMode::User,
        RequestBodyKind::Json,
    ),
    doc(
        "get",
        "/user/balance",
        "用户端账户",
        "查询余额",
        "返回当前用户资料和资金账户余额。",
        AuthMode::User,
        RequestBodyKind::None,
    ),
    doc(
        "get",
        "/user/ledger-entries",
        "用户端账户",
        "查询资金流水",
        "返回当前用户自己的资金流水。",
        AuthMode::User,
        RequestBodyKind::None,
    ),
    doc(
        "get",
        "/user/bet/page-config/{lottery_id}",
        "用户端投注",
        "下注页配置",
        "返回当前销售彩种的可投注期号、最近开奖、玩法配置、赔率和合买配置。",
        AuthMode::User,
        RequestBodyKind::None,
    ),
    doc(
        "get",
        "/user/bet/orders",
        "用户端投注",
        "用户注单列表",
        "返回当前用户自己的投注订单记录。",
        AuthMode::User,
        RequestBodyKind::None,
    ),
    doc(
        "post",
        "/user/bet/orders",
        "用户端投注",
        "提交投注订单",
        "按当前用户、彩种、期号、玩法和选号批量创建投注订单，并从资金账户扣款。",
        AuthMode::User,
        RequestBodyKind::Json,
    ),
    doc(
        "get",
        "/user/group-buy/plans",
        "用户端合买",
        "合买大厅列表",
        "返回当前用户可查看的合买计划，支持按彩种和分组筛选。",
        AuthMode::User,
        RequestBodyKind::None,
    ),
    doc(
        "post",
        "/user/group-buy/plans",
        "用户端合买",
        "发起合买计划",
        "当前用户按彩种、期号、玩法、投注内容和自购金额发起合买；满员时自动生成真实投注订单。",
        AuthMode::User,
        RequestBodyKind::Json,
    ),
    doc(
        "get",
        "/user/group-buy/plans/{id}",
        "用户端合买",
        "合买计划详情",
        "返回合买计划详情、进度和当前用户参与份额。",
        AuthMode::User,
        RequestBodyKind::None,
    ),
    doc(
        "post",
        "/user/group-buy/plans/{id}/participants",
        "用户端合买",
        "参与合买计划",
        "当前用户按认购金额参与合买；满员时自动生成真实投注订单。",
        AuthMode::User,
        RequestBodyKind::Json,
    ),
    doc(
        "get",
        "/user/group-buy/my",
        "用户端合买",
        "我的合买",
        "返回当前用户发起或参与过的合买计划。",
        AuthMode::User,
        RequestBodyKind::None,
    ),
    doc(
        "get",
        "/user/group-buy/create-options",
        "用户端合买",
        "发起合买选项",
        "返回可发起合买的彩种、当前可售期号、玩法和合买金额配置。",
        AuthMode::User,
        RequestBodyKind::None,
    ),
    doc(
        "get",
        "/user/realtime",
        "用户端实时",
        "用户实时事件",
        "用户端通过 WebSocket 接收开奖、余额、订单、充值、提现和客服消息事件。",
        AuthMode::User,
        RequestBodyKind::None,
    ),
    doc(
        "get",
        "/user/recharge/config",
        "用户端充值",
        "读取充值配置",
        "返回后台启用的彩虹易支付和客服直充渠道配置。",
        AuthMode::User,
        RequestBodyKind::None,
    ),
    doc(
        "get",
        "/user/recharge/orders",
        "用户端充值",
        "用户充值订单列表",
        "返回当前用户自己的充值订单。",
        AuthMode::User,
        RequestBodyKind::None,
    ),
    doc(
        "post",
        "/user/recharge/orders",
        "用户端充值",
        "创建充值订单",
        "创建彩虹易支付订单或客服直充申请；客服直充会同步生成客服会话。",
        AuthMode::User,
        RequestBodyKind::Json,
    ),
    doc(
        "get",
        "/user/recharge/epay/notify",
        "充值回调",
        "彩虹易支付通知",
        "彩虹易支付 GET 异步通知入口，验签成功后充值入账，成功时返回裸 success。",
        AuthMode::None,
        RequestBodyKind::None,
    ),
    doc(
        "post",
        "/user/recharge/epay/notify",
        "充值回调",
        "彩虹易支付表单通知",
        "彩虹易支付 POST 表单通知入口，验签成功后充值入账，成功时返回裸 success。",
        AuthMode::None,
        RequestBodyKind::Form,
    ),
    doc(
        "get",
        "/user/recharge/epay/return",
        "充值回调",
        "彩虹易支付同步返回",
        "彩虹易支付同步返回入口，用于用户端读取返回参数。",
        AuthMode::None,
        RequestBodyKind::None,
    ),
    doc(
        "get",
        "/user/support/conversations",
        "用户端客服",
        "用户客服会话列表",
        "返回当前用户自己的客服会话。",
        AuthMode::User,
        RequestBodyKind::None,
    ),
    doc(
        "get",
        "/user/support/conversations/{id}",
        "用户端客服",
        "用户客服会话详情",
        "返回当前用户自己的单个客服会话。",
        AuthMode::User,
        RequestBodyKind::None,
    ),
    doc(
        "post",
        "/user/support/conversations/{id}/messages",
        "用户端客服",
        "用户发送客服消息",
        "当前用户向自己的客服会话追加消息。",
        AuthMode::User,
        RequestBodyKind::Json,
    ),
    doc(
        "get",
        "/user/withdrawal-methods",
        "用户端账户",
        "提现方式列表",
        "返回当前用户的支付宝、微信或银行卡提现方式。",
        AuthMode::User,
        RequestBodyKind::None,
    ),
    doc(
        "post",
        "/user/withdrawal-methods",
        "用户端账户",
        "新增提现方式",
        "新增当前用户的提现方式。",
        AuthMode::User,
        RequestBodyKind::Json,
    ),
    doc(
        "put",
        "/user/withdrawal-methods/{method_id}",
        "用户端账户",
        "更新提现方式",
        "更新当前用户的提现方式。",
        AuthMode::User,
        RequestBodyKind::Json,
    ),
    doc(
        "delete",
        "/user/withdrawal-methods/{method_id}",
        "用户端账户",
        "删除提现方式",
        "删除当前用户的提现方式。",
        AuthMode::User,
        RequestBodyKind::None,
    ),
    doc(
        "get",
        "/user/withdrawals",
        "用户端账户",
        "提现申请列表",
        "返回当前用户自己的提现申请记录。",
        AuthMode::User,
        RequestBodyKind::None,
    ),
    doc(
        "post",
        "/user/withdrawals",
        "用户端账户",
        "提交提现申请",
        "用户选择已绑定提现方式提交提现申请，后端冻结对应可用余额。",
        AuthMode::User,
        RequestBodyKind::Json,
    ),
];

const fn doc(
    method: &'static str,
    path: &'static str,
    tag: &'static str,
    summary: &'static str,
    description: &'static str,
    auth: AuthMode,
    request_body: RequestBodyKind,
) -> RouteDoc {
    RouteDoc {
        method,
        path,
        tag,
        summary,
        description,
        auth,
        request_body,
    }
}

/// 构建完整 OpenAPI 文档，保持路径、组件和安全方案都在后端同一处生成。
fn openapi_document() -> Value {
    let mut paths = Map::new();
    for route in ROUTE_DOCS {
        insert_path_operation(&mut paths, route);
    }

    json!({
        "openapi": "3.1.0",
        "info": {
            "title": "彩票管理后台 API",
            "description": "管理后台、用户端、开奖调度、财务和客服接口的 OpenAPI 文档。业务接口默认返回统一 ApiEnvelope；OpenAPI JSON 与 Swagger UI 为文档入口，不使用业务响应信封。",
            "version": env!("CARGO_PKG_VERSION")
        },
        "servers": [
            {
                "url": "/api",
                "description": "当前后端服务的 API 前缀"
            }
        ],
        "tags": openapi_tags(),
        "paths": Value::Object(paths),
        "components": openapi_components()
    })
}

/// 把同一路径下的多个 HTTP 方法合并成一个 OpenAPI PathItem。
fn insert_path_operation(paths: &mut Map<String, Value>, route: &RouteDoc) {
    let entry = paths
        .entry(route.path.to_string())
        .or_insert_with(|| Value::Object(Map::new()));

    if let Some(path_item) = entry.as_object_mut() {
        path_item.insert(route.method.to_string(), route_operation(route));
    }
}

/// 生成单个接口操作说明，统一挂载中文摘要、参数、请求体、响应和安全策略。
fn route_operation(route: &RouteDoc) -> Value {
    let mut operation = Map::new();
    operation.insert("tags".to_string(), json!([route.tag]));
    operation.insert("summary".to_string(), json!(route.summary));
    operation.insert("description".to_string(), json!(route.description));
    operation.insert("operationId".to_string(), json!(operation_id(route)));

    let parameters = path_parameters(route.path);
    if !parameters.is_empty() {
        operation.insert("parameters".to_string(), Value::Array(parameters));
    }

    if let Some(request_body) = request_body_schema(route.request_body) {
        operation.insert("requestBody".to_string(), request_body);
    }

    if route.auth != AuthMode::None {
        operation.insert("security".to_string(), json!([{ "bearerAuth": [] }]));
    }

    operation.insert("responses".to_string(), responses_schema(route.auth));
    Value::Object(operation)
}

/// 根据路径中的 `{id}`、`{code}` 等占位符生成 OpenAPI 路径参数。
fn path_parameters(path: &str) -> Vec<Value> {
    let mut parameters = Vec::new();
    let mut rest = path;

    while let Some(start) = rest.find('{') {
        let after_start = &rest[start + 1..];
        let Some(end) = after_start.find('}') else {
            break;
        };
        let name = &after_start[..end];
        parameters.push(json!({
            "name": name,
            "in": "path",
            "required": true,
            "description": format!("路径参数：{name}"),
            "schema": { "type": "string" }
        }));
        rest = &after_start[end + 1..];
    }

    parameters
}

/// 生成请求体描述；具体字段由领域类型维护，这里先给接口工具提供可提交的通用结构。
fn request_body_schema(kind: RequestBodyKind) -> Option<Value> {
    match kind {
        RequestBodyKind::None => None,
        RequestBodyKind::Form => Some(json!({
            "required": true,
            "content": {
                "application/x-www-form-urlencoded": {
                    "schema": {
                        "type": "object",
                        "additionalProperties": { "type": "string" },
                        "description": "第三方支付通知表单字段。"
                    }
                }
            }
        })),
        RequestBodyKind::Json => Some(json!({
            "required": true,
            "content": {
                "application/json": {
                    "schema": {
                        "type": "object",
                        "additionalProperties": true,
                        "description": "具体字段以对应后端领域模型为准。"
                    }
                }
            }
        })),
        RequestBodyKind::Multipart => Some(json!({
            "required": true,
            "content": {
                "multipart/form-data": {
                    "schema": {
                        "type": "object",
                        "required": ["file"],
                        "properties": {
                            "file": {
                                "type": "string",
                                "format": "binary",
                                "description": "需要上传到图床的图片文件。"
                            }
                        }
                    }
                }
            }
        })),
    }
}

/// 统一生成响应说明，所有业务接口成功和失败都按 ApiEnvelope 描述。
fn responses_schema(auth: AuthMode) -> Value {
    let mut responses = Map::new();
    responses.insert(
        "200".to_string(),
        json!({
            "description": "请求成功。",
            "content": {
                "application/json": {
                    "schema": { "$ref": "#/components/schemas/ApiEnvelope" }
                }
            }
        }),
    );
    responses.insert(
        "400".to_string(),
        json!({ "$ref": "#/components/responses/BadRequest" }),
    );
    responses.insert(
        "404".to_string(),
        json!({ "$ref": "#/components/responses/NotFound" }),
    );
    responses.insert(
        "409".to_string(),
        json!({ "$ref": "#/components/responses/Conflict" }),
    );
    responses.insert(
        "500".to_string(),
        json!({ "$ref": "#/components/responses/InternalError" }),
    );

    if auth != AuthMode::None {
        responses.insert(
            "401".to_string(),
            json!({ "$ref": "#/components/responses/Unauthorized" }),
        );
        responses.insert(
            "403".to_string(),
            json!({ "$ref": "#/components/responses/Forbidden" }),
        );
    }

    Value::Object(responses)
}

/// 生成稳定的 operationId，方便 OpenAPI 客户端生成器识别接口。
fn operation_id(route: &RouteDoc) -> String {
    let mut id = format!("api_{}", route.method);
    for ch in route.path.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            id.push(ch);
        } else if !id.ends_with('_') {
            id.push('_');
        }
    }
    id.trim_end_matches('_').to_string()
}

/// 返回文档中展示的标签分组，顺序与后台主要模块保持一致。
fn openapi_tags() -> Value {
    json!([
        { "name": "公共接口", "description": "健康检查和服务探测。" },
        { "name": "管理员认证", "description": "后台管理员登录、退出和当前账号。" },
        { "name": "管理后台概览", "description": "工作台摘要和权限裁剪后的模块入口。" },
        { "name": "用户管理", "description": "后台用户维护与注册策略。" },
        { "name": "管理员管理", "description": "后台管理员账号维护。" },
        { "name": "角色权限", "description": "角色和权限范围维护。" },
        { "name": "系统设置", "description": "系统级配置维护。" },
        { "name": "图床配置", "description": "图床上传代理和图片链接提取。" },
        { "name": "广告管理", "description": "手机端轮播广告配置。" },
        { "name": "财务管理", "description": "资金账户、流水和手工调整。" },
        { "name": "合买管理", "description": "合买计划和参与人管理。" },
        { "name": "邀请返利", "description": "代理邀请关系和返利策略。" },
        { "name": "在线客服", "description": "客服会话与消息回复。" },
        { "name": "机器人配置", "description": "合买机器人和购彩机器人配置。" },
        { "name": "彩种管理", "description": "彩种、分类、玩法赔率和销售状态。" },
        { "name": "开奖期号", "description": "期号创建、封盘、开奖和取消。" },
        { "name": "开奖源与调度", "description": "开奖源、控制号码和自动调度。" },
        { "name": "玩法规则", "description": "玩法目录和注数评估。" },
        { "name": "订单与结算", "description": "投注订单、取消、计奖和派奖。" },
        { "name": "用户端内容", "description": "手机端公开内容接口。" },
        { "name": "用户端账户", "description": "用户注册登录、余额、流水和提现方式。" },
        { "name": "用户端合买", "description": "手机端合买大厅、发起合买、参与合买和我的合买。" },
        { "name": "用户端充值", "description": "用户充值配置、充值下单和订单查询。" },
        { "name": "充值回调", "description": "第三方支付异步通知和同步返回入口。" },
        { "name": "用户端客服", "description": "用户自己的客服会话和消息发送接口。" }
    ])
}

/// 返回通用组件定义，业务数据统一放在 ApiEnvelope.data 中。
fn openapi_components() -> Value {
    json!({
        "securitySchemes": {
            "bearerAuth": {
                "type": "http",
                "scheme": "bearer",
                "bearerFormat": "opaque",
                "description": "登录接口返回的 token，调用受保护接口时使用 Authorization: Bearer <token>。"
            }
        },
        "schemas": {
            "ApiEnvelope": {
                "type": "object",
                "required": ["success", "message"],
                "properties": {
                    "success": {
                        "type": "boolean",
                        "description": "本次业务请求是否成功。"
                    },
                    "data": {
                        "description": "业务数据。不同接口返回不同结构；失败时为 null。",
                        "nullable": true
                    },
                    "message": {
                        "type": "string",
                        "description": "成功时通常为 ok，失败时为错误说明。"
                    }
                }
            },
            "ErrorEnvelope": {
                "type": "object",
                "required": ["success", "data", "message"],
                "properties": {
                    "success": { "type": "boolean", "const": false },
                    "data": { "type": "null" },
                    "message": { "type": "string" }
                }
            }
        },
        "responses": {
            "BadRequest": error_response("请求参数或业务条件不合法。"),
            "Unauthorized": error_response("未登录、令牌缺失或令牌无效。"),
            "Forbidden": error_response("当前账号没有访问该接口的权限。"),
            "NotFound": error_response("目标资源不存在。"),
            "Conflict": error_response("资源重复或状态冲突。"),
            "InternalError": error_response("服务内部错误。")
        }
    })
}

/// 生成统一错误响应组件，避免每个状态码重复写响应体结构。
fn error_response(description: &str) -> Value {
    json!({
        "description": description,
        "content": {
            "application/json": {
                "schema": { "$ref": "#/components/schemas/ErrorEnvelope" }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// 验证 OpenAPI 文档包含公共、后台和用户端核心接口。
    fn openapi_document_contains_core_paths() {
        let document = openapi_document();

        assert_eq!(document["openapi"].as_str(), Some("3.1.0"));
        assert!(document["paths"]["/health"]["get"].is_object());
        assert!(document["paths"]["/admin/users"]["get"].is_object());
        assert!(document["paths"]["/admin/advertisements"]["get"].is_object());
        assert!(document["paths"]["/admin/finance-overview"]["get"].is_object());
        assert!(document["paths"]["/admin/recharge-orders"]["get"].is_object());
        assert!(document["paths"]["/admin/recharge-orders/{id}/confirm"]["post"].is_object());
        assert!(document["paths"]["/admin/realtime"]["get"].is_object());
        assert!(document["paths"]["/admin/withdrawal-orders"]["get"].is_object());
        assert!(document["paths"]["/admin/withdrawal-orders/{id}/approve"]["post"].is_object());
        assert!(document["paths"]["/admin/withdrawal-orders/{id}/reject"]["post"].is_object());
        assert!(document["paths"]["/admin/draw-scheduler/config"]["put"].is_object());
        assert!(document["paths"]["/lottery/home"]["get"].is_object());
        assert!(document["paths"]["/lottery/groups"]["get"].is_object());
        assert!(document["paths"]["/lottery/history/latest"]["get"].is_object());
        assert!(document["paths"]["/lottery/history"]["get"].is_object());
        assert!(document["paths"]["/user/mobile/advertisements"]["get"].is_object());
        assert!(document["paths"]["/user/mobile/site-config"]["get"].is_object());
        assert!(document["paths"]["/user/register-options"]["get"].is_object());
        assert!(document["paths"]["/user/bet/page-config/{lottery_id}"]["get"].is_object());
        assert!(document["paths"]["/user/bet/orders"]["get"].is_object());
        assert!(document["paths"]["/user/bet/orders"]["post"].is_object());
        assert!(document["paths"]["/user/group-buy/plans"]["get"].is_object());
        assert!(document["paths"]["/user/group-buy/plans"]["post"].is_object());
        assert!(document["paths"]["/user/group-buy/plans/{id}"]["get"].is_object());
        assert!(document["paths"]["/user/group-buy/plans/{id}/participants"]["post"].is_object());
        assert!(document["paths"]["/user/group-buy/my"]["get"].is_object());
        assert!(document["paths"]["/user/group-buy/create-options"]["get"].is_object());
        assert!(document["paths"]["/user/realtime"]["get"].is_object());
        assert!(document["paths"]["/user/recharge/orders"]["post"].is_object());
        assert!(document["paths"]["/user/support/conversations/{id}/messages"]["post"].is_object());
        assert!(document["paths"]["/user/withdrawals"]["post"].is_object());
        assert!(document["paths"]["/user/register"]["post"].is_object());
    }

    #[test]
    /// 验证受保护接口声明 Bearer Token 安全方案，公开接口不强制认证。
    fn openapi_document_marks_security_by_auth_mode() {
        let document = openapi_document();

        assert!(document["paths"]["/admin/users"]["get"]["security"].is_array());
        assert!(document["paths"]["/user/me"]["get"]["security"].is_array());
        assert!(document["paths"]["/user/register-options"]["get"]["security"].is_null());
        assert!(document["paths"]["/user/register"]["post"]["security"].is_null());
        assert!(document["components"]["securitySchemes"]["bearerAuth"].is_object());
    }

    #[test]
    /// 验证 Swagger UI 页面指向当前服务暴露的 OpenAPI JSON。
    fn swagger_ui_points_to_openapi_json() {
        assert!(SWAGGER_UI_HTML.contains("/api/openapi.json"));
        assert!(SWAGGER_UI_HTML.contains("SwaggerUIBundle"));
    }

    #[test]
    /// 验证路径占位符能转换成 OpenAPI 路径参数。
    fn path_parameters_extracts_axum_style_placeholders() {
        let parameters = path_parameters("/admin/draw-controls/{lottery_id}");

        assert_eq!(parameters.len(), 1);
        assert_eq!(parameters[0]["name"].as_str(), Some("lottery_id"));
        assert_eq!(parameters[0]["in"].as_str(), Some("path"));
    }
}
