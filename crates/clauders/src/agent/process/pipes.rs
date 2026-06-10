use std::sync::{Arc, Mutex};

use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader, Lines};
use tokio::process::{ChildStderr, ChildStdout};

/// Upper bound on retained stderr bytes (last N kept).
const STDERR_CAP: usize = 64 * 1024;

/// Append `data` to `buf`, keeping at most the last `cap` bytes.
fn append_bounded(buf: &mut Vec<u8>, data: &[u8], cap: usize) {
    buf.extend_from_slice(data);
    let overflow = buf.len().saturating_sub(cap);
    if overflow > 0 {
        buf.drain(0..overflow);
    }
}

/// Line-oriented view over a child's stdout.
pub struct StdoutLines(Lines<BufReader<ChildStdout>>);

impl StdoutLines {
    pub(crate) fn new(stdout: ChildStdout) -> Self {
        Self(BufReader::new(stdout).lines())
    }

    /// Read the next line (without the trailing newline), or `None` at EOF.
    ///
    /// # Errors
    /// Returns the underlying I/O error if reading stdout fails.
    pub async fn next_line(&mut self) -> std::io::Result<Option<String>> {
        self.0.next_line().await
    }
}

/// A bounded, continuously-drained snapshot of a child's stderr.
///
/// A background task reads stderr to EOF so the child can never block on a
/// full stderr pipe; only the most recent 64 KiB are retained.
#[derive(Clone)]
pub struct StderrBuffer {
    inner: Arc<Mutex<Vec<u8>>>,
}

impl StderrBuffer {
    /// Spawn the drain task and return a handle to the captured bytes.
    pub(crate) fn drain(mut stderr: ChildStderr) -> Self {
        let inner = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&inner);
        tokio::spawn(async move {
            let mut chunk = [0u8; 4096];
            loop {
                match stderr.read(&mut chunk).await {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        if let Ok(mut guard) = sink.lock() {
                            append_bounded(&mut guard, &chunk[..n], STDERR_CAP);
                        }
                    }
                }
            }
        });
        Self { inner }
    }

    /// Current captured stderr as a lossy UTF-8 string.
    #[must_use]
    pub fn snapshot(&self) -> String {
        self.inner
            .lock()
            .map(|guard| String::from_utf8_lossy(&guard).into_owned())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::append_bounded;

    #[test]
    fn append_bounded_keeps_only_the_last_cap_bytes() {
        let mut buf = Vec::new();
        append_bounded(&mut buf, b"hello", 4);
        assert_eq!(buf, b"ello");

        append_bounded(&mut buf, b"XY", 4);
        assert_eq!(buf, b"loXY");

        append_bounded(&mut buf, b"", 4);
        assert_eq!(buf, b"loXY");
    }

    #[test]
    fn append_bounded_under_cap_appends_all() {
        let mut buf = Vec::new();
        append_bounded(&mut buf, b"ab", 8);
        append_bounded(&mut buf, b"cd", 8);
        assert_eq!(buf, b"abcd");
    }
}
