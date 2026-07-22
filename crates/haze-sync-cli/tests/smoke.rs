use std::process::Command;

const FORBIDDEN_MARKERS: &[&str] = &[
    "postgres://",
    "database_url",
    "db_url",
    "credential",
    "oauth",
    "redacted-test-value",
    "provider_payload",
    "/srv/",
    "C:\\",
    "backtrace",
    "stack",
    "token_hash",
];

fn run_cli(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_haze-sync"))
        .args(args)
        .output()
        .expect("CLI process should launch")
}

fn render_stdout(output: &std::process::Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout should be utf8")
}

fn render_stderr(output: &std::process::Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr should be utf8")
}

fn assert_no_sensitive_leaks(output: &str) {
    for forbidden in FORBIDDEN_MARKERS {
        assert!(
            !output.contains(forbidden),
            "CLI output leaked forbidden marker {forbidden}: {output}"
        );
    }
}

#[test]
fn top_level_help_smoke_is_read_only_and_safe() {
    let output = run_cli(&["--help"]);
    assert!(output.status.success());

    let rendered = render_stdout(&output);
    assert!(rendered.contains("usage: haze-sync [global options] <command>"));
    assert!(rendered.contains("doctor [--offline]"));
    assert!(rendered.contains("read-only"));
    assert_no_sensitive_leaks(&rendered);
}

#[test]
fn status_command_smoke_is_safe() {
    let output = run_cli(&["status"]);
    assert!(output.status.success());

    let rendered = render_stdout(&output);
    assert!(rendered.contains("status: not_configured"));
    assert!(rendered.contains("live server calls: not attempted"));
    assert_no_sensitive_leaks(&rendered);
}

#[test]
fn adapters_list_command_smoke_is_safe() {
    let output = run_cli(&["adapters", "list"]);
    assert!(output.status.success());

    let rendered = render_stdout(&output);
    assert!(rendered.contains("adapters: not_configured"));
    assert!(rendered.contains("live server calls: not attempted"));
    assert_no_sensitive_leaks(&rendered);
}

#[test]
fn doctor_command_smoke_is_safe_and_read_only() {
    let output = run_cli(&["doctor"]);
    assert!(output.status.success());

    let rendered = render_stdout(&output);
    assert!(rendered.contains("doctor summary"));
    assert!(rendered.contains("total: 4"));
    assert_no_sensitive_leaks(&rendered);
}

#[test]
fn doctor_help_smoke_is_safe() {
    let output = run_cli(&["doctor", "--help"]);
    assert!(output.status.success());

    let rendered = render_stdout(&output);
    assert!(rendered.contains("usage: haze-sync doctor [--offline]"));
    assert!(rendered.contains("read-only offline summary"));
    assert_no_sensitive_leaks(&rendered);
}

#[test]
fn invalid_command_exits_non_zero_without_sensitive_output() {
    let output = run_cli(&["tokens"]);
    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(2));

    let rendered = render_stderr(&output);
    assert!(rendered.contains("unknown command"));
    assert_no_sensitive_leaks(&rendered);
}
