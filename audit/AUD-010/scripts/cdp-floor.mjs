// AUD-010 scripts/cdp-floor.mjs - measure WebView2 floor: ms to page target and to
// first contentful paint of a trivial data: page. Usage: node cdp-floor.mjs <port> <t0EpochMs> <out> <miles>
import fs from 'node:fs';
const port = Number(process.argv[2]);
const t0 = Number(process.argv[3]);
const out = process.argv[4];
const miles = process.argv[5];
let target = null; let targetMs = null;
for (let i = 0; i < 600; i++) {
  try {
    const r = await fetch(`http://127.0.0.1:${port}/json/list`);
    const list = await r.json();
    const t = list.find((x) => x.type === 'page');
    if (t) { target = t; targetMs = Math.round(Date.now() - t0); break; }
  } catch {}
  await new Promise((res) => setTimeout(res, 50));
}
fs.appendFileSync(miles, JSON.stringify({ k: 'pageTarget', ms: targetMs, url: target ? target.url : null, hasTarget: !!target }) + '\n');
if (!target) { fs.writeFileSync(out, JSON.stringify({ floor: 'NO_PAGE_TARGET', why: 'runtime never exposed a page target' }) + '\n'); process.exit(0); }
const ws = new WebSocket(`ws://127.0.0.1:${port}/devtools/page/${target.id}`);
let msgId = 1; const pending = new Map(); let onOpenMs = null;
ws.onmessage = (ev) => { let m; try { m = JSON.parse(ev.data); } catch { return; } if (m.id && pending.has(m.id)) { const { res, rej } = pending.get(m.id); pending.delete(m.id); m.error ? rej(new Error(m.error.message)) : res(m.result); } };
await new Promise((res, rej) => { ws.onopen = () => { onOpenMs = Math.round(Date.now() - t0); res(); }; ws.onerror = () => rej(); });
const cmd = (method, params = {}) => new Promise((res, rej) => { const id = msgId++; pending.set(id, { res, rej }); ws.send(JSON.stringify({ id, method, params })); });
await cmd('Runtime.enable');
const expr = `(() => { let paint = []; new PerformanceObserver((l) => { for (const e of l.getEntries()) paint.push({ n: e.name, s: e.startTime }); }).observe({ type: 'paint', buffered: true }); return { paint, ready: document.readyState, title: document.title, timeOrigin: performance.timeOrigin }; })()`;
let blob = null;
try { const r = await cmd('Runtime.evaluate', { expression: expr, returnByValue: true }); blob = r.result.value; } catch (e) { blob = { evalErr: String(e.message || e) }; }
const collectedMs = Math.round(Date.now() - t0);
fs.writeFileSync(out, JSON.stringify({ hasTarget: true, pageTargetMs: targetMs, wsOpenMs: onOpenMs, collectedMs, timeOrigin: blob ? blob.timeOrigin : null, timeOriginOffT0: blob && blob.timeOrigin ? Math.round(blob.timeOrigin - t0) : null, paint: blob ? blob.paint : null, ready: blob ? blob.ready : null, title: blob ? blob.title : null }) + '\n');
try { ws.close(); } catch {}
process.exit(0);