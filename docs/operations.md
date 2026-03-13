# Operacao

## Verificar se o servidor esta rodando

```
ss -tlnp | grep 8765
```

## Logs do agente

```
cat /tmp/agent.log | tail -50
```

## Matar o servidor

```
pkill -f nexus_agent_server
```

## Containers ativos

```
docker ps --format "table {{.Names}}\t{{.Status}}\t{{.Ports}}"
```

## Redefinir senha do kb_reader

```
docker exec pg_copia psql -U kb_admin -d knowledge_base \
  -c "ALTER USER kb_reader PASSWORD 'nova_senha';"
```

## Verificar collections do Qdrant (REST)

```
curl -s http://localhost:6335/collections
```

## Operacao do control server

```
bash config/scripts/nexus_ctl.sh status
bash config/scripts/nexus_ctl.sh start
bash config/scripts/nexus_ctl.sh stop
bash config/scripts/nexus_ctl.sh restart
```

## Testes rapidos

```
curl -s http://localhost:8765/health

curl -s -X POST http://localhost:8765/query \
  -H 'Content-Type: application/json' \
  -d '{"query":"what is SQL injection?","domain":"security"}'
```

## Testes do workspace

```
cargo test --workspace
```

```
NEXUS_INTEGRATION_TESTS=1 cargo test --workspace -- --include-ignored
```

## Problemas conhecidos

- PowerShell quebra aspas em comandos complexos: prefira GitHub Desktop para push.
- `POSTGRES_HOST` muda ao reiniciar WSL: redescobrir com `ip route | grep default | awk '{print $3}'`.
- `QDRANT_URL` deve ser 6336 (gRPC), nao 6335 (REST).
- `/api/logs` no control server e endpoint legado; UI atual usa `Servicos + Terminal`.
