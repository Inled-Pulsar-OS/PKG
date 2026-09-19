// ==============================================================================
// Tube OS - Installer & OOTB Controller
// ==============================================================================

const steps = ['network', 'disk', 'edition', 'user', 'confirm', 'progress', 'done'];
let currentStepIndex = 0;
let maxVisitedStepIndex = 0;
let isOotbMode = false;

const wizardData = {
  distro: 'arch',
  disk: null,
  diskName: '',
  mode: 'clean',
  target_partition: null,
  fs_type: 'btrfs',
  edition: null,
  editionTitle: '',
  hostname: 'tubeos',
  username: 'tubeos',
  password: 'tubeos',
  timezone: 'Europe/Madrid',
  keymap: 'es',
};

let progressInterval = null;

// Clean vector icons
function getVectorIcon(name) {
  switch (name) {
    case 'tv-server':
      return `<svg class="card-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8"><rect x="2" y="7" width="20" height="15" rx="2"></rect><polyline points="17 2 12 7 7 2"></polyline></svg>`;
    case 'tv':
      return `<svg class="card-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8"><rect x="2" y="7" width="20" height="15" rx="2"></rect><polyline points="17 2 12 7 7 2"></polyline></svg>`;
    case 'server':
      return `<svg class="card-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8"><rect x="2" y="2" width="20" height="8" rx="2"></rect><rect x="2" y="14" width="20" height="8" rx="2"></rect><line x1="6" y1="6" x2="6.01" y2="6"></line><line x1="6" y1="18" x2="6.01" y2="18"></line></svg>`;
    case 'disk':
    default:
      return `<svg class="card-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8"><ellipse cx="12" cy="5" rx="9" ry="3"></ellipse><path d="M21 12c0 1.66-4 3-9 3s-9-1.34-9-3"></path><path d="M3 5v14c0 1.66 4 3 9 3s9-1.34 9-3V5"></path></svg>`;
  }
}

document.addEventListener('DOMContentLoaded', async () => {
  const info = await fetchSystemInfo();
  if (info && info.mode === 'ootb') {
    initOotbMode(info);
  } else {
    await checkNetwork();
    await loadDisks();
    await loadEditions();
    updateStepView();
  }
});

// ─── Backup & DockerMigrate Helpers ──────────────────────────────────────────

function toggleBackupAck() {
  const chk = document.getElementById('backup-ack');
  if (chk) {
    chk.checked = !chk.checked;
    backupAcknowledged = chk.checked;
    document.getElementById('backup-ack-row').classList.toggle('selected', backupAcknowledged);
    const btnCont = document.getElementById('btn-continue');
    if (btnCont && steps[currentStepIndex] === 'backup') {
      btnCont.disabled = !backupAcknowledged;
    }
  }
}

async function openDockerMigrate() {
  const host = window.location.hostname || 'tubeos.local';
  try {
    fetch('/api/dockermigrate/start', { method: 'POST' }).catch(() => {});
  } catch (e) {}
  const url = `http://${host}:8070`;
  window.open(url, '_blank');
}

// ─── OOTB First-Boot Mode ───────────────────────────────────────────────────

function initOotbMode(info) {
  isOotbMode = true;
  document.getElementById('base-badge').innerText = 'Welcome Wizard';
  
  // Hide standard installer stepper and show welcome section
  const sidebarNav = document.getElementById('sidebar-nav-links');
  if (sidebarNav) {
    sidebarNav.innerHTML = `
      <div class="nav-item active">
        <svg class="nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"></path><polyline points="9 22 9 12 15 12 15 22"></polyline></svg>
        <span>Welcome</span>
      </div>
      <div class="nav-item" onclick="openDockerMigrate()">
        <svg class="nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z"></path></svg>
        <span>DockerMigrate (:8070)</span>
      </div>
    `;
  }

  document.querySelectorAll('.step-container').forEach(el => el.classList.remove('active'));
  const ootbSection = document.getElementById('step-ootb');
  if (ootbSection) {
    ootbSection.classList.add('active');
  }

  const bottomBar = document.getElementById('window-bottombar');
  if (bottomBar) {
    bottomBar.style.display = 'none';
  }
}

async function completeOotbAndLaunchCasaOS() {
  try {
    const res = await fetch('/api/ootb/complete', { method: 'POST' });
    const data = await res.json();
    const targetUrl = data.redirect || `http://${window.location.hostname || 'tubeos.local'}/`;
    
    // Show smooth transition
    document.body.innerHTML = `
      <div style="display:flex;flex-direction:column;align-items:center;justify-content:center;height:100vh;color:#fff;font-family:sans-serif;text-align:center;">
        <div style="width:48px;height:48px;border:3px solid rgba(155,89,221,0.3);border-top-color:#9b59dd;border-radius:50%;animation:spin 1s linear infinite;margin-bottom:16px;"></div>
        <h2 style="font-size:20px;margin-bottom:6px;">Launching Tube OS / CasaOS Dashboard...</h2>
        <p style="color:#a1a1aa;font-size:13px;" id="launch-subtext">Starting services and opening dashboard...</p>
      </div>
      <style>@keyframes spin { 0% { transform: rotate(0deg); } 100% { transform: rotate(360deg); } }</style>
    `;

    let attempts = 0;
    const checkInterval = setInterval(async () => {
      attempts++;
      try {
        const check = await fetch(targetUrl, { method: 'HEAD', cache: 'no-store' });
        if (check.ok || attempts >= 8) {
          clearInterval(checkInterval);
          window.location.href = targetUrl;
        }
      } catch (_) {
        if (attempts >= 8) {
          clearInterval(checkInterval);
          window.location.href = targetUrl;
        }
      }
    }, 500);

  } catch (err) {
    console.error('OOTB complete failed:', err);
    window.location.href = `http://${window.location.hostname || 'tubeos.local'}/`;
  }
}

// ─── Step Flow & Clickable Sidebar ──────────────────────────────────────────

function goToStep(stepId) {
  const idx = steps.indexOf(stepId);
  if (idx !== -1) {
    currentStepIndex = idx;
    if (currentStepIndex > maxVisitedStepIndex) {
      maxVisitedStepIndex = currentStepIndex;
    }
    updateStepView();
  }
}

function onNavClick(targetIndex) {
  if (isOotbMode || currentStepIndex >= steps.indexOf('progress')) {
    return;
  }
  if (targetIndex <= maxVisitedStepIndex) {
    currentStepIndex = targetIndex;
    updateStepView();
  }
}

function handleContinue() {
  const cur = steps[currentStepIndex];
  if (cur === 'confirm') {
    startInstallation();
  } else if (currentStepIndex < steps.length - 1) {
    currentStepIndex++;
    if (currentStepIndex > maxVisitedStepIndex) {
      maxVisitedStepIndex = currentStepIndex;
    }
    updateStepView();
  }
}

function handleBack() {
  if (currentStepIndex > 0 && currentStepIndex < steps.indexOf('progress')) {
    currentStepIndex--;
    updateStepView();
  }
}

function updateStepView() {
  if (isOotbMode) return;
  const currentStep = steps[currentStepIndex];

  // Update Sections
  document.querySelectorAll('.step-container').forEach(el => el.classList.remove('active'));
  const activeSection = document.getElementById(`step-${currentStep}`);
  if (activeSection) {
    activeSection.classList.add('active');
  }

  // Update Sidebar Tabs
  document.querySelectorAll('.nav-item').forEach((item, idx) => {
    item.classList.remove('active', 'completed', 'disabled');
    if (idx === currentStepIndex) {
      item.classList.add('active');
    } else if (idx < currentStepIndex) {
      item.classList.add('completed');
    } else if (idx > maxVisitedStepIndex || currentStepIndex >= steps.indexOf('progress')) {
      item.classList.add('disabled');
    }
  });

  // Update Bottom Controls
  const btnBack = document.getElementById('btn-back');
  const btnCont = document.getElementById('btn-continue');
  const bottomBar = document.getElementById('window-bottombar');

  if (currentStep === 'progress' || currentStep === 'done') {
    bottomBar.style.display = 'none';
  } else {
    bottomBar.style.display = 'flex';
  }

  if (currentStepIndex === 0) {
    btnBack.style.display = 'none';
  } else {
    btnBack.style.display = 'inline-flex';
  }

  btnCont.disabled = false;

  if (currentStep === 'confirm') {
    populateSummary();
    btnCont.innerText = 'Install Tube OS';
    btnCont.className = 'mac-btn mac-btn-danger';
  } else {
    btnCont.innerText = 'Continue';
    btnCont.className = 'mac-btn mac-btn-primary';
  }
}

// ─── API & Network ──────────────────────────────────────────────────────────

async function fetchSystemInfo() {
  try {
    const res = await fetch('/api/info');
    const data = await res.json();
    wizardData.distro = data.distro;
    
    const baseBadge = document.getElementById('base-badge');
    if (baseBadge) {
      baseBadge.innerText = `${data.distro.toUpperCase()} Base`;
    }
    const footerUrl = document.getElementById('footer-ip-url');
    if (footerUrl) {
      footerUrl.innerText = `Web UI: ${data.url || 'http://tubeos.local'}`;
    }
    return data;
  } catch (err) {
    console.error('System info fetch failed:', err);
    return null;
  }
}

async function checkNetwork() {
  const infoText = document.getElementById('net-info-text');
  const ticIcon = document.getElementById('net-tic-icon');
  const btnCont = document.getElementById('btn-continue');

  try {
    const res = await fetch('/api/network');
    const data = await res.json();
    const st = data.status;

    if (st.online && st.repo_reachable) {
      ticIcon.className = 'status-tic-icon';
      ticIcon.innerHTML = `<svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3"><polyline points="20 6 9 17 4 12"></polyline></svg>`;
      infoText.innerHTML = `<strong>Connected to Internet</strong><br>Repositories are accessible. Ready for installation.<br><span style="color: var(--text-tertiary); font-size: 11.5px;">Host Address: ${st.ip} (tubeos.local)</span>`;
      if (steps[currentStepIndex] === 'network') btnCont.disabled = false;
    } else if (st.online) {
      ticIcon.className = 'status-tic-icon';
      ticIcon.innerHTML = `<svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3"><polyline points="20 6 9 17 4 12"></polyline></svg>`;
      infoText.innerHTML = `<strong>Network Connected</strong><br>Verifying repository mirrors...`;
      if (steps[currentStepIndex] === 'network') btnCont.disabled = false;
    } else {
      ticIcon.className = 'status-tic-icon error';
      ticIcon.innerHTML = `<svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg>`;
      infoText.innerHTML = `<strong>No Internet Connection</strong><br>An active internet connection is required to install components. Connect to Wi-Fi below or plug in an Ethernet cable.`;
      if (steps[currentStepIndex] === 'network') btnCont.disabled = true;
      renderWifiList(data.wifi || []);
    }
  } catch (err) {
    console.error('Network check error:', err);
  }
}

function renderWifiList(networks) {
  const section = document.getElementById('wifi-section');
  const list = document.getElementById('wifi-list');
  section.style.display = 'block';
  list.innerHTML = '';

  if (networks.length === 0) {
    list.innerHTML = '<div style="color: var(--text-secondary); font-size: 12px; padding: 6px 0;">No Wi-Fi networks found. Connect an Ethernet cable.</div>';
    return;
  }

  networks.forEach(net => {
    const div = document.createElement('div');
    div.className = 'option-row';
    div.innerHTML = `
      <div style="flex: 1;">
        <strong>${net.ssid}</strong>
        <span style="font-size: 11px; color: var(--text-tertiary);">${net.security}</span>
      </div>
      <span style="font-size: 12px; color: var(--text-secondary);">${net.signal}%</span>
    `;
    div.onclick = () => {
      document.getElementById('wifi-connect-box').style.display = 'flex';
      window.selectedSsid = net.ssid;
    };
    list.appendChild(div);
  });
}

async function submitWifiConnect() {
  const pwd = document.getElementById('wifi-password').value;
  const ssid = window.selectedSsid;
  if (!ssid) return;

  const res = await fetch('/api/network/connect', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ ssid, password: pwd }),
  });
  const data = await res.json();
  if (data.status === 'ok') {
    checkNetwork();
  }
}

// ─── Storage & Partitioning ─────────────────────────────────────────────────

async function loadDisks() {
  const grid = document.getElementById('disks-grid');
  try {
    const res = await fetch('/api/disks');
    const data = await res.json();
    grid.innerHTML = '';

    if (!data.disks || data.disks.length === 0) {
      grid.innerHTML = '<div style="color: #ef4444; font-size: 13px;">No installable storage drives found.</div>';
      return;
    }

    data.disks.forEach((d, idx) => {
      const card = document.createElement('div');
      card.className = 'selection-card';
      if (idx === 0) {
        card.classList.add('selected');
        selectDisk(d);
      }
      card.innerHTML = `
        <div class="card-top">
          ${getVectorIcon('disk')}
          <span class="card-badge">${d.type.toUpperCase()}</span>
        </div>
        <div class="card-title">${d.model}</div>
        <div class="card-detail">${d.size} &bull; /dev/${d.name}</div>
      `;
      card.onclick = () => {
        document.querySelectorAll('#disks-grid .selection-card').forEach(el => el.classList.remove('selected'));
        card.classList.add('selected');
        selectDisk(d);
      };
      grid.appendChild(card);
    });
  } catch (err) {
    console.error('Disk load failed:', err);
  }
}

function selectDisk(disk) {
  wizardData.disk = disk.path;
  wizardData.diskName = `${disk.model} (${disk.size})`;

  const select = document.getElementById('select-partition');
  select.innerHTML = '';
  if (disk.partitions && disk.partitions.length > 0) {
    disk.partitions.forEach(p => {
      const opt = document.createElement('option');
      opt.value = p.name;
      opt.innerText = `${p.name} (${p.size}, ${p.fstype}) ${p.label ? '[' + p.label + ']' : ''}`;
      select.appendChild(opt);
    });
    wizardData.target_partition = disk.partitions[0].name;
  } else {
    const opt = document.createElement('option');
    opt.value = '';
    opt.innerText = 'No partitions found on this drive';
    select.appendChild(opt);
    wizardData.target_partition = null;
  }
}

function setInstallMode(mode) {
  wizardData.mode = mode;
  document.querySelectorAll('.options-list .option-row').forEach(el => el.classList.remove('selected'));
  if (mode === 'clean') {
    document.getElementById('mode-clean-card').classList.add('selected');
    document.getElementById('partition-select-container').style.display = 'none';
  } else {
    document.getElementById('mode-dual-card').classList.add('selected');
    document.getElementById('partition-select-container').style.display = 'block';
  }
}

function setFsType(fs) {
  wizardData.fs_type = fs;
  document.getElementById('fs-btrfs').classList.toggle('active', fs === 'btrfs');
  document.getElementById('fs-ext4').classList.toggle('active', fs === 'ext4');
}

// ─── Editions ───────────────────────────────────────────────────────────────

async function loadEditions() {
  const grid = document.getElementById('editions-grid');
  try {
    const res = await fetch('/api/editions');
    const data = await res.json();
    grid.innerHTML = '';

    data.editions.forEach((ed, idx) => {
      const card = document.createElement('div');
      card.className = 'selection-card';
      if (idx === 0) {
        card.classList.add('selected');
        selectEdition(ed);
      }
      card.innerHTML = `
        <div class="card-top">
          ${getVectorIcon(ed.icon)}
          <span class="card-badge">${ed.badge}</span>
        </div>
        <div class="card-title">${ed.title}</div>
        <div class="card-detail">${ed.desc}</div>
      `;
      card.onclick = () => {
        document.querySelectorAll('#editions-grid .selection-card').forEach(el => el.classList.remove('selected'));
        card.classList.add('selected');
        selectEdition(ed);
      };
      grid.appendChild(card);
    });
  } catch (err) {
    console.error('Editions load error:', err);
  }
}

function selectEdition(ed) {
  wizardData.edition = ed.id;
  wizardData.editionTitle = ed.title;
}

// ─── Summary & Execution ────────────────────────────────────────────────────

function populateSummary() {
  wizardData.hostname = document.getElementById('cfg-hostname').value || 'tubeos';
  wizardData.username = document.getElementById('cfg-username').value || 'tubeos';
  wizardData.password = document.getElementById('cfg-password').value || 'tubeos';
  wizardData.timezone = document.getElementById('cfg-timezone').value;
  wizardData.keymap = document.getElementById('cfg-keymap').value;

  const summary = document.getElementById('install-summary');
  summary.innerHTML = `
    <div class="summary-item">
      <span class="summary-key">Target Drive</span>
      <span class="summary-val">${wizardData.diskName} (${wizardData.mode === 'clean' ? 'Erase & Clean GPT' : 'Partition ' + wizardData.target_partition})</span>
    </div>
    <div class="summary-item">
      <span class="summary-key">Filesystem</span>
      <span class="summary-val">${wizardData.fs_type.toUpperCase()}</span>
    </div>
    <div class="summary-item">
      <span class="summary-key">Edition</span>
      <span class="summary-val">${wizardData.editionTitle}</span>
    </div>
    <div class="summary-item">
      <span class="summary-key">Hostname</span>
      <span class="summary-val">${wizardData.hostname} (tubeos.local)</span>
    </div>
    <div class="summary-item">
      <span class="summary-key">Admin User</span>
      <span class="summary-val">${wizardData.username}</span>
    </div>
    <div class="summary-item">
      <span class="summary-key">Timezone & Layout</span>
      <span class="summary-val">${wizardData.timezone} (${wizardData.keymap})</span>
    </div>
  `;
}

async function startInstallation() {
  document.body.classList.add('installing');
  goToStep('progress');

  try {
    const res = await fetch('/api/install', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(wizardData),
    });
    
    if (res.ok) {
      progressInterval = setInterval(pollProgress, 1000);
    } else {
      document.body.classList.remove('installing');
      const err = await res.json();
      alert('Installation failed to start: ' + (err.error || 'Unknown error'));
    }
  } catch (err) {
    document.body.classList.remove('installing');
    console.error('Install start failed:', err);
  }
}

async function pollProgress() {
  try {
    const res = await fetch('/api/install/progress');
    const data = await res.json();

    const pct = Math.round((data.progress || 0) * 100);
    document.getElementById('progress-fill').style.width = `${pct}%`;
    document.getElementById('progress-pct').innerText = `${pct}%`;
    document.getElementById('progress-step-lbl').innerText = data.step || 'Processing...';

    const term = document.getElementById('terminal-body');
    if (data.logs && data.logs.length > 0) {
      term.innerText = data.logs.join('\n');
      term.scrollTop = term.scrollHeight;
    }

    if (data.status === 'done') {
      clearInterval(progressInterval);
      document.body.classList.remove('installing');
      setTimeout(() => goToStep('done'), 1200);
    } else if (data.status === 'error') {
      clearInterval(progressInterval);
      document.body.classList.remove('installing');
      document.getElementById('progress-step-lbl').innerHTML = `<span style="color: #ef4444;">Error: ${data.error}</span>`;
    }
  } catch (err) {
    console.error('Progress poll error:', err);
  }
}

async function rebootSystem() {
  const host = window.location.hostname || 'tubeos.local';
  const targetUrl = `http://${host}/`;

  // Show reboot waiting inside the installer glass panel without breaking ambient background
  const panel = document.querySelector('.installer-panel');
  if (panel) {
    panel.innerHTML = `
      <div style="display:flex;flex-direction:column;align-items:center;justify-content:center;height:100%;width:100%;padding:48px 32px;text-align:center;">
        <div style="width:48px;height:48px;border:3px solid rgba(139,92,246,0.2);border-top-color:#8b5cf6;border-radius:50%;animation:spin 1s cubic-bezier(0.4,0,0.2,1) infinite;margin-bottom:20px;"></div>
        <h2 style="font-size:22px;font-weight:600;margin-bottom:8px;color:#fff;letter-spacing:-0.3px;">Restarting Tube OS...</h2>
        <p style="color:var(--text-secondary);font-size:13.5px;max-width:440px;line-height:1.5;margin-bottom:22px;" id="reboot-status">
          Your system is restarting. Waiting for the system and CasaOS services to come online...
        </p>
        <div id="manual-redirect-box" style="display:none;">
          <a href="${targetUrl}" class="mac-btn mac-btn-primary" style="text-decoration:none;padding:9px 24px;">Open Dashboard</a>
        </div>
      </div>
      <style>
        @keyframes spin { 0% { transform: rotate(0deg); } 100% { transform: rotate(360deg); } }
      </style>
    `;
  }

  try {
    await fetch('/api/reboot', { method: 'POST' });
  } catch (err) {
    console.log('Reboot triggered:', err);
  }

  // After 25 seconds, display manual redirect button as fallback
  setTimeout(() => {
    const btnBox = document.getElementById('manual-redirect-box');
    if (btnBox) btnBox.style.display = 'block';
  }, 25000);

  // Poll for CasaOS / system response
  let pollCount = 0;
  const pollTimer = setInterval(async () => {
    pollCount++;
    try {
      const res = await fetch(`http://${host}/v1/sys/version`, {
        method: 'GET',
        cache: 'no-store',
        headers: { 'Accept': 'application/json' },
      });

      if (res.status > 0) {
        clearInterval(pollTimer);
        const st = document.getElementById('reboot-status');
        if (st) st.innerText = 'Tube OS is online! Loading CasaOS...';
        setTimeout(() => {
          window.location.href = targetUrl;
        }, 1200);
      }
    } catch (_) {
      try {
        const rootRes = await fetch(`http://${host}/`, {
          method: 'HEAD',
          cache: 'no-store',
        });
        if (rootRes.status > 0) {
          clearInterval(pollTimer);
          const st = document.getElementById('reboot-status');
          if (st) st.innerText = 'Tube OS is online! Loading CasaOS...';
          setTimeout(() => {
            window.location.href = targetUrl;
          }, 1200);
        }
      } catch (__) {
        // Still rebooting
      }
    }
  }, 2000);
}
