//! The command-line grammar and dispatch (clap lives only in this crate).

#![expect(
    clippy::redundant_pub_crate,
    reason = "explicit pub(crate) documents the crate-wide visibility intent at each item"
)]

use std::path::PathBuf;

use clap::{Parser, Subcommand};

/// Claude Code plugin lifecycle CLI.
#[derive(Debug, Parser)]
#[command(name = "claudevs", version, about)]
pub(crate) struct Cli {
    /// What to do.
    #[command(subcommand)]
    pub command: Command,
}

/// The available subcommands.
#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    /// Run a plugin's case suite plus its declared native suites.
    Test {
        /// Only run cases whose name contains this substring.
        #[arg(long)]
        case: Option<String>,
        /// Run against a throwaway copy in the installed cache layout.
        #[arg(long)]
        installed: bool,
        /// Emit the machine-readable report instead of the human one.
        #[arg(long)]
        json: bool,
        /// The plugin directory.
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Convert a YAML case to its data-Lua form.
    Migrate {
        /// Replace the .yaml file with the .lua file instead of printing.
        #[arg(long)]
        write: bool,
        /// The YAML case file.
        file: PathBuf,
    },
    /// Validate, check wiring, then run the suite in both layouts.
    Check {
        /// Emit the machine-readable report instead of the human one.
        #[arg(long)]
        json: bool,
        /// The plugin directory.
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Report what this environment can and cannot do.
    Doctor {
        /// Emit the machine-readable report instead of the human one.
        #[arg(long)]
        json: bool,
        /// The plugin directory.
        #[arg(default_value = ".")]
        path: PathBuf,
    },
}

/// Runs the parsed command; returns the process exit code.
pub(crate) fn run(cli: Cli) -> i32 {
    match cli.command {
        Command::Test {
            case,
            installed,
            json,
            path,
        } => run_test(case, installed, json, &path),
        Command::Migrate { write, file } => run_migrate(write, &file),
        Command::Check { json, path } => run_check(json, &path),
        Command::Doctor { json, path } => run_doctor(json, &path),
    }
}

/// `claudevs test`.
fn run_test(case: Option<String>, installed: bool, json: bool, path: &std::path::Path) -> i32 {
    let options = claudevs::SuiteOptions { case_filter: case };
    let outcome = if installed {
        claudevs::run_suite_installed(path, &options)
    } else {
        claudevs::run_suite(path, &options)
    };
    match outcome {
        Ok(report) => {
            if json {
                match claudevs::render_json(&report) {
                    Ok(text) => println!("{text}"),
                    Err(error) => {
                        eprintln!("claudevs: {error}");
                        return 2;
                    }
                }
            } else {
                print!("{}", claudevs::render_human(&report));
            }
            claudevs::exit_code(&report)
        }
        Err(error) => {
            eprintln!("claudevs: {error}");
            2
        }
    }
}

/// `claudevs migrate`.
fn run_migrate(write: bool, file: &std::path::Path) -> i32 {
    match claudevs::case::migrate_to_lua(file) {
        Ok(lua_text) => {
            if write {
                let lua_path = file
                    .parent()
                    .unwrap_or_else(|| std::path::Path::new("."))
                    .join(format!(
                        "{}_test.lua",
                        file.file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("case")
                            .replace('-', "_")
                    ));
                if let Err(error) = std::fs::write(&lua_path, &lua_text) {
                    eprintln!("claudevs: write {}: {error}", lua_path.display());
                    return 2;
                }
                if let Err(error) = std::fs::remove_file(file) {
                    eprintln!("claudevs: remove {}: {error}", file.display());
                    return 2;
                }
                println!("{} -> {}", file.display(), lua_path.display());
            } else {
                print!("{lua_text}");
            }
            0
        }
        Err(error) => {
            eprintln!("claudevs: {error}");
            2
        }
    }
}

/// `claudevs check`.
fn run_check(json: bool, path: &std::path::Path) -> i32 {
    match claudevs::check::run(path) {
        Ok(report) => {
            if json {
                match claudevs::render_json(&report) {
                    Ok(text) => println!("{text}"),
                    Err(error) => {
                        eprintln!("claudevs: {error}");
                        return 2;
                    }
                }
            } else {
                print!("{}", claudevs::render_check_human(&report));
            }
            claudevs::check_exit_code(&report)
        }
        Err(error) => {
            eprintln!("claudevs: {error}");
            2
        }
    }
}

/// `claudevs doctor`.
fn run_doctor(json: bool, path: &std::path::Path) -> i32 {
    let diagnosis = claudevs::doctor::run(path);
    if json {
        match claudevs::render_json(&diagnosis) {
            Ok(text) => println!("{text}"),
            Err(error) => {
                eprintln!("claudevs: {error}");
                return 2;
            }
        }
    } else {
        print!("{}", claudevs::render_doctor_human(&diagnosis));
    }
    claudevs::doctor_exit_code(&diagnosis)
}

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used, reason = "tests unwrap known-valid fixtures")]
    #![expect(
        clippy::panic,
        reason = "let-else diagnostics in tests panic by design"
    )]

    use clap::Parser as _;

    use super::{Cli, Command};

    #[test]
    fn test_defaults_to_the_current_directory() {
        let Cli { command } = Cli::try_parse_from(["claudevs", "test"]).unwrap();
        let Command::Test {
            path,
            case,
            installed,
            json,
        } = command
        else {
            panic!("expected test");
        };
        assert_eq!(path, std::path::Path::new("."));
        assert!(case.is_none());
        assert!(!installed);
        assert!(!json);
    }

    #[test]
    fn the_installed_flag_parses_and_reaches_the_command() {
        // Without this the flag is only ever asserted absent, so swapping the
        // two dispatch arms — or renaming the flag — leaves the suite green.
        let Cli { command } =
            Cli::try_parse_from(["claudevs", "test", "--installed", "some/plugin"]).unwrap();
        let Command::Test {
            path,
            case,
            installed,
            json,
        } = command
        else {
            panic!("expected test");
        };
        assert!(installed);
        assert_eq!(path, std::path::Path::new("some/plugin"));
        assert!(case.is_none());
        assert!(!json);
    }

    #[test]
    fn migrate_requires_a_file() {
        assert!(Cli::try_parse_from(["claudevs", "migrate"]).is_err());
    }

    #[test]
    fn the_check_json_flag_parses_and_reaches_the_command() {
        // Without this the flag is only ever asserted absent, so swapping the
        // two dispatch arms — or renaming the flag — leaves the suite green.
        let Cli { command } =
            Cli::try_parse_from(["claudevs", "check", "--json", "some/plugin"]).unwrap();
        let Command::Check { json, path } = command else {
            panic!("expected check");
        };
        assert!(json);
        assert_eq!(path, std::path::Path::new("some/plugin"));
    }

    #[test]
    fn the_doctor_json_flag_parses_and_reaches_the_command() {
        // Without this the flag is only ever asserted absent, so swapping the
        // two dispatch arms — or renaming the flag — leaves the suite green.
        let Cli { command } =
            Cli::try_parse_from(["claudevs", "doctor", "--json", "some/plugin"]).unwrap();
        let Command::Doctor { json, path } = command else {
            panic!("expected doctor");
        };
        assert!(json);
        assert_eq!(path, std::path::Path::new("some/plugin"));
    }
}
