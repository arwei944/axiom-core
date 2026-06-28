#!/bin/sh
# install-hooks.sh - Install git hooks for axiom projects.
# Copies hooks/pre-commit and hooks/pre-push into .git/hooks/.
# Safe to run multiple times (idempotent).
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
GIT_DIR="$(git rev-parse --git-dir 2>/dev/null || echo ".git")"
HOOKS_DIR="$GIT_DIR/hooks"

mkdir -p "$HOOKS_DIR"

cp "$SCRIPT_DIR/pre-commit" "$HOOKS_DIR/pre-commit"
cp "$SCRIPT_DIR/pre-push" "$HOOKS_DIR/pre-push"

chmod +x "$HOOKS_DIR/pre-commit" "$HOOKS_DIR/pre-push"

echo "axiom hooks installed to $HOOKS_DIR"
