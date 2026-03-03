//! HTTP/SSE transport placeholder for v0.3.
//!
//! This is intentionally non-functional in v0.2 and exists to stabilize
//! transport boundaries before full implementation.

use crate::client::transport::{Transport, TransportCapabilities};
use crate::error::TransportError;

pub struct HttpSseTransport {
    endpoint: String,
}

impl HttpSseTransport {
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
        }
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
        Err(TransportError::Unsupported(format!(
            "HTTP/SSE transport is not implemented yet (endpoint: {})",
            self.endpoint
        )))
    }

    async fn request_raw(&mut self, _message: &str) -> Result<Option<String>, TransportError> {
        Err(TransportError::Unsupported(format!(
            "HTTP/SSE transport is not implemented yet (endpoint: {})",
            self.endpoint
        )))
    }

    async fn shutdown(&mut self) -> Result<(), TransportError> {
        Ok(())
    }
}
