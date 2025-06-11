use std::process::Command;

fn run_command_with_fallback(cmd: &str, args: &[&str], fallback: &str) -> String {
    Command::new(cmd)
        .args(args)
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| fallback.to_owned())
}

fn main() {
    let git_tag = run_command_with_fallback("git", &["describe", "--tags", "--abbrev=0"], "none");

    let git_commit = run_command_with_fallback("git", &["rev-parse", "HEAD"], "none");

    let dockerfile_hash = run_command_with_fallback(
        "sh",
        &["-c", "sha256sum Dockerfile | cut -d' ' -f1"],
        "none",
    );

    println!("cargo:rustc-env=GIT_TAG={}", git_tag);
    println!("cargo:rustc-env=GIT_COMMIT={}", git_commit);
    println!("cargo:rustc-env=DOCKERFILE_HASH={}", dockerfile_hash);
}
