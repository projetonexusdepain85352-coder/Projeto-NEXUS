# Security Notes

## Estado das auditorias (2026-03-12)

### Vulnerabilidades com mitigacao documentada
- RUSTSEC-2023-0071 (rsa via sqlx-mysql)
  - Caminho: sqlx-mysql -> sqlx-macros-core -> sqlx-macros -> sqlx
  - Situacao: nao ha versao corrigida disponivel no advisory.
  - Contexto: o NEXUS usa somente PostgreSQL. O driver MySQL nao e utilizado no runtime.
  - Mitigacao atual: desativamos default-features do sqlx e removemos derives de `sqlx::FromRow` para reduzir dependencia de macros. Mesmo assim o pacote ainda aparece no Cargo.lock.
  - Plano: migrar o acesso a banco para `sqlx-postgres`/`sqlx-core` ou `tokio-postgres`, removendo o crate `sqlx` de alto nivel, ou aplicar upgrade assim que houver patch do advisory.

### Avisos (nao bloqueantes)
- RUSTSEC-2025-0052 (async-std descontinuado) via httpmock.
- RUSTSEC-2025-0119 (number_prefix unmaintained) via indicatif.
- RUSTSEC-2024-0436 (paste unmaintained) via tokenizers/candle.
- RUSTSEC-2025-0134 (rustls-pemfile unmaintained) via tonic/qdrant-client.
- RUSTSEC-2026-0002 (lru unsound) via ratatui.

Plano geral: acompanhar releases dos crates acima e atualizar quando houver alternativas mantidas ou correcoes upstream.