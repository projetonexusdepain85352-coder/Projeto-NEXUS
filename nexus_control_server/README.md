# NEXUS Control Server

Servidor local para gerenciar o NEXUS pelo navegador (Google Chrome), com autenticação por token e opcionalmente por conta Google.

## Fluxo recomendado (seguro)

1. Rodar o servidor em `127.0.0.1` (padrão).
2. Proteger com token forte e/ou Google allowlist.
3. Expor para internet apenas via túnel HTTPS (Cloudflare Tunnel), nunca com porta aberta no roteador.

## 1) Autenticação

Você pode usar as duas ao mesmo tempo.

### Opção A: token estático

PowerShell:

```powershell
$env:NEXUS_CONTROL_TOKEN = "troque-por-um-token-forte"
```

### Opção B: conta Google

PowerShell:

```powershell
$env:NEXUS_GOOGLE_CLIENT_ID = "seu-client-id.apps.googleusercontent.com"
$env:NEXUS_GOOGLE_ALLOWED_EMAILS = "voce@gmail.com,admin@seu-dominio.com"
```

`NEXUS_GOOGLE_ALLOWED_EMAILS` é opcional, mas fortemente recomendado.

## 2) Iniciar servidor local

```powershell
python nexus_control_server/server.py
```

Painel local:

- [http://localhost:8787](http://localhost:8787)

## 3) Acesso externo rápido (teste) com URL temporária

Esse modo cria uma URL `trycloudflare.com` temporária.

PowerShell (chamando WSL):

```powershell
wsl -e bash -lc "cd '/mnt/c/Users/dulan/OneDrive/Documentos/GitHub/Projeto-NEXUS' && ./nexus_control_server/scripts/start_quick_tunnel_wsl.sh"
```

Saída esperada no terminal:

- `Local : http://127.0.0.1:8787`
- `Public: https://<random>.trycloudflare.com`

## 4) Acesso externo fixo (produção) com seu domínio

### 4.1 Pré-requisitos

- Conta Cloudflare.
- Domínio gerenciado pela Cloudflare (DNS).

### 4.2 Configurar túnel nomeado (uma vez)

PowerShell:

```powershell
$env:NEXUS_TUNNEL_NAME = "nexus-control"
$env:NEXUS_TUNNEL_HOSTNAME = "nexus-control.seudominio.com"
wsl -e bash -lc "cd '/mnt/c/Users/dulan/OneDrive/Documentos/GitHub/Projeto-NEXUS' && ./nexus_control_server/scripts/setup_named_tunnel_wsl.sh"
```

### 4.3 Iniciar túnel nomeado

PowerShell:

```powershell
wsl -e bash -lc "cd '/mnt/c/Users/dulan/OneDrive/Documentos/GitHub/Projeto-NEXUS' && ./nexus_control_server/scripts/start_named_tunnel_wsl.sh"
```

Acesso público:

- `https://nexus-control.seudominio.com`

## 5) Configuração no Google Cloud (OAuth)

No seu OAuth Client (tipo Web application), adicione em **Authorized JavaScript origins**:

- `http://localhost:8787`
- `https://nexus-control.seudominio.com` (ou sua URL pública final)

Depois, use o `client_id` no `NEXUS_GOOGLE_CLIENT_ID`.

## API

- `GET /api/health`
- `GET /api/auth/config`
- `POST /api/auth/google`
- `GET /api/services` (auth)
- `POST /api/services/<name>/start` (auth)
- `POST /api/services/<name>/stop` (auth)
- `GET /api/logs/<name>?lines=200` (auth)

Auth aceito:

- `Authorization: Bearer <NEXUS_CONTROL_TOKEN>`
- `Authorization: Bearer <session_token_google>`

## Configuração de serviços

Arquivo: `nexus_control_server/services.json`

Exemplo:

```json
{
  "services": {
    "sugestor": {
      "command": ["{python}", "nexus_sugestor/servidor.py"],
      "cwd": ".",
      "env": {
        "PYTHONUNBUFFERED": "1"
      }
    }
  }
}
```

`{python}` usa o mesmo interpretador Python do servidor de controle.

## Segurança mínima obrigatória

- Não abra a porta `8787` no roteador.
- Use `NEXUS_GOOGLE_ALLOWED_EMAILS` com sua allowlist.
- Mantenha também `NEXUS_CONTROL_TOKEN` como fallback de emergência.
- Registre logs e revise acessos periodicamente.
