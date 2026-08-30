#!/usr/bin/env bash
# AIODashboard 机械门禁（与 CI 同构，一条命令定义"能合并"）
#
# 用法：bash scripts/check.sh
# 任何准备交付的改动必须先通过本脚本；人记不得跑的检查不算门禁。

set -euo pipefail
cd "$(dirname "$0")/.."

step() { printf '\n==> %s\n' "$1"; }

step "[1/5] cargo fmt --check"
cargo fmt --all --check

step "[2/5] cargo clippy (-D warnings)"
cargo clippy --workspace --all-targets -- -D warnings

step "[3/5] cargo test (workspace)"
cargo test --workspace

step "[4/5] cargo build (workspace)"
cargo build --workspace

step "[5/5] frontend build (tsc + vite production build)"
(
  cd apps/desktop
  if [ ! -d node_modules ]; then
    echo "node_modules 不存在，先执行 npm install"
    npm install
  fi
  npm run build
)

printf '\n✅ 全部检查通过\n'
