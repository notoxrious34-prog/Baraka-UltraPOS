// TASK-DSN-054-C2 — suppliers default-list + teaching capture (thin copy of
// DSN-056 verify-ink-muted.mjs; mechanism: CDP over fetch+WebSocket, vite dev,
// Tauri invoke stub with synthetic data only. MODE=before|teaching|list|nomatch,
// THEME=light|dark via env. Usage: MODE=before node capture.mjs
import fs from 'node:fs';
import path from 'node:path';
import { spawn } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const dirname = path.dirname(fileURLToPath(import.meta.url));
const ROOT = path.resolve(dirname, '..', '..', '..');
const OUT = path.join(ROOT, 'audit', 'DSN-054-C2', 'screenshots');

const VITE_PORT = 5173;
const CDP_PORT = 9222;
const CHROME = 'C:/Program Files/Google/Chrome/Application/chrome.exe';
const MODE = (process.argv[2] || process.env.MODE || 'before').trim();
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

const children = [];
const RUN_LOG = path.join(ROOT, 'node_modules', '.dsn054c2-vite-run.log');
const TMP_OUT = path.join(ROOT, 'node_modules', '.dsn054c2-shots');
const CHROME_PROFILE = path.join(ROOT, 'node_modules', '.dsn054c2-chrome-profile');
function launch(cmd, args) {
  const child = spawn(cmd, args, { cwd: ROOT, stdio: 'ignore', shell: false });
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
async function poll(evalFn, check, timeoutMs, label) {
  const deadline = Date.now() + timeoutMs;
  let last = null, lastErr = '';
  for (;;) {
    last = await evalFn().catch((e) => { lastErr = String((e && e.message) || e).slice(0, 200); return null; });
    if (check(last)) return last;
    if (Date.now() > deadline) throw new Error(`poll timeout: ${label} (last=${JSON.stringify(last).slice(0, 200)} lastErr=${lastErr})`);
    await sleep(200);
  }
}

// 5 synthetic suppliers (Algerian hardware-store): + / debt / zero balances.
const FIVE = JSON.stringify([
  ['f1', 'Quincaillerie El Amel', '0550123456', '1525000'],
  ['f2', 'Ets Benali & Frères', '0550987654', '-425075'],
  ['f3', 'Sarl Atlas Négoce', '0661234567', '0'],
  ['f4', 'Grossiste Sidi Moussa', '0770112233', '980000'],
  ['f5', 'Comptoir du Bâtiment', '0555443322', '-12000'],
]);

const PRELOAD = `
window.__TAURI_INTERNALS__ = {
  invoke: function (cmd, args) {
    var req = (args && args.request) || {};
    var respond = function (v) { return Promise.resolve(v); };
    var fail = function (code) { return Promise.reject(code); };
    if (cmd === 'check_first_run_status') return respond({ requires_setup: false });
    if (cmd === 'auth_login') return respond({ session_token: 'mock-token-1', user_id: 'u-gerant', username: 'gerant', role: 'GERANT', expires_at: '2030-01-01T00:00:00' });
    if (cmd === 'warehouses_list') return respond({ warehouses: [{ id: 'w1', code: 'DEP-01', name: 'Dépôt Central', is_active: true }] });
    if (cmd === 'cash_get_active_session') return respond(null);
    if (cmd === 'suppliers_list') {
      var rows = JSON.parse(${JSON.stringify(FIVE)});
      if (${MODE === 'list' || MODE === 'markup' ? 'true' : 'false'}) {
        return respond({ suppliers: rows.map(function (r) {
          return { id: r[0], name: r[1], phone: r[2], address: 'Alger', nif: null, rc: null, notes: null, is_active: true, current_balance_centimes: r[3], origin_device_id: 'dev-1', created_at: '2026-01-01T00:00:00', updated_at: '2026-01-01T00:00:00' };
        }) });
      }
      return respond({ suppliers: [] });
    }
    if (cmd === 'suppliers_search') {
      if (!req.query || req.query.indexOf('Ciment') >= 0) return respond({ suppliers: [] });
      var s = function (id, name, bal) {
        return { id: id, name: name, phone: '0550000000', address: 'Alger', nif: null, rc: null, notes: null, is_active: true, current_balance_centimes: bal, origin_device_id: 'dev-1', created_at: '2026-01-01T00:00:00', updated_at: '2026-01-01T00:00:00' };
      };
      return respond({ suppliers: [s('s1', 'Fournisseur Test Alpha', '250000'), s('s2', 'Fournisseur Test Beta', '0')] });
    }
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

const MEAS_FN = `
function (specs) {
  var cs = getComputedStyle(document.body);
  var out = { dark: document.documentElement.classList.contains('dark'), items: [] };
  var lum = function (rgb) {
    var m = String(rgb).match(/rgba?\\(([^)]+)\\)/);
    if (!m) return 0;
    var c = m[1].split(',').slice(0, 3).map(function (x) { return parseFloat(x) / 255; }).map(function (v) { return v <= 0.03928 ? v / 12.92 : Math.pow((v + 0.055) / 1.055, 2.4); });
    return 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];
  };
  var findByNeedle = function (needle) {
    var walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
    var node;
    while ((node = walker.nextNode())) {
      if ((node.nodeValue || '').indexOf(needle) >= 0) return node.parentElement;
    }
    return null;
  };
  for (var i = 0; i < specs.length; i++) {
    var n = specs[i].needle ? findByNeedle(specs[i].needle) : document.querySelector(specs[i].css);
    if (!n) { out.items.push({ key: specs[i].key, found: false }); continue; }
    var st = getComputedStyle(n);
    var bg = null, q = n;
    while (q && q !== document.body) {
      var c = getComputedStyle(q).backgroundColor;
      if (c && c !== 'rgba(0, 0, 0, 0)' && c !== 'transparent') { bg = c; break; }
      q = q.parentElement;
    }
    if (!bg) bg = getComputedStyle(document.body).backgroundColor;
    var a = lum(st.color), b = lum(bg);
    var ratio = (Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05);
    var fs = parseFloat(st.fontSize) || 0;
    var large = fs >= 24 || (fs >= 18.66 && parseInt(st.fontWeight, 10) >= 700);
    var th = large ? 3 : 4.5;
    out.items.push({ key: specs[i].key, found: true, tag: n.tagName, text: (n.innerText || n.textContent || '').trim().slice(0, 60), fg: st.color, bg: bg, fs: fs, fw: st.fontWeight, ratio: Math.round(ratio * 100) / 100, threshold: th, verdict: ratio >= th ? 'PASS' : 'FAIL' });
  }
  return out;
}
`;

const SHOTS = {
  before: [['before', 'dsn054c2-BEFORE-empty-query-broken-state.png', [{ key: 'before-text', needle: 'Aucun fournisseur' }], '']],
  teaching: [
    ['teaching-light', 'dsn054c2-suppliers-zero-db-teaching-light.png', [{ key: 'teach-title' }, { key: 'teach-body' }, { key: 'teach-cta' }], 'light'],
    ['teaching-dark', 'dsn054c2-suppliers-zero-db-teaching-dark.png', [{ key: 'teach-title' }, { key: 'teach-body' }, { key: 'teach-cta' }], 'dark'],
  ],
  list: [
    ['list-light', 'dsn054c2-suppliers-default-list-light.png', [{ key: 'row-name' }, { key: 'row-bal-pos' }, { key: 'row-bal-neg' }, { key: 'row-bal-zero' }], 'light'],
    ['list-dark', 'dsn054c2-suppliers-default-list-dark.png', [{ key: 'row-name' }, { key: 'row-bal-pos' }, { key: 'row-bal-neg' }, { key: 'row-bal-zero' }], 'dark'],
  ],
  nomatch: [
    ['nomatch-light', 'dsn054c2-suppliers-nomatch-recheck-light.png', [{ key: 'nomatch-lead' }], 'light'],
    ['nomatch-dark', 'dsn054c2-suppliers-nomatch-recheck-dark.png', [{ key: 'nomatch-lead' }], 'dark'],
  ],
  markup: [['markup', '', [], 'light']],
};

const NEEDLES = {
  'teach-title': 'premier fournisseur',
  'teach-body': 'Aucun fournisseur',
  'teach-cta': 'Nouveau fournisseur',
  'row-name': 'Quincaillerie El Amel',
  'row-bal-pos': '15 250',
  'row-bal-neg': '-4 250',
  'row-bal-zero': '0,00',
  'nomatch-lead': 'ne correspond',
  'before-text': 'Aucun fournisseur',
};

const results = [];
async function main() {
  fs.mkdirSync(TMP_OUT, { recursive: true });
  fs.mkdirSync(OUT, { recursive: true });
  launchLogged(process.execPath, ['node_modules/vite/bin/vite.js', '--port', String(VITE_PORT), '--strictPort', '--host', '127.0.0.1'], RUN_LOG);
  await waitPort(VITE_PORT, 180000, 'vite');
  fs.mkdirSync(CHROME_PROFILE, { recursive: true });
  launch(CHROME, ['--headless=new', `--remote-debugging-port=${CDP_PORT}`, '--disable-gpu', '--no-first-run', '--no-default-browser-check', `--user-data-dir=${CHROME_PROFILE}`, 'about:blank']);
  await waitPort(CDP_PORT, 60000, 'cdp');
  await sleep(1500);
  const { send, evaluate } = await session();
  await send('Page.addScriptToEvaluateOnNewDocument', { source: PRELOAD });
  await send('Page.navigate', { url: `http://127.0.0.1:${VITE_PORT}/` });
  await poll(() => evaluate(`document.querySelector('input[autocomplete="username"]') ? 'yes' : 'no'`), (v) => v === 'yes', 60000, 'login form');
  await evaluate(PRELOAD);
  await evaluate(`(${FILL})('input[autocomplete="username"]', 'gerant')`);
  await evaluate(`(${FILL})('input[autocomplete="current-password"]', 'secret')`);
  await evaluate(`document.querySelector('form').requestSubmit()`);
  await poll(() => evaluate(`(function(){var n=document.querySelector('nav');return n ? 'yes' : 'no';})()`), (v) => v === 'yes', 30000, 'post-login nav');
  await sleep(3000);

  const clickByText = async (selector, texts) => {
    const want = JSON.stringify(texts);
    const hit = await evaluate(`(function(){var cands=JSON.parse(${JSON.stringify(want)});var els=document.querySelectorAll(${JSON.stringify(selector)});for(var i=0;i<els.length;i++){var t=(els[i].textContent||'').trim();if(t && cands.indexOf(t)>=0){els[i].click();return t;}}return 'miss';})()`);
    if (hit === 'miss' || hit == null) throw new Error('clickByText miss: ' + JSON.stringify(texts));
    return hit;
  };
  const navTo = async (fr) => { await clickByText('nav button', [fr]); await sleep(1500); };
  await send('Emulation.setDeviceMetricsOverride', { width: 1280, height: 720, deviceScaleFactor: 1, mobile: false });
  await sleep(600);

  const meas = async (specs) => evaluate(`(${MEAS_FN})(${JSON.stringify(specs)})`);
  const dims = async () => evaluate(`document.documentElement.scrollWidth+'/'+document.documentElement.clientWidth+'/'+document.documentElement.scrollHeight+'/'+document.documentElement.clientHeight`);
  const has = (s) => evaluate(`document.body.innerText.indexOf(${JSON.stringify(s)})>=0 ? 'yes':'no'`);
  const setDark = async (on) => {
    await evaluate(on ? `document.documentElement.classList.add('dark')` : `document.documentElement.classList.remove('dark')`);
    await sleep(800);
  };
  const shot = async (name) => {
    const tmp = path.join(TMP_OUT, name);
    await send('Page.captureScreenshot', { format: 'png' }).then((r) => {
      fs.writeFileSync(tmp, Buffer.from(r.data, 'base64'));
    });
    fs.copyFileSync(tmp, path.join(OUT, name));
    console.log('SHOT ' + name);
  };

  const jobs = SHOTS[MODE];
  if (!jobs) throw new Error('unknown MODE=' + MODE);
  await navTo('Fournisseurs');
  await sleep(1200);
  for (const [screen, file, keys, theme] of jobs) {
    if (MODE === 'markup') {
      const html = await evaluate(`(function(){var tb=document.querySelector('tbody');var bdi=tb?tb.querySelector('td bdi'):null;return bdi?bdi.outerHTML:'no-bdi';})()`);
      console.log('BALANCE_CELL_MARKUP=' + html);
      const emoji = await evaluate(`(function(){var h=document.documentElement.outerHTML;try{var re=/\\p{Extended_Pictographic}/u;return re.test(h)?'EMOJI_FOUND':'emoji-clean';}catch(e){return 'emoji-regex-unsupported';}})()`);
      console.log('EMOJI_SCAN=' + emoji);
      const fixed = await evaluate(`(function(){var els=document.querySelectorAll('section div,section p,section h2,section button,section table');var bad=[];for(var i=0;i<els.length;i++){if(getComputedStyle(els[i]).position==='fixed')bad.push(els[i].tagName+'.'+els[i].className.slice(0,40));}return bad.length?bad.join('|'):'no-fixed';})()`);
      console.log('FIXED_SCAN=' + fixed);
      continue;
    }
    if (MODE === 'nomatch') {
      await evaluate(`(${FILL})('input[placeholder*="fournisseur"]', 'Ciment introuvable')`);
      await sleep(2000);
    } else {
      await poll(() => has(MODE === 'list' ? 'Quincaillerie El Amel' : 'ournisseur'), (v) => v === 'yes', 20000, screen + ' content');
    }
    await setDark(theme === 'dark');
    const specs = keys.map((k) => ({ key: k.key, needle: NEEDLES[k.key] }));
    const m = await meas(specs);
    m.screen = screen;
    results.push(m);
    console.log('SAMPLE ' + screen + ' ' + JSON.stringify(m.items.map((i) => i.key + '=' + i.ratio + '/' + i.threshold + '/' + i.verdict)));
    console.log('SCROLL ' + screen + ' ' + (await dims()));
    await shot(file);
    if (MODE === 'nomatch') {
      await evaluate(`(${FILL})('input[placeholder*="fournisseur"]', '')`);
      await sleep(1500);
    }
  }
  fs.writeFileSync(path.join(OUT, `contrast-${MODE}.json`), JSON.stringify(results, null, 2));
  console.log('MODE_DONE=' + MODE);
}

main().then(() => { cleanupRun(); process.exit(0); }).catch((e) => { console.error('SCAN_FAILED ' + (e && e.message)); cleanupRun(); process.exit(1); });
