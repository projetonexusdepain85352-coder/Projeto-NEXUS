#!/usr/bin/env python3
import json
import os
import re
import secrets
import signal
import hashlib
import subprocess
import sys
import threading
import time
import urllib.parse
import urllib.request
try:
    import pty
except Exception:
    pty = None
from collections import deque
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
MAX_JSON_BODY_BYTES = 32 * 1024
MAX_TOKEN_LENGTH = 256
TOKEN_RE = re.compile(r"^[A-Za-z0-9._~-]{20,256}$")
MAX_STDIN_INPUT_BYTES = 2048
TERMINAL_BUFFER_MAX_CHUNKS = 4096
TERMINAL_RESPONSE_MAX_CHARS = 64000
SESSION_MAX_PER_EMAIL = 5
SESSION_BIND_CONTEXT = os.environ.get("NEXUS_SESSION_BIND_CONTEXT", "1").strip().lower() not in {"0", "false", "no"}
AUTH_RATE_LIMIT_WINDOW_SECONDS = 5 * 60
AUTH_RATE_LIMIT_MAX = 20
API_RATE_LIMIT_WINDOW_SECONDS = 60
API_RATE_LIMIT_MAX = 240
SERVICE_MUTATION_RATE_LIMIT_MAX = 60
LOGS_RATE_LIMIT_MAX = 120
RATE_LIMITS = {}
RATE_LIMITS_LOCK = threading.Lock()
FRONTEND_DIR = Path(__file__).resolve().parent / "frontend"
FRONTEND_INDEX_PATH = FRONTEND_DIR / "index.html"
FRONTEND_CSS_PATH = FRONTEND_DIR / "styles.css"
FRONTEND_JS_PATH = FRONTEND_DIR / "app.js"



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


def read_frontend_asset(path: Path):
    try:
        return path.read_text(encoding="utf-8")
    except Exception:
        return None


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


def google_client_id() -> str:
    return os.environ.get("NEXUS_GOOGLE_CLIENT_ID", "").strip()


def google_enabled() -> bool:
    return bool(google_client_id())


def allowed_google_emails() -> set:
    raw = os.environ.get("NEXUS_GOOGLE_ALLOWED_EMAILS", "").strip()
    if not raw:
        return set()
    return {p.strip().lower() for p in raw.split(",") if p.strip()}


def client_ip(handler: BaseHTTPRequestHandler) -> str:
    cf_ip = (handler.headers.get("CF-Connecting-IP") or "").strip()
    if cf_ip:
        return cf_ip

    xff = (handler.headers.get("X-Forwarded-For") or "").strip()
    if xff:
        return xff.split(",")[0].strip()

    try:
        return str(handler.client_address[0])
    except Exception:
        return "unknown"


def _normalize_user_agent(user_agent: str) -> str:
    return (user_agent or "").strip()[:512]


def session_context_from_request(handler: BaseHTTPRequestHandler) -> dict:
    ua = _normalize_user_agent(handler.headers.get("User-Agent", ""))
    return {
        "ip": client_ip(handler),
        "ua_hash": hashlib.sha256(ua.encode("utf-8", errors="ignore")).hexdigest()[:24],
    }


def _rate_limit_key(handler: BaseHTTPRequestHandler, scope: str) -> str:
    return f"{scope}:{client_ip(handler)}"


def _rate_limit_check(key: str, max_requests: int, window_seconds: int) -> tuple:
    now = time.time()
    with RATE_LIMITS_LOCK:
        bucket = RATE_LIMITS.setdefault(key, deque())

        while bucket and (now - bucket[0]) > window_seconds:
            bucket.popleft()

        if len(bucket) >= max_requests:
            retry_after = max(1, int(window_seconds - (now - bucket[0])))
            return False, retry_after

        bucket.append(now)
        return True, 0


def enforce_rate_limit(handler: BaseHTTPRequestHandler, scope: str, max_requests: int, window_seconds: int) -> tuple:
    return _rate_limit_check(_rate_limit_key(handler, scope), max_requests, window_seconds)


def purge_expired_sessions() -> None:
    now = int(time.time())
    with SESSIONS_LOCK:
        dead = [tok for tok, meta in SESSIONS.items() if int(meta.get("exp", 0)) <= now]
        for tok in dead:
            SESSIONS.pop(tok, None)


def _enforce_session_limit(email: str) -> None:
    if SESSION_MAX_PER_EMAIL <= 0:
        return

    with SESSIONS_LOCK:
        user_sessions = [(tok, meta) for tok, meta in SESSIONS.items() if meta.get("email") == email]
        if len(user_sessions) < SESSION_MAX_PER_EMAIL:
            return

        user_sessions.sort(key=lambda item: int(item[1].get("created_ts", 0)))
        while len(user_sessions) >= SESSION_MAX_PER_EMAIL:
            oldest_token, _ = user_sessions.pop(0)
            SESSIONS.pop(oldest_token, None)


def create_session(email: str, context: dict | None = None) -> dict:
    purge_expired_sessions()
    _enforce_session_limit(email)

    token = secrets.token_urlsafe(32)
    exp = int(time.time()) + SESSION_TTL_SECONDS
    meta = {
        "email": email,
        "exp": exp,
        "created_at": now_iso(),
        "created_ts": int(time.time()),
    }

    if SESSION_BIND_CONTEXT and context:
        meta["ip"] = context.get("ip")
        meta["ua_hash"] = context.get("ua_hash")

    with SESSIONS_LOCK:
        SESSIONS[token] = meta

    return {"token": token, "email": email, "expires_at": exp}


def validate_session(token: str, context: dict | None = None) -> bool:
    if not token or len(token) > MAX_TOKEN_LENGTH:
        return False
    if not TOKEN_RE.fullmatch(token):
        return False

    purge_expired_sessions()
    with SESSIONS_LOCK:
        session = SESSIONS.get(token)

    if not session:
        return False

    if SESSION_BIND_CONTEXT and context:
        expected_ip = session.get("ip")
        expected_ua = session.get("ua_hash")
        if expected_ip and expected_ip != context.get("ip"):
            return False
        if expected_ua and expected_ua != context.get("ua_hash"):
            return False

    return True


def revoke_session(token: str) -> bool:
    if not token:
        return False
    with SESSIONS_LOCK:
        return SESSIONS.pop(token, None) is not None


def verify_google_id_token(id_token: str) -> str:
    client_id = google_client_id()
    if not client_id:
        raise ValueError("Google auth desabilitado no servidor")

    if not id_token or len(id_token) > 4096:
        raise ValueError("Token Google invalido")

    url = GOOGLE_TOKENINFO_URL + urllib.parse.quote(id_token, safe="")
    req = urllib.request.Request(url, headers={"Accept": "application/json"})

    try:
        with urllib.request.urlopen(req, timeout=10) as resp:
            body = resp.read().decode("utf-8", errors="replace")
    except Exception as exc:
        raise ValueError("Falha na validacao do token Google") from exc

    try:
        data = json.loads(body)
    except Exception as exc:
        raise ValueError("Resposta de validacao Google invalida") from exc

    issuer = (data.get("iss") or "").strip()
    if issuer and issuer not in {"accounts.google.com", "https://accounts.google.com"}:
        raise ValueError("Issuer Google invalido")

    if data.get("aud") != client_id:
        raise ValueError("Token Google invalido para este projeto")

    email = (data.get("email") or "").strip().lower()
    if not email:
        raise ValueError("Token Google sem e-mail")

    if str(data.get("email_verified", "")).lower() != "true":
        raise ValueError("E-mail Google nao verificado")

    now = int(time.time())
    exp = int(data.get("exp", "0") or 0)
    nbf = int(data.get("nbf", "0") or 0)
    if exp and exp < now:
        raise ValueError("Token Google expirado")
    if nbf and nbf > now:
        raise ValueError("Token Google ainda nao valido")

    allow = allowed_google_emails()
    if allow and email not in allow:
        raise PermissionError("E-mail nao autorizado")

    return email


class ServiceManager:
    def __init__(self, root_dir: Path, config_path: Path, state_path: Path):
        self.root_dir = root_dir
        self.config_path = config_path
        self.state_path = state_path
        configured_root = (os.environ.get("NEXUS_PROJECT_ROOT", "") or "").strip()
        if configured_root:
            expanded_root = os.path.expandvars(os.path.expanduser(configured_root))
            candidate_root = Path(expanded_root).resolve()
            self.service_root = candidate_root if candidate_root.exists() else root_dir
        else:
            self.service_root = root_dir
        self.lock = threading.Lock()
        self.runtime_processes = {}

    def config(self):
        raw = read_json(self.config_path, {"services": {}})
        return raw.get("services", {})

    def _state(self):
        return read_json(self.state_path, {"services": {}})

    def _save_state(self, state) -> None:
        write_json(self.state_path, state)

    def _is_interactive_service(self, spec: dict) -> bool:
        return bool(spec.get("interactive", False))

    def _resolve_command(self, cmd):
        resolved = []
        for token in cmd:
            if token == "{python}":
                resolved.append(sys.executable)
                continue
            if isinstance(token, str):
                text = token.replace("{project_root}", str(self.service_root))
                text = os.path.expandvars(os.path.expanduser(text))
                resolved.append(text)
                continue
            resolved.append(token)
        return resolved

    def _resolve_cwd(self, raw_cwd) -> Path:
        value = str(raw_cwd or ".")
        value = value.replace("{project_root}", str(self.service_root))
        value = os.path.expandvars(os.path.expanduser(value))
        path = Path(value)
        if path.is_absolute():
            return path.resolve()
        return (self.service_root / path).resolve()

    def _terminate_pid(self, pid: int) -> None:
        if pid <= 0:
            return

        if os.name == "nt":
            subprocess.run(["taskkill", "/PID", str(pid), "/T", "/F"], check=False)
            return

        try:
            os.kill(pid, signal.SIGTERM)
        except ProcessLookupError:
            return

        deadline = time.time() + 5
        while time.time() < deadline and is_pid_running(pid):
            time.sleep(0.2)

        if is_pid_running(pid):
            try:
                os.kill(pid, signal.SIGKILL)
            except ProcessLookupError:
                pass

    def _teardown_runtime(self, name: str) -> None:
        info = self.runtime_processes.pop(name, None)
        if not info:
            return

        stdin_obj = info.get("stdin")
        if stdin_obj:
            try:
                stdin_obj.close()
            except Exception:
                pass

        pty_master_fd = info.get("pty_master_fd")
        if isinstance(pty_master_fd, int):
            try:
                os.close(pty_master_fd)
            except OSError:
                pass

    def _push_terminal_output(self, name: str, text: str) -> None:
        if not text:
            return

        with self.lock:
            info = self.runtime_processes.get(name)
            if not info:
                return
            chunks = info.get("output_chunks")
            if isinstance(chunks, deque):
                chunks.append(text)

    def _start_pty_reader(self, name: str, master_fd: int, log_file: Path):
        def _reader_loop():
            with log_file.open("a", encoding="utf-8") as log_fp:
                while True:
                    try:
                        data = os.read(master_fd, 4096)
                    except OSError:
                        break

                    if not data:
                        break

                    text = data.decode("utf-8", errors="replace")
                    log_fp.write(text)
                    log_fp.flush()
                    self._push_terminal_output(name, text)

            try:
                os.close(master_fd)
            except OSError:
                pass

        thread = threading.Thread(target=_reader_loop, daemon=True, name=f"nexus-pty-{name}")
        thread.start()
        return thread

    def _start_pipe_reader(self, name: str, proc: subprocess.Popen, log_file: Path):
        def _reader_loop():
            with log_file.open("a", encoding="utf-8") as log_fp:
                stream = proc.stdout
                if stream is None:
                    return

                while True:
                    chunk = stream.readline()
                    if not chunk:
                        break

                    log_fp.write(chunk)
                    log_fp.flush()
                    self._push_terminal_output(name, chunk)

        thread = threading.Thread(target=_reader_loop, daemon=True, name=f"nexus-pipe-{name}")
        thread.start()
        return thread

    def _cleanup(self, state) -> None:
        to_delete = []
        for name, meta in state.get("services", {}).items():
            pid = int(meta.get("pid", 0) or 0)
            if pid and not is_pid_running(pid):
                to_delete.append(name)

        for name in to_delete:
            state["services"].pop(name, None)
            self._teardown_runtime(name)

        dead_runtime = []
        for name, info in list(self.runtime_processes.items()):
            proc = info.get("proc")
            if not proc:
                dead_runtime.append(name)
                continue
            try:
                if proc.poll() is not None:
                    dead_runtime.append(name)
            except Exception:
                dead_runtime.append(name)

        for name in dead_runtime:
            self._teardown_runtime(name)

    def list_services(self):
        with self.lock:
            cfg = self.config()
            state = self._state()
            self._cleanup(state)
            self._save_state(state)

            out = []
            for name, spec in cfg.items():
                proc_state = state.get("services", {}).get(name, {})
                pid = int(proc_state.get("pid", 0) or 0)
                info = self.runtime_processes.get(name)
                runtime_proc = info.get("proc") if info else None
                running_runtime = bool(runtime_proc and runtime_proc.poll() is None)
                stdin_available = bool(
                    info
                    and running_runtime
                    and (info.get("pty_master_fd") is not None or info.get("stdin") is not None)
                )

                out.append({
                    "name": name,
                    "running": is_pid_running(pid),
                    "pid": pid if pid else None,
                    "command": spec.get("command", []),
                    "cwd": spec.get("cwd", "."),
                    "interactive": self._is_interactive_service(spec),
                    "stdin_available": stdin_available,
                    "log_file": proc_state.get("log_file"),
                    "started_at": proc_state.get("started_at"),
                })
            return out

    def start_service(self, name: str):
        with self.lock:
            cfg = self.config()
            if name not in cfg:
                raise ValueError(f"Servico '{name}' nao existe")

            spec = cfg[name]
            interactive = self._is_interactive_service(spec)

            state = self._state()
            self._cleanup(state)

            existing = state.get("services", {}).get(name)
            if existing:
                pid = int(existing.get("pid", 0) or 0)
                if is_pid_running(pid):
                    info = self.runtime_processes.get(name)
                    if interactive and not info:
                        # Processo legado sem canal TTY em memoria.
                        # Reinicia para anexar terminal remoto corretamente.
                        self._terminate_pid(pid)
                        state["services"].pop(name, None)
                    else:
                        return {
                            "name": name,
                            "running": True,
                            "pid": pid,
                            "already_running": True,
                        }

            cmd = self._resolve_command(spec.get("command", []))
            if not cmd:
                raise ValueError(f"Servico '{name}' sem comando")

            cwd = self._resolve_cwd(spec.get("cwd", "."))
            if not cwd.exists() or not cwd.is_dir():
                raise ValueError(f"cwd invalido para servico '{name}': {cwd}")
            env = os.environ.copy()
            env.update(spec.get("env", {}))

            LOG_DIR.mkdir(parents=True, exist_ok=True)
            log_file = LOG_DIR / f"{name}.log"
            creationflags = subprocess.CREATE_NEW_PROCESS_GROUP if os.name == "nt" else 0

            proc = None
            runtime_info = {
                "proc": None,
                "interactive": interactive,
                "pty_master_fd": None,
                "stdin": None,
                "reader_thread": None,
                "output_chunks": deque(maxlen=TERMINAL_BUFFER_MAX_CHUNKS),
            }

            if interactive and os.name != "nt" and pty is not None:
                master_fd, slave_fd = pty.openpty()
                try:
                    proc = subprocess.Popen(
                        cmd,
                        cwd=str(cwd),
                        env=env,
                        stdin=slave_fd,
                        stdout=slave_fd,
                        stderr=slave_fd,
                        start_new_session=True,
                    )
                finally:
                    try:
                        os.close(slave_fd)
                    except OSError:
                        pass

                runtime_info["proc"] = proc
                runtime_info["pty_master_fd"] = master_fd
                self.runtime_processes[name] = runtime_info
                runtime_info["reader_thread"] = self._start_pty_reader(name, master_fd, log_file)

            elif interactive:
                proc = subprocess.Popen(
                    cmd,
                    cwd=str(cwd),
                    env=env,
                    stdin=subprocess.PIPE,
                    stdout=subprocess.PIPE,
                    stderr=subprocess.STDOUT,
                    text=True,
                    bufsize=1,
                    creationflags=creationflags,
                )

                runtime_info["proc"] = proc
                runtime_info["stdin"] = proc.stdin
                self.runtime_processes[name] = runtime_info
                runtime_info["reader_thread"] = self._start_pipe_reader(name, proc, log_file)

            else:
                log_fp = log_file.open("a", encoding="utf-8")
                try:
                    proc = subprocess.Popen(
                        cmd,
                        cwd=str(cwd),
                        env=env,
                        stdout=log_fp,
                        stderr=subprocess.STDOUT,
                        stdin=subprocess.DEVNULL,
                        creationflags=creationflags,
                    )
                finally:
                    log_fp.close()

                runtime_info["proc"] = proc
                self.runtime_processes[name] = runtime_info

            state.setdefault("services", {})[name] = {
                "pid": proc.pid,
                "started_at": now_iso(),
                "log_file": str(log_file),
                "command": cmd,
                "cwd": str(cwd),
            }
            self._save_state(state)

            return {
                "name": name,
                "running": True,
                "pid": proc.pid,
                "already_running": False,
                "interactive": interactive,
            }

    def stop_service(self, name: str):
        with self.lock:
            state = self._state()
            item = state.get("services", {}).get(name)
            if not item:
                return {"name": name, "running": False, "stopped": False, "reason": "not running"}

            pid = int(item.get("pid", 0) or 0)
            self._terminate_pid(pid)
            self._teardown_runtime(name)

            state["services"].pop(name, None)
            self._save_state(state)
            return {"name": name, "running": False, "stopped": True}

    def send_input(self, name: str, input_text: str, append_newline: bool = True):
        with self.lock:
            cfg = self.config()
            spec = cfg.get(name)
            if not spec:
                raise ValueError("servico nao encontrado")
            if not self._is_interactive_service(spec):
                raise PermissionError("servico nao aceita entrada interativa")

            state = self._state()
            self._cleanup(state)
            self._save_state(state)

            info = self.runtime_processes.get(name)
            if not info:
                return {
                    "name": name,
                    "running": False,
                    "accepted": False,
                    "reason": "terminal indisponivel; reinicie o servico por este painel",
                }

            proc = info.get("proc")
            if not proc or proc.poll() is not None:
                return {
                    "name": name,
                    "running": False,
                    "accepted": False,
                    "reason": "servico parado",
                }

            clean = (input_text or "").replace("\x00", "")
            payload = clean + ("\n" if append_newline else "")
            encoded = payload.encode("utf-8", errors="replace")
            if len(encoded) > MAX_STDIN_INPUT_BYTES:
                raise ValueError("entrada excede limite")

            pty_master_fd = info.get("pty_master_fd")
            if isinstance(pty_master_fd, int):
                try:
                    os.write(pty_master_fd, encoded)
                except OSError:
                    return {
                        "name": name,
                        "running": False,
                        "accepted": False,
                        "reason": "falha ao escrever no terminal",
                    }
            else:
                stdin_obj = info.get("stdin")
                if not stdin_obj:
                    return {
                        "name": name,
                        "running": True,
                        "accepted": False,
                        "reason": "stdin nao disponivel",
                    }
                try:
                    stdin_obj.write(payload)
                    stdin_obj.flush()
                except Exception:
                    return {
                        "name": name,
                        "running": False,
                        "accepted": False,
                        "reason": "falha ao escrever no processo",
                    }

            return {
                "name": name,
                "running": True,
                "accepted": True,
                "sent_bytes": len(encoded),
            }

    def read_terminal_output(self, name: str, max_chars: int = 12000) -> str:
        max_chars = max(256, min(max_chars, TERMINAL_RESPONSE_MAX_CHARS))

        with self.lock:
            info = self.runtime_processes.get(name)
            text = ""
            if info:
                chunks = info.get("output_chunks")
                if isinstance(chunks, deque):
                    text = "".join(chunks)

        if not text:
            text = self.read_logs(name, 200)

        if len(text) > max_chars:
            text = text[-max_chars:]

        return text

    def _safe_log_path(self, path_value: str):
        try:
            resolved = Path(path_value).resolve()
            base = LOG_DIR.resolve()
            resolved.relative_to(base)
            return resolved
        except Exception:
            return None

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

        safe_path = self._safe_log_path(log_file)
        if not safe_path or not safe_path.exists() or not safe_path.is_file():
            return ""

        content = safe_path.read_text(encoding="utf-8", errors="replace").splitlines()
        return "\n".join(content[-lines:])

def _is_production() -> bool:
    return (os.environ.get("NEXUS_ENV", "").strip().lower() == "production")


def _send_security_headers(handler: BaseHTTPRequestHandler, is_html: bool = False, extra_headers: dict | None = None):
    handler.send_header("X-Content-Type-Options", "nosniff")
    handler.send_header("X-Frame-Options", "DENY")
    handler.send_header("Referrer-Policy", "no-referrer")
    handler.send_header("Permissions-Policy", "geolocation=(), microphone=(), camera=()")
    handler.send_header("Cross-Origin-Opener-Policy", "same-origin-allow-popups")
    handler.send_header("Cross-Origin-Resource-Policy", "same-origin")
    handler.send_header("Cache-Control", "no-store")
    handler.send_header("Pragma", "no-cache")

    if is_html:
        csp = (
            "default-src 'self'; "
            "script-src 'self' https://accounts.google.com 'unsafe-inline'; "
            "style-src 'self' 'unsafe-inline' https://fonts.googleapis.com https://accounts.google.com; "
            "font-src 'self' https://fonts.gstatic.com data:; "
            "connect-src 'self' https://oauth2.googleapis.com https://accounts.google.com; "
            "frame-src https://accounts.google.com; "
            "img-src 'self' data:; "
            "base-uri 'self'; form-action 'self'; frame-ancestors 'none'"
        )
    else:
        csp = "default-src 'none'; frame-ancestors 'none'; base-uri 'none'; form-action 'none'"

    handler.send_header("Content-Security-Policy", csp)

    if _is_production():
        handler.send_header("Strict-Transport-Security", "max-age=31536000; includeSubDomains")

    if extra_headers:
        for key, value in extra_headers.items():
            handler.send_header(str(key), str(value))


def json_response(handler: BaseHTTPRequestHandler, code: int, payload, extra_headers: dict | None = None):
    raw = json.dumps(payload, ensure_ascii=False).encode("utf-8")
    handler.send_response(code)
    _send_security_headers(handler, is_html=False, extra_headers=extra_headers)
    handler.send_header("Content-Type", "application/json; charset=utf-8")
    handler.send_header("Content-Length", str(len(raw)))
    handler.end_headers()
    handler.wfile.write(raw)


def text_response(handler: BaseHTTPRequestHandler, code: int, text: str, content_type: str, extra_headers: dict | None = None):
    raw = text.encode("utf-8")
    handler.send_response(code)
    _send_security_headers(handler, is_html=content_type.startswith("text/html"), extra_headers=extra_headers)
    handler.send_header("Content-Type", content_type)
    handler.send_header("Content-Length", str(len(raw)))
    handler.end_headers()
    handler.wfile.write(raw)


def read_token_from_request(handler: BaseHTTPRequestHandler, query):
    auth = handler.headers.get("Authorization", "")
    if not auth.lower().startswith("bearer "):
        return ""

    token = auth[7:].strip()
    if not token or len(token) > MAX_TOKEN_LENGTH:
        return ""
    if not TOKEN_RE.fullmatch(token):
        return ""
    return token


def clean_service_name(raw: str) -> str:
    name = urllib.parse.unquote(raw or "").strip().strip("/")
    if SERVICE_NAME_RE.fullmatch(name):
        return name
    return ""


def require_auth(handler: BaseHTTPRequestHandler, query, scope: str = "api") -> bool:
    allowed, retry_after = enforce_rate_limit(
        handler,
        f"auth:{scope}",
        API_RATE_LIMIT_MAX,
        API_RATE_LIMIT_WINDOW_SECONDS,
    )
    if not allowed:
        json_response(
            handler,
            429,
            {"error": "muitas requisicoes"},
            extra_headers={"Retry-After": str(retry_after)},
        )
        return False

    provided = read_token_from_request(handler, query)
    context = session_context_from_request(handler)

    if validate_session(provided, context):
        return True

    if not google_enabled():
        json_response(handler, 503, {"error": "autenticacao indisponivel"})
        return False

    json_response(
        handler,
        401,
        {"error": "nao autenticado"},
        extra_headers={"WWW-Authenticate": "Bearer realm=\"nexus-control\""},
    )
    return False


def read_json_body(handler: BaseHTTPRequestHandler):
    content_type = (handler.headers.get("Content-Type") or "").lower()
    if not content_type.startswith("application/json"):
        raise ValueError("content-type deve ser application/json")

    try:
        length = int(handler.headers.get("Content-Length", "0") or "0")
    except ValueError:
        raise ValueError("content-length invalido")

    if length <= 0:
        raise ValueError("body vazio")
    if length > MAX_JSON_BODY_BYTES:
        raise ValueError("body excede limite")

    raw = handler.rfile.read(length)
    if not raw:
        raise ValueError("body vazio")

    try:
        parsed = json.loads(raw.decode("utf-8"))
    except Exception:
        raise ValueError("json invalido")

    if not isinstance(parsed, dict):
        raise ValueError("json deve ser objeto")

    return parsed


def make_handler(manager: ServiceManager):
    class Handler(BaseHTTPRequestHandler):
        server_version = "NexusControl/1.0"

        def log_message(self, fmt, *args):
            print(f"[HTTP] {self.address_string()} - {fmt % args}")

        def do_GET(self):
            try:
                parsed = urlparse(self.path)
                query = parse_qs(parsed.query)

                if parsed.path == "/":
                    html = read_frontend_asset(FRONTEND_INDEX_PATH)
                    if html is None:
                        return json_response(self, 500, {"error": "frontend indisponivel"})
                    return text_response(self, 200, html, "text/html; charset=utf-8")

                if parsed.path == "/frontend/styles.css":
                    css = read_frontend_asset(FRONTEND_CSS_PATH)
                    if css is None:
                        return json_response(self, 404, {"error": "asset nao encontrado"})
                    return text_response(self, 200, css, "text/css; charset=utf-8")

                if parsed.path == "/frontend/app.js":
                    js = read_frontend_asset(FRONTEND_JS_PATH)
                    if js is None:
                        return json_response(self, 404, {"error": "asset nao encontrado"})
                    return text_response(self, 200, js, "application/javascript; charset=utf-8")

                if parsed.path == "/api/health":
                    return json_response(self, 200, {"status": "ok", "time": now_iso()})

                if parsed.path == "/api/auth/config":
                    return json_response(self, 200, {
                        "google_enabled": google_enabled(),
                        "google_client_id": google_client_id() if google_enabled() else None,
                    })

                if parsed.path == "/api/services":
                    if not require_auth(self, query, scope="services:list"):
                        return
                    return json_response(self, 200, {"services": manager.list_services()})

                if parsed.path.startswith("/api/terminal/"):
                    if not require_auth(self, query, scope="terminal:read"):
                        return

                    allowed, retry_after = enforce_rate_limit(
                        self,
                        "terminal:read",
                        LOGS_RATE_LIMIT_MAX,
                        API_RATE_LIMIT_WINDOW_SECONDS,
                    )
                    if not allowed:
                        return json_response(
                            self,
                            429,
                            {"error": "muitas requisicoes"},
                            extra_headers={"Retry-After": str(retry_after)},
                        )

                    raw_name = parsed.path.split("/api/terminal/", 1)[1]
                    name = clean_service_name(raw_name)
                    if not name:
                        return json_response(self, 400, {"error": "nome de servico invalido"})

                    try:
                        chars = int((query.get("chars", ["12000"])[0] or "12000"))
                    except ValueError:
                        chars = 12000

                    output = manager.read_terminal_output(name, chars)
                    return json_response(self, 200, {"name": name, "output": output})

                if parsed.path.startswith("/api/logs/"):
                    if not require_auth(self, query, scope="logs:read"):
                        return

                    allowed, retry_after = enforce_rate_limit(
                        self,
                        "logs:read",
                        LOGS_RATE_LIMIT_MAX,
                        API_RATE_LIMIT_WINDOW_SECONDS,
                    )
                    if not allowed:
                        return json_response(
                            self,
                            429,
                            {"error": "muitas requisicoes"},
                            extra_headers={"Retry-After": str(retry_after)},
                        )

                    raw_name = parsed.path.split("/api/logs/", 1)[1]
                    name = clean_service_name(raw_name)
                    if not name:
                        return json_response(self, 400, {"error": "nome de servico invalido"})

                    try:
                        lines = int((query.get("lines", ["200"])[0] or "200"))
                    except ValueError:
                        lines = 200

                    logs = manager.read_logs(name, lines)
                    return json_response(self, 200, {"name": name, "logs": logs})

                return json_response(self, 404, {"error": "rota nao encontrada"})
            except Exception as exc:
                print(f"[NEXUS Control] GET error: {exc!r}")
                return json_response(self, 500, {"error": "erro interno"})

        def do_POST(self):
            try:
                parsed = urlparse(self.path)
                query = parse_qs(parsed.query)

                if parsed.path == "/api/auth/logout":
                    token = read_token_from_request(self, query)
                    if token:
                        revoke_session(token)
                    return json_response(self, 200, {"ok": True})

                if parsed.path == "/api/auth/google":
                    allowed, retry_after = enforce_rate_limit(
                        self,
                        "auth:google",
                        AUTH_RATE_LIMIT_MAX,
                        AUTH_RATE_LIMIT_WINDOW_SECONDS,
                    )
                    if not allowed:
                        return json_response(
                            self,
                            429,
                            {"error": "muitas tentativas de autenticacao"},
                            extra_headers={"Retry-After": str(retry_after)},
                        )

                    if not google_enabled():
                        return json_response(self, 503, {"error": "autenticacao Google desabilitada"})

                    try:
                        payload = read_json_body(self)
                    except ValueError as exc:
                        return json_response(self, 400, {"error": str(exc)})

                    id_token = payload.get("id_token")
                    if not isinstance(id_token, str) or not id_token.strip():
                        return json_response(self, 400, {"error": "id_token ausente"})

                    try:
                        email = verify_google_id_token(id_token.strip())
                        session = create_session(email, session_context_from_request(self))
                        return json_response(self, 200, session)
                    except PermissionError:
                        return json_response(self, 403, {"error": "acesso nao autorizado"})
                    except ValueError:
                        return json_response(self, 401, {"error": "token Google invalido"})
                    except Exception as exc:
                        print(f"[NEXUS Control] auth/google error: {exc!r}")
                        return json_response(self, 500, {"error": "erro interno"})

                if parsed.path.startswith("/api/services/") and parsed.path.endswith("/start"):
                    if not require_auth(self, query, scope="services:mutate"):
                        return

                    allowed, retry_after = enforce_rate_limit(
                        self,
                        "services:mutate",
                        SERVICE_MUTATION_RATE_LIMIT_MAX,
                        API_RATE_LIMIT_WINDOW_SECONDS,
                    )
                    if not allowed:
                        return json_response(
                            self,
                            429,
                            {"error": "muitas requisicoes de controle"},
                            extra_headers={"Retry-After": str(retry_after)},
                        )

                    raw_name = parsed.path[len("/api/services/"):-len("/start")]
                    name = clean_service_name(raw_name)
                    if not name:
                        return json_response(self, 400, {"error": "nome de servico invalido"})

                    try:
                        return json_response(self, 200, manager.start_service(name))
                    except ValueError:
                        return json_response(self, 404, {"error": "servico nao encontrado"})
                    except Exception as exc:
                        print(f"[NEXUS Control] start service error: {exc!r}")
                        return json_response(self, 500, {"error": "erro interno"})

                if parsed.path.startswith("/api/services/") and parsed.path.endswith("/stop"):
                    if not require_auth(self, query, scope="services:mutate"):
                        return

                    allowed, retry_after = enforce_rate_limit(
                        self,
                        "services:mutate",
                        SERVICE_MUTATION_RATE_LIMIT_MAX,
                        API_RATE_LIMIT_WINDOW_SECONDS,
                    )
                    if not allowed:
                        return json_response(
                            self,
                            429,
                            {"error": "muitas requisicoes de controle"},
                            extra_headers={"Retry-After": str(retry_after)},
                        )

                    raw_name = parsed.path[len("/api/services/"):-len("/stop")]
                    name = clean_service_name(raw_name)
                    if not name:
                        return json_response(self, 400, {"error": "nome de servico invalido"})

                    try:
                        return json_response(self, 200, manager.stop_service(name))
                    except Exception as exc:
                        print(f"[NEXUS Control] stop service error: {exc!r}")
                        return json_response(self, 500, {"error": "erro interno"})

                if parsed.path.startswith("/api/services/") and parsed.path.endswith("/stdin"):
                    if not require_auth(self, query, scope="services:mutate"):
                        return

                    allowed, retry_after = enforce_rate_limit(
                        self,
                        "services:mutate",
                        SERVICE_MUTATION_RATE_LIMIT_MAX,
                        API_RATE_LIMIT_WINDOW_SECONDS,
                    )
                    if not allowed:
                        return json_response(
                            self,
                            429,
                            {"error": "muitas requisicoes de controle"},
                            extra_headers={"Retry-After": str(retry_after)},
                        )

                    raw_name = parsed.path[len("/api/services/"):-len("/stdin")]
                    name = clean_service_name(raw_name)
                    if not name:
                        return json_response(self, 400, {"error": "nome de servico invalido"})

                    try:
                        payload = read_json_body(self)
                    except ValueError as exc:
                        return json_response(self, 400, {"error": str(exc)})

                    input_text = payload.get("input", "")
                    append_newline = payload.get("append_newline", True)

                    if not isinstance(input_text, str):
                        return json_response(self, 400, {"error": "campo 'input' deve ser string"})
                    if not isinstance(append_newline, bool):
                        append_newline = True

                    try:
                        result = manager.send_input(name, input_text, append_newline)
                        return json_response(self, 200, result)
                    except PermissionError:
                        return json_response(self, 403, {"error": "servico nao aceita entrada interativa"})
                    except ValueError as exc:
                        return json_response(self, 400, {"error": str(exc)})
                    except Exception as exc:
                        print(f"[NEXUS Control] stdin service error: {exc!r}")
                        return json_response(self, 500, {"error": "erro interno"})

                return json_response(self, 404, {"error": "rota nao encontrada"})
            except Exception as exc:
                print(f"[NEXUS Control] POST error: {exc!r}")
                return json_response(self, 500, {"error": "erro interno"})

    return Handler


def main():
    host = os.environ.get("NEXUS_CONTROL_HOST", DEFAULT_HOST).strip() or DEFAULT_HOST
    port = int(os.environ.get("NEXUS_CONTROL_PORT", str(DEFAULT_PORT)))

    manager = ServiceManager(ROOT_DIR, CONFIG_PATH, STATE_PATH)
    handler = make_handler(manager)

    server = ThreadingHTTPServer((host, port), handler)
    print(f"[NEXUS Control] Listening on http://{host}:{port}")

    if google_enabled():
        allow = allowed_google_emails()
        allow_msg = ", ".join(sorted(allow)) if allow else "(sem allowlist)"
        print("[NEXUS Control] Google auth: ENABLED")
        print(f"[NEXUS Control] Google allowlist: {allow_msg}")
    else:
        print("[NEXUS Control] Google auth: DISABLED")

    print("[NEXUS Control] Abra no Google Chrome e faca login no painel.")
    server.serve_forever()


if __name__ == "__main__":
    main()



