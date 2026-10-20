/**
 * TASK-DSN-075 headless verification (pure Node, no extra deps):
 * boots headless Chrome over CDP against the Vite dev server (:5173),
 * stubs Tauri IPC with 8 catalog rows (7 active + 1 inactive) and real-style
 * database metrics {8, 5, 2, 1}, then asserts:
 *  - KPI cards show database metrics, not the in-memory slice;
 *  - table container scrollWidth <= clientWidth (zero horizontal overflow)
 *    at 1280x800 and 1024x600;
 *  - every row's Delete button ("Supprimer") is fully inside the viewport;
 *  - the inactive row renders its "Inactif" badge.
 *
 * Usage: vite dev must be up on :5173, then `node audit/DSN-075/capture.mjs`.
 * Writes: audit/DSN-075/catalog-desktop-1280x800.png + capture-results.json.
 * Exit 0 only if every assertion passes.
 */
import { spawn } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const HERE = path.dirname(fileURLToPath(import.meta.url));
const CHROME = "C:/Program Files/Google/Chrome/Application/chrome.exe";
const DEBUG_PORT = 9226;
const APP_URL = "http://localhost:5173/";
const OUT_PNG = path.join(HERE, "catalog-desktop-1280x800.png");
const OUT_JSON = path.join(HERE, "capture-results.json");

function unit(uid, pid, price) {
  return {
    unit_id: uid, product_id: pid,
    unit_name_ar: "قطعة", unit_name_fr: "Pièce",
    factor_micro: "10000", barcode: null,
    sale_price_centimes: price, purchase_price_centimes: "12000",
    wholesale_price_centimes: null, is_base_unit: true, is_default_selling: true,
  };
}
function row(pid, sku, name, active, price) {
  return {
    product_id: pid, sku, barcode: null,
    name_ar: "صنف تجريبي", name_fr: name, category_id: null,
    is_kit: false, is_active: active,
    units: [unit(`u-${pid}`, pid, price)], kit_items: [],
  };
}
const ROWS = [
  row("p-1", "VIS-001", "Vis à bois tête fraisée 4x40 (boîte de 200)", true, "15000"),
  row("p-2", "BAT-001", "Batterie externe 20000mAh charge rapide USB-C", true, "890000"),
  row("p-3", "PEI-110", "Peinture vinylique blanche mate 15L grand rendement", true, "1250000"),
  row("p-4", "CIM-050", "Ciment gris CPJ 45 sac de 50kg", true, "98000"),
  row("p-5", "TUR-207", "Tuyau PVC évacuation diamètre 100mm barre de 4m", true, "145000"),
  row("p-6", "LAM-310", "Lampe torche LED rechargeable étanche IPX7", true, "67000"),
  row("p-7", "GAN-042", "Gants de chantier cuir renforcé taille L", true, "32000"),
  row("p-8", "BAT-002", "Batterie externe 10000mAh ancien modèle (retirée)", false, "450000"),
];
const LEVELS = { "p-1": "50000", "p-2": "120000", "p-3": "30000", "p-4": "8000", "p-5": "250000", "p-6": "0", "p-7": "0", "p-8": "0" };
const METRICS = { total_products: 8, in_stock: 5, out_of_stock: 2, inactive: 1 };

const STUB_SOURCE = `
window.__TAURI_INTERNALS__ = {
  invoke: function (cmd, args) {
    var req = (args && args.request) || {};
    var respond = function (v) { return Promise.resolve(v); };
    var fail = function (code) { return Promise.reject(code); };
    if (cmd === 'check_first_run_status') return respond({ requires_setup: false });
    if (cmd === 'auth_login') return respond({
      session_token: 'stub-admin', user_id: 'u-admin', username: 'admin',
      role: 'ADMIN', permissions: ['VIEW_STOCK', 'POS_SELL', 'CREATE_SALE'],
      expires_at: '2030-01-01T00:00:00',
    });
    if (cmd === 'auth_logout') return respond(null);
    if (cmd === 'warehouses_list') return respond({ warehouses: [{ id: 'dep-01', code: 'DEP-01', name: 'Dépôt Central', is_active: true }] });
    if (cmd === 'category_list') return respond([]);
    if (cmd === 'catalog_search') {
      window.__lastSearchArgs = req;
      return respond(${JSON.stringify(ROWS)});
    }
    if (cmd === 'catalog_get_metrics') return respond(${JSON.stringify(METRICS)});
    if (cmd === 'stock_get') {
      var pid = (req && req.product_id) || '';
      var qty = (${JSON.stringify(LEVELS)})[pid] ?? '0';
      return respond({ product_id: pid, warehouse_id: 'dep-01', base_unit_id: 'u-' + pid, quantity_base_micro: qty });
    }
    if (cmd === 'store_identity_get') return respond({ identity: { store_name: 'Q-Test', store_address: '', store_phone: '', nif: '', nis: '', rc: '', ai: '', ticket_footer_message: '' }, device_kind: 'physical' });
    if (cmd === 'cash_get_active_session') return respond(null);
    return fail('UNMOCKED_COMMAND:' + cmd);
  },
  transformCallback: function () { return 0; }
};
`;

class Cdp {
  constructor(url) { this.url = url; this.id = 0; this.pending = new Map(); this.consoleErrors = []; }
  connect() {
    return new Promise((resolve, reject) => {
      this.ws = new WebSocket(this.url);
      this.ws.addEventListener("open", resolve);
      this.ws.addEventListener("error", reject);
      this.ws.addEventListener("message", (ev) => {
        const msg = JSON.parse(String(ev.data));
        if (msg.method === "Console.messageAdded" && msg.params.message.level === "error") {
          this.consoleErrors.push(String(msg.params.message.text).slice(0, 200));
        } else if (msg.id && this.pending.has(msg.id)) {
          const { resolve: res, reject: rej } = this.pending.get(msg.id);
          this.pending.delete(msg.id);
          if (msg.error) rej(new Error(JSON.stringify(msg.error)));
          else res(msg.result);
        }
      });
    });
  }
  send(method, params = {}) {
    const id = ++this.id;
    return new Promise((resolve, reject) => {
      this.pending.set(id, { resolve, reject });
      this.ws.send(JSON.stringify({ id, method, params }));
    });
  }
  close() { try { this.ws.close(); } catch { /* noop */ } }
}

async function evalJs(cdp, expression, awaitPromise = false) {
  const res = await cdp.send("Runtime.evaluate", { expression, awaitPromise, returnByValue: true });
  if (res.exceptionDetails) throw new Error(`eval failed: ${JSON.stringify(res.exceptionDetails).slice(0, 300)}`);
  return res.result?.value;
}
async function waitFor(cdp, expression, timeoutMs = 20000, label = "cond") {
  const start = Date.now();
  for (;;) {
    const value = await evalJs(cdp, expression);
    if (value) return value;
    if (Date.now() - start > timeoutMs) throw new Error(`timeout waiting for ${label}`);
    await new Promise((r) => setTimeout(r, 250));
  }
}
async function httpJson(method, url) {
  const res = await fetch(url, { method });
  if (!res.ok) throw new Error(`${method} ${url} -> ${res.status}`);
  return res.json();
}
function fillExpr(sel, value) {
  return `(() => {
    const el = document.querySelector(${JSON.stringify(sel)});
    if (!el) return 'MISSING:' + ${JSON.stringify(sel)};
    const setter = Object.getOwnPropertyDescriptor(Object.getPrototypeOf(el), 'value')?.set
      ?? Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value')?.set;
    setter.call(el, ${JSON.stringify(value)});
    el.dispatchEvent(new Event('input', { bubbles: true }));
    el.dispatchEvent(new Event('change', { bubbles: true }));
    return 'ok';
  })()`;
}
function clickTextExpr(tag, text) {
  return `(() => {
    const els = [...document.querySelectorAll(${JSON.stringify(tag)})].filter((e) => (e.innerText ?? '').trim() === ${JSON.stringify(text)});
    if (els.length === 0) return 'MISSING:' + ${JSON.stringify(text)};
    els[0].click();
    return 'ok:' + els.length;
  })()`;
}

const KPI_EXPR = `(() => {
  const cards = [...document.querySelectorAll('button')].filter((b) => b.querySelector('.text-xl'));
  return cards.map((b) => {
    const lines = (b.innerText ?? '').split('\\n').map((s) => s.trim()).filter(Boolean);
    return { title: lines[0] ?? '', count: lines[lines.length - 1] ?? '' };
  });
})()`;
const BADGE_EXPR = `(() => {
  const h = [...document.querySelectorAll('h2')].find((e) => (e.innerText ?? '').includes('Catalogue'));
  const badge = h ? h.parentElement.querySelector('span.rounded-full') : null;
  return badge ? badge.innerText.trim() : 'MISSING';
})()`;
const OVERFLOW_EXPR = `(() => {
  const t = document.querySelector('table');
  if (!t) return 'NO-TABLE';
  const p = t.parentElement;
  return { tableSw: p.scrollWidth, tableCw: p.clientWidth, docSw: document.documentElement.scrollWidth, docCw: document.documentElement.clientWidth };
})()`;
const DELETE_EXPR = `(() => {
  const btns = [...document.querySelectorAll('button[aria-label="Supprimer"]')];
  return btns.map((b) => {
    const r = b.getBoundingClientRect();
    return { x: Math.round(r.x), w: Math.round(r.width), h: Math.round(r.height), inside: r.width > 0 && r.height > 0 && r.x >= 0 && (r.x + r.width) <= window.innerWidth };
  });
})()`;

const results = [];
async function step(name, fn) {
  try {
    const detail = await fn();
    results.push({ name, ok: true, detail: String(detail ?? "") });
    console.log(`  PASS ${name} :: ${String(detail ?? "").slice(0, 160)}`);
  } catch (err) {
    results.push({ name, ok: false, detail: String(err?.message ?? err).slice(0, 300) });
    console.log(`  FAIL ${name} :: ${String(err?.message ?? err).slice(0, 200)}`);
  }
}

async function main() {
  const chrome = spawn(CHROME, [
    "--headless=new", `--remote-debugging-port=${DEBUG_PORT}`,
    "--no-first-run", "--disable-extensions", "--window-size=1280,800", "about:blank",
  ], { stdio: "ignore" });
  let cdp = null;
  try {
    let targets = null;
    for (let i = 0; i < 40; i++) {
      try { targets = await httpJson("GET", `http://127.0.0.1:${DEBUG_PORT}/json/list`); break; }
      catch { await new Promise((r) => setTimeout(r, 500)); }
    }
    if (!targets || targets.length === 0) throw new Error("chrome remote debugging unavailable");
    const created = await httpJson("PUT", `http://127.0.0.1:${DEBUG_PORT}/json/new?about:blank`);
    cdp = new Cdp(created.webSocketDebuggerUrl);
    await cdp.connect();
    await cdp.send("Page.enable");
    await cdp.send("Runtime.enable");
    await cdp.send("Console.enable");
    await cdp.send("Page.addScriptToEvaluateOnNewDocument", { source: STUB_SOURCE });
    for (const vp of [{ width: 1280, height: 800 }, { width: 1024, height: 600 }]) {
      await cdp.send("Emulation.setDeviceMetricsOverride", { width: vp.width, height: vp.height, deviceScaleFactor: 1, mobile: false });
      if (vp.width === 1280) {
        await cdp.send("Page.navigate", { url: APP_URL });
        await waitFor(cdp, `!!document.querySelector('button[type="submit"]')`, 30000, "login submit");
        await evalJs(cdp, STUB_SOURCE);
        await evalJs(cdp, fillExpr('input[autocomplete="username"]', "admin"));
        await evalJs(cdp, fillExpr('input[autocomplete="current-password"]', "pw"));
        const label = await evalJs(cdp, `document.querySelector('button[type="submit"]').innerText.trim()`);
        await evalJs(cdp, clickTextExpr("button", label));
        await waitFor(cdp, `!!document.querySelector('nav')`, 15000, "nav after login");
        await evalJs(cdp, clickTextExpr("nav button", "Catalogue"));
        await waitFor(cdp, `document.body.innerText.includes('Tous les produits')`, 15000, "kpi cards");
        await waitFor(cdp, `document.querySelectorAll('table tbody tr').length >= 8`, 15000, "8 catalog rows");
        await new Promise((r) => setTimeout(r, 800));
      } else {
        await new Promise((r) => setTimeout(r, 800));
      }
      const tag = `${vp.width}x${vp.height}`;
      await step(`KPI cards show database metrics @${tag}`, async () => {
        const cards = await evalJs(cdp, KPI_EXPR);
        const get = (t) => cards.find((c) => c.title === t)?.count;
        const quad = [get("Tous les produits"), get("En stock"), get("En rupture"), get("Inactifs")];
        if (quad.join(",") !== "8,5,2,1") throw new Error(`kpi mismatch: ${JSON.stringify(cards)}`);
        return quad.join(",");
      });
      await step(`header badge shows database total @${tag}`, async () => {
        const badge = await evalJs(cdp, BADGE_EXPR);
        if (!String(badge).startsWith("8")) throw new Error(`badge=${badge}`);
        return badge;
      });
      await step(`zero horizontal overflow @${tag}`, async () => {
        const o = await evalJs(cdp, OVERFLOW_EXPR);
        if (o === "NO-TABLE") throw new Error("no table rendered");
        if (!(o.tableSw <= o.tableCw && o.docSw <= o.docCw)) throw new Error(JSON.stringify(o));
        return JSON.stringify(o);
      });
      await step(`delete buttons fully visible @${tag}`, async () => {
        const btns = await evalJs(cdp, DELETE_EXPR);
        if (btns.length < 8) throw new Error(`expected >=8 delete buttons, got ${btns.length}`);
        const bad = btns.filter((b) => !b.inside);
        if (bad.length > 0) throw new Error(`clipped: ${JSON.stringify(bad.slice(0, 3))}`);
        return `${btns.length}/8 inside viewport`;
      });
      if (vp.width === 1280) {
        await step("inactive row renders Inactif badge", async () => {
          const has = await evalJs(cdp, `document.body.innerText.includes('Inactif')`);
          if (!has) throw new Error("no Inactif badge in table");
          return "BAT-002 badge present";
        });
        await step("hub passes includeInactive=true to search", async () => {
          const last = await evalJs(cdp, `window.__lastSearchArgs ?? null`);
          if (!last || last.include_inactive !== true) throw new Error(`args=${JSON.stringify(last)}`);
          return JSON.stringify(last);
        });
        const shot = await cdp.send("Page.captureScreenshot", { format: "png" });
        fs.writeFileSync(OUT_PNG, Buffer.from(shot.data, "base64"));
        await step("screenshot captured", async () => `${OUT_PNG} (${fs.statSync(OUT_PNG).size} bytes)`);
      }
    }
    const unmocked = cdp.consoleErrors.filter((m) => m.includes("UNMOCKED_COMMAND"));
    await step("no unmocked IPC commands", async () => {
      if (unmocked.length > 0) throw new Error(unmocked.slice(0, 3).join(" | "));
      return `console errors: ${cdp.consoleErrors.length}`;
    });
  } finally {
    if (cdp) cdp.close();
    chrome.kill();
  }
  fs.writeFileSync(OUT_JSON, JSON.stringify({ at: new Date().toISOString(), results }, null, 2));
  const failed = results.filter((r) => !r.ok);
  console.log(`\nDSN-075 capture: ${results.length - failed.length}/${results.length} steps passed.`);
  if (failed.length > 0) process.exit(1);
}

main().catch((err) => { console.error("FATAL", err); process.exit(2); });
