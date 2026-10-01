// AUD-009 scripts/cdp-run.mjs - CDP milestones + maximized-state metrics for one run.
// Usage: node cdp-run.mjs <port> <t0EpochMs> <outJsonl> <shotPath|none> [-metrics]
// Logs: c-pageTarget, wsOpen, readyState-complete, rootChildren, bodyText,
//       screenshot (optional save), facts (lang/dir/navigator), metrics (sizes+overflow).
import fs from 'node:fs';
const port = Number(process.argv[2]);
const t0 = Number(process.argv[3]);
const outFile = process.argv[4];
const shotPath = process.argv[5];
const wantMetrics = process.argv.includes('-metrics');
const base = `http://127.0.0.1:${port}`;
const append = (o) => fs.appendFileSync(outFile, JSON.stringify(o) + '\n', 'utf8');
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

let pageUrl = null;
const portDeadline = Date.now() + 120000;
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
const ev = (expr) => send('Runtime.evaluate', { expression: expr, returnByValue: true }).then((r) => (r.result ? r.result.value : null));

const seen = {};
const mark = (k, extra) => { if (!seen[k]) { seen[k] = true; append({ milestone: k, elapsedMs: Date.now() - t0, ...(extra || {}) }); } };
const deadline = Date.now() + 60000;
let maxGap = 0; let last = Date.now();
while (Date.now() < deadline) {
  const now = Date.now(); const gap = now - last; if (gap > maxGap) maxGap = gap; last = now;
  try {
    const v = await Promise.race([
      ev(`(() => ({ rs: document.readyState, kids: (document.querySelector('#root') && document.querySelector('#root').childElementCount) || 0, tl: document.body ? document.body.innerText.trim().length : 0, head: document.body ? document.body.innerText.trim().slice(0, 120) : '' }))()`),
      sleep(300).then(() => null) ]);
    if (v) {
      if (v.rs === 'complete') mark('readyState-complete');
      if (v.kids > 0) mark('rootChildren', { count: v.kids });
      if (v.tl > 0) mark('bodyText', { len: v.tl, head: v.head });
    }
  } catch {}
  if (seen['bodyText']) break;
  await sleep(20);
}
append({ milestone: 'loopGapMaxMs', value: maxGap });

if (seen['bodyText']) {
  await send('Page.enable').catch(() => {});
  const cap = await send('Page.captureScreenshot', { format: 'png', fromSurface: true }).catch(() => null);
  if (cap) {
    append({ milestone: 'screenshot', elapsedMs: Date.now() - t0, bytes: Buffer.from(cap.data, 'base64').length });
    if (shotPath !== 'none') fs.writeFileSync(shotPath, Buffer.from(cap.data, 'base64'));
  }
  const fx = await ev(`(() => ({ lang: document.documentElement.lang, dir: document.documentElement.dir, navLang: navigator.language }))()`).catch(() => null);
  append({ milestone: 'facts', elapsedMs: Date.now() - t0, value: fx });
  if (wantMetrics) {
    const m = await ev(`(() => ({ iw: window.innerWidth, ih: window.innerHeight, ow: window.outerWidth, oh: window.outerHeight, dpr: window.devicePixelRatio, availW: screen.availWidth, availH: screen.availHeight, sw: document.documentElement.scrollWidth, cw: document.documentElement.clientWidth, sh: document.documentElement.scrollHeight, ch: document.documentElement.clientHeight }))()`).catch(() => null);
    append({ milestone: 'metrics', elapsedMs: Date.now() - t0, value: m });
  }
  append({ milestone: 'harness-done', elapsedMs: Date.now() - t0 });
}
try { ws.close(); } catch {}
process.exit(0);