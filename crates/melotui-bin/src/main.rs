mod input_processor;
mod logging;
#[cfg(target_os = "windows")]
mod media_window;
#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
mod mpris;
mod runtime;

use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::Parser;
use melotui_core::config::AppConfig;

/// A terminal music player with a spectrum visualizer, DSP effects, synced
/// lyrics and YouTube downloads.
#[derive(Debug, Parser)]
#[command(
    version,
    about,
    after_help = "Inside the player, press ? for the keys and : for commands."
)]
struct Cli {
    /// Read the configuration from FILE instead of the usual place
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Start with the built-in settings, reading no config file, theme or
    /// keymaps
    #[arg(long, conflicts_with = "config")]
    no_config: bool,

    /// Start without restoring the last session
    #[arg(long)]
    no_restore: bool,

    /// Print where melotui keeps its files, and exit
    #[arg(long)]
    paths: bool,
}

fn main() -> ExitCode {
    // Help, the version and mistakes in the arguments are answered here,
    // before anything is read or written.
    let cli = Cli::parse();

    let config_file = cli.config.clone().unwrap_or_else(AppConfig::config_path);
    let load = || match &cli.config {
        _ if cli.no_config => Ok(AppConfig::builtin()),
        Some(path) => AppConfig::load_from(path).map_err(|e| {
            eprintln!("melotui: cannot read {}: {e}", path.display());
            ExitCode::from(2)
        }),
        None => Ok(AppConfig::load()),
    };

    if cli.paths {
        return match load() {
            Ok(config) => {
                print_paths(&config, &config_file, cli.no_config);
                ExitCode::SUCCESS
            }
            Err(code) => code,
        };
    }

    if !std::io::stdin().is_terminal() || !std::io::stdout().is_terminal() {
        eprintln!("melotui: needs an interactive terminal to run in");
        return ExitCode::FAILURE;
    }

    let log_guard = logging::init();
    std::panic::set_hook(Box::new(|panic| {
        let _ = melotui_tui::restore_terminal();
        tracing::error!(
            "panic: {panic}\n{}",
            std::backtrace::Backtrace::force_capture()
        );
        // Not eprintln!, which panics itself if the terminal has gone.
        let _ = writeln!(std::io::stderr(), "panic: {panic}");
    }));

    let mut config = match load() {
        Ok(config) => config,
        Err(code) => return code,
    };
    if cli.no_restore {
        config.ui.restore_session = false;
    }

    match runtime::run(config) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("melotui: {e}");
            // Flush the buffered log writer before exiting, so the lines that
            // explain the failure are written.
            drop(log_guard);
            ExitCode::FAILURE
        }
    }
}

/// Every file and directory melotui reads or writes, one per line, noting
/// those that do not exist yet.
fn print_paths(config: &AppConfig, config_file: &Path, no_config: bool) {
    let config_dir = AppConfig::config_dir();
    let data_dir = AppConfig::data_dir();
    let cache_dir = dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("melotui");
    let mut rows: Vec<(&str, PathBuf)> = vec![
        ("config", config_file.to_path_buf()),
        ("themes", config_dir.join("themes")),
        ("keymaps", config_dir.join("keymaps.toml")),
        ("presets", config_dir.join("presets")),
        ("library", config.db_path()),
        ("session", data_dir.join("last_session.toml")),
        ("history", data_dir.join("command_history")),
        ("logs", logging::log_dir()),
        ("covers", cache_dir.join("covers")),
    ];
    rows.extend(
        config
            .resolve_music_dirs()
            .into_iter()
            .map(|dir| ("music", dir)),
    );
    let mut out = std::io::stdout().lock();
    for (name, path) in rows {
        let note = if no_config && matches!(name, "config" | "themes" | "keymaps") {
            "  (not read: --no-config)"
        } else if path.exists() {
            ""
        } else {
            "  (not there yet)"
        };
        // Stop quietly once whatever reads this has gone, as `| head` does,
        // instead of panicking the way println! would.
        if writeln!(out, "{name:<8} {}{note}", path.display()).is_err() {
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::error::ErrorKind;

    fn parse(args: &[&str]) -> Result<Cli, ErrorKind> {
        Cli::try_parse_from(std::iter::once("melotui").chain(args.iter().copied()))
            .map_err(|e| e.kind())
    }

    #[test]
    fn help_and_version_are_answered() {
        for args in [&["--help"][..], &["-h"]] {
            assert_eq!(parse(args).unwrap_err(), ErrorKind::DisplayHelp);
        }
        for args in [&["--version"][..], &["-V"]] {
            assert_eq!(parse(args).unwrap_err(), ErrorKind::DisplayVersion);
        }
    }

    #[test]
    fn anything_else_unknown_is_an_error() {
        for args in [&["help"][..], &["version"], &["--bogus"], &["-x"]] {
            assert!(
                matches!(
                    parse(args).unwrap_err(),
                    ErrorKind::UnknownArgument | ErrorKind::InvalidSubcommand
                ),
                "{args:?}"
            );
        }
    }

    #[test]
    fn options() {
        let cli = parse(&["--config", "/tmp/x.toml", "--no-restore"]).unwrap();
        assert_eq!(cli.config.as_deref(), Some(Path::new("/tmp/x.toml")));
        assert!(cli.no_restore && !cli.paths);
        assert!(parse(&["-c", "a.toml", "--paths"]).unwrap().paths);
        assert!(parse(&["--config"]).is_err(), "--config without a file");
        assert!(parse(&["--no-config"]).unwrap().no_config);
        assert_eq!(
            parse(&["--no-config", "--config", "a.toml"]).unwrap_err(),
            ErrorKind::ArgumentConflict
        );
    }

    #[test]
    fn the_definition_is_valid() {
        use clap::CommandFactory;
        Cli::command().debug_assert();
    }
}
