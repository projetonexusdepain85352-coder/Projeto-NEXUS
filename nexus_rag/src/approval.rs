//! Human approval gate.
//!
//! Activated only when NEXUS_ENV=production.
//! In all other environments (development, staging, CI) this gate is bypassed
//! silently so that `nexus_rag index` can be run without interactive prompts.
//!
//! Thread-safety: stateless, safe to call from any context.

use std::io::{self, Write};
use crate::error::{NexusError, Result};

/// Requires a human to type "CONFIRMAR" before proceeding.
///
/// Only active when `NEXUS_ENV=production`.
/// In any other environment, logs a debug message and returns `Ok(())` immediately.
pub fn require_human_approval(operation: &str, details: &str) -> Result<()> {
    let is_production = std::env::var("NEXUS_ENV").as_deref() == Ok("production");

    if !is_production {
        tracing::debug!(
            operation = operation,
            nexus_env = std::env::var("NEXUS_ENV").as_deref().unwrap_or("(not set)"),
            "Human approval gate skipped (not in production)"
        );
        return Ok(());
    }

    eprintln!();
    eprintln!("╔══════════════════════════════════════════╗");
    eprintln!("║   NEXUS — APROVAÇÃO HUMANA REQUERIDA     ║");
    eprintln!("╚══════════════════════════════════════════╝");
    eprintln!("  Operação : {}", operation);
    eprintln!("  Detalhes : {}", details);
    eprintln!();
    eprint!("  Digite 'CONFIRMAR' para prosseguir (qualquer outra coisa cancela): ");
    io::stderr().flush().map_err(NexusError::Io)?;

    let mut input = String::new();
    io::stdin().read_line(&mut input).map_err(NexusError::Io)?;

    if input.trim() != "CONFIRMAR" {
        tracing::warn!(operation = operation, "Human approval denied by operator");
        return Err(NexusError::Cancelled);
    }

    tracing::info!(operation = operation, "Human approval granted");
    Ok(())
}
