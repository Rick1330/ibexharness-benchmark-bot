use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_ibex-benchmark-bot");

fn run(args: &[&str]) -> String {
    let output = Command::new(BIN)
        .args(args)
        .output()
        .expect("spawn ibex-benchmark-bot");
    assert!(
        output.status.success(),
        "{args:?} exited with {:?}",
        output.status
    );
    String::from_utf8(output.stdout).expect("stdout is utf-8")
}

#[test]
fn build_info_prints_machine_readable_provenance() {
    let stdout = run(&["build-info"]);
    let mut name = None;
    let mut version = None;
    let mut build_sha = None;
    for line in stdout.lines() {
        let (key, value) = line.split_once('=').unwrap_or_else(|| {
            panic!("build-info line is not key=value: {line:?}");
        });
        match key {
            "name" => name = Some(value),
            "version" => version = Some(value),
            "build_sha" => build_sha = Some(value),
            other => panic!("unexpected build-info key: {other}"),
        }
    }
    assert_eq!(name, Some("ibex-benchmark-bot"));
    assert_eq!(version, Some(env!("CARGO_PKG_VERSION")));
    assert!(build_sha.is_some_and(|sha| !sha.is_empty()));
}

#[test]
fn version_flag_reports_the_same_build_sha_as_build_info() {
    let build_sha = run(&["build-info"])
        .lines()
        .find_map(|line| line.strip_prefix("build_sha=").map(str::to_string))
        .expect("build_sha line");
    let version = run(&["--version"]);
    assert!(
        version.contains(env!("CARGO_PKG_VERSION")),
        "--version missing crate version: {version:?}"
    );
    assert!(
        version.contains(&build_sha),
        "--version missing build sha {build_sha}: {version:?}"
    );
}
