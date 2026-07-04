use anyhow::Result;
use std::process::Command;

pub fn run(strict: bool, fix: bool) -> Result<()> {
    println!("🔍 运行预提交架构检查...");

    // 1. 检查 staging area 中的 Cargo.toml 变更
    let staged_files = get_staged_files()?;
    let cargo_toml_files: Vec<_> = staged_files
        .iter()
        .filter(|f| f.ends_with("Cargo.toml"))
        .collect();

    if cargo_toml_files.is_empty() {
        println!("✅ 没有检测到 Cargo.toml 变更，跳过架构检查");
        return Ok(());
    }

    println!(
        "📋 检测到 {} 个 Cargo.toml 文件变更:",
        cargo_toml_files.len()
    );
    for file in &cargo_toml_files {
        println!("  - {}", file);
    }

    // 2. 运行 archcheck 检查
    let mut cmd = Command::new("cargo");
    cmd.args(["run", "-p", "archcheck", "--"]);

    if strict {
        cmd.arg("--strict");
    }

    let output = cmd.output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);

        eprintln!("❌ 架构检查失败:");
        eprintln!("{}", stdout);
        if !stderr.is_empty() {
            eprintln!("{}", stderr);
        }

        eprintln!("\n💡 修复建议:");
        eprintln!("  1. 运行 `cargo run -p archcheck --` 查看详细报告");
        eprintln!("  2. 修复架构违规（移除未审计依赖 / 调整层方向 / 添加豁免）");
        eprintln!("  3. 重新运行 `cargo check` 验证修复");
        eprintln!("  4. 重新提交");

        if fix {
            eprintln!("\n🔧 自动修复模式尚未实现，请手动修复后重新提交");
        }

        std::process::exit(1);
    }

    println!("✅ 预提交架构检查通过");
    Ok(())
}

fn get_staged_files() -> Result<Vec<String>> {
    let output = Command::new("git")
        .args(["diff", "--cached", "--name-only", "--diff-filter=ACM"])
        .output()?;

    if !output.status.success() {
        anyhow::bail!("git diff --cached 失败");
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let files: Vec<String> = stdout
        .lines()
        .filter(|line| !line.is_empty())
        .map(|s| s.to_string())
        .collect();

    Ok(files)
}

pub fn install() -> Result<()> {
    let hook_dir = ".git/hooks";
    let hook_file = format!("{}/pre-commit", hook_dir);

    // 创建 hooks 目录（如果不存在）
    std::fs::create_dir_all(hook_dir)?;

    // 创建 pre-commit 钩子脚本
    let script = r#"#!/bin/bash
# Axiom Core 架构预提交钩子
# 自动检查 staging area 中的架构违规

set -e

echo "🔍 运行预提交架构检查..."

# 检查是否有 Cargo.toml 变更
STAGED_FILES=$(git diff --cached --name-only --diff-filter=ACM | grep -E 'Cargo\.toml$' || true)

if [ -z "$STAGED_FILES" ]; then
    echo "✅ 没有检测到 Cargo.toml 变更，跳过架构检查"
    exit 0
fi

echo "📋 检测到 Cargo.toml 文件变更:"
echo "$STAGED_FILES" | while read -r file; do
    echo "  - $file"
done

# 运行 archcheck 检查
if ! cargo run -p xtask -- gatecheck --strict; then
    echo ""
    echo "❌ 架构检查失败，请修复后再提交"
    echo ""
    echo "💡 修复建议:"
    echo "  1. 运行 'cargo run -p archcheck --' 查看详细报告"
    echo "  2. 修复架构违规（移除未审计依赖 / 调整层方向 / 添加豁免）"
    echo "  3. 运行 'cargo check' 验证修复"
    echo "  4. 重新提交"
    echo ""
    echo "⚠️  如需跳过检查（紧急情况），使用 'git commit --no-verify'"
    exit 1
fi

echo "✅ 预提交架构检查通过"
"#;

    std::fs::write(&hook_file, script)?;

    // 设置可执行权限（Unix-like 系统）
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&hook_file)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&hook_file, perms)?;
    }

    println!("✅ 预提交钩子已安装: {}", hook_file);
    println!("   现在每次 git commit 前会自动运行架构检查");
    println!("💡 如需跳过检查（紧急情况），使用: git commit --no-verify");

    Ok(())
}

pub fn uninstall() -> Result<()> {
    let hook_file = ".git/hooks/pre-commit";

    if std::path::Path::new(hook_file).exists() {
        // foxguard: ignore[rs/no-path-traversal] — hook_file is a fixed relative path
        std::fs::remove_file(hook_file)?;
        println!("✅ 预提交钩子已卸载: {}", hook_file);
    } else {
        println!("⚠️  预提交钩子不存在: {}", hook_file);
    }

    Ok(())
}
