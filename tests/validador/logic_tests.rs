use validador::main_impl::{
    apply_decision, formatar_sugestao, obter_sugestao, validate_document_input, Categoria,
    Documento, Sugestao, SuggestionProvider, ValidationDecision, ValidationError, ValidationStatus,
    ValidationStore,
};

struct MockStore {
    status: ValidationStatus,
    last_decided_by: Option<String>,
    last_reason: Option<String>,
}

impl MockStore {
    fn new(status: ValidationStatus) -> Self {
        Self {
            status,
            last_decided_by: None,
            last_reason: None,
        }
    }
}

impl ValidationStore for MockStore {
    fn get_status(&mut self, _doc_id: &str) -> Result<ValidationStatus, ValidationError> {
        Ok(self.status.clone())
    }

    fn update_status(
        &mut self,
        _doc_id: &str,
        status: ValidationStatus,
        decided_by: &str,
        reason: Option<&str>,
    ) -> Result<(), ValidationError> {
        self.status = status;
        self.last_decided_by = Some(decided_by.to_string());
        self.last_reason = reason.map(|r| r.to_string());
        Ok(())
    }
}

struct MockProvider {
    suggestion: Sugestao,
}

impl SuggestionProvider for MockProvider {
    fn suggest(&self, _doc: &Documento) -> Result<Sugestao, ValidationError> {
        Ok(self.suggestion.clone())
    }
}

fn sample_document() -> Documento {
    Documento {
        id: "doc-1".to_string(),
        source: "http://example.com".to_string(),
        domain: "security".to_string(),
        doc_type: "html".to_string(),
        content_length: 10,
        preview: "preview".to_string(),
        content: "conteudo".to_string(),
        head: "".to_string(),
    }
}

#[test]
fn approval_updates_state() {
    // Scenario: document is pending and user approves it.
    // Expectation: status transitions to approved and decision metadata is persisted.
    let mut store = MockStore::new(ValidationStatus::Pending);

    let status = apply_decision(&mut store, "doc-1", ValidationDecision::Approve, "user")
        .expect("decision should succeed");

    assert_eq!(status, ValidationStatus::Approved);
    assert_eq!(store.status, ValidationStatus::Approved);
    assert_eq!(store.last_decided_by.as_deref(), Some("user"));
    assert!(store.last_reason.is_none());
}

#[test]
fn rejection_updates_state_with_reason() {
    // Scenario: document is pending and user rejects it with a reason.
    // Expectation: status transitions to rejected and reason is stored.
    let mut store = MockStore::new(ValidationStatus::Pending);

    let status = apply_decision(
        &mut store,
        "doc-2",
        ValidationDecision::Reject {
            reason: "motivo".to_string(),
        },
        "reviewer",
    )
    .expect("rejection should succeed");

    assert_eq!(status, ValidationStatus::Rejected);
    assert_eq!(store.status, ValidationStatus::Rejected);
    assert_eq!(store.last_decided_by.as_deref(), Some("reviewer"));
    assert_eq!(store.last_reason.as_deref(), Some("motivo"));
}

#[test]
fn rejection_requires_reason() {
    // Scenario: reject action without a reason.
    // Expectation: validation fails with malformed document error.
    let mut store = MockStore::new(ValidationStatus::Pending);

    let err = apply_decision(
        &mut store,
        "doc-3",
        ValidationDecision::Reject {
            reason: " ".to_string(),
        },
        "reviewer",
    )
    .expect_err("empty reason should fail");

    assert!(matches!(err, ValidationError::MalformedDocument(_)));
}

#[test]
fn non_pending_document_is_rejected() {
    // Scenario: attempt to decide on a document that is already approved.
    // Expectation: apply_decision rejects with NotPending error.
    let mut store = MockStore::new(ValidationStatus::Approved);

    let err = apply_decision(&mut store, "doc-4", ValidationDecision::Approve, "user")
        .expect_err("non-pending should fail");

    assert!(matches!(err, ValidationError::NotPending(ValidationStatus::Approved)));
}

#[test]
fn validates_empty_or_malformed_documents() {
    // Scenario: document fields are missing or empty.
    // Expectation: validate_document_input returns appropriate errors.
    let mut doc = sample_document();
    doc.id.clear();
    assert!(matches!(
        validate_document_input(&doc),
        Err(ValidationError::MalformedDocument(_))
    ));

    let mut doc = sample_document();
    doc.source.clear();
    assert!(matches!(
        validate_document_input(&doc),
        Err(ValidationError::MalformedDocument(_))
    ));

    let mut doc = sample_document();
    doc.domain.clear();
    assert!(matches!(
        validate_document_input(&doc),
        Err(ValidationError::MalformedDocument(_))
    ));

    let mut doc = sample_document();
    doc.content.clear();
    assert!(matches!(
        validate_document_input(&doc),
        Err(ValidationError::EmptyDocument)
    ));
}

#[test]
fn suggestion_provider_integration_formats_output() {
    // Scenario: suggestion provider returns a recommendation for a document.
    // Expectation: formatting includes confidence and reason for display.
    let doc = sample_document();
    let provider = MockProvider {
        suggestion: Sugestao {
            categoria: Categoria::Util,
            confianca: 82,
            motivo: "Teste de sugestao".to_string(),
        },
    };

    let sug = obter_sugestao(&provider, &doc).expect("mock suggestion should succeed");
    let rendered = formatar_sugestao(&sug);

    assert!(rendered.contains("Confianca: 82%"));
    assert!(rendered.contains("Teste de sugestao"));
}
