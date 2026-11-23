use std::{env, path::Path, process::Command};

fn git(args: &[&str]) -> Option<String> {
    let root = Path::new("../../..");
    if !root.join(".git").exists() {
        return None;
    }
    let result = Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .ok()?;
    result.status.success().then_some(())?;
    Some(String::from_utf8(result.stdout).ok()?.trim().to_owned())
}

fn main() {
    println!("cargo:rerun-if-env-changed=OTERYN_BUILD_COMMIT");
    if let Some(git_path) = git(&["rev-parse", "--path-format=absolute", "--git-path", "HEAD"]) {
        println!("cargo:rerun-if-changed={git_path}");
        if let Some(reference) = git(&["symbolic-ref", "-q", "HEAD"])
            && let Some(path) = git(&[
                "rev-parse",
                "--path-format=absolute",
                "--git-path",
                &reference,
            ])
        {
            let reference = Path::new(&path);
            if reference.is_file() {
                println!("cargo:rerun-if-changed={path}");
            } else {
                if let Some(directory) = reference.ancestors().find(|path| path.is_dir()) {
                    println!("cargo:rerun-if-changed={}", directory.display());
                }
                if let Some(packed) = git(&[
                    "rev-parse",
                    "--path-format=absolute",
                    "--git-path",
                    "packed-refs",
                ]) && Path::new(&packed).is_file()
                {
                    println!("cargo:rerun-if-changed={packed}");
                }
            }
        }
    }
    let valid_commit =
        |commit: &String| commit.len() == 40 && commit.bytes().all(|byte| byte.is_ascii_hexdigit());
    let commit = env::var("OTERYN_BUILD_COMMIT")
        .ok()
        .filter(valid_commit)
        .or_else(|| git(&["rev-parse", "--verify", "HEAD"]).filter(valid_commit))
        .unwrap_or_default();
    println!(
        "cargo:rustc-env=OTERYN_SYSTEM_COMMIT={}",
        commit.to_ascii_lowercase()
    );
}
