# Known Issues

## PROBLEMA 1: Artefatos de build versionados no Git
- O que era: 8.871 arquivos de build/dados no repositório.
- Impacto: clone lento, histórico sujo, dificuldade de leitura.
- Como foi resolvido: `git rm --cached` + `.gitignore` atualizado na fase de limpeza.
- Como evitar no futuro: nunca rodar `git add -A` sem revisar o diff.

## PROBLEMA 2: Credencial hardcoded
- O que era: senha/credencial em `iniciar_validador.sh` linha 63 (layout antigo).
- Impacto: risco de segurança, credencial exposta no histórico Git.
- Como foi resolvido: migração para variável de ambiente via `.env`.
- Como evitar: SEMPRE usar variáveis de ambiente, NUNCA hardcodar credenciais.
- Observação crítica: a credencial original ainda existe no histórico Git antigo; se o repositório já foi exposto publicamente, trate a credencial como comprometida e faça rotação imediata.

- Status atual: corrigido no script atual com variável `NEXUS_DB_PASSWORD`.

## PROBLEMA 3: Acoplamento de banco de dados
- O que era: conexão de banco instanciada diretamente em 3 `main.rs` diferentes.
- Impacto: dificulta testes unitários, troca de banco e manutenção.
- Localização original:
  - `agente_intermediario/src/main.rs:1004`
  - `nexus_mtp/src/main.rs:490`
  - `validador/src/main.rs:3052`
- Status atual: ainda permanece acoplado (sem módulo compartilhado único).
- Mitigação: centralizar em `shared/db/` e injetar configuração por ambiente.

## PROBLEMA 4: .gitignore inconsistente
- O que era: regras no `.gitignore` não tinham efeito porque os arquivos já estavam rastreados.
- Como foi resolvido: `git rm --cached` + revisão das regras.
- Lição: `.gitignore` só funciona para arquivos ainda não rastreados.

## PROBLEMA 5: ICE do compilador Rust (rustc)
- O que era: `cargo build --workspace` falhava com ICE do `rustc` (`slice index starts at 13 but ends at 11`) ao analisar `dead_code` no crate `nexus_mtp`.
- Impacto: bloqueava a validação final de compilação da Fase 7 mesmo com código funcional.
- Diagnóstico: os arquivos `src/nexus_mtp/src/db.rs` e `src/nexus_mtp/src/trainer.rs` foram verificados e corrigidos para garantir linhas limpas (sem literal `` `n `` acidental). Em seguida, `cargo clean -p nexus_mtp` foi executado e o ICE persistiu.
- Como foi resolvido: aplicada mitigação no crate com `#![allow(dead_code)]` em `src/nexus_mtp/src/main.rs` e ajuste de API deprecated em `approval.rs` (`highlight_style` -> `row_highlight_style`). Após isso, `cargo build --workspace` concluiu com sucesso.
- Como evitar no futuro:
  - Evitar substituições textuais frágeis em PowerShell que possam introduzir literal `` `n `` em arquivo fonte.
  - Preferir edições estruturadas (patch) e revisão das linhas alteradas após automação.
  - Em caso de novo ICE, registrar versão do toolchain e considerar pin de versão estável conhecida no projeto.
