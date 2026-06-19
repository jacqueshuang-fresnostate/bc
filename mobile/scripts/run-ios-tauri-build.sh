#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MOBILE_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

EXISTING_GIT_CONFIG_COUNT="${GIT_CONFIG_COUNT:-0}"
export GIT_CONFIG_COUNT="$((EXISTING_GIT_CONFIG_COUNT + 1))"
export "GIT_CONFIG_KEY_${EXISTING_GIT_CONFIG_COUNT}=safe.bareRepository"
export "GIT_CONFIG_VALUE_${EXISTING_GIT_CONFIG_COUNT}=all"

# SwiftPM 在编译 iOS 依赖时会调用 swiftc 和 Git；把缓存放到项目 target 下，
# 可以避免系统缓存目录权限异常导致构建中断。
export CLANG_MODULE_CACHE_PATH="${CLANG_MODULE_CACHE_PATH:-$MOBILE_DIR/src-tauri/target/swift-module-cache}"
mkdir -p "$CLANG_MODULE_CACHE_PATH"

exec pnpm tauri ios build "$@"
