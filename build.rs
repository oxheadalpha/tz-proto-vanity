use std::process::Command;

fn main() {
    let output = Command::new("git")
        .arg("describe")
        .arg("--tags")
        .output()
        .expect("Failed to execute command");
    // If we set CARGO_PKG_VERSION this way, then it will override the default value, which is
    // taken from the `version` in Cargo.toml.
    println!(
        "cargo:rustc-env=CARGO_PKG_VERSION={}",
        String::from_utf8_lossy(&output.stdout)
    )
}
