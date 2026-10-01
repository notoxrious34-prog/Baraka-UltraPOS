// AUD-008 scripts/cdp-harness.mjs - CDP startup milestones with a single eager connection.
// Sub-milestones (all elapsedMs relative to the launch T0 passed on argv):
//   c-pageTarget  : /json/list first reports a 'page' target
//   wsOpen        : WebSocket to the page first opens
//   c-readyState  : document.readyState === 'complete'
//   d-rootChildren: #root has childElementCount > 0
//   f-bodyText    : document.body innerText.trim().length > 0
//   g-screenshot  : first Page.captureScreenshot (saved only for run 1)
//   language-facts: lang/dir/navigator/localStorage/sessionStorage names (when -facts)
// Node 26 global fetch + WebSocket, no deps.
// Usage: node cdp-harness.mjs <port> <t0EpochMs> <outFile> <runIndex> [-facts]
import fs from 'node:fs';
const port = Number(process.argv[2]);
const t0 = Number(process.argv[3]);
const outFile = process.argv[4];
const run = Number(process.argv[5]);
const facts = process.argv.includes('-facts');
const base = `http://127.0.0.1:${port}`;
const append = (o) => fs.appendFileSync(outFile, JSON.stringify({ run, ...o }) + '\n', 'utf8');
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

// --- Phase 1: wait for a page target (milestone c) ---
let pageUrl = null;
const portDeadline = Date.now() + 90000;
let pollCount = 0;
while (Date.now() < portDeadline) {
  pollCount++;
  try {
    const r = await fetch(`${base}/json/list`);
    const list = await r.json();
    const p = list.find((t) => t.type === 'page');
    if (p) { pageUrl = p.webSocketDebuggerUrl; append({ milestone: 'c-pageTarget', elapsedMs: Date.now() - t0, url: p.url, pollCount }); break; }
  } catch {}
  await sleep(20);
}
if (!pageUrl) { append({ milestone: 'no-page-target', pollCount }); process.exit(2); }

// --- Phase 2: open ONE WebSocket, retrying only until it opens ---
let ws;
const wsDeadline = Date.now() + 30000;
while (!ws && Date.now() < wsDeadline) {
  try {
    await new Promise((res, rej) => {
      const w = new WebSocket(pageUrl);
      w.addEventListener('open', () => { ws = w; res(); }, { once: true });
      w.addEventListener('error', () => rej(new Error('ws')));
    });
    append({ milestone: 'wsOpen', elapsedMs: Date.now() - t0 });
  } catch { ws = null; await sleep(50); }
}
if (!ws) { append({ milestone: 'ws-fail' }); process.exit(3); }

// --- CDP client over the single ws ---
let id = 0; const pending = new Map();
ws.addEventListener('message', (ev) => {
  let m; try { m = JSON.parse(ev.data); } catch { return; }
  if (m.id !== undefined && pending.has(m.id)) { pending.get(m.id)(m); pending.delete(m.id); }
});
const send = (method, params = {}) => new Promise((res, rej) => {
  const mid = ++id;
  pending.set(mid, (m) => (m.error ? rej(new Error(method)) : res(m.result)));
  ws.send(JSON.stringify({ id: mid, method, params }));
  setTimeout(() => { if (pending.has(mid)) { pending.delete(mid); rej(new Error('timeout')); } }, 8000);
});

// --- Phase 3: poll content milestones on the established socket ---
const seen = {};
const mark = (k, extra) => { if (!seen[k]) { seen[k] = true; append({ milestone: k, elapsedMs: Date.now() - t0, ...(extra || {}) }); } };
const deadline = Date.now() + 30000;
let maxGap = 0; let last = Date.now();
while (Date.now() < deadline) {
  const now = Date.now(); const gap = now - last; if (gap > maxGap) maxGap = gap; last = now;
  try {
    const v = await Promise.race([
      send('Runtime.evaluate', { expression: `(() => ({ rs: document.readyState, kids: (document.querySelector('#root') && document.querySelector('#root').childElementCount) || 0, tl: document.body ? document.body.innerText.trim().length : 0, head: document.body ? document.body.innerText.trim().slice(0, 120) : '' }))()`, returnByValue: true }).then((r) => (r.result ? r.result.value : null)),
      sleep(300).then(() => null) ]);
    if (v) {
      if (v.rs === 'complete') mark('c-readyState');
      if (v.kids > 0) mark('d-rootChildren', { count: v.kids });
      if (v.tl > 0) mark('f-bodyText', { len: v.tl, head: v.head });
    }
  } catch {}
  if (seen['f-bodyText']) break;
  await sleep(20);
}
append({ milestone: 'loopGapMaxMs', value: maxGap });

if (seen['f-bodyText']) {
  await send('Page.enable').catch(() => {});
  const cap = await send('Page.captureScreenshot', { format: 'png', fromSurface: true }).catch(() => null);
  if (cap) {
    append({ milestone: 'g-screenshot', elapsedMs: Date.now() - t0, bytes: Buffer.from(cap.data, 'base64').length });
    if (run === 1) fs.writeFileSync('audit/AUD-008/screenshots/aud008-run1-first-screen.png', Buffer.from(cap.data, 'base64'));
  }
  append({ milestone: 'harness-done', elapsedMs: Date.now() - t0 });
}
if (facts && seen['f-bodyText']) {
  const kv = await send('Runtime.evaluate', { expression: `(() => ({ lang: document.documentElement.lang, dir: document.documentElement.dir, navLang: navigator.language, navLangs: navigator.languages, ls: JSON.parse(JSON.stringify(localStorage)) }))()`, returnByValue: true }).then((r) => (r.result ? r.result.value : null));
  const ssc = await send('Runtime.evaluate', { expression: `(() => { const o = {}; for (let i = 0; i < sessionStorage.length; i++) o[sessionStorage.key(i)] = true; return o; })()`, returnByValue: true }).then((r) => (r.result ? r.result.value : {}));
  append({ milestone: 'language-facts', value: kv, sessionStorageKeyNames: Object.keys(ssc || {}) });
}
try { ws.close(); } catch {}
process.exit(0);