// AUD-007 scripts/cdp.mjs — minimal CDP client over global fetch + WebSocket (Node 26, no deps).
// Talks to WebView2 remote-debugging endpoint on 127.0.0.1 only.
// Usage:
//   node cdp.mjs <port> tabs
//   node cdp.mjs <port> eval <js-expression>
//   node cdp.mjs <port> metrics
//   node cdp.mjs <port> timeline <t0EpochMs> <timeoutMs>
//   node cdp.mjs <port> shot <outPngPath>
//   node cdp.mjs <port> overflow
import fs from 'node:fs';

const port = process.argv[2];
const cmd = process.argv[3];
const base = `http://127.0.0.1:${port}`;

async function pageWsUrl() {
  const r = await fetch(`${base}/json/list`);
  if (!r.ok) throw new Error('json/list HTTP ' + r.status);
  const list = await r.json();
  const page = list.find((t) => t.type === 'page');
  if (!page) throw new Error('no page target (got ' + list.length + ' targets)');
  return page.webSocketDebuggerUrl;
}

function connect(url) {
  return new Promise((resolve, reject) => {
    const ws = new WebSocket(url);
    ws.addEventListener('open', () => resolve(ws), { once: true });
    ws.addEventListener('error', (e) => reject(new Error('ws error: ' + (e.message || 'unknown'))), { once: true });
  });
}

async function session() {
  const ws = await connect(await pageWsUrl());
  let id = 0;
  const pending = new Map();
  ws.addEventListener('message', (ev) => {
    let m;
    try { m = JSON.parse(ev.data); } catch { return; }
    if (m.id !== undefined && pending.has(m.id)) {
      pending.get(m.id)(m);
      pending.delete(m.id);
    }
  });
  const send = (method, params = {}) => new Promise((resolve, reject) => {
    const mid = ++id;
    pending.set(mid, (m) => (m.error ? reject(new Error(method + ': ' + JSON.stringify(m.error))) : resolve(m.result)));
    ws.send(JSON.stringify({ id: mid, method, params }));
    setTimeout(() => {
      if (pending.has(mid)) { pending.delete(mid); reject(new Error('timeout ' + method)); }
    }, 20000);
  });
  const evaluate = async (expr, awaitPromise = true) => {
    const r = await send('Runtime.evaluate', { expression: expr, returnByValue: true, awaitPromise });
    return r.result ? r.result.value : undefined;
  };
  return { ws, send, evaluate };
}

const out = (o) => console.log(JSON.stringify(o));

if (cmd === 'tabs') {
  const r = await fetch(`${base}/json/list`);
  out(await r.json());
} else if (cmd === 'eval') {
  const s = await session();
  out(await s.evaluate(process.argv.slice(4).join(' ')));
  s.ws.close();
} else if (cmd === 'metrics') {
  const s = await session();
  out(await s.evaluate(`(() => ({
    innerWidth, innerHeight, outerWidth, outerHeight, devicePixelRatio,
    screenW: screen.width, screenH: screen.height,
    availW: screen.availWidth, availH: screen.availHeight,
    docClientW: document.documentElement.clientWidth, docClientH: document.documentElement.clientHeight,
    docScrollW: document.documentElement.scrollWidth, docScrollH: document.documentElement.scrollHeight,
    bodyFont: getComputedStyle(document.body).fontFamily,
    fontsCount: document.fonts.size,
    fonts: [...document.fonts].map(f => f.family + ':' + f.status),
    title: document.title,
    readyState: document.readyState
  }))()`));
  s.ws.close();
} else if (cmd === 'timeline') {
  const t0 = Number(process.argv[4]);
  const timeout = Number(process.argv[5] || '60000');
  const seen = {};
  const mark = (k, extra) => {
    if (!seen[k]) { seen[k] = true; console.log(JSON.stringify({ milestone: k, elapsedMs: Date.now() - t0, ...(extra || {}) })); }
  };
  const deadline = Date.now() + timeout;
  let s = null;
  while (Date.now() < deadline) {
    try {
      if (!s) s = await session();
      const v = await s.evaluate(`(() => ({
        rs: document.readyState,
        rootKids: (document.querySelector('#root') && document.querySelector('#root').childElementCount) || 0,
        textLen: document.body ? document.body.innerText.trim().length : 0,
        textHead: document.body ? document.body.innerText.trim().slice(0, 200) : ''
      }))()`, false);
      if (v.rs === 'complete') mark('c-readyState-complete');
      if (v.rootKids > 0) mark('d-root-children', { count: v.rootKids });
      if (v.textLen > 0) { mark('e-body-text', { len: v.textLen, head: v.textHead }); break; }
    } catch (e) { s = null; }
    await new Promise((r) => setTimeout(r, 50));
  }
  if (s) s.ws.close();
  if (!seen['e-body-text']) { console.log(JSON.stringify({ milestone: 'TIMEOUT', elapsedMs: Date.now() - t0 })); process.exitCode = 2; }
} else if (cmd === 'shot') {
  const s = await session();
  await s.send('Page.enable');
  const r = await s.send('Page.captureScreenshot', { format: 'png', fromSurface: true });
  fs.writeFileSync(process.argv[4], Buffer.from(r.data, 'base64'));
  console.log(JSON.stringify({ shot: process.argv[4], bytes: fs.statSync(process.argv[4]).size }));
  s.ws.close();
} else if (cmd === 'overflow') {
  const s = await session();
  out(await s.evaluate(`(() => {
    const vw = window.innerWidth, vh = window.innerHeight;
    const els = [...document.querySelectorAll('*')].filter(e => {
      const r = e.getBoundingClientRect();
      return r.width > 0 && r.height > 0 && (r.right > vw + 1 || r.bottom > vh + 1);
    });
    return {
      scrollW: document.documentElement.scrollWidth, clientW: document.documentElement.clientWidth,
      scrollH: document.documentElement.scrollHeight, clientH: document.documentElement.clientHeight,
      beyondCount: els.length,
      beyond: els.slice(0, 5).map(e => e.tagName + (e.id ? '#' + e.id : '') + ':' + (e.innerText || '').trim().slice(0, 60))
    };
  })()`));
  s.ws.close();
} else {
  console.error('unknown cmd');
  process.exitCode = 1;
}
