let selectedDisk = null;
let isOotb = false;

function showScreen(id) {
  document.querySelectorAll('.screen').forEach(s => s.classList.remove('active'));
  const el = document.getElementById(id);
  if (el) el.classList.add('active');
  if (id === 'screen-disks') loadDisks();
  if (id === 'screen-timezone') loadTimezones();
  if (id === 'screen-keyboard') loadKeyboards();
}

async function init() {
  try {
    const r = await fetch('/api/ip');
    const d = await r.json();
    const urlEl = document.getElementById('install-url');
    if (urlEl) urlEl.textContent = d.url;
    const qrBox = document.getElementById('qr-box');
    if (qrBox && d.qr_svg) {
      qrBox.innerHTML = d.qr_svg;
      qrBox.querySelector('svg') && (qrBox.querySelector('svg').style.width = '160px');
    }
  } catch(e) {}

  // Check mode
  try {
    const r = await fetch('/api/mode');
    const d = await r.json();
    if (d.mode === 'ootb') {
      isOotb = true;
      showScreen('screen-ootb-welcome');
      return;
    }
  } catch(e) {}
}

async function loadDisks() {
  const list = document.getElementById('disk-list');
  list.innerHTML = '<p style="color:var(--text2)">Cargando discos...</p>';
  try {
    const r = await fetch('/api/disks');
    const d = await r.json();
    list.innerHTML = '';
    d.disks.forEach(disk => {
      const el = document.createElement('div');
      el.className = 'disk-item';
      el.innerHTML = '<div><span class="disk-name">' + disk.name + '</span><br><span class="disk-info">' + disk.model + ' - ' + disk.type + '</span></div><span class="disk-size">' + disk.size + '</span>';
      el.onclick = () => {
        document.querySelectorAll('.disk-item').forEach(x => x.classList.remove('selected'));
        el.classList.add('selected');
        selectedDisk = disk.name;
        document.getElementById('btn-install').disabled = false;
      };
      list.appendChild(el);
    });
    if (d.disks.length === 0) {
      list.innerHTML = '<p style="color:var(--text2)">No se encontraron discos</p>';
    }
  } catch(e) {
    list.innerHTML = '<p style="color:var(--danger)">Error al cargar discos</p>';
  }
}

async function startInstall() {
  if (!selectedDisk) return;
  if (!confirm('Se formateara ' + selectedDisk + '. Todos los datos se perderan. Continuar?')) return;
  showScreen('screen-installing');
  try {
    await fetch('/api/install', {
      method: 'POST',
      headers: {'Content-Type': 'application/json'},
      body: JSON.stringify({disk: selectedDisk, hostname: 'tubeos'})
    });
    pollInstallProgress();
  } catch(e) {
    document.getElementById('install-status').textContent = 'Error: ' + e.message;
  }
}

function pollInstallProgress() {
  const interval = setInterval(async () => {
    try {
      const r = await fetch('/api/install/progress');
      const d = await r.json();
      if (d.state === 'done') {
        clearInterval(interval);
        showScreen('screen-done');
      } else if (d.state === 'error') {
        clearInterval(interval);
        document.getElementById('install-status').textContent = 'Error: ' + (d.message || 'Desconocido');
      } else if (d.state === 'running') {
        document.getElementById('install-status').textContent = d.message || 'Instalando...';
        document.getElementById('install-progress').style.width = '60%';
      } else {
        document.getElementById('install-status').textContent = 'Preparando...';
      }
    } catch(e) {}
  }, 1000);
}

async function reboot() {
  try { await fetch('/api/reboot', {method: 'POST'}); } catch(e) {}
  document.getElementById('install-status').textContent = 'Reiniciando...';
}

async function loadTimezones() {
  const sel = document.getElementById('tz-select');
  try {
    const r = await fetch('/api/timezones');
    const d = await r.json();
    sel.innerHTML = '';
    d.timezones.forEach(tz => {
      const opt = document.createElement('option');
      opt.value = tz; opt.textContent = tz;
      if (tz === 'Europe/Madrid') opt.selected = true;
      sel.appendChild(opt);
    });
  } catch(e) { sel.innerHTML = '<option>UTC</option>'; }
}

async function loadKeyboards() {
  const sel = document.getElementById('kb-select');
  try {
    const r = await fetch('/api/keyboards');
    const d = await r.json();
    sel.innerHTML = '';
    d.keyboards.forEach(kb => {
      const opt = document.createElement('option');
      opt.value = kb; opt.textContent = kb;
      if (kb === 'es') opt.selected = true;
      sel.appendChild(opt);
    });
  } catch(e) { sel.innerHTML = '<option>us</option>'; }
}

async function setTimezone() {
  const tz = document.getElementById('tz-select').value;
  await fetch('/api/ootb/timezone', {
    method: 'POST',
    headers: {'Content-Type': 'application/json'},
    body: JSON.stringify({timezone: tz})
  });
  showScreen('screen-keyboard');
}

async function setKeyboard() {
  const kb = document.getElementById('kb-select').value;
  await fetch('/api/ootb/keyboard', {
    method: 'POST',
    headers: {'Content-Type': 'application/json'},
    body: JSON.stringify({keyboard: kb})
  });
  showScreen('screen-user');
}

async function createUser(e) {
  e.preventDefault();
  const errEl = document.getElementById('user-error');
  errEl.textContent = '';
  const fullname = document.getElementById('inp-fullname').value.trim();
  const username = document.getElementById('inp-username').value.trim();
  const pw = document.getElementById('inp-password').value;
  const pw2 = document.getElementById('inp-password2').value;
  if (pw !== pw2) { errEl.textContent = 'Las contrasenas no coinciden'; return; }
  try {
    const r = await fetch('/api/ootb/user', {
      method: 'POST',
      headers: {'Content-Type': 'application/json'},
      body: JSON.stringify({username, password: pw, fullname})
    });
    const d = await r.json();
    if (d.error) { errEl.textContent = d.error; return; }
    await fetch('/api/ootb/finish', {method: 'POST'});
    document.getElementById('dashboard-link').href = d.dashboard_url || 'http://' + location.hostname;
    try {
      const r2 = await fetch('/api/ootb/dockermigrate');
      const d2 = await r2.json();
      document.getElementById('dockermigrate-link').href = d2.url;
    } catch(e) {}
    showScreen('screen-complete');
  } catch(e) { errEl.textContent = 'Error: ' + e.message; }
}

document.addEventListener('DOMContentLoaded', init);
