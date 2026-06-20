#!/usr/bin/env bash

set -euo pipefail

usage() {
  cat <<'EOF'
用途：构建 Android APK，并可为本次打包临时指定 Android 包名。

用法：
  bash mobile/scripts/build-android-apk.sh
  bash mobile/scripts/build-android-apk.sh --package-id com.example.app
  bash mobile/scripts/build-android-apk.sh --random-package-id --install

参数：
  --package-id <包名>  指定本次 APK 使用的 Android applicationId
  --random-package-id  每次打包自动生成新包名，格式为 <当前包名>.build<时间戳>
  --install            构建完成后通过 adb 安装到已连接设备
  --device <序列号>    指定 adb 设备序列号，需配合 --install 使用
  --debug              构建 debug APK
  --release            构建 release APK，默认值
  -h, --help           显示帮助

说明：
  默认包名仍是 com.hongfu.app。
  指定包名只影响本次构建，不会修改可提交的主工程包名。
  包名变化后，Android 会把它视为一个全新的 App，旧登录态和本地缓存不会继承。
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

validate_android_package_id() {
  local value="$1"
  case "$value" in
    *..*|.*|*.) return 1 ;;
  esac
  printf '%s' "$value" | grep -Eq '^[A-Za-z][A-Za-z0-9_]*(\.[A-Za-z][A-Za-z0-9_]*)+$'
}

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MOBILE_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
ANDROID_DIR="$MOBILE_DIR/src-tauri/gen/android"
DEFAULT_PACKAGE_ID="com.hongfu.app"
PACKAGE_ID_OVERRIDE="${HONGFU_ANDROID_PACKAGE_ID:-}"
RANDOM_PACKAGE_ID=0
INSTALL_AFTER_BUILD=0
ADB_DEVICE=""
BUILD_KIND="release"

while [ "$#" -gt 0 ]; do
  case "$1" in
    --package-id)
      [ "${2:-}" != "" ] || fail "--package-id 需要跟一个 Android 包名。"
      PACKAGE_ID_OVERRIDE="$2"
      shift 2
      ;;
    --random-package-id)
      RANDOM_PACKAGE_ID=1
      shift
      ;;
    --install)
      INSTALL_AFTER_BUILD=1
      shift
      ;;
    --device)
      [ "${2:-}" != "" ] || fail "--device 需要跟一个 adb 设备序列号。"
      ADB_DEVICE="$2"
      shift 2
      ;;
    --debug)
      BUILD_KIND="debug"
      shift
      ;;
    --release)
      BUILD_KIND="release"
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

require_command pnpm
[ -d "$ANDROID_DIR" ] || fail "未找到 Android 工程目录：$ANDROID_DIR。请先执行 pnpm tauri android init。"

if [ "$RANDOM_PACKAGE_ID" -eq 1 ] && [ "$PACKAGE_ID_OVERRIDE" != "" ]; then
  fail "--package-id 和 --random-package-id 只能选择一个。"
fi
if [ "$RANDOM_PACKAGE_ID" -eq 1 ]; then
  PACKAGE_ID_OVERRIDE="${DEFAULT_PACKAGE_ID}.build$(date +%Y%m%d%H%M%S)"
fi

FINAL_PACKAGE_ID="${PACKAGE_ID_OVERRIDE:-$DEFAULT_PACKAGE_ID}"
validate_android_package_id "$FINAL_PACKAGE_ID" || fail "Android 包名格式不合法：$FINAL_PACKAGE_ID。请使用类似 com.example.app 的格式，每段以字母开头。"

export HONGFU_ANDROID_PACKAGE_ID="$FINAL_PACKAGE_ID"
log "本次 Android APK 包名：$HONGFU_ANDROID_PACKAGE_ID"

TAURI_ARGS=(android build --apk --ci)
if [ "$BUILD_KIND" = "debug" ]; then
  TAURI_ARGS+=(--debug)
fi

log "开始构建 ${BUILD_KIND} APK..."
(cd "$MOBILE_DIR" && pnpm tauri "${TAURI_ARGS[@]}")

if [ "$BUILD_KIND" = "debug" ]; then
  APK_PATH="$ANDROID_DIR/app/build/outputs/apk/universal/debug/app-universal-debug.apk"
else
  APK_PATH="$ANDROID_DIR/app/build/outputs/apk/universal/release/app-universal-release.apk"
fi
[ -f "$APK_PATH" ] || fail "构建完成但未找到 APK：$APK_PATH"

log "APK 已生成："
printf '%s\n' "$APK_PATH"
ls -lh "$APK_PATH"

if [ "$INSTALL_AFTER_BUILD" -eq 1 ]; then
  require_command adb
  ADB_ARGS=()
  if [ "$ADB_DEVICE" != "" ]; then
    ADB_ARGS=(-s "$ADB_DEVICE")
  fi
  log "安装 APK 到 Android 设备..."
  adb "${ADB_ARGS[@]}" install -r "$APK_PATH"
  log "安装完成，包名：$HONGFU_ANDROID_PACKAGE_ID"
else
  warn "如需构建后直接安装，可追加 --install；指定设备可追加 --device <序列号>。"
fi
