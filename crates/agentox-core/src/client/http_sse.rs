//! HTTP/SSE transport beta implementation.

use crate::client::transport::{Transport, TransportCapabilities};
use crate::error::TransportError;
use reqwest::header::CONTENT_TYPE;
use std::time::Duration;

pub struct HttpSseTransport {
    endpoint: String,
    client: reqwest::Client,
    timeout: Duration,
    max_retries: usize,
}

impl HttpSseTransport {
    pub fn new(endpoint: impl Into<String>, timeout: Duration) -> Self {
        let timeout = if timeout.is_zero() {
            Duration::from_secs(30)
        } else {
            timeout
        };
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            endpoint: endpoint.into(),
            client,
            timeout,
            max_retries: 2,
        }
    }

    async fn post_raw_once(&self, message: &str) -> Result<reqwest::Response, TransportError> {
        let resp = self
            .client
            .post(&self.endpoint)
            .header(CONTENT_TYPE, "application/json")
            .body(message.to_string())
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    TransportError::Timeout(self.timeout)
                } else {
                    TransportError::Http(format!("transport request failed: {e}"))
                }
            })?;
        Ok(resp)
    }

    fn parse_sse_first_data(body: &str) -> Option<String> {
        let mut data_lines: Vec<String> = Vec::new();
        for line in body.lines() {
            let line = line.trim();
            if line.is_empty() {
                if !data_lines.is_empty() {
                    return Some(data_lines.join("\n"));
                }
                continue;
            }
            if line.starts_with(':') {
                continue;
            }
            if let Some(data) = line.strip_prefix("data:") {
                data_lines.push(data.trim().to_string());
            }
        }
        if data_lines.is_empty() {
            None
        } else {
            Some(data_lines.join("\n"))
        }
    }

    fn is_retryable_status(status: reqwest::StatusCode) -> bool {
        matches!(
            status,
            reqwest::StatusCode::BAD_GATEWAY
                | reqwest::StatusCode::SERVICE_UNAVAILABLE
                | reqwest::StatusCode::GATEWAY_TIMEOUT
        )
    }

    fn maybe_retry_delay(attempt: usize) -> Duration {
        Duration::from_millis(50 * (attempt as u64 + 1))
    }

    async fn execute_with_retry(
        &self,
        message: &str,
        expect_response_body: bool,
    ) -> Result<Option<String>, TransportError> {
        let mut last_err: Option<TransportError> = None;

        for attempt in 0..=self.max_retries {
            let response = self.post_raw_once(message).await;
            match response {
                Ok(resp) => {
                    let status = resp.status();
                    if status.is_client_error() {
                        return Err(TransportError::Http(format!(
                            "HTTP client error {}",
                            status.as_u16()
                        )));
                    }
                    if !status.is_success() {
                        if Self::is_retryable_status(status) && attempt < self.max_retries {
                            tokio::time::sleep(Self::maybe_retry_delay(attempt)).await;
                            continue;
                        }
                        return Err(TransportError::Http(format!(
                            "HTTP server error {}",
                            status.as_u16()
                        )));
                    }

                    if !expect_response_body {
                        return Ok(None);
                    }

                    let content_type = resp
                        .headers()
                        .get(CONTENT_TYPE)
                        .and_then(|v| v.to_str().ok())
                        .unwrap_or("")
                        .to_ascii_lowercase();

                    let body = resp
                        .text()
                        .await
                        .map_err(|e| TransportError::Http(format!("response read failed: {e}")))?;
                    if body.trim().is_empty() {
                        return Ok(None);
                    }

                    let payload = if content_type.contains("text/event-stream") {
                        Self::parse_sse_first_data(&body).ok_or_else(|| {
                            TransportError::Http(
                                "SSE response contained no data event payload".to_string(),
                            )
                        })?
                    } else {
                        body.trim().to_string()
                    };

                    serde_json::from_str::<serde_json::Value>(&payload).map_err(|e| {
                        TransportError::Http(format!("response payload is not valid JSON: {e}"))
                    })?;
                    return Ok(Some(payload));
                }
                Err(e) => {
                    let retryable = matches!(e, TransportError::Timeout(_))
                        || matches!(&e, TransportError::Http(msg) if msg.contains("transport request failed"));
                    last_err = Some(e);
                    if retryable && attempt < self.max_retries {
                        tokio::time::sleep(Self::maybe_retry_delay(attempt)).await;
                        continue;
                    }
                }
            }
            break;
        }

        Err(last_err
            .unwrap_or_else(|| TransportError::Http("request failed after retries".to_string())))
    }
}

#[async_trait::async_trait]
impl Transport for HttpSseTransport {
    fn capabilities(&self) -> TransportCapabilities {
        TransportCapabilities {
            request_response: true,
            streaming_notifications: true,
        }
    }

    async fn write_raw(&mut self, message: &str) -> Result<(), TransportError> {
        self.execute_with_retry(message, false).await?;
        Ok(())
    }

    async fn request_raw(&mut self, message: &str) -> Result<Option<String>, TransportError> {
        self.execute_with_retry(message, true).await
    }

    async fn shutdown(&mut self) -> Result<(), TransportError> {
        Ok(())
    }
}
