fn main() {
    println!("cargo:rerun-if-changed=.git/HEAD");
    let suffix = build_suffix().unwrap_or_default();
    println!("cargo:rustc-env=PRATDIFF_VERSION_SUFFIX={suffix}");
}

fn build_suffix() -> Option<String> {
    let describe = git_output(&["describe", "--tags", "--long", "--always", "--dirty"])?;
    // %cs gives the committer date as YYYY-MM-DD, reproducible regardless of build time
    let date = git_output(&["log", "-1", "--format=%cs"])?;
    Some(format!(" ({describe}, built {date})"))
}

fn git_output(args: &[&str]) -> Option<String> {
    let out = std::process::Command::new("git").args(args).output().ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8(out.stdout).ok()?.trim().to_string())
}
