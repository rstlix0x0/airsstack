//! Controllable child process for `agent::process` and `CliRuntime` wiring
//! tests.
//!
//! Flags:
//!   (default)          echo each stdin line to stdout; exit 0 on EOF.
//!   --ignore-eof       never exit on stdin EOF; sleep forever.
//!   --spam-stderr      write ~256 KiB to stderr, then behave per other flags.
//!   --exit-code `<N>`  exit with code N on stdin EOF.
//!   --fork-grandchild  spawn a detached `--ignore-eof` copy of self (same
//!                      process group), then behave per other flags.
//!   --init-caps `<csv>` answer the pending `initialize` control request read
//!                      off stdin, then emit two `system`/`init` message
//!                      frames: the first carries `capabilities` (`csv`
//!                      split on `,`), the second omits the key — for tests
//!                      that drive `CliRuntime::connect` end to end. Falls
//!                      through to the default echo loop afterward.

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
    let init_caps = parse_init_caps(&args);

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

    let stdin = std::io::stdin();
    let mut handle = stdin.lock();

    if let Some(caps) = init_caps {
        emit_handshake_then_init(&mut handle, &caps);
    }

    if ignore_eof {
        loop {
            thread::sleep(Duration::from_secs(3600));
        }
    }

    // Default / exit-code path: echo stdin lines until EOF, then exit.
    for line in handle.lines() {
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

/// Value of `--init-caps <csv>`, if present.
fn parse_init_caps(args: &[String]) -> Option<String> {
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if arg == "--init-caps" {
            return iter.next().cloned();
        }
    }
    None
}

/// Read the one pending `initialize` `control_request` line off `handle`,
/// answer it with a matching `control_response`, then write two
/// `system`/`init` message frames: the first carrying `caps` (comma-split)
/// as its `capabilities` list, the second with no `capabilities` key at all.
fn emit_handshake_then_init(handle: &mut impl BufRead, caps: &str) {
    let mut line = String::new();
    if handle.read_line(&mut line).unwrap_or(0) == 0 {
        return;
    }
    let request_id = extract_request_id(&line).unwrap_or_default();

    let mut out = std::io::stdout();
    let response = format!(
        r#"{{"type":"control_response","response":{{"subtype":"success","request_id":"{request_id}","response":{{}}}}}}"#
    );
    let _ = writeln!(out, "{response}");

    let items: Vec<String> = caps
        .split(',')
        .filter(|s| !s.is_empty())
        .map(|s| format!("\"{s}\""))
        .collect();
    let first = format!(
        r#"{{"type":"system","subtype":"init","session_id":"s1","capabilities":[{}]}}"#,
        items.join(",")
    );
    let _ = writeln!(out, "{first}");

    let second = r#"{"type":"system","subtype":"init","session_id":"s2"}"#;
    let _ = writeln!(out, "{second}");
    let _ = out.flush();
}

/// Pull `request_id` out of a raw `control_request` line without a JSON
/// parser: scan for the `"request_id":"` marker and take the quoted value
/// that follows. Good enough for a test-only fixture that always receives
/// well-formed frames from this crate's own encoder.
fn extract_request_id(line: &str) -> Option<String> {
    let key = "\"request_id\":\"";
    let start = line.find(key)? + key.len();
    let end = line[start..].find('"')? + start;
    Some(line[start..end].to_string())
}

#[cfg(test)]
mod tests {
    use super::{extract_request_id, parse_init_caps};

    #[test]
    fn extract_request_id_reads_the_quoted_value() {
        let line =
            r#"{"type":"control_request","request_id":"req_7","request":{"subtype":"initialize"}}"#;
        assert_eq!(extract_request_id(line).as_deref(), Some("req_7"));
    }

    #[test]
    fn extract_request_id_is_none_when_the_key_is_absent() {
        let line = r#"{"type":"control_request","request":{"subtype":"initialize"}}"#;
        assert!(extract_request_id(line).is_none());
    }

    #[test]
    fn parse_init_caps_reads_the_following_value() {
        let args = vec!["--init-caps".to_string(), "a,b".to_string()];
        assert_eq!(parse_init_caps(&args).as_deref(), Some("a,b"));
    }

    #[test]
    fn parse_init_caps_is_none_when_the_flag_is_absent() {
        let args = vec!["--ignore-eof".to_string()];
        assert!(parse_init_caps(&args).is_none());
    }
}
