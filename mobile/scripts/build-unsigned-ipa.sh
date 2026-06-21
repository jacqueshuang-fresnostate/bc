#!/usr/bin/env bash

set -euo pipefail

usage() {
  cat <<'EOF'
用途：在 macOS 上生成无签名 IPA。

用法：
  bash mobile/scripts/build-unsigned-ipa.sh
  bash mobile/scripts/build-unsigned-ipa.sh --output /tmp/app.ipa
  bash mobile/scripts/build-unsigned-ipa.sh --skip-web-build

参数：
  --output <路径>      指定输出 IPA 路径，默认输出到 mobile/src-tauri/gen/apple/build/<应用名>-unsigned.ipa
  --skip-web-build     跳过 pnpm build，直接使用 mobile/dist
  --api-base <地址>    指定 App 打包后请求的后端地址，会写入 Vite 构建产物
  --branding-api-base <地址>
                       指定用于同步打包 Logo 和 iOS 图标的后端地址；未指定时复用 --api-base
  --bundle-id <包名>   指定本次 IPA 使用的 iOS Bundle Identifier，只修改临时构建工程
  --random-bundle-id   每次打包自动生成新包名，格式为 <当前包名>.build<时间戳>
  --skip-branding-sync 跳过后台品牌资源同步
  --keep-temp          保留临时 iOS 工程，方便排查构建问题
  -h, --help           显示帮助

说明：
  这个脚本会跳过 Xcode 签名，只生成 Payload/*.app 格式的无签名 IPA。
  无签名 IPA 不能直接安装到普通 iPhone，需要后续重签名或使用签名工具处理。
  每次变更包名会让 iOS 认为这是一个全新的 App，旧登录态和本地缓存不会继承。
EOF
}

log() {
  printf '\033[1;34m%s\033[0m\n' "$*"
}

warn() {
  printf '\033[1;33m%s\033[0m\n' "$*" >&2
}

fail() {
  printf '\033[1;31m%s\033[0m\n' "$*" >&2
  exit 1
}

require_command() {
  command -v "$1" >/dev/null 2>&1 || fail "缺少命令：$1，请先安装或配置后再重试。"
}

validate_bundle_id() {
  local value="$1"
  case "$value" in
    *..*|.*|*.) return 1 ;;
  esac
  printf '%s' "$value" | grep -Eq '^[A-Za-z0-9][A-Za-z0-9.-]*$'
}

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MOBILE_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
REPO_ROOT="$(cd "$MOBILE_DIR/.." && pwd)"
IOS_DIR="$MOBILE_DIR/src-tauri/gen/apple"
DIST_DIR="$MOBILE_DIR/dist"
OUT_DIR="$IOS_DIR/build"
DEFAULT_API_BASE="https://ad.16888888.live"
APP_API_BASE="${VITE_API_BASE_URL:-${VITE_API_BASE:-}}"
APP_API_BASE_EXPLICIT=0
if [ "$APP_API_BASE" != "" ]; then
  APP_API_BASE_EXPLICIT=1
else
  APP_API_BASE="$DEFAULT_API_BASE"
fi
BRANDING_API_BASE=""
BRANDING_API_BASE_EXPLICIT=0
OUTPUT_IPA=""
SKIP_WEB_BUILD=0
SKIP_BRANDING_SYNC=0
KEEP_TEMP=0
BUNDLE_ID_OVERRIDE=""
RANDOM_BUNDLE_ID=0

while [ "$#" -gt 0 ]; do
  case "$1" in
    --output)
      [ "${2:-}" != "" ] || fail "--output 需要跟一个输出路径。"
      OUTPUT_IPA="$2"
      shift 2
      ;;
    --skip-web-build)
      SKIP_WEB_BUILD=1
      shift
      ;;
    --api-base)
      [ "${2:-}" != "" ] || fail "--api-base 需要跟一个后端地址。"
      APP_API_BASE="$2"
      APP_API_BASE_EXPLICIT=1
      if [ "$BRANDING_API_BASE_EXPLICIT" -eq 0 ]; then
        BRANDING_API_BASE="$2"
      fi
      shift 2
      ;;
    --branding-api-base)
      [ "${2:-}" != "" ] || fail "--branding-api-base 需要跟一个后端地址。"
      BRANDING_API_BASE="$2"
      BRANDING_API_BASE_EXPLICIT=1
      if [ "$APP_API_BASE_EXPLICIT" -eq 0 ]; then
        APP_API_BASE="$2"
      fi
      shift 2
      ;;
    --bundle-id)
      [ "${2:-}" != "" ] || fail "--bundle-id 需要跟一个 iOS 包名。"
      BUNDLE_ID_OVERRIDE="$2"
      shift 2
      ;;
    --random-bundle-id)
      RANDOM_BUNDLE_ID=1
      shift
      ;;
    --skip-branding-sync)
      SKIP_BRANDING_SYNC=1
      shift
      ;;
    --keep-temp)
      KEEP_TEMP=1
      shift
      ;;
    --)
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      fail "未知参数：$1。可执行 --help 查看用法。"
      ;;
  esac
done

[ "$(uname -s)" = "Darwin" ] || fail "该脚本只能在 macOS 上执行。"
require_command pnpm
require_command xcodebuild
require_command rsync
require_command perl
require_command zip
require_command unzip
require_command curl
require_command node
require_command sips

[ -d "$IOS_DIR" ] || fail "未找到 iOS 工程目录：$IOS_DIR。请先执行 pnpm tauri:ios:init。"
[ -f "$IOS_DIR/hongfu-mobile.xcodeproj/project.pbxproj" ] || fail "未找到 Xcode 工程文件，请先初始化 iOS 工程。"
APP_API_BASE="$(printf '%s' "$APP_API_BASE" | sed 's:/*$::')"
BRANDING_API_BASE="$(printf '%s' "$BRANDING_API_BASE" | sed 's:/*$::')"
[ "$APP_API_BASE" != "" ] || fail "缺少 App 后端地址，请通过 --api-base、--branding-api-base 或 VITE_API_BASE_URL 指定。"
if [ "$BRANDING_API_BASE" = "" ]; then
  BRANDING_API_BASE="$APP_API_BASE"
fi
if [ "$RANDOM_BUNDLE_ID" -eq 1 ] && [ "$BUNDLE_ID_OVERRIDE" != "" ]; then
  fail "--bundle-id 和 --random-bundle-id 只能选择一个。"
fi

LIBAPP="$IOS_DIR/Externals/arm64/release/libapp.a"
if [ ! -f "$LIBAPP" ]; then
  cat >&2 <<EOF
缺少 iOS 真机原生库：
  $LIBAPP

这个无签名脚本会跳过 Tauri 的 Rust iOS 构建脚本，因此需要先存在 libapp.a。
如果你已经配置好 Apple Developer Team，可以先运行一次：
  cd "$MOBILE_DIR"
  pnpm tauri:build:ios

生成过 Externals/arm64/release/libapp.a 后，再重新执行本脚本。
EOF
  exit 1
fi

if [ "$SKIP_WEB_BUILD" -eq 0 ]; then
  log "开始构建手机端前端资源，后端地址：$APP_API_BASE"
  (cd "$MOBILE_DIR" && VITE_API_BASE_URL="$APP_API_BASE" VITE_API_BASE="$APP_API_BASE" pnpm build)
else
  warn "已跳过前端构建，将直接使用现有 mobile/dist。"
fi

[ -f "$DIST_DIR/index.html" ] || fail "未找到 $DIST_DIR/index.html，请先执行 pnpm build。"
mkdir -p "$OUT_DIR"

if [ "$SKIP_BRANDING_SYNC" -eq 0 ]; then
  log "同步后台品牌 Logo 到前端打包资源..."
  node "$SCRIPT_DIR/sync-branding-assets.mjs" \
    --api-base "$BRANDING_API_BASE" \
    --dist-dir "$DIST_DIR" \
    --work-dir "$OUT_DIR/branding"
else
  warn "已跳过后台品牌资源同步，IPA 将继续使用当前 dist 和 iOS 工程里的图标资源。"
fi

NATIVE_SOURCE_NEWER="$(find "$MOBILE_DIR/src-tauri/src" "$MOBILE_DIR/src-tauri/Cargo.toml" "$MOBILE_DIR/src-tauri/tauri.conf.json" -newer "$LIBAPP" -print -quit 2>/dev/null || true)"
if [ "$NATIVE_SOURCE_NEWER" != "" ]; then
  warn "检测到 iOS 原生源码或 Tauri 配置比 libapp.a 更新：$NATIVE_SOURCE_NEWER"
  warn "本脚本会继续复用现有 libapp.a；如果改过 Rust/Tauri 原生代码，请先重新生成 iOS 原生库。"
fi

APP_NAME="$(awk -F': *' '/PRODUCT_NAME:/ {print $2; exit}' "$IOS_DIR/project.yml" | tr -d '"' || true)"
[ "$APP_NAME" != "" ] || APP_NAME="HongFu"
SAFE_APP_NAME="$(printf '%s' "$APP_NAME" | tr '/:' '__')"
mkdir -p "$OUT_DIR"

if [ "$OUTPUT_IPA" = "" ]; then
  OUTPUT_IPA="$OUT_DIR/${SAFE_APP_NAME}-unsigned.ipa"
else
  case "$OUTPUT_IPA" in
    /*) ;;
    *) OUTPUT_IPA="$(pwd)/$OUTPUT_IPA" ;;
  esac
fi

BUILD_LOG="$OUT_DIR/ios-unsigned-build.log"
TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/hongfu-ios-unsigned.XXXXXX")"
cleanup() {
  if [ "$KEEP_TEMP" -eq 1 ]; then
    warn "已保留临时目录：$TMP_DIR"
  else
    rm -rf "$TMP_DIR"
  fi
}
trap cleanup EXIT

log "复制临时 iOS 工程..."
rsync -a --delete --exclude build "$IOS_DIR/" "$TMP_DIR/"

if [ "$SKIP_BRANDING_SYNC" -eq 0 ]; then
  log "同步后台品牌 Logo 到 iOS 桌面图标资源..."
  node "$SCRIPT_DIR/sync-branding-assets.mjs" \
    --api-base "$BRANDING_API_BASE" \
    --ios-appicon-dir "$TMP_DIR/Assets.xcassets/AppIcon.appiconset" \
    --work-dir "$OUT_DIR/branding"
fi

log "同步最新前端资源到 iOS 工程..."
rm -rf "$TMP_DIR/assets"
mkdir -p "$TMP_DIR/assets"
rsync -a --delete "$DIST_DIR/" "$TMP_DIR/assets/"

CURRENT_BUNDLE_ID="$(awk -F' = ' '/PRODUCT_BUNDLE_IDENTIFIER =/ {gsub(/;/, "", $2); print $2; exit}' "$TMP_DIR/hongfu-mobile.xcodeproj/project.pbxproj" || true)"
[ "$CURRENT_BUNDLE_ID" != "" ] || CURRENT_BUNDLE_ID="com.hongfu.app"
if [ "$RANDOM_BUNDLE_ID" -eq 1 ]; then
  BUNDLE_ID_OVERRIDE="${CURRENT_BUNDLE_ID}.build$(date +%Y%m%d%H%M%S)"
fi
if [ "$BUNDLE_ID_OVERRIDE" != "" ]; then
  validate_bundle_id "$BUNDLE_ID_OVERRIDE" || fail "包名格式不合法：$BUNDLE_ID_OVERRIDE。只能使用字母、数字、点和短横线，不能以点开头或结尾。"
  log "本次 IPA 使用临时包名：$BUNDLE_ID_OVERRIDE"
  perl -0pi -e "s/PRODUCT_BUNDLE_IDENTIFIER = [^;]+;/PRODUCT_BUNDLE_IDENTIFIER = $BUNDLE_ID_OVERRIDE;/g" "$TMP_DIR/hongfu-mobile.xcodeproj/project.pbxproj"
fi

log "临时跳过 Tauri iOS Rust 构建脚本和 Xcode 签名..."
perl -0pi -e 's/shellScript = "(?:\\.|[^"\\])*tauri ios xcode-script(?:\\.|[^"\\])*";/shellScript = "echo 使用已有 iOS 原生库生成无签名 IPA";/s' "$TMP_DIR/hongfu-mobile.xcodeproj/project.pbxproj"
if grep -q "tauri ios xcode-script" "$TMP_DIR/hongfu-mobile.xcodeproj/project.pbxproj"; then
  fail "未能替换 Xcode Rust 构建脚本，请检查 project.pbxproj 格式是否变化。"
fi

log "开始构建无签名 iOS App..."
if ! xcodebuild \
  -project "$TMP_DIR/hongfu-mobile.xcodeproj" \
  -scheme hongfu-mobile_iOS \
  -configuration release \
  -sdk iphoneos \
  -destination 'generic/platform=iOS' \
  -derivedDataPath "$TMP_DIR/build/unsigned-derived" \
  CODE_SIGNING_ALLOWED=NO \
  CODE_SIGNING_REQUIRED=NO \
  CODE_SIGN_IDENTITY='' \
  DEVELOPMENT_TEAM='' \
  build >"$BUILD_LOG" 2>&1; then
  warn "iOS App 构建失败，最近日志如下："
  tail -n 120 "$BUILD_LOG" >&2 || true
  fail "完整日志：$BUILD_LOG"
fi

APP_PATH="$(find "$TMP_DIR/build/unsigned-derived/Build/Products/release-iphoneos" -maxdepth 1 -name '*.app' -type d | head -n 1)"
[ "$APP_PATH" != "" ] || fail "构建结束但未找到 .app，日志：$BUILD_LOG"

if [ ! -f "$APP_PATH/assets/index.html" ]; then
  fail "构建出的 .app 中缺少 assets/index.html，请检查前端资源是否正确同步。"
fi

log "封装 Payload 并生成 IPA..."
rm -rf "$TMP_DIR/Payload"
mkdir -p "$TMP_DIR/Payload"
cp -R "$APP_PATH" "$TMP_DIR/Payload/"
rm -f "$OUTPUT_IPA"
(cd "$TMP_DIR" && zip -qry "$OUTPUT_IPA" Payload)

PACKAGED_BRANDING="$(unzip -p "$OUTPUT_IPA" 'Payload/*.app/assets/mobile-branding.json' 2>/dev/null || true)"
if [ "$PACKAGED_BRANDING" != "" ]; then
  log "包内品牌配置："
  printf '%s\n' "$PACKAGED_BRANDING"
fi

log "无签名 IPA 已生成："
printf '%s\n' "$OUTPUT_IPA"
ls -lh "$OUTPUT_IPA"

log "包信息："
/usr/libexec/PlistBuddy -c 'Print :CFBundleName' "$APP_PATH/Info.plist" 2>/dev/null || true
/usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' "$APP_PATH/Info.plist" 2>/dev/null || true

warn "提示：该 IPA 未签名，不能直接安装到普通 iPhone，需要后续重签名或用签名工具处理。"
