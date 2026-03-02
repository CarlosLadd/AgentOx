//! Stdio transport — spawns a child process and communicates via stdin/stdout.

use crate::client::transport::Transport;
use crate::error::TransportError;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::process::{Child, ChildStdin, ChildStdout};

/// Default read timeout for waiting on server responses.
const DEFAULT_READ_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

/// MCP transport over stdio — spawns a subprocess and pipes JSON-RPC messages.
pub struct StdioTransport {
    child: Child,
    /// Wrapped in Option so we can take() it during shutdown to explicitly close stdin.
    stdin: Option<BufWriter<ChildStdin>>,
    stdout: BufReader<ChildStdout>,
    /// The original command string, stored for reconnection.
    command: String,
    /// Maximum time to wait for a response line from the server.
    read_timeout: std::time::Duration,
}

impl StdioTransport {
    /// Spawn a child process from a shell command string.
    ///
    /// The command is split using shell word-splitting rules (handles quotes).
    /// stdin and stdout are piped for JSON-RPC communication; stderr is inherited
    /// so the user can see server logs.
    pub async fn spawn(command: &str) -> Result<Self, TransportError> {
        Self::spawn_inner(command, false).await
    }

    /// Spawn a child process with stderr suppressed.
    ///
    /// Used for disposable sessions where server startup messages would clutter
    /// the terminal output.
    pub async fn spawn_quiet(command: &str) -> Result<Self, TransportError> {
        Self::spawn_inner(command, true).await
    }

    /// Internal spawn implementation.
    async fn spawn_inner(command: &str, quiet: bool) -> Result<Self, TransportError> {
        let words =
            shell_words::split(command).map_err(|e| TransportError::CommandParse(e.to_string()))?;

        if words.is_empty() {
            return Err(TransportError::CommandParse("empty command".to_string()));
        }

        let program = &words[0];
        let args = &words[1..];

        tracing::debug!(program = %program, args = ?args, quiet, "spawning MCP server process");

        let stderr_cfg = if quiet {
            std::process::Stdio::null()
        } else {
            std::process::Stdio::inherit()
        };

        let mut child = tokio::process::Command::new(program)
            .args(args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(stderr_cfg)
            .spawn()
            .map_err(TransportError::Io)?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| TransportError::CommandParse("failed to capture stdin".to_string()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| TransportError::CommandParse("failed to capture stdout".to_string()))?;

        Ok(Self {
            child,
            stdin: Some(BufWriter::new(stdin)),
            stdout: BufReader::new(stdout),
            command: command.to_string(),
            read_timeout: DEFAULT_READ_TIMEOUT,
        })
    }

    /// Set the read timeout for this transport.
    pub fn set_read_timeout(&mut self, timeout: std::time::Duration) {
        self.read_timeout = timeout;
    }

    /// Get the original command string (for reconnection).
    pub fn command(&self) -> &str {
        &self.command
    }

    /// Internal helper — write a line to the child's stdin.
    async fn write_line(&mut self, message: &str) -> Result<(), TransportError> {
        let stdin = self
            .stdin
            .as_mut()
            .ok_or_else(|| TransportError::ProcessExit("stdin already closed".to_string()))?;

        stdin
            .write_all(message.as_bytes())
            .await
            .map_err(TransportError::Io)?;
        stdin.write_all(b"\n").await.map_err(TransportError::Io)?;
        stdin.flush().await.map_err(TransportError::Io)?;

        Ok(())
    }

    /// Internal helper — read one line from the child's stdout, with timeout.
    async fn read_line(&mut self) -> Result<Option<String>, TransportError> {
        let read_future = async {
            let mut line = String::new();
            let bytes_read = self
                .stdout
                .read_line(&mut line)
                .await
                .map_err(TransportError::Io)?;

            if bytes_read == 0 {
                return Err(TransportError::ProcessExit(
                    "server closed stdout".to_string(),
                ));
            }

            let trimmed = line.trim().to_string();
            if trimmed.is_empty() {
                return Ok(None);
            }

            Ok(Some(trimmed))
        };

        match tokio::time::timeout(self.read_timeout, read_future).await {
            Ok(result) => result,
            Err(_) => Err(TransportError::Timeout(self.read_timeout)),
        }
    }
}

#[async_trait::async_trait]
impl Transport for StdioTransport {
    async fn write_raw(&mut self, message: &str) -> Result<(), TransportError> {
        self.write_line(message).await
    }

    async fn request_raw(&mut self, message: &str) -> Result<Option<String>, TransportError> {
        self.write_line(message).await?;
        self.read_line().await
    }

    async fn shutdown(&mut self) -> Result<(), TransportError> {
        // Explicitly drop stdin to close the pipe and signal EOF to the child
        drop(self.stdin.take());

        // Give the process time to exit gracefully, then force-kill
        match tokio::time::timeout(std::time::Duration::from_secs(5), self.child.wait()).await {
            Ok(Ok(status)) => {
                tracing::debug!(status = %status, "server process exited");
                Ok(())
            }
            Ok(Err(e)) => Err(TransportError::Io(e)),
            Err(_) => {
                tracing::warn!("server process did not exit within 5s, sending SIGKILL");
                self.child.kill().await.map_err(TransportError::Io)?;
                Ok(())
            }
        }
    }
}
