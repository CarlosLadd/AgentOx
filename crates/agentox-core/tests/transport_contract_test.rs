use agentox_core::client::http_sse::HttpSseTransport;
use agentox_core::client::transport::{Transport, TransportCapabilities};
use agentox_core::error::TransportError;
use agentox_core::protocol::jsonrpc::{JsonRpcNotification, JsonRpcRequest};
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    thread,
    time::Duration,
};

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

fn read_http_request(stream: &mut TcpStream) -> Option<String> {
    let mut buf = vec![0_u8; 4096];
    let n = stream.read(&mut buf).ok()?;
    if n == 0 {
        return None;
    }
    Some(String::from_utf8_lossy(&buf[..n]).to_string())
}

fn write_http_response(stream: &mut TcpStream, content_type: &str, body: &str) {
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();
}

fn start_one_shot_server(body: String, content_type: &'static str) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind one-shot http server");
    let addr = listener.local_addr().expect("local addr");
    thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let _ = read_http_request(&mut stream);
            write_http_response(&mut stream, content_type, &body);
        }
    });
    format!("http://{}", addr)
}

#[tokio::test]
async fn test_http_sse_transport_json_response_contract() {
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "result": {"ok": true}
    })
    .to_string();
    let endpoint = start_one_shot_server(body, "application/json");
    let mut tx = HttpSseTransport::new(endpoint, Duration::from_secs(5));
    let resp = tx
        .request_raw("{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\"}")
        .await
        .expect("http transport should return response");
    assert!(resp.is_some());
}

#[tokio::test]
async fn test_http_sse_transport_sse_response_contract() {
    let payload = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "result": {"ok": true}
    })
    .to_string();
    let body = format!("event: message\ndata: {payload}\n\n");
    let endpoint = start_one_shot_server(body, "text/event-stream");
    let mut tx = HttpSseTransport::new(endpoint, Duration::from_secs(5));
    let resp = tx
        .request_raw("{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"ping\"}")
        .await
        .expect("http sse transport should parse first data event");
    assert_eq!(resp.unwrap_or_default(), payload);
}
