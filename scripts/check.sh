#!/usr/bin/env bash
# AIODashboard 按影响范围分层门禁；full 与 CI 同构。
#
# 用法：bash scripts/check.sh [frontend|rust|full]
# 日常迭代跑最小相关范围；跨层、发布和 CI 才跑 full，避免重复验证未变更层。

set -euo pipefail
cd "$(dirname "$0")/.."

step() { printf '\n==> %s\n' "$1"; }

usage() {
  echo "用法：bash scripts/check.sh [frontend|rust|full]" >&2
  exit 2
}

ensure_frontend_dependencies() {
  if [ ! -d apps/desktop/node_modules ]; then
    echo "node_modules 不存在，先执行 npm install"
    (cd apps/desktop && npm ci)
  fi
}

check_rust() {
  step "[rust 1/4] cargo fmt --check"
  cargo fmt --all --check

  step "[rust 2/4] cargo clippy (-D warnings)"
  cargo clippy --workspace --all-targets -- -D warnings

  step "[rust 3/4] cargo test (workspace)"
  local test_snapshot_dir
  test_snapshot_dir="$(mktemp -d)"
  DASHBOARD_WIDGET_SNAPSHOT_PATH="$test_snapshot_dir/widget.json" cargo test --workspace

  step "[rust 4/4] cargo build (workspace)"
  cargo build --workspace
}

check_frontend() {
  ensure_frontend_dependencies
  if [ ! -d node_modules/typescript ]; then npm ci; fi
  step "[frontend SDK] 公共类型、测试桩与示例"
  npm run build
  npm test

  step "[frontend 1/2] vitest"
  (cd apps/desktop && npm run test)

  step "[frontend 2/2] tsc + vite production build"
  (cd apps/desktop && npm run build)
}

[ "$#" -eq 1 ] || usage
scope="$1"
case "$scope" in
  frontend)
    check_frontend
    ;;
  rust)
    check_rust
    ;;
  full)
    check_rust
    check_frontend
    ;;
  *)
    usage
    ;;
esac

printf '\n✅ %s 范围检查通过\n' "$scope"
