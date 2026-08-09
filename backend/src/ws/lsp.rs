use std::io::{self, Read, Write};

use rsground_runner::{error::RunnerError, Child, Runner};
use tokio::sync::mpsc;

/// Keep a single LSP frame bounded even though the WebSocket itself accepts
/// larger project messages. Rust Analyzer responses are expected to be small
/// enough for an editor notification, and this prevents a malformed server
/// frame from allocating unbounded memory.
pub const MAX_LSP_MESSAGE_BYTES: usize = 1 << 20;
const MAX_LSP_HEADER_BYTES: usize = 8 << 10;
const LSP_WRITE_QUEUE_CAPACITY: usize = 64;

pub struct LspProcess {
    pub child: Child,
    pub outgoing: mpsc::Sender<String>,
}

impl LspProcess {
    pub fn start(runner: &Runner, incoming: mpsc::Sender<String>) -> Result<Self, RunnerError> {
        let (child, stdin, stdout, stderr) = runner.start_rust_analyzer()?;
        let (outgoing, mut outgoing_rx) = mpsc::channel::<String>(LSP_WRITE_QUEUE_CAPACITY);

        tokio::task::spawn_blocking(move || {
            let mut stdin = stdin;

            while let Some(message) = outgoing_rx.blocking_recv() {
                if let Err(error) = write_lsp_message(&mut stdin, &message) {
                    log::debug!("Rust Analyzer stdin closed: {error}");
                    break;
                }
            }
        });

        tokio::task::spawn_blocking(move || {
            let mut stdout = stdout;

            loop {
                match read_lsp_message(&mut stdout) {
                    Ok(Some(message)) => {
                        if incoming.blocking_send(message).is_err() {
                            break;
                        }
                    }
                    Ok(None) => break,
                    Err(error) => {
                        log::debug!("Rust Analyzer stdout closed: {error}");
                        break;
                    }
                }
            }
        });

        tokio::task::spawn_blocking(move || {
            let mut stderr = stderr;
            let mut buffer = [0; 4096];

            loop {
                match stderr.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(size) => {
                        log::debug!(
                            "Rust Analyzer stderr: {}",
                            String::from_utf8_lossy(&buffer[..size]).trim_end()
                        );
                    }
                    Err(error) => {
                        log::debug!("Rust Analyzer stderr closed: {error}");
                        break;
                    }
                }
            }
        });

        Ok(Self { child, outgoing })
    }
}

fn write_lsp_message(writer: &mut impl Write, message: &str) -> io::Result<()> {
    let content = message.as_bytes();
    if content.len() > MAX_LSP_MESSAGE_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "LSP message exceeds the maximum size",
        ));
    }

    write!(writer, "Content-Length: {}\r\n\r\n", content.len())?;
    writer.write_all(content)?;
    writer.flush()
}

fn read_lsp_message(reader: &mut impl Read) -> io::Result<Option<String>> {
    let mut header = Vec::with_capacity(128);
    let mut byte = [0; 1];

    loop {
        let size = reader.read(&mut byte)?;
        if size == 0 {
            return if header.is_empty() {
                Ok(None)
            } else {
                Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "truncated LSP header",
                ))
            };
        }

        header.push(byte[0]);
        if header.ends_with(b"\r\n\r\n") {
            break;
        }
        if header.len() > MAX_LSP_HEADER_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "LSP header exceeds the maximum size",
            ));
        }
    }

    let header = std::str::from_utf8(&header).map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid LSP header: {error}"),
        )
    })?;

    let content_length = header
        .split("\r\n")
        .filter_map(|line| line.split_once(':'))
        .find_map(|(name, value)| {
            name.eq_ignore_ascii_case("content-length")
                .then_some(value.trim())
        })
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "LSP frame has no length"))?
        .parse::<usize>()
        .map_err(|error| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid LSP content length: {error}"),
            )
        })?;

    if content_length > MAX_LSP_MESSAGE_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "LSP message exceeds the maximum size",
        ));
    }

    let mut content = vec![0; content_length];
    reader.read_exact(&mut content)?;
    String::from_utf8(content)
        .map(Some)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::{read_lsp_message, write_lsp_message};

    #[test]
    fn round_trips_lsp_frames() {
        let message = r#"{"jsonrpc":"2.0","id":1}"#;
        let mut frame = Vec::new();

        write_lsp_message(&mut frame, message).unwrap();

        assert_eq!(
            read_lsp_message(&mut Cursor::new(frame)).unwrap(),
            Some(message.into())
        );
    }

    #[test]
    fn accepts_additional_headers_and_mixed_case() {
        let frame = b"content-length: 2\r\nContent-Type: application/json\r\n\r\n{}";

        assert_eq!(
            read_lsp_message(&mut Cursor::new(frame)).unwrap(),
            Some("{}".into())
        );
    }
}
