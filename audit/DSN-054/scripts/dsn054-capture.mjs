// TASK-DSN-054 capture script — Empty-State Deserts (Suppliers & Customers).
// Mechanism reused from audit/AUD-007/scripts/cdp.mjs (CDP over Node 26
// global fetch + WebSocket, zero deps): remote-debugging Chrome + vite dev.
// The app runs unmodified; a Tauri `invoke` stub is injected via
// Page.addScriptToEvaluateOnNewDocument (browser has no Tauri backend, and
// `call()` in src/lib/ipc.ts throws BRIDGE_UNAVAILABLE there). The stub
// answers the exact wire shapes the frontend already expects — no source
// change, no new dependency, one execution captures the whole matrix.
//
// Usage: node dsn054-capture.mjs
// Output: docs/design/evidence/TASK-DSN-054/*.png + dsn054-assertions.json
import fs from 'node:fs';
import path from 'node:path';
import { spawn } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const dirname = path.dirname(fileURLToPath(import.meta.url));
const ROOT = path.resolve(dirname, '..', '..', '..');
const OUT = path.join(ROOT, 'docs', 'design', 'evidence', 'TASK-DSN-054');
// Created lazily at publish time (after the browser run), so the Vite
// watcher never sees mid-run writes under docs/.

const VITE_PORT = 5173;
const CDP_PORT = 9222;
const CHROME = 'C:/Program Files/Google/Chrome/Application/chrome.exe';
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

const children = [];
// NOTE: everything the run writes continuously (dev-server log, in-flight
// screenshots, browser profile) MUST live under node_modules, which Vite's
// watcher ignores by default. Writing them under audit/ or docs/ triggers
// a full page reload on every write and races all CDP evaluates.
const RUN_LOG = path.join(ROOT, 'node_modules', '.dsn054-vite-run.log');
const TMP_OUT = path.join(ROOT, 'node_modules', '.dsn054-shots');
const CHROME_PROFILE = path.join(ROOT, 'node_modules', '.dsn054-chrome-profile');
function launch(cmd, args, opts = {}) {
  const child = spawn(cmd, args, { cwd: ROOT, stdio: 'ignore', shell: false, ...opts });
  children.push(child);
  return child;
}
function launchLogged(cmd, args, logFile) {
  const fs2 = fs.openSync(logFile, 'w');
  const child = spawn(cmd, args, { cwd: ROOT, stdio: ['ignore', fs2, fs2], shell: false });
  children.push(child);
  return child;
}
function cleanupRun() {
  for (const c of children) { try { c.kill(); } catch {} }
  try { fs.rmSync(RUN_LOG, { force: true }); } catch {}
  try { fs.rmSync(TMP_OUT, { recursive: true, force: true }); } catch {}
  try { fs.rmSync(CHROME_PROFILE, { recursive: true, force: true }); } catch {}
}
async function waitPort(port, timeoutMs, label) {
  const deadline = Date.now() + timeoutMs;
  for (;;) {
    try {
      const r = await fetch(`http://127.0.0.1:${port}/`);
      if (r.ok || r.status < 500) return;
    } catch {}
    if (Date.now() > deadline) throw new Error(`port wait timeout: ${label}:${port}`);
    await sleep(250);
  }
}

// --- CDP client (same shape as audit/AUD-007/scripts/cdp.mjs) ---
async function pageWsUrl() {
  const r = await fetch(`http://127.0.0.1:${CDP_PORT}/json/list`);
  const list = await r.json();
  const page = list.find((t) => t.type === 'page');
  if (!page) throw new Error('no page target');
  return page.webSocketDebuggerUrl;
}
function connect(url) {
  return new Promise((resolve, reject) => {
    const ws = new WebSocket(url);
    ws.addEventListener('open', () => resolve(ws), { once: true });
    ws.addEventListener('error', () => reject(new Error('ws error')), { once: true });
  });
}
async function session() {
  const ws = await connect(await pageWsUrl());
  let id = 0;
  const pending = new Map();
  ws.addEventListener('message', (ev) => {
    let m;
    try { m = JSON.parse(ev.data); } catch { return; }
    if (m.id !== undefined && pending.has(m.id)) { pending.get(m.id)(m); pending.delete(m.id); }
  });
  const send = (method, params = {}) => new Promise((resolve, reject) => {
    const mid = ++id;
    pending.set(mid, (m) => (m.error ? reject(new Error(method + ': ' + JSON.stringify(m.error))) : resolve(m.result)));
    ws.send(JSON.stringify({ id: mid, method, params }));
    setTimeout(() => { if (pending.has(mid)) { pending.delete(mid); reject(new Error('timeout ' + method)); } }, 30000);
  });
  const evaluate = async (expr) => {
    const r = await send('Runtime.evaluate', { expression: expr, returnByValue: true, awaitPromise: true });
    if (r.exceptionDetails) throw new Error('evaluate threw: ' + JSON.stringify(r.exceptionDetails).slice(0, 400));
    return r.result ? r.result.value : undefined;
  };
  return { send, evaluate };
}
async function poll(evalFn, check, timeoutMs, label, diagnose) {
  const deadline = Date.now() + timeoutMs;
  let last = null, lastErr = '';
  for (;;) {
    last = await evalFn().catch((e) => { lastErr = String((e && e.message) || e).slice(0, 200); return null; });
    if (check(last)) return last;
    if (Date.now() > deadline) {
      let extra = '';
      if (diagnose) {
        try { extra = ' diag=' + JSON.stringify(await diagnose()).slice(0, 300); } catch {}
      }
      throw new Error(`poll timeout: ${label} (last=${JSON.stringify(last).slice(0, 200)} lastErr=${lastErr}${extra})`);
    }
    await sleep(200);
  }
}

// --- Tauri invoke stub (preloaded before app code runs) ---
const PRELOAD = `
window.__SCEN__ = { suppliersMode: 'empty' };
window.__TAURI_INTERNALS__ = {
  invoke: function (cmd, args) {
    var scen = window.__SCEN__ || {};
    var req = (args && args.request) || {};
    var respond = function (v) { return Promise.resolve(v); };
    var fail = function (code) { return Promise.reject(code); };
    if (cmd === 'check_first_run_status') return respond({ requires_setup: false });
    if (cmd === 'auth_login') return respond({ session_token: 'mock-token-1', user_id: 'u-gerant', username: 'gerant', role: 'GERANT', expires_at: '2030-01-01T00:00:00' });
    if (cmd === 'warehouses_list') return respond({ warehouses: [{ id: 'w1', code: 'DEP-01', name: 'Dépôt Central', is_active: true }] });
    if (cmd === 'cash_get_active_session') return respond(null);
    if (cmd === 'suppliers_search') {
      if (scen.suppliersMode === 'error') return fail('DATABASE_ERROR');
      return respond({ suppliers: [] });
    }
    if (cmd === 'customers_list') return respond({ customers: [{ id: 'c1' }, { id: 'c2' }] });
    if (cmd === 'customers_search') return respond({ customers: [] });
    if (cmd === 'customers_get') {
      var id = req.customer_id || 'c1';
      var first = id === 'c2'
        ? { name: 'Sara Mansouri', phone: '0550987654', balance: '0' }
        : { name: 'Amine Benali', phone: '0550123456', balance: '125050' };
      return respond({ id: id, name: first.name, phone: first.phone, address: 'Alger', credit_policy: 'LIMITED', credit_limit_centimes: '5000000', customer_tier: 'retail', current_balance_centimes: first.balance, is_active: true });
    }
    if (cmd === 'credit_get_statement') return respond({ entries: [
      { entry_type: 'SALE', amount_centimes: '250000', reference_type: 'TICKET', reference_id: 'T-0001', created_at: '2026-10-01T10:00:00', balance_after_centimes: '250000' },
      { entry_type: 'PAYMENT', amount_centimes: '124950', reference_type: 'RECEIPT', reference_id: 'R-0007', created_at: '2026-10-05T15:30:00', balance_after_centimes: '125050' }
    ] });
    if (cmd === 'auth_logout') return respond(null);
    return fail('MOCK_UNMAPPED:' + cmd);
  },
  transformCallback: function () { return 0; }
};
`;

const FILL = `
function (selector, value) {
  var el = document.querySelector(selector);
  if (!el) return 'missing:' + selector;
  var proto = el instanceof HTMLTextAreaElement ? HTMLTextAreaElement.prototype : HTMLInputElement.prototype;
  Object.getOwnPropertyDescriptor(proto, 'value').set.call(el, value);
  el.dispatchEvent(new Event('input', { bubbles: true }));
  el.dispatchEvent(new Event('change', { bubbles: true }));
  return 'ok';
}
`;

const ASSERT_FN = `
function (needles) {
  var el = null;
  var walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
  var node;
  while ((node = walker.nextNode())) {
    var text = node.nodeValue || '';
    for (var i = 0; i < needles.length; i++) {
      if (text.indexOf(needles[i]) >= 0) { el = node.parentElement; break; }
    }
    if (el) break;
  }
  if (!el) return { found: false };
  var scope = el.closest('section') || el.closest('div') || el;
  var de = document.documentElement;
  var out = {
    found: true,
    tag: scope.tagName,
    dark: de.classList.contains('dark'),
    dir: (document.querySelector('#root > div') || document.documentElement).getAttribute('dir'),
    scrollW: de.scrollWidth, clientW: de.clientWidth,
    scrollH: de.scrollHeight, clientH: de.clientHeight,
    fixed: 0, emoji: false, digits: []
  };
  var all = scope.querySelectorAll('*');
  for (var k = 0; k < all.length; k++) {
    if (getComputedStyle(all[k]).position === 'fixed') out.fixed++;
  }
  var re = /[\\u{1F300}-\\u{1FAFF}\\u{2600}-\\u{27BF}]/u;
  if (re.test(scope.innerText || '')) out.emoji = true;
  var tw = document.createTreeWalker(scope, NodeFilter.SHOW_TEXT);
  var tn;
  while ((tn = tw.nextNode())) {
    var t = tn.nodeValue || '';
    if (/[0-9]/.test(t)) {
      var p = tn.parentElement, bdi = null;
      while (p && p !== scope.parentElement) {
        if (p.tagName === 'BDI' && p.getAttribute('dir') === 'ltr') { bdi = p; break; }
        p = p.parentElement;
      }
      out.digits.push({ text: t.slice(0, 40), bdiLtr: !!bdi, tabular: bdi ? getComputedStyle(bdi).fontVariantNumeric : '' });
    }
  }
  var h = scope.querySelector('h1,h2');
  var body = scope.querySelector('p');
  var pick = function (n) {
    if (!n) return null;
    var bg = null, q = n;
    while (q && q !== document.body) {
      var c = getComputedStyle(q).backgroundColor;
      if (c && c !== 'rgba(0, 0, 0, 0)' && c !== 'transparent') { bg = c; break; }
      q = q.parentElement;
    }
    if (!bg) bg = getComputedStyle(document.body).backgroundColor;
    return { fg: getComputedStyle(n).color, bg: bg, dir: getComputedStyle(n).direction };
  };
  out.title = pick(h);
  out.body = pick(body);
  var btn = scope.querySelector('button');
  if (btn) {
    var r = btn.getBoundingClientRect(), sr = scope.getBoundingClientRect();
    out.cta = { dir: getComputedStyle(btn).direction, btnCx: r.left + r.width / 2, scopeCx: sr.left + sr.width / 2, w: r.width };
  }
  var svg = scope.querySelector('svg');
  if (svg) {
    var s = svg.getBoundingClientRect(), ss = scope.getBoundingClientRect();
    out.icon = { cx: s.left + s.width / 2, scopeCx: ss.left + ss.width / 2 };
  }
  return out;
}
`;

function lum(rgb) {
  const m = rgb.match(/rgba?\(([^)]+)\)/);
  if (!m) return 0;
  const c = m[1].split(',').slice(0, 3).map((x) => parseFloat(x) / 255).map((v) => (v <= 0.03928 ? v / 12.92 : Math.pow((v + 0.055) / 1.055, 2.4)));
  return 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];
}
function ratio(fg, bg) {
  const a = lum(fg), b = lum(bg);
  return ((Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05));
}

const TIERS = { S: [1024, 650], M: [1280, 720], L: [1920, 1080] };
const results = [];

async function main() {
  launchLogged(process.execPath, ['node_modules/vite/bin/vite.js', '--port', String(VITE_PORT), '--strictPort', '--host', '127.0.0.1'], RUN_LOG);
  await waitPort(VITE_PORT, 180000, 'vite');
  const udir = CHROME_PROFILE;
  fs.mkdirSync(udir, { recursive: true });
  launch(CHROME, [`--headless=new`, `--remote-debugging-port=${CDP_PORT}`, '--disable-gpu', '--no-first-run', '--no-default-browser-check', `--user-data-dir=${udir}`, 'about:blank']);
  await waitPort(CDP_PORT, 60000, 'cdp');
  await sleep(1500);
  const { send, evaluate } = await session();
  await send('Page.addScriptToEvaluateOnNewDocument', { source: PRELOAD });
  await send('Page.navigate', { url: `http://127.0.0.1:${VITE_PORT}/` });
  await poll(() => evaluate(`document.querySelector('input[autocomplete="username"]') ? 'yes' : 'no'`), (v) => v === 'yes', 60000, 'login form');

  // The preload stub is belt-and-braces; the authoritative install happens
  // here, after load. Safe because isTauri()/invoke resolve lazily per call,
  // and the only pre-login IPC (first-run probe) falls back to the login
  // form on failure by design.
  await evaluate(PRELOAD);
  const stubOk = await evaluate(`'__TAURI_INTERNALS__' in window && typeof window.__TAURI_INTERNALS__.invoke === 'function' ? 'yes' : 'no'`);
  if (stubOk !== 'yes') throw new Error('tauri stub injection failed (in-window check: ' + stubOk + ')');
  const stubCall = await evaluate(`window.__TAURI_INTERNALS__.invoke('check_first_run_status', {}).then(function(v){return 'ok:'+JSON.stringify(v);}).catch(function(e){return 'fail:'+e;})`);
  if (stubCall !== 'ok:{"requires_setup":false}') throw new Error('tauri stub self-test failed: ' + stubCall);

  // Login (mocked backend, real form + real store flow).
  await evaluate(`(${FILL})('input[autocomplete="username"]', 'gerant')`);
  await evaluate(`(${FILL})('input[autocomplete="current-password"]', 'secret')`);
  await evaluate(`document.querySelector('form').requestSubmit()`);
  await poll(() => evaluate(`(function(){var n=document.querySelector('nav');return n ? n.innerText.slice(0,80) : 'no';})()`), (v) => v && v !== 'no', 30000, 'post-login nav');
  await sleep(3000);

  const state = { locale: 'fr', theme: 'light' };
  const bodyHead = () => evaluate(`document.body ? document.body.innerText.slice(0, 300) : 'no-body'`);
  const clickByText = async (selector, texts) => {
    const want = JSON.stringify(texts);
    const hit = await evaluate(`(function(){var cands=JSON.parse(${JSON.stringify(want)});var els=document.querySelectorAll(${JSON.stringify(selector)});for(var i=0;i<els.length;i++){var t=(els[i].textContent||'').trim();if(t && cands.indexOf(t)>=0){els[i].click();return t;}}return 'miss';})()`);
    if (hit === 'miss' || hit === null || hit === undefined) {
      const ctx = await evaluate(`(function(){var bs=document.querySelectorAll('button');var t=[];for(var i=0;i<bs.length && t.length<40;i++){var x=(bs[i].textContent||'').trim().slice(0,24);if(x)t.push(x);}return 'BTNS['+t.join('|')+'] HEAD:'+(document.body?document.body.innerText.slice(0,200):'no-body');})()`).catch(() => 'eval-failed');
      throw new Error('clickByText miss: ' + JSON.stringify(texts) + ' ctx=' + JSON.stringify(ctx));
    }
    return hit;
  };
  const navTo = async (fr, ar) => {
    await clickByText('nav button', [fr, ar]);
    await sleep(1500);
  };
  // Topbar (with the locale toggle) exists ONLY in the POS view, and its
  // dark-mode toggle is local state that resets on unmount. So: locale is
  // switched through the real Topbar button inside the POS view (global
  // store, persists across views); theme is driven by toggling the exact
  // same `dark` class on documentElement that the Topbar hook toggles.
  const gotoPos = async () => {
    await navTo('Caisse', 'الصندوق');
  };
  const ensureLocaleUI = async (lang) => {
    const wantDir = lang === 'ar' ? 'rtl' : 'ltr';
    for (let i = 0; i < 3; i++) {
      // NOTE: App sets dir on its own root div (#root > div), NOT on
      // documentElement — read it there.
      const dir = await evaluate(`(document.querySelector('#root > div') || document.documentElement).getAttribute('dir')`);
      if (dir === wantDir) {
        state.locale = lang;
        return;
      }
      await gotoPos();
      const label = lang === 'ar' ? 'العربية' : 'Français';
      try {
        await clickByText('button', [label]);
      } catch (e) {
        // Tolerate an already-applied toggle (stale dir read / double
        // invocation): re-read dir, continue only if it now matches.
        const dir2 = await evaluate(`(document.querySelector('#root > div') || document.documentElement).getAttribute('dir')`).catch(() => null);
        if (dir2 === wantDir) {
          state.locale = lang;
          return;
        }
        throw e;
      }
      await sleep(800);
    }
    throw new Error('locale switch failed: ' + lang);
  };
  const setThemeClass = async (theme) => {
    await evaluate(`document.documentElement.classList.toggle('dark', ${theme === 'dark' ? 'true' : 'false'})`);
    await sleep(400);
    state.theme = theme;
  };
  const setViewport = async (w, h, dpr) => {
    await send('Emulation.setDeviceMetricsOverride', { width: w, height: h, deviceScaleFactor: dpr || 1, mobile: false });
    await sleep(600);
  };
  const shot = async (name, needles) => {
    const facts = await evaluate(`(${ASSERT_FN})(${JSON.stringify(needles)})`);
    const img = await send('Page.captureScreenshot', { format: 'png' });
    fs.mkdirSync(TMP_OUT, { recursive: true });
    fs.writeFileSync(path.join(TMP_OUT, name), Buffer.from(img.data, 'base64'));
    const row = { file: name, ...facts };
    if (facts.title) row.titleRatio = +ratio(facts.title.fg, facts.title.bg).toFixed(2);
    if (facts.body) row.bodyRatio = +ratio(facts.body.fg, facts.body.bg).toFixed(2);
    results.push(row);
    console.log(name, JSON.stringify({ found: facts.found, dark: facts.dark, dir: facts.dir, titleRatio: row.titleRatio, bodyRatio: row.bodyRatio, fixed: facts.fixed, emoji: facts.emoji, digits: facts.digits }));
  };

  const setScen = async (mode) => {
    await evaluate(`window.__SCEN__ = { suppliersMode: ${JSON.stringify(mode)} }; 'ok'`);
  };

  // ---- 1. Customers empty-selection teaching panel: full matrix (12) ----
  const custNeedles = ['Sélectionnez un client', 'Choisissez un client', 'اختر زبونًا'];
  const waitTeaching = () => poll(() => evaluate(`document.body.innerText.indexOf('Sélectionnez un client')>=0 || document.body.innerText.indexOf('اختر زبونًا')>=0 ? 'yes':'no'`), (v) => v === 'yes', 30000, 'customers teaching', bodyHead);
  for (const lang of ['fr', 'ar']) {
    await ensureLocaleUI(lang);
    await navTo('Clients & Crédit', 'الزبائن والديون');
    await waitTeaching();
    for (const tier of ['S', 'M', 'L']) {
      const [w, h] = TIERS[tier];
      await setViewport(w, h, 1);
      for (const theme of ['light', 'dark']) {
        await setThemeClass(theme);
        await shot(`dsn054-customers-empty-${tier}-${lang}-${theme}.png`, custNeedles);
      }
    }
  }
  // UHD spot-check: 1920x1080 CSS @ DPR 2, fr, light.
  await ensureLocaleUI('fr');
  await navTo('Clients & Crédit', 'الزبائن والديون');
  await waitTeaching();
  await setViewport(1920, 1080, 2);
  await setThemeClass('light');
  await shot('dsn054-customers-empty-L-fr-light-dpr2.png', custNeedles);

  // ---- 2. Customers selected regression (M × fr/ar × light) ----
  await setViewport(1280, 720, 1);
  for (const lang of ['fr', 'ar']) {
    await ensureLocaleUI(lang);
    await navTo('Clients & Crédit', 'الزبائن والديون');
    await waitTeaching();
    await setThemeClass('light');
    await evaluate(`(function(){var bs=document.querySelectorAll('ul li button');if(bs.length){bs[0].click();return bs.length;}return 0;})()`);
    await sleep(1200);
    await shot(`dsn054-customers-selected-M-${lang}-light.png`, ['Amine Benali', 'Relevé de compte', 'كشف الحساب']);
  }

  // ---- 3. Suppliers no-match (M × fr/ar × light/dark) ----
  await setScen('empty');
  const supNeedles = ['Aucun fournisseur ne correspond', 'لا يوجد مورّد يطابق'];
  const waitSuppliers = () => poll(() => evaluate(`document.querySelector('input[placeholder*="fournisseur"],input[placeholder*="مورد"]') ? 'yes':'no'`), (v) => v === 'yes', 20000, 'suppliers view');
  for (const lang of ['fr', 'ar']) {
    await ensureLocaleUI(lang);
    await navTo('Fournisseurs', 'الموردون');
    await waitSuppliers();
    const ph = state.locale === 'fr' ? 'input[placeholder*="fournisseur"]' : 'input[placeholder*="مورد"]';
    await evaluate(`(${FILL})(${JSON.stringify(ph)}, 'Ciment CPJ 45')`);
    await sleep(1300);
    for (const theme of ['light', 'dark']) {
      await setViewport(1280, 720, 1);
      await setThemeClass(theme);
      await shot(`dsn054-suppliers-nomatch-M-${lang}-${theme}.png`, supNeedles);
    }
    await evaluate(`(${FILL})(${JSON.stringify(ph)}, '')`);
    await sleep(900);
  }

  // ---- 4. Suppliers fetch-error (M × fr/ar × light) ----
  await setScen('error');
  for (const lang of ['fr', 'ar']) {
    await ensureLocaleUI(lang);
    await navTo('Fournisseurs', 'الموردون');
    await waitSuppliers();
    await setViewport(1280, 720, 1);
    await setThemeClass('light');
    const ph = state.locale === 'fr' ? 'input[placeholder*="fournisseur"]' : 'input[placeholder*="مورد"]';
    await evaluate(`(${FILL})(${JSON.stringify(ph)}, 'Ciment CPJ 45')`);
    await sleep(1300);
    await shot(`dsn054-suppliers-error-M-${lang}-light.png`, ['Impossible de charger', 'تعذّر تحميل']);
    await evaluate(`(${FILL})(${JSON.stringify(ph)}, '')`);
    await sleep(900);
  }

  // Publish: move finished PNGs into the evidence dir AFTER the browser is
  // done (writing into docs/ mid-run would trigger Vite HMR full-reloads).
  fs.mkdirSync(OUT, { recursive: true });
  for (const file of fs.readdirSync(TMP_OUT)) {
    fs.renameSync(path.join(TMP_OUT, file), path.join(OUT, file));
  }
  fs.writeFileSync(path.join(OUT, 'dsn054-assertions.json'), JSON.stringify(results, null, 2));
  console.log('CAPTURES_DONE=' + results.length);
}

main().then(() => { cleanupRun(); process.exit(0); })
  .catch((e) => { console.error('CAPTURE_FAILED', e.message); cleanupRun(); process.exit(1); });
