use std::collections::{HashMap, HashSet};
use std::fs;
use std::time::Duration;

use agente_intermediario::main_impl::{
    baixar_conteudo_com_config, extrair_texto_limpo, extrair_texto_pdf, ingest_text_document,
    IngestOutcome, IngestStore, IngestTransaction, NewDocument, BoxError,
};
use httpmock::Method::GET;
use httpmock::MockServer;
use uuid::Uuid;

struct MockStore {
    hashes: HashMap<String, String>,
    inserted_ids: Vec<Uuid>,
    fail_validation: bool,
    rollback_called: bool,
}

impl MockStore {
    fn new() -> Self {
        Self {
            hashes: HashMap::new(),
            inserted_ids: Vec::new(),
            fail_validation: false,
            rollback_called: false,
        }
    }
}

struct MockTx<'a> {
    store: &'a mut MockStore,
    staged: Option<(String, String, Uuid)>,
}

impl IngestTransaction for MockTx<'_> {
    fn insert_document(&mut self, doc: &NewDocument) -> Result<Uuid, BoxError> {
        let id = Uuid::new_v4();
        self.staged = Some((doc.url.to_string(), doc.hash.to_string(), id));
        Ok(id)
    }

    fn insert_validation(&mut self, _document_id: Uuid) -> Result<(), BoxError> {
        if self.store.fail_validation {
            return Err("forced validation failure".to_string().into());
        }
        Ok(())
    }

    fn commit(self: Box<Self>) -> Result<(), BoxError> {
        if let Some((url, hash, id)) = self.staged {
            self.store.hashes.insert(url, hash);
            self.store.inserted_ids.push(id);
        }
        Ok(())
    }

    fn rollback(self: Box<Self>) -> Result<(), BoxError> {
        self.store.rollback_called = true;
        Ok(())
    }
}

impl IngestStore for MockStore {
    fn begin_tx(&mut self) -> Result<Box<dyn IngestTransaction + '_>, BoxError> {
        Ok(Box::new(MockTx {
            store: self,
            staged: None,
        }))
    }

    fn get_hash_by_source(&mut self, url: &str) -> Result<Option<String>, BoxError> {
        Ok(self.hashes.get(url).cloned())
    }

    fn update_document(&mut self, url: &str, content: &str, hash: &str) -> Result<(), BoxError> {
        let _ = content;
        self.hashes.insert(url.to_string(), hash.to_string());
        Ok(())
    }
}

#[test]
fn dedup_sha256_prevents_reinsert() {
    // Scenario: inserting the same content twice for the same URL.
    // Expectation: the second ingest is treated as a duplicate and does not insert again.
    let mut store = MockStore::new();
    let content = "same content for hashing";

    let first = ingest_text_document(&mut store, "http://example.com/a", "security", "doc", content)
        .expect("first ingest should succeed");
    let second = ingest_text_document(&mut store, "http://example.com/a", "security", "doc", content)
        .expect("second ingest should succeed");

    assert!(matches!(first, IngestOutcome::Inserted(_)));
    assert_eq!(second, IngestOutcome::IgnoredDuplicate);
    assert_eq!(store.inserted_ids.len(), 1);
}

#[test]
fn uuid_is_unique_for_each_ingested_document() {
    // Scenario: ingest two different documents.
    // Expectation: each insert generates a distinct UUID.
    let mut store = MockStore::new();

    let first = ingest_text_document(&mut store, "http://example.com/a", "security", "doc", "alpha")
        .expect("first ingest should succeed");
    let second = ingest_text_document(&mut store, "http://example.com/b", "security", "doc", "beta")
        .expect("second ingest should succeed");

    let id1 = match first { IngestOutcome::Inserted(id) => id, _ => panic!("expected insert") };
    let id2 = match second { IngestOutcome::Inserted(id) => id, _ => panic!("expected insert") };

    assert_ne!(id1, id2);
    let uniq: HashSet<Uuid> = store.inserted_ids.iter().cloned().collect();
    assert_eq!(uniq.len(), store.inserted_ids.len());
}

#[test]
fn rollback_on_partial_failure() {
    // Scenario: document insert succeeds, validation insert fails.
    // Expectation: transaction is rolled back and no document is persisted.
    let mut store = MockStore::new();
    store.fail_validation = true;

    let result = ingest_text_document(&mut store, "http://example.com/c", "security", "doc", "payload");
    assert!(result.is_err());
    assert!(store.rollback_called);
    assert!(store.hashes.is_empty());
}

#[test]
fn html_parsing_extracts_clean_text() {
    // Scenario: HTML fixture contains nav/footer noise plus main article content.
    // Expectation: extracted text keeps the article body and drops navigation/footer noise.
    let html = fs::read_to_string("tests/fixtures/sample.html").expect("fixture missing");
    let text = extrair_texto_limpo(&html);

    assert!(text.contains("main content paragraph"));
    assert!(!text.contains("Menu Item"));
    assert!(!text.contains("Footer text"));
}

#[test]
fn pdf_extraction_reads_fixture_text() {
    // Scenario: small PDF fixture with known text.
    // Expectation: PDF extraction returns text containing the fixture phrase.
    let bytes = fs::read("tests/fixtures/hello.pdf").expect("pdf fixture missing");
    let text = extrair_texto_pdf(&bytes).expect("pdf extraction failed");

    assert!(text.contains("Hello PDF"));
}

#[test]
fn http_errors_return_failure() {
    // Scenario: server returns 404 and 500.
    // Expectation: download helper returns errors for both cases.
    let server = MockServer::start();

    server.mock(|when, then| {
        when.method(GET).path("/notfound");
        then.status(404).body("nope");
    });

    server.mock(|when, then| {
        when.method(GET).path("/error");
        then.status(500).body("fail");
    });

    let err_404 = baixar_conteudo_com_config(
        &server.url("/notfound"),
        1024,
        Duration::from_millis(200),
        1,
    )
    .expect_err("expected 404 to error");
    assert!(err_404.to_string().contains("HTTP 404"));

    let err_500 = baixar_conteudo_com_config(
        &server.url("/error"),
        1024,
        Duration::from_millis(200),
        1,
    )
    .expect_err("expected 500 to error");
    assert!(err_500.to_string().contains("HTTP 500"));
}

#[test]
fn http_timeout_is_handled() {
    // Scenario: server responds slower than the configured timeout.
    // Expectation: download helper returns a timeout error.
    let server = MockServer::start();

    server.mock(|when, then| {
        when.method(GET).path("/slow");
        then.status(200)
            .body("ok")
            .delay(Duration::from_millis(200));
    });

    let result = baixar_conteudo_com_config(
        &server.url("/slow"),
        1024,
        Duration::from_millis(50),
        1,
    );

    assert!(result.is_err());
}
