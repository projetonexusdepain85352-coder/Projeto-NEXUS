#!/usr/bin/env python3
import json
import os
import re
import secrets
import signal
import subprocess
import sys
import threading
import time
import urllib.parse
import urllib.request
from datetime import datetime, timezone
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import parse_qs, urlparse

ROOT_DIR = Path(__file__).resolve().parent.parent
CONFIG_PATH = Path(__file__).resolve().parent / "services.json"
STATE_PATH = Path(__file__).resolve().parent / ".state.json"
LOG_DIR = ROOT_DIR / "logs" / "control"

DEFAULT_HOST = "127.0.0.1"
DEFAULT_PORT = 8787
MAX_LOG_LINES = 1000
GOOGLE_TOKENINFO_URL = "https://oauth2.googleapis.com/tokeninfo?id_token="
SESSION_TTL_SECONDS = 12 * 60 * 60
SERVICE_NAME_RE = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._-]{0,63}$")
SESSIONS = {}
SESSIONS_LOCK = threading.Lock()

DASHBOARD_HTML = """<!doctype html>
<html lang="pt-BR">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>NEXUS Control Server</title>
  <script src="https://accounts.google.com/gsi/client" async defer></script>
  <style>
    :root {
      --bg: #0f172a;
      --card: #111827;
      --ink: #e5e7eb;
      --muted: #94a3b8;
      --ok: #16a34a;
      --bad: #dc2626;
      --line: #334155;
      --btn: #1d4ed8;
    }
    * { box-sizing: border-box; }
    body { margin: 0; font-family: ui-sans-serif, Segoe UI, Arial, sans-serif; background: linear-gradient(135deg, #0f172a, #111827); color: var(--ink); }
    .wrap { max-width: 1040px; margin: 0 auto; padding: 24px; }
    h1 { margin: 0 0 16px; font-size: 28px; }
    .card { background: rgba(17,24,39,.92); border: 1px solid var(--line); border-radius: 14px; padding: 16px; margin-bottom: 16px; }
    .row { display: flex; gap: 8px; flex-wrap: wrap; align-items: center; }
    input, button, select { border-radius: 10px; border: 1px solid var(--line); background: #0b1220; color: var(--ink); padding: 10px 12px; }
    button { background: var(--btn); border: 1px solid #1e40af; cursor: pointer; }
    button:hover { filter: brightness(1.08); }
    .tbl { width: 100%; border-collapse: collapse; }
    .tbl th, .tbl td { padding: 10px; border-bottom: 1px solid var(--line); text-align: left; font-size: 14px; }
    .muted { color: var(--muted); }
    .ok { color: var(--ok); font-weight: 700; }
    .bad { color: var(--bad); font-weight: 700; }
    pre { margin: 0; white-space: pre-wrap; background: #020617; border: 1px solid var(--line); border-radius: 10px; padding: 12px; max-height: 320px; overflow: auto; }
    .hidden { display: none; }
  </style>
</head>
<body>
  <div class="wrap">
    <h1>NEXUS Control Server</h1>

    <div class="card">
      <div class="row">
        <label for="token">Token de acesso:</label>
        <input id="token" type="password" placeholder="Bearer token" style="min-width:320px" />
        <button onclick="refreshServices()">Atualizar</button>
      </div>
      <p class="muted">Gestao do NEXUS pelo navegador (Google Chrome) com autenticacao por token e opcionalmente por conta Google.</p>
      <div id="googleBlock" class="hidden">
        <div class="row">
          <div id="googleSignIn"></div>
          <span id="googleMsg" class="muted"></span>
        </div>
      </div>
    </div>

    <div class="card">
      <table class="tbl" id="svcTable">
        <thead>
          <tr><th>Servico</th><th>Status</th><th>PID</th><th>Comando</th><th>Acoes</th></tr>
        </thead>
        <tbody></tbody>
      </table>
    </div>

    <div class="card">
      <div class="row">
        <label for="logsService">Logs:</label>
        <select id="logsService"></select>
        <input id="logLines" type="number" min="1" max="1000" value="200" style="width:120px" />
        <button onclick="loadLogs()">Carregar logs</button>
      </div>
      <pre id="logsBox"></pre>
    </div>
  </div>

<script>
let googleCfg = { google_enabled: false, google_client_id: null };

function tokenHeader() {
  const t = document.getElementById('token').value.trim();
  return t ? { 'Authorization': 'Bearer ' + t } : {};
}

async function api(path, opts={}) {
  const headers = Object.assign({}, tokenHeader(), opts.headers || {});
  const res = await fetch(path, Object.assign({}, opts, { headers }));
  const data = await res.json().catch(() => ({}));
  if (!res.ok) throw new Error(data.error || ('HTTP ' + res.status));
  return data;
}

async function refreshServices() {
  try {
    const data = await api('/api/services');
    const tb = document.querySelector('#svcTable tbody');
    tb.innerHTML = '';

    const sel = document.getElementById('logsService');
    sel.innerHTML = '';

    data.services.forEach(s => {
      const tr = document.createElement('tr');
      tr.innerHTML = `
        <td>${s.name}</td>
        <td class="${s.running ? 'ok' : 'bad'}">${s.running ? 'RUNNING' : 'STOPPED'}</td>
        <td>${s.pid || '--'}</td>
        <td class="muted">${s.command.join(' ')}</td>
        <td>
          <button onclick="startSvc('${s.name}')">Start</button>
          <button onclick="stopSvc('${s.name}')">Stop</button>
        </td>`;
      tb.appendChild(tr);

      const opt = document.createElement('option');
      opt.value = s.name;
      opt.textContent = s.name;
      sel.appendChild(opt);
    });
  } catch (e) {
    alert(e.message);
  }
}

async function startSvc(name) {
  try {
    await api('/api/services/' + encodeURIComponent(name) + '/start', { method: 'POST' });
    await refreshServices();
  } catch (e) {
    alert(e.message);
  }
}

async function stopSvc(name) {
  try {
    await api('/api/services/' + encodeURIComponent(name) + '/stop', { method: 'POST' });
    await refreshServices();
  } catch (e) {
    alert(e.message);
  }
}

async function loadLogs() {
  const name = document.getElementById('logsService').value;
  const lines = Number(document.getElementById('logLines').value || 200);
  if (!name) return;
  try {
    const data = await api('/api/logs/' + encodeURIComponent(name) + '?lines=' + lines);
    document.getElementById('logsBox').textContent = data.logs || '(sem logs)';
  } catch (e) {
    alert(e.message);
  }
}

async function fetchAuthConfig() {
  try {
    const data = await fetch('/api/auth/config').then(r => r.json());
    googleCfg = data;
  } catch {
    googleCfg = { google_enabled: false, google_client_id: null };
  }
}

async function onGoogleCredential(response) {
  try {
    const res = await fetch('/api/auth/google', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ id_token: response.credential })
    });
    const data = await res.json();
    if (!res.ok) throw new Error(data.error || 'Falha na autenticacao Google');
    document.getElementById('token').value = data.token;
    document.getElementById('googleMsg').textContent = 'Autenticado como ' + data.email;
    await refreshServices();
  } catch (e) {
    alert(e.message);
  }
}
window.onGoogleCredential = onGoogleCredential;

function setupGoogleSignIn() {
  if (!googleCfg.google_enabled || !googleCfg.google_client_id || !window.google || !google.accounts || !google.accounts.id) {
    return;
  }
  document.getElementById('googleBlock').classList.remove('hidden');
  google.accounts.id.initialize({
    client_id: googleCfg.google_client_id,
    callback: onGoogleCredential
  });
  google.accounts.id.renderButton(
    document.getElementById('googleSignIn'),
    { theme: 'outline', size: 'large', text: 'signin_with' }
  );
}

(async function boot() {
  await fetchAuthConfig();
  setupGoogleSignIn();
  refreshServices();
})();
</script>
</body>
</html>"""


def now_iso() -> str:
    return datetime.now(timezone.utc).isoformat()


def read_json(path: Path, default):
    if not path.exists():
        return default
    try:
        return json.loads(path.read_text(encoding="utf-8-sig"))
    except Exception:
        return default


def write_json(path: Path, data) -> None:
    path.write_text(json.dumps(data, indent=2, ensure_ascii=False), encoding="utf-8")


def is_pid_running(pid: int) -> bool:
    if not pid or pid <= 0:
        return False
    try:
        if os.name == "nt":
            out = subprocess.check_output(
                ["tasklist", "/FI", f"PID eq {pid}"],
                text=True,
                stderr=subprocess.STDOUT,
            )
            return str(pid) in out
        os.kill(pid, 0)
        return True
    except Exception:
        return False


def expected_token() -> str:
    return os.environ.get("NEXUS_CONTROL_TOKEN", "").strip()


def google_client_id() -> str:
    return os.environ.get("NEXUS_GOOGLE_CLIENT_ID", "").strip()


def google_enabled() -> bool:
    return bool(google_client_id())


def allowed_google_emails() -> set:
    raw = os.environ.get("NEXUS_GOOGLE_ALLOWED_EMAILS", "").strip()
    if not raw:
        return set()
    return {p.strip().lower() for p in raw.split(",") if p.strip()}


def purge_expired_sessions() -> None:
    now = int(time.time())
    with SESSIONS_LOCK:
        dead = [tok for tok, meta in SESSIONS.items() if int(meta.get("exp", 0)) <= now]
        for tok in dead:
            SESSIONS.pop(tok, None)


def create_session(email: str) -> dict:
    purge_expired_sessions()
    token = secrets.token_urlsafe(32)
    exp = int(time.time()) + SESSION_TTL_SECONDS
    with SESSIONS_LOCK:
        SESSIONS[token] = {"email": email, "exp": exp, "created_at": now_iso()}
    return {"token": token, "email": email, "expires_at": exp}


def validate_session(token: str) -> bool:
    if not token:
        return False
    purge_expired_sessions()
    with SESSIONS_LOCK:
        return token in SESSIONS


def verify_google_id_token(id_token: str) -> str:
    client_id = google_client_id()
    if not client_id:
        raise ValueError("Google auth desabilitado no servidor")

    url = GOOGLE_TOKENINFO_URL + urllib.parse.quote(id_token, safe="")
    req = urllib.request.Request(url, headers={"Accept": "application/json"})
    with urllib.request.urlopen(req, timeout=10) as resp:
        data = json.loads(resp.read().decode("utf-8"))

    if data.get("aud") != client_id:
        raise ValueError("Token Google invalido para este projeto (aud mismatch)")

    email = (data.get("email") or "").strip().lower()
    if not email:
        raise ValueError("Token Google sem e-mail")

    if str(data.get("email_verified", "")).lower() != "true":
        raise ValueError("E-mail Google nao verificado")

    exp = int(data.get("exp", "0") or 0)
    if exp and exp < int(time.time()):
        raise ValueError("Token Google expirado")

    allow = allowed_google_emails()
    if allow and email not in allow:
        raise PermissionError(f"E-mail {email} nao autorizado")

    return email


class ServiceManager:
    def __init__(self, root_dir: Path, config_path: Path, state_path: Path):
        self.root_dir = root_dir
        self.config_path = config_path
        self.state_path = state_path
        self.lock = threading.Lock()

    def config(self):
        raw = read_json(self.config_path, {"services": {}})
        return raw.get("services", {})

    def _state(self):
        return read_json(self.state_path, {"services": {}})

    def _save_state(self, state) -> None:
        write_json(self.state_path, state)

    def _cleanup(self, state) -> None:
        to_delete = []
        for name, meta in state.get("services", {}).items():
            pid = int(meta.get("pid", 0) or 0)
            if pid and not is_pid_running(pid):
                to_delete.append(name)
        for name in to_delete:
            state["services"].pop(name, None)

    def list_services(self):
        with self.lock:
            cfg = self.config()
            state = self._state()
            self._cleanup(state)
            self._save_state(state)

            out = []
            for name, spec in cfg.items():
                proc = state.get("services", {}).get(name, {})
                pid = int(proc.get("pid", 0) or 0)
                out.append({
                    "name": name,
                    "running": is_pid_running(pid),
                    "pid": pid if pid else None,
                    "command": spec.get("command", []),
                    "cwd": spec.get("cwd", "."),
                    "log_file": proc.get("log_file"),
                    "started_at": proc.get("started_at"),
                })
            return out

    def _resolve_command(self, cmd):
        resolved = []
        for token in cmd:
            if token == "{python}":
                resolved.append(sys.executable)
            else:
                resolved.append(token)
        return resolved

    def start_service(self, name: str):
        with self.lock:
            cfg = self.config()
            if name not in cfg:
                raise ValueError(f"Servico '{name}' nao existe")

            state = self._state()
            self._cleanup(state)

            existing = state.get("services", {}).get(name)
            if existing:
                pid = int(existing.get("pid", 0) or 0)
                if is_pid_running(pid):
                    return {
                        "name": name,
                        "running": True,
                        "pid": pid,
                        "already_running": True,
                    }

            spec = cfg[name]
            cmd = self._resolve_command(spec.get("command", []))
            if not cmd:
                raise ValueError(f"Servico '{name}' sem comando")

            cwd = (self.root_dir / spec.get("cwd", ".")).resolve()
            env = os.environ.copy()
            env.update(spec.get("env", {}))

            LOG_DIR.mkdir(parents=True, exist_ok=True)
            log_file = LOG_DIR / f"{name}.log"
            log_fp = log_file.open("a", encoding="utf-8")

            creationflags = 0
            if os.name == "nt":
                creationflags = subprocess.CREATE_NEW_PROCESS_GROUP

            proc = subprocess.Popen(
                cmd,
                cwd=str(cwd),
                env=env,
                stdout=log_fp,
                stderr=subprocess.STDOUT,
                creationflags=creationflags,
            )
            log_fp.close()

            state.setdefault("services", {})[name] = {
                "pid": proc.pid,
                "started_at": now_iso(),
                "log_file": str(log_file),
                "command": cmd,
                "cwd": str(cwd),
            }
            self._save_state(state)

            return {"name": name, "running": True, "pid": proc.pid, "already_running": False}

    def stop_service(self, name: str):
        with self.lock:
            state = self._state()
            item = state.get("services", {}).get(name)
            if not item:
                return {"name": name, "running": False, "stopped": False, "reason": "not running"}

            pid = int(item.get("pid", 0) or 0)
            if pid <= 0:
                state["services"].pop(name, None)
                self._save_state(state)
                return {"name": name, "running": False, "stopped": False, "reason": "invalid pid"}

            if os.name == "nt":
                subprocess.run(["taskkill", "/PID", str(pid), "/T", "/F"], check=False)
            else:
                try:
                    os.kill(pid, signal.SIGTERM)
                except ProcessLookupError:
                    pass
                deadline = time.time() + 5
                while time.time() < deadline and is_pid_running(pid):
                    time.sleep(0.2)
                if is_pid_running(pid):
                    try:
                        os.kill(pid, signal.SIGKILL)
                    except ProcessLookupError:
                        pass

            state["services"].pop(name, None)
            self._save_state(state)
            return {"name": name, "running": False, "stopped": True}

    def read_logs(self, name: str, lines: int = 200) -> str:
        lines = max(1, min(lines, MAX_LOG_LINES))

        state = self._state()
        log_file = None

        item = state.get("services", {}).get(name)
        if item:
            log_file = item.get("log_file")

        if not log_file:
            candidate = LOG_DIR / f"{name}.log"
            if candidate.exists():
                log_file = str(candidate)

        if not log_file:
            return ""

        p = Path(log_file)
        if not p.exists():
            return ""

        content = p.read_text(encoding="utf-8", errors="replace").splitlines()
        return "\n".join(content[-lines:])


def json_response(handler: BaseHTTPRequestHandler, code: int, payload):
    raw = json.dumps(payload, ensure_ascii=False).encode("utf-8")
    handler.send_response(code)
    handler.send_header("Content-Type", "application/json; charset=utf-8")
    handler.send_header("Content-Length", str(len(raw)))
    handler.end_headers()
    handler.wfile.write(raw)


def text_response(handler: BaseHTTPRequestHandler, code: int, text: str, content_type: str):
    raw = text.encode("utf-8")
    handler.send_response(code)
    handler.send_header("Content-Type", content_type)
    handler.send_header("Content-Length", str(len(raw)))
    handler.end_headers()
    handler.wfile.write(raw)


def read_token_from_request(handler: BaseHTTPRequestHandler, query):
    auth = handler.headers.get("Authorization", "")
    if auth.lower().startswith("bearer "):
        return auth[7:].strip()
    return (query.get("token", [""])[0] or "").strip()


def clean_service_name(raw: str) -> str:
    name = urllib.parse.unquote(raw or "").strip().strip("/")
    if SERVICE_NAME_RE.fullmatch(name):
        return name
    return ""


def require_auth(handler: BaseHTTPRequestHandler, query) -> bool:
    provided = read_token_from_request(handler, query)
    static_token = expected_token()

    if static_token and provided == static_token:
        return True

    if validate_session(provided):
        return True

    if not static_token and not google_enabled():
        json_response(
            handler,
            503,
            {
                "error": "Autenticacao nao configurada. Defina NEXUS_CONTROL_TOKEN ou habilite Google auth.",
            },
        )
        return False

    json_response(handler, 401, {"error": "token invalido"})
    return False


def read_json_body(handler: BaseHTTPRequestHandler):
    try:
        length = int(handler.headers.get("Content-Length", "0") or "0")
    except ValueError:
        length = 0
    if length <= 0:
        return {}
    raw = handler.rfile.read(length)
    if not raw:
        return {}
    try:
        return json.loads(raw.decode("utf-8"))
    except Exception:
        return {}


def make_handler(manager: ServiceManager):
    class Handler(BaseHTTPRequestHandler):
        server_version = "NexusControl/0.2"

        def log_message(self, fmt, *args):
            print(f"[HTTP] {self.address_string()} - {fmt % args}")

        def do_GET(self):
            parsed = urlparse(self.path)
            query = parse_qs(parsed.query)

            if parsed.path == "/":
                return text_response(self, 200, DASHBOARD_HTML, "text/html; charset=utf-8")

            if parsed.path == "/api/health":
                return json_response(self, 200, {"status": "ok", "time": now_iso()})

            if parsed.path == "/api/auth/config":
                return json_response(
                    self,
                    200,
                    {
                        "google_enabled": google_enabled(),
                        "google_client_id": google_client_id() if google_enabled() else None,
                    },
                )

            if parsed.path == "/api/services":
                if not require_auth(self, query):
                    return
                return json_response(self, 200, {"services": manager.list_services()})

            if parsed.path.startswith("/api/logs/"):
                if not require_auth(self, query):
                    return
                raw_name = parsed.path.split("/api/logs/", 1)[1]
                name = clean_service_name(raw_name)
                if not name:
                    return json_response(self, 400, {"error": "nome de servico ausente"})
                try:
                    lines = int((query.get("lines", ["200"])[0] or "200"))
                except ValueError:
                    lines = 200
                logs = manager.read_logs(name, lines)
                return json_response(self, 200, {"name": name, "logs": logs})

            return json_response(self, 404, {"error": "rota nao encontrada"})

        def do_POST(self):
            parsed = urlparse(self.path)
            query = parse_qs(parsed.query)

            if parsed.path == "/api/auth/google":
                if not google_enabled():
                    return json_response(self, 503, {"error": "Google auth desabilitado"})
                payload = read_json_body(self)
                id_token = (payload.get("id_token") or "").strip()
                if not id_token:
                    return json_response(self, 400, {"error": "id_token ausente"})
                try:
                    email = verify_google_id_token(id_token)
                    session = create_session(email)
                    return json_response(self, 200, session)
                except PermissionError as e:
                    return json_response(self, 403, {"error": str(e)})
                except Exception as e:
                    return json_response(self, 401, {"error": str(e)})

            if parsed.path.startswith("/api/services/") and parsed.path.endswith("/start"):
                if not require_auth(self, query):
                    return
                raw_name = parsed.path[len("/api/services/") : -len("/start")]
                name = clean_service_name(raw_name)
                if not name:
                    return json_response(self, 400, {"error": "nome de servico ausente"})
                try:
                    payload = manager.start_service(name)
                    return json_response(self, 200, payload)
                except ValueError as e:
                    return json_response(self, 404, {"error": str(e)})
                except Exception as e:
                    return json_response(self, 500, {"error": str(e)})

            if parsed.path.startswith("/api/services/") and parsed.path.endswith("/stop"):
                if not require_auth(self, query):
                    return
                raw_name = parsed.path[len("/api/services/") : -len("/stop")]
                name = clean_service_name(raw_name)
                if not name:
                    return json_response(self, 400, {"error": "nome de servico ausente"})
                try:
                    payload = manager.stop_service(name)
                    return json_response(self, 200, payload)
                except Exception as e:
                    return json_response(self, 500, {"error": str(e)})

            return json_response(self, 404, {"error": "rota nao encontrada"})

    return Handler


def main():
    host = os.environ.get("NEXUS_CONTROL_HOST", DEFAULT_HOST).strip() or DEFAULT_HOST
    port = int(os.environ.get("NEXUS_CONTROL_PORT", str(DEFAULT_PORT)))

    manager = ServiceManager(ROOT_DIR, CONFIG_PATH, STATE_PATH)
    handler = make_handler(manager)

    server = ThreadingHTTPServer((host, port), handler)
    print(f"[NEXUS Control] Listening on http://{host}:{port}")
    if expected_token():
        print("[NEXUS Control] Static token auth: ENABLED")
    else:
        print("[NEXUS Control] Static token auth: DISABLED")

    if google_enabled():
        allow = allowed_google_emails()
        allow_msg = ", ".join(sorted(allow)) if allow else "(sem allowlist)"
        print("[NEXUS Control] Google auth: ENABLED")
        print(f"[NEXUS Control] Google allowlist: {allow_msg}")
    else:
        print("[NEXUS Control] Google auth: DISABLED")

    print("[NEXUS Control] Abra no Google Chrome e faça login/token no painel.")
    server.serve_forever()


if __name__ == "__main__":
    main()



