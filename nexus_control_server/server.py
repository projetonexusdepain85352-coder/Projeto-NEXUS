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
DASHBOARD_HTML = """<!DOCTYPE html>
<html lang="pt-BR">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>NEXUS — Control Server</title>
<script src="https://accounts.google.com/gsi/client" async defer></script>
<style>
  @import url('https://fonts.googleapis.com/css2?family=Share+Tech+Mono&family=Orbitron:wght@400;700;900&family=VT323&display=swap');

  :root {
    --bg: #020b08;
    --bg2: #041410;
    --green: #00ff88;
    --green-dim: #00994d;
    --green-faint: #00ff8812;
    --amber: #ffb700;
    --red: #ff3b5c;
    --cyan: #00d4ff;
    --text: #b8ffe0;
    --text-dim: #4a8f6a;
    --border: #1a4a30;
    --glow: 0 0 12px #00ff8844, 0 0 30px #00ff8811;
    --glow-strong: 0 0 20px #00ff8888, 0 0 60px #00ff8822;
  }

  *, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }
  html { scroll-behavior: smooth; }

  body {
    background: var(--bg);
    color: var(--text);
    font-family: 'Share Tech Mono', monospace;
    font-size: 14px;
    line-height: 1.7;
    overflow-x: hidden;
    min-height: 100vh;
  }

  body::before {
    content: '';
    position: fixed; inset: 0;
    background: repeating-linear-gradient(0deg, transparent, transparent 2px, rgba(0,0,0,0.07) 2px, rgba(0,0,0,0.07) 4px);
    pointer-events: none; z-index: 9999;
  }

  @keyframes flicker { 0%,100%{opacity:1} 92%{opacity:1} 93%{opacity:.93} 94%{opacity:1} 97%{opacity:.96} }
  body { animation: flicker 8s infinite; }

  .grid-bg {
    position: fixed; inset: 0;
    background-image: linear-gradient(rgba(0,255,136,.03) 1px, transparent 1px), linear-gradient(90deg, rgba(0,255,136,.03) 1px, transparent 1px);
    background-size: 40px 40px;
    pointer-events: none; z-index: 0;
  }

  /* LOGIN */
  #login-screen {
    position: fixed; inset: 0;
    display: flex; align-items: center; justify-content: center;
    z-index: 10;
  }

  .login-wrap {
    position: relative;
    width: 440px;
    border: 1px solid var(--border);
    background: rgba(2,15,10,.97);
    padding: 48px;
    text-align: center;
  }

  .login-wrap::before, .login-wrap::after,
  .login-wrap .corner-br, .login-wrap .corner-bl {
    content: '';
    position: absolute;
    width: 18px; height: 18px;
  }
  .login-wrap::before  { top:-1px;    left:-1px;  border-top: 2px solid var(--green);  border-left:  2px solid var(--green); }
  .login-wrap::after   { top:-1px;    right:-1px; border-top: 2px solid var(--green);  border-right: 2px solid var(--green); }
  .login-wrap .corner-br { bottom:-1px; right:-1px; border-bottom: 2px solid var(--green); border-right: 2px solid var(--green); }
  .login-wrap .corner-bl { bottom:-1px; left:-1px;  border-bottom: 2px solid var(--green); border-left:  2px solid var(--green); }

  .login-wrap-glow {
    position: absolute; top: -1px; left: 20%; right: 20%;
    height: 1px;
    background: linear-gradient(90deg, transparent, var(--green), transparent);
  }

  .login-sys-tag {
    font-size: 10px; letter-spacing: 4px; color: var(--text-dim);
    text-transform: uppercase; margin-bottom: 32px;
  }
  .login-sys-tag span { color: var(--green-dim); }

  .login-logo {
    font-family: 'Orbitron', monospace;
    font-size: 54px; font-weight: 900;
    color: var(--green);
    text-shadow: var(--glow-strong);
    letter-spacing: 10px;
    line-height: 1;
    margin-bottom: 4px;
  }

  .login-subtitle {
    font-size: 10px; letter-spacing: 5px; color: var(--text-dim);
    text-transform: uppercase; margin-bottom: 40px;
  }

  .login-divider {
    border: none; border-top: 1px solid var(--border);
    margin: 32px 0;
    position: relative;
  }
  .login-divider::after {
    content: 'AUTH';
    position: absolute; top: -8px; left: 50%; transform: translateX(-50%);
    background: rgba(2,15,10,.97);
    padding: 0 12px;
    font-size: 9px; letter-spacing: 3px; color: var(--text-dim);
  }

  .google-btn-wrap {
    width: 100%;
    border: 1px solid var(--green-dim);
    padding: 14px 0 10px;
    display: flex; flex-direction: column; align-items: center; gap: 10px;
    position: relative;
    transition: border-color 0.2s, box-shadow 0.2s;
  }
  .google-btn-wrap:hover {
    border-color: var(--green);
    box-shadow: var(--glow);
  }
  .google-btn-label {
    font-family: 'Orbitron', monospace;
    font-size: 9px; font-weight: 700; letter-spacing: 4px;
    color: var(--green-dim); text-transform: uppercase;
  }
  #google-btn-real { display: flex; justify-content: center; }
  /* Strip Google button border/shadow to blend in */
  #google-btn-real > div { box-shadow: none !important; }

  #login-status {
    margin-top: 20px;
    font-size: 11px; letter-spacing: 2px;
    color: var(--text-dim); min-height: 18px;
  }

  @keyframes blink { 0%,100%{opacity:1} 50%{opacity:0} }
  .status-blink { animation: blink 1s step-end infinite; }

  .login-footer {
    margin-top: 36px;
    font-size: 10px; letter-spacing: 2px; color: var(--text-dim);
    opacity: 0.5;
  }

  /* DASHBOARD */
  #dashboard { display: none; position: relative; z-index: 2; }

  nav {
    position: sticky; top: 0; z-index: 100;
    background: rgba(2,11,8,.96);
    border-bottom: 1px solid var(--border);
    backdrop-filter: blur(12px);
    padding: 12px 0;
  }
  .nav-inner {
    max-width: 1100px; margin: 0 auto; padding: 0 28px;
    display: flex; align-items: center; gap: 20px;
  }
  .nav-logo {
    font-family: 'Orbitron', monospace; font-size: 16px; font-weight: 900;
    color: var(--green); letter-spacing: 6px;
    text-shadow: var(--glow); margin-right: auto;
  }
  .nav-logo span { font-size: 10px; letter-spacing: 3px; color: var(--text-dim); font-weight: 400; font-family: 'Share Tech Mono', monospace; display: block; }

  .nav-user {
    display: flex; align-items: center; gap: 10px;
    font-size: 11px; color: var(--text-dim); letter-spacing: 1px;
  }
  .nav-user-dot {
    width: 7px; height: 7px; border-radius: 50%;
    background: var(--green); box-shadow: 0 0 8px var(--green);
  }
  .nav-user em { color: var(--green); font-style: normal; }

  .nav-logout {
    background: transparent; border: 1px solid var(--border);
    color: var(--text-dim); font-family: 'Share Tech Mono', monospace;
    font-size: 10px; letter-spacing: 2px; padding: 5px 12px;
    cursor: pointer; transition: all 0.2s; text-transform: uppercase;
  }
  .nav-logout:hover { border-color: var(--red); color: var(--red); }

  main { max-width: 1100px; margin: 0 auto; padding: 32px 28px; }

  .section-label {
    font-size: 10px; color: var(--green-dim);
    letter-spacing: 4px; text-transform: uppercase; margin-bottom: 6px;
  }
  h2 {
    font-family: 'Orbitron', monospace; font-size: 14px; font-weight: 700;
    color: var(--green); letter-spacing: 3px; text-transform: uppercase;
    margin-bottom: 20px;
  }

  .panel {
    border: 1px solid var(--border);
    background: rgba(4,20,16,.7);
    margin-bottom: 24px;
    position: relative; overflow: hidden;
  }
  .panel::before {
    content: '';
    position: absolute; top: 0; left: 0; right: 0; height: 1px;
    background: linear-gradient(90deg, transparent, var(--green-dim), transparent);
  }

  .panel-header {
    background: rgba(0,0,0,.3);
    border-bottom: 1px solid var(--border);
    padding: 10px 20px;
    display: flex; align-items: center; gap: 8px;
    font-size: 10px; color: var(--text-dim); letter-spacing: 3px; text-transform: uppercase;
  }
  .panel-dot { width: 7px; height: 7px; border-radius: 50%; }
  .pd-red   { background: #ff3b5c44; border: 1px solid #ff3b5c55; }
  .pd-amber { background: #ffb70044; border: 1px solid #ffb70055; }
  .pd-green { background: #00ff8844; border: 1px solid #00ff8855; }
  .panel-body { padding: 20px 24px; }

  .svc-table { width: 100%; border-collapse: collapse; font-size: 13px; }
  .svc-table th {
    font-family: 'Orbitron', monospace; font-size: 9px; letter-spacing: 2px;
    color: var(--text-dim); text-transform: uppercase;
    border-bottom: 1px solid var(--border);
    padding: 8px 12px 8px 0; text-align: left;
  }
  .svc-table td {
    padding: 12px 12px 12px 0;
    border-bottom: 1px solid rgba(26,74,48,.35);
    vertical-align: middle;
  }
  .svc-table tr:last-child td { border-bottom: none; }
  .svc-table tr:hover td { background: var(--green-faint); }

  .svc-name { color: var(--cyan); font-weight: 600; }
  .svc-cmd  { color: var(--text-dim); font-size: 11px; }
  .svc-pid  { color: var(--text-dim); font-size: 12px; }

  @keyframes pulse { 0%,100%{opacity:1;box-shadow:0 0 8px var(--green)} 50%{opacity:.5;box-shadow:0 0 3px var(--green)} }

  .status-running {
    display: inline-flex; align-items: center; gap: 7px;
    font-family: 'Orbitron', monospace; font-size: 9px; font-weight: 700;
    letter-spacing: 2px; color: var(--green);
  }
  .status-running::before {
    content: ''; width: 6px; height: 6px; border-radius: 50%;
    background: var(--green); box-shadow: 0 0 8px var(--green);
    animation: pulse 2s ease infinite;
  }
  .status-stopped {
    display: inline-flex; align-items: center; gap: 7px;
    font-family: 'Orbitron', monospace; font-size: 9px; font-weight: 700;
    letter-spacing: 2px; color: var(--red);
  }
  .status-stopped::before {
    content: ''; width: 6px; height: 6px; border-radius: 50%;
    background: var(--red);
  }

  .btn-start, .btn-stop {
    font-family: 'Share Tech Mono', monospace;
    font-size: 10px; letter-spacing: 2px; text-transform: uppercase;
    padding: 5px 14px; border: 1px solid;
    background: transparent; cursor: pointer; transition: all 0.15s;
    margin-right: 4px;
  }
  .btn-start { color: var(--green); border-color: var(--green-dim); }
  .btn-start:hover { background: var(--green-faint); box-shadow: 0 0 10px #00ff8833; }
  .btn-stop  { color: var(--red); border-color: #66001a; }
  .btn-stop:hover  { background: rgba(255,59,92,.08); }

  .btn-refresh {
    font-family: 'Orbitron', monospace; font-size: 9px; font-weight: 700;
    letter-spacing: 3px; text-transform: uppercase;
    background: transparent; border: 1px solid var(--green-dim);
    color: var(--green); padding: 7px 20px;
    cursor: pointer; transition: all 0.2s;
  }
  .btn-refresh:hover { box-shadow: var(--glow); background: var(--green-faint); }

  .logs-controls {
    display: flex; align-items: center; gap: 10px; margin-bottom: 14px; flex-wrap: wrap;
  }
  .logs-select, .logs-lines {
    background: #010d08; border: 1px solid var(--border);
    color: var(--text); font-family: 'Share Tech Mono', monospace;
    font-size: 12px; padding: 7px 12px;
  }
  .logs-select:focus, .logs-lines:focus { outline: none; border-color: var(--green-dim); }

  .btn-load-logs {
    font-family: 'Orbitron', monospace; font-size: 9px; font-weight: 700;
    letter-spacing: 2px; text-transform: uppercase;
    background: transparent; border: 1px solid var(--cyan);
    color: var(--cyan); padding: 7px 18px;
    cursor: pointer; transition: all 0.2s;
  }
  .btn-load-logs:hover { background: rgba(0,212,255,.08); box-shadow: 0 0 12px rgba(0,212,255,.2); }

  .logs-output {
    background: #010d08; border: 1px solid var(--border);
    padding: 16px 20px;
    font-size: 12px; line-height: 1.7; color: var(--text-dim);
    white-space: pre-wrap; word-break: break-all;
    max-height: 360px; overflow-y: auto; min-height: 80px;
  }
  .logs-output::-webkit-scrollbar { width: 4px; }
  .logs-output::-webkit-scrollbar-thumb { background: var(--border); }

  .status-bar {
    display: flex; align-items: center; gap: 20px;
    padding: 10px 0; border-top: 1px solid var(--border);
    margin-top: 24px;
    font-size: 11px; color: var(--text-dim); letter-spacing: 1px;
  }
  .status-bar-item { display: flex; align-items: center; gap: 6px; }
  .status-bar-item .dot { width: 5px; height: 5px; border-radius: 50%; background: var(--green-dim); }
  .status-bar-time { margin-left: auto; font-size: 10px; }

  #toast {
    position: fixed; bottom: 24px; right: 24px;
    background: rgba(2,20,12,.95);
    border: 1px solid var(--green-dim);
    color: var(--green);
    font-size: 11px; letter-spacing: 2px;
    padding: 12px 20px; z-index: 9998;
    transform: translateY(20px); opacity: 0;
    transition: all 0.25s; pointer-events: none;
  }
  #toast.show { transform: translateY(0); opacity: 1; }
  #toast.error { border-color: #66001a; color: var(--red); }

  .empty-state {
    text-align: center; padding: 40px 20px;
    color: var(--text-dim); font-size: 12px; letter-spacing: 2px;
  }
  .empty-state .icon { font-size: 32px; margin-bottom: 12px; opacity: .4; }
</style>
</head>
<body>

<div class="grid-bg"></div>

<div id="login-screen">
  <div class="login-wrap">
    <div class="login-wrap-glow"></div>
    <div class="corner-br"></div>
    <div class="corner-bl"></div>
    <div class="login-sys-tag">Sistema de controle remoto · <span>NEXUS-OS</span></div>
    <div class="login-logo">NEXUS</div>
    <div class="login-subtitle">Control Server · v1.0</div>
    <hr class="login-divider">
    <div class="google-btn-wrap">
      <div class="google-btn-label">— AUTENTICAR COM GOOGLE —</div>
      <div id="google-btn-real"></div>
    </div>
    <div id="login-status"><span class="status-blink" id="login-cursor" style="display:none">█</span><span id="login-msg"></span></div>
    <div class="login-footer">ACESSO RESTRITO · SOMENTE CONTAS AUTORIZADAS</div>
  </div>
</div>

<div id="dashboard">
  <nav>
    <div class="nav-inner">
      <div class="nav-logo">
        NEXUS
        <span>Control Server</span>
      </div>
      <div class="nav-user">
        <div class="nav-user-dot"></div>
        Sessão ativa · <em id="nav-email">—</em>
      </div>
      <button class="nav-logout" onclick="logout()">[ sair ]</button>
    </div>
  </nav>

  <main>
    <div class="section-label">// 01</div>
    <h2>Serviços</h2>
    <div class="panel">
      <div class="panel-header">
        <div class="panel-dot pd-red"></div>
        <div class="panel-dot pd-amber"></div>
        <div class="panel-dot pd-green"></div>
        <span style="margin-left:8px;">nexus — processos gerenciados</span>
        <button class="btn-refresh" style="margin-left:auto" onclick="refreshServices()">↺ Atualizar</button>
      </div>
      <div class="panel-body">
        <table class="svc-table">
          <thead>
            <tr>
              <th>Serviço</th><th>Status</th><th>PID</th><th>Comando</th><th>Ações</th>
            </tr>
          </thead>
          <tbody id="svc-tbody">
            <tr><td colspan="5"><div class="empty-state"><div class="icon">⟳</div>Carregando...</div></td></tr>
          </tbody>
        </table>
      </div>
    </div>

    <div class="section-label">// 02</div>
    <h2>Logs</h2>
    <div class="panel">
      <div class="panel-header">
        <div class="panel-dot pd-red"></div>
        <div class="panel-dot pd-amber"></div>
        <div class="panel-dot pd-green"></div>
        <span style="margin-left:8px;">stdout — stream de saída</span>
      </div>
      <div class="panel-body">
        <div class="logs-controls">
          <select id="logs-service" class="logs-select"></select>
          <input id="logs-lines" type="number" min="1" max="1000" value="200" class="logs-lines" style="width:90px">
          <button class="btn-load-logs" onclick="loadLogs()">▶ Carregar</button>
        </div>
        <div class="logs-output" id="logs-output">(sem logs)</div>
      </div>
    </div>

    <div class="status-bar">
      <div class="status-bar-item"><div class="dot"></div>Worker: nexus-control.projeton-e-x-u-sdepain85352.workers.dev</div>
      <div class="status-bar-item"><div class="dot"></div>Porta local: 8787</div>
      <div class="status-bar-time" id="status-time">—</div>
    </div>
  </main>
</div>

<div id="toast"></div>

<script>
let authToken = null;
let googleCfg = { google_enabled: false, google_client_id: null };

function toast(msg, isError = false) {
  const t = document.getElementById('toast');
  t.textContent = msg;
  t.className = 'show' + (isError ? ' error' : '');
  clearTimeout(t._timer);
  t._timer = setTimeout(() => { t.className = ''; }, 3500);
}

function loginStatus(msg, blinking = false) {
  document.getElementById('login-msg').textContent = msg;
  document.getElementById('login-cursor').style.display = blinking ? 'inline' : 'none';
}

async function api(path, opts = {}) {
  const headers = { ...(authToken ? { 'Authorization': 'Bearer ' + authToken } : {}), ...(opts.headers || {}) };
  const res = await fetch(path, { ...opts, headers, credentials: 'same-origin', cache: 'no-store' });
  const data = await res.json().catch(() => ({}));

  if (!res.ok) {
    const msg = data.error || ('HTTP ' + res.status);
    if (res.status === 401 && path !== '/api/auth/google') {
      authToken = null;
      document.getElementById('login-screen').style.display = 'flex';
      document.getElementById('dashboard').style.display = 'none';
      loginStatus('Sessao expirada. Faca login novamente.', false);
    }
    throw new Error(msg);
  }

  return data;
}

async function fetchAuthConfig() {
  try {
    const data = await fetch('/api/auth/config').then(r => r.json());
    googleCfg = data;
  } catch { googleCfg = { google_enabled: false, google_client_id: null }; }
}

function setupGoogleButton() {
  if (!window.google || !google.accounts || !googleCfg.google_client_id) {
    setTimeout(setupGoogleButton, 500);
    return;
  }
  google.accounts.id.initialize({
    client_id: googleCfg.google_client_id,
    callback: onGoogleCredential,
    auto_select: false,
    ux_mode: 'popup'
  });
  const container = document.getElementById('google-btn-real');
  container.innerHTML = '';
  google.accounts.id.renderButton(container, {
    theme: 'filled_black',
    size: 'large',
    width: 300,
    text: 'signin_with',
    shape: 'rectangular'
  });
  loginStatus('Pronto. Clique em autenticar.', false);
}

async function onGoogleCredential(response) {
  loginStatus('Verificando credenciais...', true);
  try {
    const res = await fetch('/api/auth/google', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ id_token: response.credential })
    });
    const data = await res.json();
    if (!res.ok) throw new Error(data.error || 'Acesso negado');
    authToken = data.token;
    showDashboard(data.email);
  } catch (e) {
    loginStatus('⚠ ' + e.message, false);
  }
}

function showDashboard(email) {
  document.getElementById('login-screen').style.display = 'none';
  document.getElementById('dashboard').style.display = 'block';
  document.getElementById('nav-email').textContent = email;
  refreshServices();
  updateClock();
  setInterval(updateClock, 1000);
}

async function logout() {
  try {
    if (authToken) {
      await fetch('/api/auth/logout', {
        method: 'POST',
        headers: { 'Authorization': 'Bearer ' + authToken },
        credentials: 'same-origin',
        cache: 'no-store'
      });
    }
  } catch (_) {}

  authToken = null;
  document.getElementById('login-screen').style.display = 'flex';
  document.getElementById('dashboard').style.display = 'none';
  loginStatus('Sessao encerrada.', false);
  if (window.google?.accounts?.id) google.accounts.id.disableAutoSelect();
}

async function refreshServices() {
  try {
    const data = await api('/api/services');
    const tbody = document.getElementById('svc-tbody');
    const sel   = document.getElementById('logs-service');
    const prev  = sel.value;
    tbody.innerHTML = '';
    sel.innerHTML   = '';

    if (!data.services?.length) {
      tbody.innerHTML = '<tr><td colspan="5"><div class="empty-state"><div class="icon">◌</div>Nenhum servico configurado</div></td></tr>';
      return;
    }

    data.services.forEach(s => {
      const tr = document.createElement('tr');
      const statusHtml = s.running
        ? '<span class="status-running">RUNNING</span>'
        : '<span class="status-stopped">STOPPED</span>';
      tr.innerHTML =
        '<td><span class="svc-name">' + s.name + '</span></td>' +
        '<td>' + statusHtml + '</td>' +
        '<td><span class="svc-pid">' + (s.pid || '--') + '</span></td>' +
        '<td><span class="svc-cmd">' + s.command.join(' ') + '</span></td>' +
        '<td>' +
          '<button class="btn-start" onclick="startSvc(\'' + s.name + '\')">&#9654; Start</button>' +
          '<button class="btn-stop"  onclick="stopSvc(\''  + s.name + '\')">&#9632; Stop</button>' +
        '</td>';
      tbody.appendChild(tr);

      const opt = document.createElement('option');
      opt.value = s.name; opt.textContent = s.name;
      sel.appendChild(opt);
    });

    if (prev) sel.value = prev;
  } catch (e) {
    toast(e.message, true);
  }
}

async function startSvc(name) {
  try {
    await api('/api/services/' + encodeURIComponent(name) + '/start', { method: 'POST' });
    toast('▶ ' + name + ' iniciado');
    await refreshServices();
  } catch (e) { toast(e.message, true); }
}

async function stopSvc(name) {
  try {
    await api('/api/services/' + encodeURIComponent(name) + '/stop', { method: 'POST' });
    toast('■ ' + name + ' encerrado');
    await refreshServices();
  } catch (e) { toast(e.message, true); }
}

async function loadLogs() {
  const name  = document.getElementById('logs-service').value;
  const lines = document.getElementById('logs-lines').value || 200;
  if (!name) return;
  const box = document.getElementById('logs-output');
  box.textContent = 'Carregando...';
  try {
    const data = await api('/api/logs/' + encodeURIComponent(name) + '?lines=' + lines);
    box.textContent = data.logs || '(sem logs)';
    box.scrollTop = box.scrollHeight;
  } catch (e) { box.textContent = '⚠ ' + e.message; }
}

function updateClock() {
  const now = new Date();
  document.getElementById('status-time').textContent =
    now.toLocaleDateString('pt-BR') + ' · ' + now.toLocaleTimeString('pt-BR');
}

(async function boot() {
  await fetchAuthConfig();
  setupGoogleButton();
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
        return [sys.executable if token == "{python}" else token for token in cmd]

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
                    return {"name": name, "running": True, "pid": pid, "already_running": True}

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

            creationflags = subprocess.CREATE_NEW_PROCESS_GROUP if os.name == "nt" else 0

            proc = subprocess.Popen(
                cmd, cwd=str(cwd), env=env,
                stdout=log_fp, stderr=subprocess.STDOUT,
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
    handler.send_header("Cross-Origin-Opener-Policy", "same-origin")
    handler.send_header("Cross-Origin-Resource-Policy", "same-origin")
    handler.send_header("Cache-Control", "no-store")
    handler.send_header("Pragma", "no-cache")

    if is_html:
        csp = (
            "default-src 'self'; "
            "script-src 'self' https://accounts.google.com 'unsafe-inline'; "
            "style-src 'self' 'unsafe-inline' https://fonts.googleapis.com; "
            "font-src 'self' https://fonts.gstatic.com data:; "
            "connect-src 'self' https://oauth2.googleapis.com; "
            "frame-src https://accounts.google.com; "
            "img-src 'self' data:; "
            "base-uri 'self'; form-action 'self'; frame-ancestors 'none'"
        )
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
                    return text_response(self, 200, DASHBOARD_HTML, "text/html; charset=utf-8")

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
