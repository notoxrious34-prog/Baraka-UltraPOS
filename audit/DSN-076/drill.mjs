/**
 * TASK-DSN-076 live end-to-end integration drill (pure Node, no extra deps).
 * Boots headless Chrome over CDP against the Vite dev server (:5173) with a
 * MUT shared stub backend (products array + stock map), proving Catalog↔POS
 * consistency through the real IPC boundary:
 *  T1 catalog real metrics + header badge accuracy;
 *  T2 zero horizontal scroll + Delete-button visibility (1280x800, 1024x600);
 *  T3 product creation + deactivation flow through real IPC commands;
 *  T4 POS inactive filtering (grid, search, barcode scan-to-cart);
 *  T5 quick-products toggle persistence + Alt+1..8 suppression;
 *  T6 slot customization (clear/assign via active-only search) + dispatch;
 *  T7 dark theme + Arabic RTL layout integrity.
 *
 * Usage: vite dev must be up on :5173, then `node audit/DSN-076/drill.mjs`.
 * Writes: screenshots + drill-results.json under audit/DSN-076/.
 * Exit 0 only if every step passes.
 */
import { spawn } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const HERE = path.dirname(fileURLToPath(import.meta.url));
const CHROME = "C:/Program Files/Google/Chrome/Application/chrome.exe";
const DEBUG_PORT = 9227;
const APP_URL = "http://localhost:5173/";

function unit(uid, pid, price) {
  return {
    unit_id: uid, product_id: pid,
    unit_name_ar: "قطعة", unit_name_fr: "Pièce",
    factor_micro: "10000", barcode: null,
    sale_price_centimes: price, purchase_price_centimes: "12000",
    wholesale_price_centimes: null, is_base_unit: true, is_default_selling: true,
  };
}
function row(pid, sku, name, active, price, barcode) {
  return {
    product_id: pid, sku, barcode: barcode ?? null,
    name_ar: "صنف تجريبي", name_fr: name, category_id: null,
    is_kit: false, is_active: active,
    units: [{ ...unit(`u-${pid}`, pid, price), barcode: barcode ?? null }],
    kit_items: [],
  };
}
const SEED_ROWS = [
  row("p-1", "VIS-001", "Vis à bois tête fraisée 4x40 (boîte de 200)", true, "15000", "6111250090100"),
  row("p-2", "BAT-001", "Batterie externe 20000mAh charge rapide USB-C", true, "890000"),
  row("p-3", "PEI-110", "Peinture vinylique blanche mate 15L", true, "1250000"),
  row("p-4", "CIM-050", "Ciment gris CPJ 45 sac de 50kg", true, "98000"),
  row("p-5", "TUR-207", "Tuyau PVC évacuation 100mm barre de 4m", true, "145000"),
  row("p-6", "LAM-310", "Lampe torche LED rechargeable étanche IPX7", true, "67000"),
  row("p-7", "GAN-042", "Gants de chantier cuir renforcé taille L", true, "32000"),
  row("p-8", "BAT-002", "Batterie externe 10000mAh ancien modèle (retirée)", false, "450000"),
];
const SEED_LEVELS = { "p-1": "50000", "p-2": "120000", "p-3": "30000", "p-4": "8000", "p-5": "250000", "p-6": "0", "p-7": "0", "p-8": "0" };

const STUB_SOURCE = `
window.__DB__ = { products: ${JSON.stringify(SEED_ROWS)}, levels: ${JSON.stringify(SEED_LEVELS)}, seq: 100 };
window.__TAURI_INTERNALS__ = {
  invoke: function (cmd, args) {
    var req = (args && args.request) || {};
    var db = window.__DB__;
    var respond = function (v) { return Promise.resolve(v); };
    var fail = function (code) { return Promise.reject(code); };
    function metrics() {
      var total = db.products.length, inactive = 0, inStock = 0, out = 0;
      for (var i = 0; i < db.products.length; i++) {
        var p = db.products[i];
        if (!p.is_active) { inactive++; continue; }
        if (BigInt(db.levels[p.product_id] ?? '0') > 0n) inStock++; else out++;
      }
      return { total_products: total, in_stock: inStock, out_of_stock: out, inactive: inactive };
    }
    if (cmd === 'check_first_run_status') return respond({ requires_setup: false });
    if (cmd === 'auth_login') return respond({
      session_token: 'stub-admin', user_id: 'u-admin', username: 'admin',
      role: 'ADMIN', permissions: ['VIEW_STOCK', 'POS_SELL', 'CREATE_SALE', 'VIEW_CUSTOMERS'],
      expires_at: '2030-01-01T00:00:00',
    });
    if (cmd === 'auth_logout') return respond(null);
    if (cmd === 'warehouses_list') return respond({ warehouses: [{ id: 'dep-01', code: 'DEP-01', name: 'Dépôt Central', is_active: true }] });
    if (cmd === 'category_list') return respond([]);
    if (cmd === 'catalog_search') {
      window.__lastSearchArgs = req;
      var q = String(req.query ?? '').trim().toLowerCase();
      var list = db.products.filter(function (p) {
        if (req.include_inactive !== true && !p.is_active) return false;
        if (q === '') return true;
        return p.name_fr.toLowerCase().includes(q) || p.sku.toLowerCase() === q ||
          (p.barcode && p.barcode.toLowerCase() === q) ||
          p.units.some(function (u) { return u.barcode && u.barcode.toLowerCase() === q; });
      });
      var limit = Math.max(1, Math.min(500, Number(req.limit) || 20));
      return respond(list.slice(0, limit));
    }
    if (cmd === 'catalog_get_metrics') return respond(metrics());
    if (cmd === 'catalog_get_product') {
      var got = db.products.find(function (p) { return p.product_id === req.product_id; });
      return got ? respond(got) : fail('NOT_FOUND');
    }
    if (cmd === 'catalog_create_product') {
      var n = db.seq++;
      var created = {
        product_id: 'p-new-' + n, sku: req.sku, barcode: req.barcode ?? null,
        name_ar: req.name_ar, name_fr: req.name_fr, category_id: req.category_id ?? null,
        is_kit: !!req.is_kit, is_active: true,
        units: (req.units || []).map(function (u, i) {
          return { unit_id: 'u-new-' + n + '-' + i, product_id: 'p-new-' + n,
            unit_name_ar: u.unit_name_ar, unit_name_fr: u.unit_name_fr,
            factor_micro: u.factor_micro, barcode: u.barcode ?? null,
            sale_price_centimes: u.sale_price_centimes,
            purchase_price_centimes: u.purchase_price_centimes,
            wholesale_price_centimes: u.wholesale_price_centimes ?? null,
            is_base_unit: !!u.is_base_unit, is_default_selling: !!u.is_default_selling };
        }),
        kit_items: [],
      };
      db.products.push(created);
      db.levels[created.product_id] = '0';
      return respond(created);
    }
    if (cmd === 'catalog_update_product') {
      var target = db.products.find(function (p) { return p.product_id === req.product_id; });
      if (!target) return fail('NOT_FOUND');
      target.name_ar = req.name_ar; target.name_fr = req.name_fr;
      target.is_active = req.is_active !== false;
      return respond(null);
    }
    if (cmd === 'catalog_delete_product') {
      var idx = db.products.findIndex(function (p) { return p.product_id === req.product_id; });
      if (idx < 0) return fail('NOT_FOUND');
      var removed = db.products.splice(idx, 1)[0];
      return respond({ id: removed.product_id, mode: 'hard_deleted', message: 'ok' });
    }
    if (cmd === 'stock_get') {
      var pid = req.product_id || '';
      return respond({ product_id: pid, warehouse_id: 'dep-01', base_unit_id: 'u-' + pid, quantity_base_micro: db.levels[pid] ?? '0' });
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
const OVERFLOW_EXPR = `(() => {
  const t = document.querySelector('table');
  if (!t) return 'NO-TABLE';
  const p = t.parentElement;
  return { tableSw: p.scrollWidth, tableCw: p.clientWidth, docSw: document.documentElement.scrollWidth, docCw: document.documentElement.clientWidth };
})()`;
const DELETE_EXPR = `(() => {
  const btns = [...document.querySelectorAll('button[aria-label="Supprimer"]')];
  return { count: btns.length, bad: btns.filter((b) => {
    const r = b.getBoundingClientRect();
    return !(r.width > 0 && r.height > 0 && r.x >= 0 && (r.x + r.width) <= window.innerWidth);
  }).length };
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
async function shot(cdp, name) {
  const res = await cdp.send("Page.captureScreenshot", { format: "png" });
  const file = new URL(name, import.meta.url).pathname;
  void file;
  const out = path.join(HERE, name);
  fs.writeFileSync(out, Buffer.from(res.data, "base64"));
  return `${name} (${fs.statSync(out).size} bytes)`;
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
    await cdp.send("Emulation.setDeviceMetricsOverride", { width: 1280, height: 800, deviceScaleFactor: 1, mobile: false });
    await cdp.send("Page.addScriptToEvaluateOnNewDocument", { source: STUB_SOURCE });
    await cdp.send("Page.navigate", { url: APP_URL });
    await waitFor(cdp, `!!document.querySelector('button[type="submit"]')`, 30000, "login submit");
    await evalJs(cdp, STUB_SOURCE);
    await evalJs(cdp, fillExpr('input[autocomplete="username"]', "admin"));
    await evalJs(cdp, fillExpr('input[autocomplete="current-password"]', "pw"));
    const label = await evalJs(cdp, `document.querySelector('button[type="submit"]').innerText.trim()`);
    await evalJs(cdp, clickTextExpr("button", label));
    await waitFor(cdp, `!!document.querySelector('nav')`, 15000, "nav after login");

    // Seed quick slots by visiting POS once (palette hydrates from catalog).
    await evalJs(cdp, clickTextExpr("nav button", "Caisse"));
    await waitFor(cdp, `!!document.querySelector('div[role="toolbar"]')`, 15000, "palette toolbar");
    await waitFor(cdp, `document.querySelectorAll('div[role="toolbar"] button').length >= 8`, 15000, "8 slot chips");

    console.log("T1+T2 catalog metrics and layout @1280x800");
    await evalJs(cdp, clickTextExpr("nav button", "Catalogue"));
    await waitFor(cdp, `document.body.innerText.includes('Tous les produits')`, 15000, "kpi cards");
    await waitFor(cdp, `document.querySelectorAll('table tbody tr').length >= 8`, 15000, "8 catalog rows");
    await new Promise((r) => setTimeout(r, 800));
    await step("T1 KPI cards show database aggregates 8/5/2/1", async () => {
      const cards = await evalJs(cdp, KPI_EXPR);
      const get = (t) => cards.find((c) => c.title === t)?.count;
      const quad = [get("Tous les produits"), get("En stock"), get("En rupture"), get("Inactifs")];
      if (quad.join(",") !== "8,5,2,1") throw new Error(`kpi mismatch: ${JSON.stringify(cards)}`);
      const badge = await evalJs(cdp, `(() => {
        const h = [...document.querySelectorAll('h2')].find((e) => (e.innerText ?? '').includes('Catalogue'));
        return h ? h.parentElement.querySelector('span.rounded-full')?.innerText.trim() : 'MISSING';
      })()`);
      if (!String(badge).startsWith("8")) throw new Error(`badge=${badge}`);
      return `kpi=${quad.join(",")} badge=${badge}`;
    });
    await step("T2 zero horizontal overflow at 1280x800", async () => {
      const o = await evalJs(cdp, OVERFLOW_EXPR);
      if (o === "NO-TABLE") throw new Error("no table");
      if (!(o.tableSw <= o.tableCw && o.docSw <= o.docCw)) throw new Error(JSON.stringify(o));
      return JSON.stringify(o);
    });
    await step("T2 delete buttons fully visible (8 rows)", async () => {
      const d = await evalJs(cdp, DELETE_EXPR);
      if (d.count < 8 || d.bad > 0) throw new Error(JSON.stringify(d));
      return `${d.count} buttons, 0 clipped`;
    });
    await step("T2 screenshot catalog", async () => shot(cdp, "catalog-1280x800.png"));

    console.log("T3 create + deactivate through real IPC");
    await step("T3 create product via catalog_create_product", async () => {
      const id = await evalJs(cdp, `window.__TAURI_INTERNALS__.invoke('catalog_create_product', { request: {
        session_token: 'stub-admin', sku: 'DRILL-NEW', barcode: 'DRILL-BARCODE-1',
        name_ar: 'صنف تجريبي جديد', name_fr: 'Produit Drill Nouveau', category_id: null, is_kit: false,
        units: [{ unit_name_ar: 'قطعة', unit_name_fr: 'Pièce', factor_micro: '10000', barcode: 'DRILL-BARCODE-1',
          sale_price_centimes: '75000', purchase_price_centimes: '50000', wholesale_price_centimes: null,
          is_base_unit: true, is_default_selling: true }],
      }}).then((p) => p.product_id)`, true);
      await evalJs(cdp, clickTextExpr("button", "Actualiser"));
      await waitFor(cdp, `document.body.innerText.includes('DRILL-NEW')`, 15000, "new row visible");
      const cards = await evalJs(cdp, KPI_EXPR);
      const get = (t) => cards.find((c) => c.title === t)?.count;
      if (get("Tous les produits") !== "9") throw new Error(`total should be 9: ${JSON.stringify(cards)}`);
      return `created ${id}, total=9`;
    });
    await step("T3 deactivate via UI status toggle", async () => {
      const clicked = await evalJs(cdp, `(() => {
        const rows = [...document.querySelectorAll('table tbody tr')];
        const row = rows.find((r) => (r.innerText ?? '').includes('DRILL-NEW'));
        if (!row) return 'MISSING-ROW';
        const btn = [...row.querySelectorAll('button')].find((b) => /Actif/.test(b.innerText ?? ''));
        if (!btn) return 'MISSING-TOGGLE';
        btn.click();
        return 'ok';
      })()`);
      if (clicked !== "ok") throw new Error(clicked);
      await waitFor(cdp, `(() => {
        const cards = [...document.querySelectorAll('button')].filter((b) => b.querySelector('.text-xl'));
        const ina = cards.find((b) => (b.innerText ?? '').includes('Inactifs'));
        return ina && ina.innerText.includes('2');
      })()`, 15000, "inactifs=2");
      const last = await evalJs(cdp, `window.__lastSearchArgs ?? null`);
      if (!last || last.include_inactive !== true) throw new Error(`hub search args: ${JSON.stringify(last)}`);
      return "inactifs=2, hub search passes include_inactive=true";
    });

    console.log("T4 POS inactive exclusion");
    await evalJs(cdp, clickTextExpr("nav button", "Caisse"));
    await waitFor(cdp, `!!document.querySelector('#pos-search-input')`, 15000, "pos search");
    await new Promise((r) => setTimeout(r, 1000));
    await step("T4 discovery grid hides inactive rows", async () => {
      const text = await evalJs(cdp, `document.body.innerText`);
      if (text.includes("DRILL-NEW") || text.includes("ancien modèle")) {
        throw new Error("inactive product leaked into POS");
      }
      return "grid clean";
    });
    await step("T4 inactive search yields no results", async () => {
      await evalJs(cdp, fillExpr("#pos-search-input", "DRILL-NEW"));
      await new Promise((r) => setTimeout(r, 1200));
      const text = await evalJs(cdp, `document.body.innerText`);
      if (text.includes("DRILL-NEW")) throw new Error("inactive searchable in POS");
      if (!text.includes("Aucun produit trouv")) throw new Error("no-results message missing");
      await evalJs(cdp, fillExpr("#pos-search-input", ""));
      await new Promise((r) => setTimeout(r, 1200));
      return "no-results shown";
    });
    await step("T4 active barcode scans to cart, inactive barcode ignored", async () => {
      await evalJs(cdp, fillExpr("#pos-search-input", "6111250090100"));
      await new Promise((r) => setTimeout(r, 1200));
      await evalJs(cdp, `document.querySelector('#pos-search-input').focus()`);
      await cdp.send("Input.dispatchKeyEvent", { type: "keyDown", code: "Enter", key: "Enter", windowsVirtualKeyCode: 13 });
      await cdp.send("Input.dispatchKeyEvent", { type: "keyUp", code: "Enter", key: "Enter", windowsVirtualKeyCode: 13 });
      await waitFor(cdp, `document.body.innerText.includes('ajouté au panier')`, 10000, "active scan banner");
      await evalJs(cdp, fillExpr("#pos-search-input", "DRILL-BARCODE-1"));
      await new Promise((r) => setTimeout(r, 1200));
      await evalJs(cdp, `document.querySelector('#pos-search-input').focus()`);
      await cdp.send("Input.dispatchKeyEvent", { type: "keyDown", code: "Enter", key: "Enter", windowsVirtualKeyCode: 13 });
      await cdp.send("Input.dispatchKeyEvent", { type: "keyUp", code: "Enter", key: "Enter", windowsVirtualKeyCode: 13 });
      await new Promise((r) => setTimeout(r, 1500));
      const text = await evalJs(cdp, `document.body.innerText`);
      if (text.includes("DRILL-NEW")) throw new Error("inactive barcode reached the cart");
      await evalJs(cdp, fillExpr("#pos-search-input", ""));
      await new Promise((r) => setTimeout(r, 800));
      return "scan gating holds";
    });

    console.log("T5 quick-products toggle + shortcut gating");
    const gridH = `document.querySelector('#pos-search-input').closest('section').querySelector('div.min-h-0.flex-1').clientHeight`;
    const hBefore = await evalJs(cdp, gridH);
    await step("T5 toggle off hides palette and persists", async () => {
      await evalJs(cdp, `document.querySelector('button[aria-label="Produits rapides"]').click()`);
      await waitFor(cdp, `!document.querySelector('div[role="toolbar"]')`, 8000, "palette gone");
      const flag = await evalJs(cdp, `localStorage.getItem('baraka-quick-products-enabled')`);
      if (flag !== "false") throw new Error(`flag=${flag}`);
      const hAfter = await evalJs(cdp, gridH);
      if (!(hAfter > hBefore)) throw new Error(`grid height ${hBefore} -> ${hAfter}`);
      return `flag=false, grid ${hBefore}px -> ${hAfter}px`;
    });
    await step("T5 Alt+1 suppressed while disabled", async () => {
      await evalJs(cdp, `document.querySelector('#pos-search-input').blur()`);
      await cdp.send("Input.dispatchKeyEvent", { type: "keyDown", code: "Digit1", key: "1", windowsVirtualKeyCode: 49, modifiers: 1 });
      await cdp.send("Input.dispatchKeyEvent", { type: "keyUp", code: "Digit1", key: "1", windowsVirtualKeyCode: 49, modifiers: 1 });
      await new Promise((r) => setTimeout(r, 800));
      const text = await evalJs(cdp, `document.body.innerText`);
      if (text.includes("Alt+1 ·")) throw new Error("shortcut fired while disabled");
      return "zero cart mutation";
    });
    await step("T5 toggle on restores palette and Alt+1 dispatches", async () => {
      await evalJs(cdp, `document.querySelector('button[aria-label="Produits rapides"]').click()`);
      await waitFor(cdp, `!!document.querySelector('div[role="toolbar"]')`, 8000, "palette back");
      const flag = await evalJs(cdp, `localStorage.getItem('baraka-quick-products-enabled')`);
      if (flag !== "true") throw new Error(`flag=${flag}`);
      await evalJs(cdp, `document.querySelector('#pos-search-input').blur()`);
      await cdp.send("Input.dispatchKeyEvent", { type: "keyDown", code: "Digit1", key: "1", windowsVirtualKeyCode: 49, modifiers: 1 });
      await cdp.send("Input.dispatchKeyEvent", { type: "keyUp", code: "Digit1", key: "1", windowsVirtualKeyCode: 49, modifiers: 1 });
      await waitFor(cdp, `document.body.innerText.includes('Alt+1 ·')`, 8000, "dispatch banner");
      return "flag=true, Alt+1 dispatched";
    });
    await step("T5 totals footer intact inside viewport", async () => {
      const box = await evalJs(cdp, `(() => {
        const spans = [...document.querySelectorAll('span')].filter((s) => (s.innerText ?? '').includes('TOTAL TTC'));
        if (spans.length === 0) return 'MISSING-TOTAL';
        const host = spans[0].closest('section') ?? spans[0].parentElement;
        const r = host.getBoundingClientRect();
        return { x: Math.round(r.x), w: Math.round(r.width), inside: r.width > 0 && r.x >= 0 && (r.x + r.width) <= window.innerWidth };
      })()`);
      if (box === "MISSING-TOTAL" || !box.inside) throw new Error(JSON.stringify(box));
      return JSON.stringify(box);
    });
    await step("T5 screenshot pos palette", async () => shot(cdp, "pos-palette-1280x800.png"));

    console.log("T6 slot customization");
    await step("T6 clear + assign slot through active-only search", async () => {
      const gearOpened = await evalJs(cdp, `(() => {
        const gear = [...document.querySelectorAll('div[role="toolbar"] button')].find((b) => (b.title ?? '').includes('Personnaliser'));
        if (!gear) return 'MISSING-GEAR';
        gear.click();
        return 'ok';
      })()`);
      if (gearOpened !== "ok") throw new Error(gearOpened);
      await waitFor(cdp, `document.body.innerText.includes('personnalisation')`, 8000, "config modal");
      const pre = await evalJs(cdp, `(JSON.parse(localStorage.getItem('baraka-quick-products') ?? '[]')).map((s) => s?.sku ?? null)`);
      const target = pre.lastIndexOf(pre.find((s, i) => s !== null && pre.slice(i + 1).every((x) => x === null)));
      if (target < 0) throw new Error(`no filled slot to recycle: ${JSON.stringify(pre)}`);
      await evalJs(cdp, `localStorage.setItem('baraka-dsn076-target', '${target}')`);
      await evalJs(cdp, `(() => {
        const clears = [...document.querySelectorAll('button')].filter((b) => (b.innerText ?? '').trim() === 'Effacer');
        if (clears.length === 0) return 'MISSING-CLEAR';
        clears[clears.length - 1].click();
        return 'ok:' + clears.length;
      })()`);
      const assigns = await evalJs(cdp, `(() => {
        const btns = [...document.querySelectorAll('button')].filter((b) => (b.innerText ?? '').trim() === 'Assigner');
        if (btns.length === 0) return 'MISSING-ASSIGN';
        btns[0].click();
        return 'ok';
      })()`);
      if (assigns !== "ok") throw new Error(assigns);
      const hint = await evalJs(cdp, `(() => {
        const inputs = [...document.querySelectorAll('input')].filter((i) => (i.placeholder ?? '').includes('Rechercher un produit actif'));
        return inputs.length;
      })()`);
      if (hint !== 1) throw new Error(`active-only search input missing (${hint})`);
      await evalJs(cdp, `(() => {
        const input = [...document.querySelectorAll('input')].find((i) => (i.placeholder ?? '').includes('Rechercher un produit actif'));
        const setter = Object.getOwnPropertyDescriptor(Object.getPrototypeOf(input), 'value')?.set;
        setter.call(input, 'Ciment');
        input.dispatchEvent(new Event('input', { bubbles: true }));
        return 'ok';
      })()`);
      await waitFor(cdp, `[...document.querySelectorAll('li button')].some((b) => (b.innerText ?? '').includes('CIM-050'))`, 10000, "assign candidate");
      await evalJs(cdp, `[...document.querySelectorAll('li button')].find((b) => (b.innerText ?? '').includes('CIM-050')).click()`);
      await new Promise((r) => setTimeout(r, 600));
      const post = await evalJs(cdp, `(JSON.parse(localStorage.getItem('baraka-quick-products') ?? '[]')).map((s) => s?.sku ?? null)`);
      const want = await evalJs(cdp, `Number(localStorage.getItem('baraka-dsn076-target'))`);
      if (post[want] !== "CIM-050") throw new Error(`slots=${JSON.stringify(post)} target=${want}`);
      return `slot ${want + 1} := CIM-050`;
    });
    await step("T6 Alt+N dispatches the assigned slot", async () => {
      const want = await evalJs(cdp, `Number(localStorage.getItem('baraka-dsn076-target'))`);
      const digit = want + 1;
      await evalJs(cdp, `[...document.querySelectorAll('button')].find((b) => (b.innerText ?? '').trim() === 'Fermer')?.click()`);
      await new Promise((r) => setTimeout(r, 500));
      await evalJs(cdp, `document.querySelector('#pos-search-input').blur()`);
      const code = `Digit${digit}`;
      const vk = 48 + digit;
      await cdp.send("Input.dispatchKeyEvent", { type: "keyDown", code, key: String(digit), windowsVirtualKeyCode: vk, modifiers: 1 });
      await cdp.send("Input.dispatchKeyEvent", { type: "keyUp", code, key: String(digit), windowsVirtualKeyCode: vk, modifiers: 1 });
      await waitFor(cdp, `document.body.innerText.includes('Alt+${digit} ·')`, 8000, "assigned slot banner");
      return `Alt+${digit} dispatched`;
    });
    await step("T6 screenshot config modal", async () => {
      await evalJs(cdp, `(() => {
        const gear = [...document.querySelectorAll('div[role="toolbar"] button')].find((b) => (b.title ?? '').includes('Personnaliser'));
        gear?.click();
        return 'ok';
      })()`);
      await waitFor(cdp, `document.body.innerText.includes('personnalisation')`, 8000, "config modal");
      return shot(cdp, "pos-config-1280x800.png");
    });

    console.log("T2b compact viewport + T7 theme/locale");
    await cdp.send("Emulation.setDeviceMetricsOverride", { width: 1024, height: 600, deviceScaleFactor: 1, mobile: false });
    await evalJs(cdp, clickTextExpr("nav button", "Catalogue"));
    await new Promise((r) => setTimeout(r, 1000));
    await step("T2 zero overflow + delete visible at 1024x600", async () => {
      const o = await evalJs(cdp, OVERFLOW_EXPR);
      const d = await evalJs(cdp, DELETE_EXPR);
      if (o === "NO-TABLE") throw new Error("no table");
      if (!(o.tableSw <= o.tableCw && o.docSw <= o.docCw)) throw new Error(JSON.stringify(o));
      if (d.count < 9 || d.bad > 0) throw new Error(JSON.stringify(d));
      return `${JSON.stringify(o)} deletes=${d.count}`;
    });
    await step("T7 dark theme keeps layout clean", async () => {
      await evalJs(cdp, clickTextExpr("nav button", "Caisse"));
      await waitFor(cdp, `!!document.querySelector('button[aria-label="Thème"]')`, 8000, "theme toggle");
      await evalJs(cdp, `document.querySelector('button[aria-label="Thème"]').click()`);
      await waitFor(cdp, `document.documentElement.classList.contains('dark')`, 8000, "dark on");
      await evalJs(cdp, clickTextExpr("nav button", "Catalogue"));
      await new Promise((r) => setTimeout(r, 800));
      const o = await evalJs(cdp, OVERFLOW_EXPR);
      const d = await evalJs(cdp, DELETE_EXPR);
      if (!(o.tableSw <= o.tableCw)) throw new Error(JSON.stringify(o));
      if (d.bad > 0) throw new Error(JSON.stringify(d));
      return shot(cdp, "catalog-dark-1280x800.png");
    });
    await step("T7 arabic RTL keeps layout clean", async () => {
      await evalJs(cdp, clickTextExpr("nav button", "Caisse"));
      const langClicked = await evalJs(cdp, `(() => {
        const btn = [...document.querySelectorAll('button')].find((b) => (b.innerText ?? '').includes('العربية'));
        if (!btn) return 'MISSING-LANG';
        btn.click();
        return 'ok';
      })()`);
      if (langClicked !== "ok") throw new Error(langClicked);
      await waitFor(cdp, `document.body.innerText.includes('جميع المنتجات') || document.body.innerText.includes('المنتجات')`, 8000, "ar locale");
      await evalJs(cdp, clickTextExpr("nav button", "الكتالوج"));
      await waitFor(cdp, `document.body.innerText.includes('جميع المنتجات')`, 8000, "ar kpi");
      const dir = await evalJs(cdp, `document.querySelector('[dir]')?.getAttribute('dir') ?? 'none'`);
      if (dir !== "rtl") throw new Error(`dir=${dir}`);
      await new Promise((r) => setTimeout(r, 600));
      const o = await evalJs(cdp, OVERFLOW_EXPR);
      const d = await evalJs(cdp, `(() => {
        const btns = [...document.querySelectorAll('button[aria-label="حذف"]')];
        return { count: btns.length, bad: btns.filter((b) => {
          const r = b.getBoundingClientRect();
          return !(r.width > 0 && (r.x + r.width) <= window.innerWidth);
        }).length };
      })()`);
      if (!(o.tableSw <= o.tableCw)) throw new Error(JSON.stringify(o));
      if (d.count < 9 || d.bad > 0) throw new Error(JSON.stringify(d));
      return `dir=rtl, ${await shot(cdp, "catalog-ar-1280x800.png")}`;
    });

    const unmocked = cdp.consoleErrors.filter((m) => m.includes("UNMOCKED_COMMAND"));
    await step("no unmocked IPC commands", async () => {
      if (unmocked.length > 0) throw new Error(unmocked.slice(0, 3).join(" | "));
      return `console errors: ${cdp.consoleErrors.length}`;
    });
  } finally {
    if (cdp) cdp.close();
    chrome.kill();
  }
  fs.writeFileSync(path.join(HERE, "drill-results.json"), JSON.stringify({ at: new Date().toISOString(), results }, null, 2));
  const failed = results.filter((r) => !r.ok);
  console.log(`\nDSN-076 drill: ${results.length - failed.length}/${results.length} steps passed.`);
  if (failed.length > 0) process.exit(1);
}

main().catch((err) => { console.error("FATAL", err); process.exit(2); });
