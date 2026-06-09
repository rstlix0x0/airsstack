//! Controllable child process for `agent::process` lifecycle tests.
//!
//! Flags:
//!   (default)         echo each stdin line to stdout; exit 0 on EOF.
//!   --ignore-eof      never exit on stdin EOF; sleep forever.
//!   --spam-stderr     write ~256 KiB to stderr, then behave per other flags.
//!   --exit-code `<N>` exit with code N on stdin EOF.
//!   --fork-grandchild spawn a detached `--ignore-eof` copy of self (same
//!                     process group), then behave per other flags.

use std::env;
use std::io::{BufRead, Write};
use std::process::Command;
use std::thread;
use std::time::Duration;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let ignore_eof = args.iter().any(|a| a == "--ignore-eof");
    let spam_stderr = args.iter().any(|a| a == "--spam-stderr");
    let fork_grandchild = args.iter().any(|a| a == "--fork-grandchild");
    let exit_code = parse_exit_code(&args);

    if spam_stderr {
        let blob = vec![b'E'; 256 * 1024];
        let _ = std::io::stderr().write_all(&blob);
        let _ = std::io::stderr().flush();
    }

    if fork_grandchild {
        if let Ok(exe) = env::current_exe() {
            // Inherits our process group (no process_group call here), so a
            // group-kill of our group reaches this grandchild too.
            let _ = Command::new(exe).arg("--ignore-eof").spawn();
        }
    }

    if ignore_eof {
        loop {
            thread::sleep(Duration::from_secs(3600));
        }
    }

    // Default / exit-code path: echo stdin lines until EOF, then exit.
    let stdin = std::io::stdin();
    for line in stdin.lock().lines() {
        match line {
            Ok(text) => {
                let mut out = std::io::stdout();
                let _ = writeln!(out, "{text}");
                let _ = out.flush();
            }
            Err(_) => break,
        }
    }
    std::process::exit(exit_code);
}

fn parse_exit_code(args: &[String]) -> i32 {
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if arg == "--exit-code" {
            if let Some(code) = iter.next() {
                return code.parse().unwrap_or(0);
            }
        }
    }
    0
}
