use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::OnceLock;
use std::thread;

use prometheus_client::encoding::text::encode;
use prometheus_client::encoding::EncodeLabelSet;
use prometheus_client::metrics::counter::Counter;
use prometheus_client::metrics::family::Family;
use prometheus_client::registry::Registry;

#[derive(Clone, Hash, PartialEq, Eq, EncodeLabelSet)]
struct QueryLabels {
    result: &'static str,
}

pub struct Metrics {
    registry: Registry,
    queries_total: Family<QueryLabels, Counter>,
}

static METRICS: OnceLock<Metrics> = OnceLock::new();

fn metrics() -> &'static Metrics {
    METRICS.get_or_init(|| {
        let mut registry = Registry::default();
        let queries_total = Family::<QueryLabels, Counter>::default();
        registry.register(
            "nexus_rag_queries_total",
            "Total de queries do nexus_rag",
            queries_total.clone(),
        );
        Metrics {
            registry,
            queries_total,
        }
    })
}

pub fn inc_query(result: &'static str) {
    let m = metrics();
    m.queries_total
        .get_or_create(&QueryLabels { result })
        .inc();
}

pub fn spawn_metrics_server() {
    let addr = std::env::var("NEXUS_METRICS_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:9898".to_string());
    let listener = match TcpListener::bind(&addr) {
        Ok(l) => l,
        Err(e) => {
            tracing::warn!(error = %e, "Metrics server bind failed");
            return;
        }
    };

    tracing::info!(addr = %addr, "Metrics server listening");

    thread::spawn(move || {
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => handle_connection(stream),
                Err(e) => tracing::warn!(error = %e, "Metrics accept failed"),
            }
        }
    });
}

fn handle_connection(mut stream: TcpStream) {
    let mut buf = [0u8; 1024];
    let n = match stream.read(&mut buf) {
        Ok(n) => n,
        Err(_) => return,
    };

    let req = String::from_utf8_lossy(&buf[..n]);
    let path = req
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .unwrap_or("/");

    if path != "/metrics" {
        let body = "not found";
        let resp = format!(
            "HTTP/1.1 404 Not Found\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        );
        let _ = stream.write_all(resp.as_bytes());
        return;
    }

    let mut body = String::new();
    if let Err(e) = encode(&mut body, &metrics().registry) {
        tracing::warn!(error = %e, "Metrics encode failed");
        let body = "encode error";
        let resp = format!(
            "HTTP/1.1 500 Internal Server Error\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        );
        let _ = stream.write_all(resp.as_bytes());
        return;
    }

    let resp = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/plain; version=0.0.4; charset=utf-8\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        body
    );
    let _ = stream.write_all(resp.as_bytes());
}
