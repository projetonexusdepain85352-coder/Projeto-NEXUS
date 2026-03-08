let authToken = null;
let googleCfg = { google_enabled: false, google_client_id: null };
let servicesByName = new Map();
let googleSetupAttempts = 0;
let clockTimer = null;
let terminalPollTimer = null;

function esc(value) {
  return String(value ?? '')
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;');
}

function toast(msg, isError = false) {
  const t = document.getElementById('toast');
  t.textContent = msg;
  t.className = 'show' + (isError ? ' error' : '');
  clearTimeout(t._timer);
  t._timer = setTimeout(() => {
    t.className = '';
  }, 3500);
}

function loginStatus(msg, blinking = false) {
  document.getElementById('login-msg').textContent = msg;
  document.getElementById('login-cursor').style.display = blinking ? 'inline' : 'none';
}

async function api(path, opts = {}) {
  const headers = {
    ...(authToken ? { Authorization: 'Bearer ' + authToken } : {}),
    ...(opts.headers || {}),
  };

  const res = await fetch(path, {
    ...opts,
    headers,
    credentials: 'same-origin',
    cache: 'no-store',
  });

  const data = await res.json().catch(() => ({}));

  if (!res.ok) {
    const msg = data.error || ('HTTP ' + res.status);
    if (res.status === 401 && path !== '/api/auth/google') {
      authToken = null;
      stopDashboardTimers();
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
    const data = await fetch('/api/auth/config', { cache: 'no-store' }).then((r) => r.json());
    googleCfg = data;
  } catch {
    googleCfg = { google_enabled: false, google_client_id: null };
  }
}

function setupGoogleButton() {
  googleSetupAttempts += 1;

  if (!googleCfg.google_client_id) {
    loginStatus('Google auth nao configurado no backend.', false);
    return;
  }

  if (!window.google || !google.accounts || !google.accounts.id) {
    if (googleSetupAttempts > 20) {
      loginStatus('SDK Google nao carregou. Verifique bloqueio de extensao/CSP.', false);
      return;
    }
    setTimeout(setupGoogleButton, 500);
    return;
  }

  google.accounts.id.initialize({
    client_id: googleCfg.google_client_id,
    callback: onGoogleCredential,
    auto_select: false,
    ux_mode: 'popup',
  });

  const container = document.getElementById('google-btn-real');
  container.innerHTML = '';

  google.accounts.id.renderButton(container, {
    theme: 'filled_black',
    size: 'large',
    width: 300,
    text: 'signin_with',
    shape: 'rectangular',
  });

  loginStatus('Pronto. Clique em autenticar.', false);
}

async function onGoogleCredential(response) {
  loginStatus('Verificando credenciais...', true);
  try {
    const res = await fetch('/api/auth/google', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ id_token: response.credential }),
      credentials: 'same-origin',
      cache: 'no-store',
    });

    const data = await res.json();
    if (!res.ok) {
      throw new Error(data.error || 'Acesso negado');
    }

    authToken = data.token;
    showDashboard(data.email);
  } catch (e) {
    loginStatus('Erro: ' + e.message, false);
  }
}

function stopDashboardTimers() {
  if (clockTimer) {
    clearInterval(clockTimer);
    clockTimer = null;
  }
  if (terminalPollTimer) {
    clearInterval(terminalPollTimer);
    terminalPollTimer = null;
  }
}

function showDashboard(email) {
  document.getElementById('login-screen').style.display = 'none';
  document.getElementById('dashboard').style.display = 'block';
  document.getElementById('nav-email').textContent = email;

  refreshServices();
  updateClock();
  stopDashboardTimers();
  clockTimer = setInterval(updateClock, 1000);
  terminalPollTimer = setInterval(() => {
    if (authToken) {
      refreshTerminalOutput(true);
    }
  }, 1500);
}

async function logout() {
  try {
    if (authToken) {
      await fetch('/api/auth/logout', {
        method: 'POST',
        headers: { Authorization: 'Bearer ' + authToken },
        credentials: 'same-origin',
        cache: 'no-store',
      });
    }
  } catch (_) {
    // ignore
  }

  stopDashboardTimers();
  authToken = null;
  document.getElementById('login-screen').style.display = 'flex';
  document.getElementById('dashboard').style.display = 'none';
  loginStatus('Sessao encerrada.', false);
  if (window.google?.accounts?.id) {
    google.accounts.id.disableAutoSelect();
  }
}

function syncSelectOptions(selectId, services, previousValue) {
  const select = document.getElementById(selectId);
  select.innerHTML = '';

  services.forEach((service) => {
    const opt = document.createElement('option');
    opt.value = service.name;
    opt.textContent = service.name;
    select.appendChild(opt);
  });

  if (previousValue && services.some((s) => s.name === previousValue)) {
    select.value = previousValue;
  }
}

async function refreshServices() {
  try {
    const data = await api('/api/services');
    const services = data.services || [];

    servicesByName = new Map(services.map((s) => [s.name, s]));

    const tbody = document.getElementById('svc-tbody');
    const prevTerm = document.getElementById('term-service').value;
    tbody.innerHTML = '';

    if (!services.length) {
      tbody.innerHTML = '<tr><td colspan="5"><div class="empty-state"><div class="icon">◌</div>Nenhum servico configurado</div></td></tr>';
      syncSelectOptions('term-service', [], prevTerm);
      onTerminalServiceChange();
      return;
    }

    services.forEach((s) => {
      const tr = document.createElement('tr');
      const statusHtml = s.running
        ? '<span class="status-running">RUNNING</span>'
        : '<span class="status-stopped">STOPPED</span>';

      const tags = [];
      if (s.interactive) {
        tags.push(s.stdin_available ? '[tty: conectado]' : '[tty: reinicie]');
      }

      tr.innerHTML =
        '<td><span class="svc-name">' + esc(s.name) + '</span></td>' +
        '<td>' + statusHtml + '</td>' +
        '<td><span class="svc-pid">' + esc(s.pid || '--') + '</span></td>' +
        '<td><span class="svc-cmd">' + esc((s.command || []).join(' ')) + ' ' + esc(tags.join(' ')) + '</span></td>' +
        '<td>' +
          '<button class="btn-start" onclick="startSvc(' + JSON.stringify(s.name) + ')">&#9654; Start</button>' +
          '<button class="btn-stop" onclick="stopSvc(' + JSON.stringify(s.name) + ')">&#9632; Stop</button>' +
        '</td>';
      tbody.appendChild(tr);
    });

    const interactiveServices = services.filter((s) => s.interactive);
    syncSelectOptions('term-service', interactiveServices, prevTerm);
    onTerminalServiceChange();
  } catch (e) {
    toast(e.message, true);
  }
}

async function startSvc(name) {
  try {
    const res = await api('/api/services/' + encodeURIComponent(name) + '/start', { method: 'POST' });
    if (res.already_running) {
      toast('Servico ja estava em execucao: ' + name);
    } else {
      toast('Servico iniciado: ' + name);
    }
    await refreshServices();
    await refreshTerminalOutput(true);
  } catch (e) {
    toast(e.message, true);
  }
}

async function stopSvc(name) {
  try {
    await api('/api/services/' + encodeURIComponent(name) + '/stop', { method: 'POST' });
    toast('Servico encerrado: ' + name);
    await refreshServices();
    await refreshTerminalOutput(true);
  } catch (e) {
    toast(e.message, true);
  }
}

function onTerminalServiceChange() {
  const name = document.getElementById('term-service').value;
  const input = document.getElementById('term-input');
  const send = document.getElementById('term-send');
  const hint = document.getElementById('term-hint');
  const meta = servicesByName.get(name);

  if (!name || !meta) {
    send.disabled = true;
    input.disabled = true;
    hint.textContent = 'Nenhum servico interativo disponivel.';
    document.getElementById('term-output').textContent = '(sem sessão)';
    return;
  }

  send.disabled = false;
  input.disabled = false;

  if (!meta.running) {
    hint.textContent = 'Servico parado. Clique em Start para abrir a sessao de terminal.';
  } else if (!meta.stdin_available) {
    hint.textContent = 'Servico ativo sem canal TTY vinculado. Clique em Stop e Start para reconectar.';
  } else {
    hint.textContent = 'Sessao remota ativa. Voce pode enviar comandos e teclas de interação.';
  }

  refreshTerminalOutput(true);
}

async function sendTerminalPayload(payload, appendNewline) {
  const name = document.getElementById('term-service').value;
  if (!name) {
    return;
  }

  try {
    const res = await api('/api/services/' + encodeURIComponent(name) + '/stdin', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ input: payload, append_newline: appendNewline }),
    });

    if (res.accepted) {
      toast('Entrada enviada para ' + name);
    } else {
      toast(res.reason || 'Entrada nao aceita', true);
    }

    await refreshServices();
    await refreshTerminalOutput(true);
  } catch (e) {
    toast(e.message, true);
  }
}

async function sendTerminalInput() {
  const input = document.getElementById('term-input');
  const appendNewline = document.getElementById('term-append-newline').checked;
  const payload = input.value;

  if (!payload.trim()) {
    return;
  }

  await sendTerminalPayload(payload, appendNewline);
  input.value = '';
  input.focus();
}

async function sendTermShortcut(cmd) {
  const appendNewline = document.getElementById('term-append-newline').checked;
  await sendTerminalPayload(cmd, appendNewline);
}

async function refreshTerminalOutput(silent = false) {
  const name = document.getElementById('term-service').value;
  const chars = document.getElementById('term-chars').value || 12000;
  const box = document.getElementById('term-output');

  if (!name) {
    box.textContent = '(sem sessão)';
    return;
  }

  if (!silent) {
    box.textContent = 'Carregando terminal...';
  }

  try {
    const data = await api('/api/terminal/' + encodeURIComponent(name) + '?chars=' + encodeURIComponent(chars));
    box.textContent = data.output || '(sem saída)';
    box.scrollTop = box.scrollHeight;
  } catch (e) {
    box.textContent = 'Erro: ' + e.message;
  }
}

function updateClock() {
  const now = new Date();
  document.getElementById('status-time').textContent =
    now.toLocaleDateString('pt-BR') + ' · ' + now.toLocaleTimeString('pt-BR');
}

function initKeyboardBindings() {
  const termInput = document.getElementById('term-input');
  termInput.addEventListener('keydown', async (ev) => {
    if (ev.key === 'Enter' && !ev.shiftKey) {
      ev.preventDefault();
      await sendTerminalInput();
    }
  });
}

(async function boot() {
  initKeyboardBindings();
  await fetchAuthConfig();
  setupGoogleButton();
})();
