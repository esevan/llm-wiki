#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 2 || $# -gt 3 ]]; then
  echo "usage: $0 <task-name> <branch-name> [start-point]" >&2
  exit 2
fi

task_name=$1
branch_name=$2
start_point=${3:-main}
start_commit=$(git rev-parse --verify "$start_point^{commit}")

if [[ ! $task_name =~ ^[a-z0-9][a-z0-9-]*$ ]]; then
  echo "task-name must contain lowercase letters, numbers, and hyphens" >&2
  exit 2
fi

common_git_dir=$(git rev-parse --path-format=absolute --git-common-dir)
repository_root=$(dirname "$common_git_dir")
worktree_path="$repository_root/.worktrees/$task_name"
script_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)

if [[ -e $worktree_path ]]; then
  echo "worktree path already exists: $worktree_path" >&2
  exit 1
fi

for required_cache in node_modules src-tauri/target src-tauri/resources/embedding-model dist; do
  if [[ ! -d "$repository_root/$required_cache" ]]; then
    echo "shared cache is not primed: $repository_root/$required_cache" >&2
    echo "prepare dependencies once in the primary checkout, then retry" >&2
    exit 1
  fi
done

node "$script_dir/prepare_embedding_model.mjs" \
  --check-only \
  --model-dir "$repository_root/src-tauri/resources/embedding-model"

git -C "$repository_root" worktree add "$worktree_path" -b "$branch_name" "$start_commit"
ln -s "$repository_root/node_modules" "$worktree_path/node_modules"
ln -s "$repository_root/src-tauri/target" "$worktree_path/src-tauri/target"
cp -R "$repository_root/dist" "$worktree_path/dist"

for asset in "$repository_root"/src-tauri/resources/embedding-model/*; do
  asset_name=$(basename "$asset")
  if [[ $asset_name == manifest.json || $asset_name == NOTICE.md ]]; then
    continue
  fi
  ln -s "$asset" "$worktree_path/src-tauri/resources/embedding-model/$asset_name"
done

echo "created cached worktree: $worktree_path"
echo "shared node_modules: $(readlink "$worktree_path/node_modules")"
echo "shared Cargo target: $(readlink "$worktree_path/src-tauri/target")"
echo "copied frontend build: $worktree_path/dist"
