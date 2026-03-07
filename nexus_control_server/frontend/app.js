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

let googleSetupAttempts = 0;

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
