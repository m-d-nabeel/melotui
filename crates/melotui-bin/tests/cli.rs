//! The binary as a command-line tool: run as a process, the way a shell runs
//! it.

use std::process::{Command, Output, Stdio};

fn melotui(args: &[&str]) -> Output {
    // Files go to a scratch home, so nothing touches the real config.
    let home = std::env::temp_dir().join(format!("melotui-cli-test-{}", std::process::id()));
    Command::new(env!("CARGO_BIN_EXE_melotui"))
        .args(args)
        .env("HOME", &home)
        .env("XDG_CONFIG_HOME", home.join("config"))
        .env("XDG_DATA_HOME", home.join("data"))
        .env("XDG_CACHE_HOME", home.join("cache"))
        .stdin(Stdio::null())
        .output()
        .expect("melotui runs")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn version() {
    for flag in ["--version", "-V"] {
        let out = melotui(&[flag]);
        assert!(out.status.success());
        assert_eq!(
            stdout(&out).trim(),
            format!("melotui {}", env!("CARGO_PKG_VERSION"))
        );
    }
}

#[test]
fn help() {
    for flag in ["--help", "-h"] {
        let out = melotui(&[flag]);
        assert!(out.status.success());
        let text = stdout(&out);
        for expected in [
            "Usage: melotui",
            "--config",
            "--no-restore",
            "--paths",
            "--version",
            "press ?",
        ] {
            assert!(
                text.contains(expected),
                "{expected:?} missing from:\n{text}"
            );
        }
    }
}

#[test]
fn an_unknown_argument_is_an_error_with_a_pointer_to_help() {
    let out = melotui(&["help"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(
        stderr(&out).contains("unexpected argument 'help'"),
        "{}",
        stderr(&out)
    );
    assert!(stderr(&out).contains("--help"));
}

#[test]
fn without_a_terminal_it_says_so_instead_of_failing_obscurely() {
    let out = melotui(&[]);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        stderr(&out).contains("needs an interactive terminal"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn a_config_file_named_but_missing_is_an_error() {
    let out = melotui(&["--config", "/nonexistent/melotui.toml", "--paths"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(stderr(&out).contains("cannot read /nonexistent/melotui.toml"));
}

#[test]
#[cfg(target_os = "linux")]
fn paths_lists_where_everything_lives() {
    let out = melotui(&["--paths"]);
    assert!(out.status.success(), "{}", stderr(&out));
    let text = stdout(&out);
    for name in ["config", "presets", "library", "session", "logs", "covers"] {
        assert!(
            text.lines().any(|line| line.starts_with(name)),
            "{name} missing from:\n{text}"
        );
    }
    assert!(
        text.contains("config/melotui/config.toml  (not there yet)"),
        "{text}"
    );
}
