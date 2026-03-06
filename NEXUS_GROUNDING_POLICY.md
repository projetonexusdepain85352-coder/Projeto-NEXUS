# NEXUS Grounding Policy

## Hard Rule

The AI agent must answer only from validated evidence stored in NEXUS databases.
If evidence is missing or weak, the agent must refuse and ask for validation/ingestion.

## Mandatory Behavior

- No parametric fallback answer.
- No "best guess" from pretrained memory.
- Every answer must be traceable to stored records (document_id/source).
- If retrieval fails threshold checks, return explicit denial.

## Current Enforcement

- `nexus_rag query` runs in `STRICT_DB_ONLY` mode.
- Results with score below the strict threshold are denied.
- Missing metadata (like `document_id`) is denied.

## Next Components

The same policy must be enforced in:

- Core inference orchestrator
- Multimodal interface layer
- Any external specialized arm

No component is allowed to bypass this policy.
