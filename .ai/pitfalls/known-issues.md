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
