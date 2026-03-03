use agentox_core::client::transport::{Transport, TransportCapabilities};
use agentox_core::error::TransportError;
use agentox_core::protocol::jsonrpc::{JsonRpcNotification, JsonRpcRequest};

struct FakeTransport {
    next_response: Option<String>,
    wrote_messages: Vec<String>,
    caps: TransportCapabilities,
}

impl FakeTransport {
    fn with_response(response: Option<String>) -> Self {
        Self {
            next_response: response,
            wrote_messages: Vec::new(),
            caps: TransportCapabilities {
                request_response: true,
                streaming_notifications: false,
            },
        }
    }
}

#[async_trait::async_trait]
impl Transport for FakeTransport {
    fn capabilities(&self) -> TransportCapabilities {
        self.caps
    }

    async fn write_raw(&mut self, message: &str) -> Result<(), TransportError> {
        self.wrote_messages.push(message.to_string());
        Ok(())
    }

    async fn request_raw(&mut self, message: &str) -> Result<Option<String>, TransportError> {
        self.wrote_messages.push(message.to_string());
        Ok(self.next_response.take())
    }

    async fn shutdown(&mut self) -> Result<(), TransportError> {
        Ok(())
    }
}

#[tokio::test]
async fn test_transport_default_send_request_contract() {
    let mut tx = FakeTransport::with_response(Some(
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 7,
            "result": {"ok": true}
        })
        .to_string(),
    ));
    let req = JsonRpcRequest::new(7, "tools/list", Some(serde_json::json!({})));
    let resp = tx
        .send_request(&req)
        .await
        .expect("send_request should parse");
    assert!(resp.result.is_some());
    assert_eq!(tx.wrote_messages.len(), 1);
}

#[tokio::test]
async fn test_transport_default_send_notification_contract() {
    let mut tx = FakeTransport::with_response(None);
    let notif = JsonRpcNotification::new("notifications/initialized", None);
    tx.send_notification(&notif)
        .await
        .expect("notification should write");
    assert_eq!(tx.wrote_messages.len(), 1);
}

#[tokio::test]
async fn test_transport_default_send_raw_contract() {
    let mut tx = FakeTransport::with_response(Some("{\"jsonrpc\":\"2.0\"}".to_string()));
    let resp = tx
        .send_raw("{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\"}")
        .await
        .expect("send_raw should proxy request_raw");
    assert!(resp.is_some());
}

#[tokio::test]
async fn test_transport_capabilities_contract() {
    let tx = FakeTransport::with_response(None);
    let caps = tx.capabilities();
    assert!(caps.request_response);
    assert!(!caps.streaming_notifications);
}
