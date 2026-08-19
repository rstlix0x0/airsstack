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
}

/// Runs the parsed command; returns the process exit code.
pub(crate) fn run(cli: Cli) -> i32 {
    match cli.command {
        Command::Test { case, json, path } => {
            let options = claudevs::SuiteOptions { case_filter: case };
            match claudevs::run_suite(&path, &options) {
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
        Command::Migrate { write, file } => match claudevs::case::migrate_to_lua(&file) {
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
                    if let Err(error) = std::fs::remove_file(&file) {
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
        },
    }
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
        let Command::Test { path, case, json } = command else {
            panic!("expected test");
        };
        assert_eq!(path, std::path::Path::new("."));
        assert!(case.is_none());
        assert!(!json);
    }

    #[test]
    fn migrate_requires_a_file() {
        assert!(Cli::try_parse_from(["claudevs", "migrate"]).is_err());
    }
}
