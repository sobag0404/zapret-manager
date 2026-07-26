use std::env;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=../../.git/HEAD");
    println!("cargo:rerun-if-changed=../../.git/refs/heads/main");
    println!("cargo:rerun-if-changed=../../.git/index");
    println!("cargo:rerun-if-env-changed=ZAPRET_MANAGER_BUILD_ID");

    let build_id = explicit_build_id().unwrap_or_else(git_build_id);
    println!("cargo:rustc-env=ZAPRET_MANAGER_BUILD_ID={build_id}");

    tauri_build::build()
}

fn explicit_build_id() -> Option<String> {
    let value = env::var("ZAPRET_MANAGER_BUILD_ID").ok()?;
    let value = value.trim();
    if value.len() < 12 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    Some(value[..12].to_ascii_lowercase())
}

fn git_build_id() -> String {
    let commit =
        git_output(["rev-parse", "--short=12", "HEAD"]).unwrap_or_else(|| "unknown".to_string());
    let dirty = git_output(["status", "--porcelain"])
        .map(|status| !status.trim().is_empty())
        .unwrap_or(true);
    if dirty {
        format!("{commit}-dirty")
    } else {
        commit
    }
}

fn git_output<const N: usize>(args: [&str; N]) -> Option<String> {
    Command::new("git")
        .args(args)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .filter(|value| !value.is_empty())
}
