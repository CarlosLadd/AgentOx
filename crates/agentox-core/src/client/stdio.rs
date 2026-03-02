//! Stdio transport — spawns a child process and communicates via stdin/stdout.

use crate::client::transport::Transport;
use crate::error::TransportError;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::process::{Child, ChildStdin, ChildStdout};

/// MCP transport over stdio — spawns a subprocess and pipes JSON-RPC messages.
pub struct StdioTransport {
    child: Child,
    /// Wrapped in Option so we can take() it during shutdown to explicitly close stdin.
    stdin: Option<BufWriter<ChildStdin>>,
    stdout: BufReader<ChildStdout>,
    /// The original command string, stored for reconnection.
    command: String,
}

impl StdioTransport {
    /// Spawn a child process from a shell command string.
    ///
    /// The command is split using shell word-splitting rules (handles quotes).
    /// stdin and stdout are piped for JSON-RPC communication; stderr is inherited.
    pub async fn spawn(command: &str) -> Result<Self, TransportError> {
        let words =
            shell_words::split(command).map_err(|e| TransportError::CommandParse(e.to_string()))?;

        if words.is_empty() {
            return Err(TransportError::CommandParse("empty command".to_string()));
        }

        let program = &words[0];
        let args = &words[1..];

        tracing::debug!(program = %program, args = ?args, "spawning MCP server process");

        let mut child = tokio::process::Command::new(program)
            .args(args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::inherit())
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
        })
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

    /// Internal helper — read one line from the child's stdout.
    async fn read_line(&mut self) -> Result<Option<String>, TransportError> {
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
