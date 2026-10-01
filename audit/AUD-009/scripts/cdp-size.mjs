// AUD-008 scripts/cdp-size.mjs - one-shot CDP viewport/metrics read. Usage: node cdp-size.mjs <port>
const port = Number(process.argv[2]);
const r = await fetch(`http://127.0.0.1:${port}/json/list`);
const list = await r.json();
const page = list.find((t) => t.type === 'page');
if (!page) { console.log(JSON.stringify({ error: 'no-page' })); process.exit(2); }
const ws = await new Promise((res, rej) => {
  const w = new WebSocket(page.webSocketDebuggerUrl);
  w.addEventListener('open', () => res(w), { once: true });
  w.addEventListener('error', () => rej(new Error('ws')), { once: true });
});
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
const v = await send('Runtime.evaluate', { expression: `(() => ({ iw: window.innerWidth, ih: window.innerHeight, ow: window.outerWidth, oh: window.outerHeight, dpr: window.devicePixelRatio, availW: screen.availWidth, availH: screen.availHeight, sw: document.documentElement.scrollWidth, cw: document.documentElement.clientWidth, sh: document.documentElement.scrollHeight, ch: document.documentElement.clientHeight }))()`, returnByValue: true });
console.log(JSON.stringify(v.result ? v.result.value : { error: 'no-result' }));
try { ws.close(); } catch {}
process.exit(0);