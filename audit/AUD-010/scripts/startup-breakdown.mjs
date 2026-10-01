// AUD-010 scripts/startup-breakdown.mjs - passive CDP breakdown of the page boot.
// Single eager WebSocket to the page target as soon as it appears. Logs milestones
// (pageTarget / ws-open / first-eval) and, at first-run-form presence, returns one
// Runtime.evaluate blob: navigation+paint+resource+longtask+LoAF+fonts+locale +
// performance.timeOrigin + Performance.getMetrics. Node built-ins only.
// Usage: node startup-breakdown.mjs <port> <t0EpochMs> <outJson> <milesJsonl> <srcStarts>
import fs from 'node:fs';
const port = Number(process.argv[2]);
const t0EpochMs = Number(process.argv[3]);        // process-start epoch ms (launch.json t0EpochMs)
const outJson = process.argv[4];
const milesJsonl = process.argv[5];
const srcStarts = process.argv.slice(6);          // url prefixes identifying the app page
let hid = 0;
const log = (m) => { const line = JSON.stringify({ ...m, wallMs: Math.round(Date.now() - t0EpochMs) }); fs.appendFileSync(milesJsonl, line + '\n'); };
log({ k: 'start', t0EpochMs, nowEpoch: Date.now() });

async function getTarget() {
  for (let i = 0; i < 400; i++) {
    try {
      const r = await fetch(`http://127.0.0.1:${port}/json/list`);
      const list = await r.json();
      const t = list.find((x) => x.type === 'page' && (srcStarts.length === 0 || srcStarts.some((s) => (x.url || '').startsWith(s))));
      if (t) { log({ k: 'pageTarget', ms: Math.round(Date.now() - t0EpochMs), url: t.url, targetId: t.id }); return t; }
    } catch {}
    await new Promise((res) => setTimeout(res, 25));
  }
  throw new Error('NO_PAGE_TARGET');
}

const target = await getTarget();
const ws = new WebSocket(`ws://127.0.0.1:${port}/devtools/page/${target.id}`);
let msgId = 1;
const pending = new Map();
ws.onmessage = (ev) => {
  let m; try { m = JSON.parse(ev.data); } catch { return; }
  const id = m.id;
  if (id && pending.has(id)) { const { res, rej } = pending.get(id); pending.delete(id); m.error ? rej(new Error(m.error.message)) : res(m.result); }
};
function cmd(method, params = {}) { return new Promise((res, rej) => { const id = msgId++; pending.set(id, { res, rej }); ws.send(JSON.stringify({ id, method, params })); }); }
await new Promise((res, rej) => { ws.onopen = () => { log({ k: 'wsOpen', ms: Math.round(Date.now() - t0EpochMs) }); res(); }; ws.onerror = () => rej(new Error('WS_ERROR')); });
await cmd('Runtime.enable');

// Register buffered observers + capture an early timeOrigin snapshot.
const initExpr = `(() => {
  window.__aud = { timeOriginNow: performance.timeOrigin, now: Date.now(), marks: {}, paint: [], longtask: [], loaf: [], lcp: [] };
  try { new PerformanceObserver((l) => { for (const e of l.getEntries()) window.__aud.paint.push({ n: e.name, s: e.startTime }); }).observe({ type: 'paint', buffered: true }); } catch (e) {}
  try { new PerformanceObserver((l) => { for (const e of l.getEntries()) window.__aud.longtask.push({ s: e.startTime, d: e.duration }); }).observe({ type: 'longtask', buffered: true }); } catch (e) {}
  try { new PerformanceObserver((l) => { for (const e of l.getEntries()) window.__aud.lcp.push({ s: e.startTime, t: e.renderTime || e.loadTime }); }).observe({ type: 'largest-contentful-paint', buffered: true }); } catch (e) {}
  try { new PerformanceObserver((l) => { for (const e of l.getEntries()) window.__aud.loaf.push({ s: e.startTime, d: e.duration, scripts: (e.scripts||[]).map((s)=>({ u: s.sourceURL, d: s.duration })) }); }).observe({ type: 'long-animation-frame', buffered: true }); } catch (e) {}
  try { new PerformanceObserver((l) => { for (const e of l.getEntries()) window.__aud.marks.push(...l.getEntries().map((x)=>({n:x.name,s:x.startTime,t:x.duration}))); }).observe({ type: 'mark', buffered: true }); } catch (e) {}
  window.__aud.timeline = performance.getEntriesByType('mark').map((m) => ({ n: m.name, s: m.startTime }));
  return window.__aud.timeOriginNow;
})()`;
let r0;
try { r0 = await cmd('Runtime.evaluate', { expression: initExpr, returnByValue: true }); log({ k: 'firstEval', ms: Math.round(Date.now() - t0EpochMs), timeOrigin: r0.result.value }); }
catch (e) { log({ k: 'firstEvalErr', err: String(e.message || e) }); }

// Poll every 50 ms (harness clock) for the first-run form to exist (upper bound).
let formFoundPage = null; let formFoundWall = null;
for (let i = 0; i < 2400; i++) {
  try {
    const r = await cmd('Runtime.evaluate', { expression: `(() => { const f = document.querySelector('form, input'); return f ? ({ has: true, tag: f.tagName }) : { has: false }; })()`, returnByValue: true });
    if (r.result && r.result.value && r.result.value.has) { formFoundPage = null; formFoundWall = Math.round(Date.now() - t0EpochMs); }
  } catch {}
  if (formFoundWall) break;
  await new Promise((res) => setTimeout(res, 50));
}
log({ k: 'formFound', ms: formFoundWall, pageClockMs: formFoundPage, note: 'poll-upper-bound' });

// Big single runtime.evaluate returning everything.
const gather = `(() => {
  const nav = performance.getEntriesByType('navigation');
  const n = nav[0] ? { startTime: nav[0].startTime, fetchStart: nav[0].fetchStart, requestStart: nav[0].requestStart, responseStart: nav[0].responseStart, responseEnd: nav[0].responseEnd, domInteractive: nav[0].domInteractive, domContentLoadedEventStart: nav[0].domContentLoadedEventStart, domContentLoadedEventEnd: nav[0].domContentLoadedEventEnd, loadEventStart: nav[0].loadEventStart, loadEventEnd: nav[0].loadEventEnd, transferSize: nav[0].transferSize, encodedBodySize: nav[0].encodedBodySize, decodedBodySize: nav[0].decodedBodySize, type: nav[0].type } : null;
  const res = performance.getEntriesByType('resource').map((e) => { let path = e.name; try { const u = new URL(e.name, location.href); path = u.pathname; } catch {} const origin = (e.name.indexOf('http://') === 0 || e.name.indexOf('https://') === 0 || e.name.indexOf('ws://') === 0 || e.name.indexOf('wss://') === 0) ? 'other' : 'tauri-local'; return { path, init: e.initiatorType, s: e.startTime, re: e.responseEnd, d: e.duration, t: e.transferSize, db: e.decodedBodySize, origin }; });
  const fonts = { status: document.fonts ? document.fonts.status : null, size: document.fonts ? document.fonts.size : null, fam: document.fonts ? Array.from(document.fonts).map((f) => ({ fam: f.family, status: f.status })) : [] };
  const de = document.documentElement;
  return {
    timeOrigin: performance.timeOrigin, now: Date.now(), readyState: document.readyState,
    docLang: de ? de.lang : null, docDir: de ? de.getAttribute('dir') : null,
    navLang: navigator.language, navLangs: navigator.languages,
    nav: n, paint: window.__aud.paint, lcp: window.__aud.lcp,
    longtask: window.__aud.longtask, loaf: window.__aud.loaf, marks: window.__aud.marks,
    resources: res, fonts, iterations: Math.round(performance.getEntriesByType('longtask').length)
  };
})()`;
let blob = null;
try { const r = await cmd('Runtime.evaluate', { expression: gather, returnByValue: true }); blob = r.result.value; log({ k: 'gathered', ms: Math.round(Date.now() - t0EpochMs) }); }
catch (e) { log({ k: 'gatherErr', err: String(e.message || e) }); }

// Performance.getMetrics once.
let metrics = null;
try { const r = await cmd('Performance.getMetrics', {}); metrics = (r.metrics || []).map((m) => ({ n: m.name, v: m.value })); log({ k: 'metricsGot', ms: Math.round(Date.now() - t0EpochMs) }); }
catch (e) { log({ k: 'metricsErr', err: String(e.message || e) }); }

const state = { milestones: [] };
fs.appendFileSync(outJson, JSON.stringify({ targetId: target.id, t0EpochMs, formFoundWall, timeOrigin: blob ? blob.timeOrigin : null, blob, metrics }) + '\n');
log({ k: 'done', ms: Math.round(Date.now() - t0EpochMs) });
try { ws.close(); } catch {}
process.exit(0);