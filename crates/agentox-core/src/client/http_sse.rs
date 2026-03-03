//! HTTP/SSE transport beta implementation.

use crate::client::transport::{Transport, TransportCapabilities};
use crate::error::TransportError;
use reqwest::header::CONTENT_TYPE;
use std::time::Duration;

pub struct HttpSseTransport {
    endpoint: String,
    client: reqwest::Client,
    timeout: Duration,
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
        }
    }

    async fn post_raw(&self, message: &str) -> Result<reqwest::Response, TransportError> {
        self.client
            .post(&self.endpoint)
            .header(CONTENT_TYPE, "application/json")
            .body(message.to_string())
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    TransportError::Timeout(self.timeout)
                } else {
                    TransportError::Http(e.to_string())
                }
            })
    }

    fn parse_sse_first_data(body: &str) -> Option<String> {
        for line in body.lines() {
            let line = line.trim();
            if let Some(data) = line.strip_prefix("data:") {
                let payload = data.trim();
                if !payload.is_empty() {
                    return Some(payload.to_string());
                }
            }
        }
        None
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

    async fn write_raw(&mut self, _message: &str) -> Result<(), TransportError> {
        let resp = self.post_raw(_message).await?;
        if !resp.status().is_success() {
            return Err(TransportError::Http(format!(
                "HTTP status {} for notification",
                resp.status()
            )));
        }
        Ok(())
    }

    async fn request_raw(&mut self, message: &str) -> Result<Option<String>, TransportError> {
        let resp = self.post_raw(message).await?;
        if !resp.status().is_success() {
            return Err(TransportError::Http(format!(
                "HTTP status {} for request",
                resp.status()
            )));
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
            .map_err(|e| TransportError::Http(e.to_string()))?;
        if body.trim().is_empty() {
            return Ok(None);
        }

        if content_type.contains("text/event-stream") {
            let payload = Self::parse_sse_first_data(&body).ok_or(TransportError::NoResponse)?;
            return Ok(Some(payload));
        }

        Ok(Some(body.trim().to_string()))
    }

    async fn shutdown(&mut self) -> Result<(), TransportError> {
        Ok(())
    }
}
