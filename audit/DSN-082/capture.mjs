/**
 * TASK-DSN-082 visual verification: typed-gate delete modal + blocked-depot guidance.
 * Headless Chrome over CDP against vite :5173, Tauri IPC stubbed in-page
 * (built-ins only). Flows: login (ADMIN) → Catalogue → row delete → fill
 * reason + typed code → assert confirm enabled → shot; Stock & Dépôts →
 * transfer modal → manage tab → assert DEP-01 lock + no delete affordance → shot.
 */
import { spawn } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const HERE = path.dirname(fileURLToPath(import.meta.url));
const APP_URL = "http://localhost:5173/";
const CHROME = "C:/Program Files/Google/Chrome/Application/chrome.exe";
const RESULTS = [];
const CONSOLE_ERRORS = [];

const PRODUCT = {
  product_id: "p-vis", sku: "VIS-8X40", barcode: "6111250000013",
  name_ar: "برغي", name_fr: "Vis bois 8x40", category_id: null,
  is_kit: false, is_active: true,
  units: [{
    unit_id: "u-vis", product_id: "p-vis", unit_name_ar: "قطعة",
    unit_name_fr: "PCS", factor_micro: "10000", barcode: "6111250000013",
    sale_price_centimes: "1500", purchase_price_centimes: "1200",
    wholesale_price_centimes: null, is_base_unit: true, is_default_selling: true,
  }],
};

const STUB_SOURCE = `
window.__TAURI_INTERNALS__ = {
  invoke: function (cmd, args) {
    var req = (args && args.request) || {};
    var respond = function (v) { return Promise.resolve(v); };
    var fail = function (code) { return Promise.reject(code); };
    if (cmd === 'check_first_run_status') return respond({ requires_setup: false });
    if (cmd === 'auth_login') return respond({
      session_token: 'stub-admin', user_id: 'u-admin', username: 'admin',
      role: 'ADMIN', permissions: ['POS_SELL', 'CREATE_SALE', 'VIEW_STOCK', 'STOCK_ADJUSTMENT', 'SYSTEM_SETTINGS_MANAGE', 'MANAGE_WAREHOUSES', 'TRANSFER_STOCK'],
      expires_at: '2030-01-01T00:00:00',
    });
    if (cmd === 'auth_logout') return respond(null);
    if (cmd === 'warehouses_list') return respond({ warehouses: [
      { id: 'dep-01', code: 'DEP-01', name: 'Dépôt Central', is_active: true },
      { id: 'aux-01', code: 'AUX-01', name: 'Annexe', is_active: true },
    ] });
    if (cmd === 'category_list') return respond([]);
    if (cmd === 'store_identity_get') return respond({ identity: { store_name: 'Q-Test' }, device_kind: 'physical' });
    if (cmd === 'cash_get_active_session') return respond(null);
    if (cmd === 'catalog_metrics' || cmd === 'catalog_get_metrics') return respond({ total_products: 1, in_stock: 1, out_of_stock: 0, inactive: 0 });
    if (cmd === 'catalog_search') return respond([${JSON.stringify(PRODUCT)}]);
    if (cmd === 'stock_get') return respond({ product_id: 'p-vis', warehouse_id: 'dep-01', base_unit_id: 'u-vis', quantity_base_micro: '500000' });
    if (cmd === 'documents_search') return respond({ documents: [] });
    if (cmd === 'reports_low_stock') return respond([]);
    if (cmd === 'reports_stock_movement_history') return respond([]);
    window.__UNMOCKED__ = (window.__UNMOCKED__ || '') + cmd + ';';
    return fail('UNMOCKED_COMMAND:' + cmd);
  },
  transformCallback: function () { return 0; }
};
window.__CALLS__ = {};
(function () {
  var orig = window.__TAURI_INTERNALS__.invoke;
  window.__TAURI_INTERNALS__.invoke = function (cmd, args) {
    window.__CALLS__[cmd] = (window.__CALLS__[cmd] || 0) + 1;
    return orig(cmd, args);
  };
})();
`;

class Cdp {
  constructor(url) { this.url = url; this.id = 0; this.pending = new Map(); }
  connect() {
    return new Promise((resolve, reject) => {
      this.ws = new WebSocket(this.url);
      this.ws.addEventListener("open", resolve);
      this.ws.addEventListener("error", reject);
      this.ws.addEventListener("message", (ev) => {
        const msg = JSON.parse(String(ev.data));
        if (msg.method === "Console.messageAdded" && msg.params.message.level === "error") {
          CONSOLE_ERRORS.push(String(msg.params.message.text).slice(0, 200));
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

async function evalJs(cdp, expression) {
  const res = await cdp.send("Runtime.evaluate", { expression, returnByValue: true });
  if (res.exceptionDetails) throw new Error("JS: " + JSON.stringify(res.exceptionDetails).slice(0, 200));
  return res.result.value;
}
async function waitFor(cdp, expression, timeoutMs, label) {
  const t0 = Date.now();
  for (;;) {
    const ok = await evalJs(cdp, expression);
    if (ok) return;
    if (Date.now() - t0 > timeoutMs) throw new Error("timeout: " + label);
    await new Promise((r) => setTimeout(r, 250));
  }
}
function clickTextExpr(tag, text) {
  return `(() => {
    const els = [...document.querySelectorAll(${JSON.stringify(tag)})].filter((e) => (e.innerText ?? '').trim() === ${JSON.stringify(text)});
    if (els.length === 0) return 'MISSING:' + ${JSON.stringify(text)};
    els[0].click();
    return 'ok:' + els.length;
  })()`;
}
function fillExpr(sel, value) {
  return `(() => {
    const el = document.querySelector(${JSON.stringify(sel)});
    if (!el) return 'MISSING:' + ${JSON.stringify(sel)};
    const proto = Object.getPrototypeOf(el);
    const setter = Object.getOwnPropertyDescriptor(proto, 'value')?.set
      ?? Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value')?.set;
    setter.call(el, ${JSON.stringify(value)});
    el.dispatchEvent(new Event('input', { bubbles: true }));
    el.dispatchEvent(new Event('change', { bubbles: true }));
    return 'ok';
  })()`;
}
async function step(name, fn) {
  try {
    const detail = await fn();
    RESULTS.push({ name, ok: true, detail: String(detail ?? "") });
    console.log(`  PASS ${name} :: ${String(detail ?? "").slice(0, 160)}`);
  } catch (err) {
    RESULTS.push({ name, ok: false, detail: String(err?.message ?? err).slice(0, 300) });
    console.log(`  FAIL ${name} :: ${String(err?.message ?? err).slice(0, 200)}`);
  }
}

async function main() {
  let cdp = null;
  const chrome = spawn(CHROME, [
    "--headless=new", "--disable-gpu", "--no-sandbox",
    "--remote-debugging-port=0", "--window-size=1280,800", "about:blank",
  ], { stdio: ["ignore", "pipe", "pipe"] });
  let dbgUrl = "";
  await new Promise((resolve, reject) => {
    const t = setTimeout(() => reject(new Error("no devtools url")), 15000);
    chrome.stderr.on("data", (d) => {
      const m = String(d).match(/ws:\/\/[^\s]+/);
      if (m) { clearTimeout(t); dbgUrl = m[0]; resolve(); }
    });
    chrome.on("error", reject);
  });
  const tabs = await (await fetch("http://127.0.0.1:" + dbgUrl.match(/:(\d+)\//)[1] + "/json/list")).json();
  const page = tabs.find((t) => t.type === "page");
  cdp = new Cdp(page.webSocketDebuggerUrl);
  await cdp.connect();
  await cdp.send("Console.enable", {});
  await cdp.send("Page.enable", {});
  await cdp.send("Runtime.enable", {});

  let currentAr = false;
  async function viewport(w, h) {
    await cdp.send("Emulation.setDeviceMetricsOverride", {
      width: w, height: h, deviceScaleFactor: 1, mobile: false,
    });
    await cdp.send("Page.navigate", { url: APP_URL });
    currentAr = false;
    await waitFor(cdp, `!!document.querySelector('button[type="submit"]')`, 30000, "login submit");
    await evalJs(cdp, STUB_SOURCE);
    await evalJs(cdp, fillExpr('input[autocomplete="username"]', "admin"));
    await evalJs(cdp, fillExpr('input[autocomplete="current-password"]', "pw"));
    const label = await evalJs(cdp, `document.querySelector('button[type="submit"]').innerText.trim()`);
    await evalJs(cdp, clickTextExpr("button", label));
    await waitFor(cdp, `!!document.querySelector('nav')`, 15000, "nav after login");
  }
  async function setDark(on) {
    const isDark = await evalJs(cdp, `document.documentElement.classList.contains('dark')`);
    if (isDark !== on) {
      await evalJs(cdp, `document.querySelector('button[aria-label="Thème"]').click()`);
      await waitFor(cdp, `document.documentElement.classList.contains('dark') === ${on}`, 8000, "theme");
      await new Promise((r) => setTimeout(r, 400));
    }
  }
  async function setArabic(on) {
    if (currentAr !== on) {
      const btn = await evalJs(cdp, `(() => {
        const all = [...document.querySelectorAll('button')];
        const lit = ${on ? "'العربية'" : "'Français'"};
        const b = all.find((x) => (x.innerText ?? '').includes(lit));
        if (!b) return 'MISSING-LOCALE-BTN';
        b.click(); return 'ok';
      })()`);
      if (btn !== "ok") throw new Error(btn);
      currentAr = on;
      await waitFor(cdp, `(!!document.querySelector('[dir="rtl"]')) === ${on}`, 8000, "locale");
      await new Promise((r) => setTimeout(r, 400));
    }
  }
  async function shot(name, assertion) {
    const m = await evalJs(cdp, assertion);
    if (!m.ok) throw new Error(m.why);
    const de = m.de;
    if (!(de.sw <= de.w && de.sh <= de.h)) throw new Error(`page scroll sw=${de.sw} sh=${de.sh}`);
    const res = await cdp.send("Page.captureScreenshot", { format: "png" });
    const out = path.join(HERE, name);
    fs.writeFileSync(out, Buffer.from(res.data, "base64"));
    console.log(`  SHOT ${name} (${fs.statSync(out).size} bytes)`);
    return m.note;
  }

  // --- Capture 1: typed-gate delete modal, fr/light, 1280x800 ---
  await viewport(1280, 800);
  await evalJs(cdp, clickTextExpr("nav button", "Catalogue"));
  await waitFor(cdp, `!!document.querySelector('button[aria-label="Supprimer"]')`, 20000, "row delete btn");
  await evalJs(cdp, `document.querySelector('button[aria-label="Supprimer"]').click()`);
  await waitFor(cdp, `!!document.querySelector('#safe-delete-reason')`, 8000, "reason textarea");
  await evalJs(cdp, fillExpr("#safe-delete-reason", "doublon de catalogue"));
  await evalJs(cdp, fillExpr("#safe-delete-typed", "VIS-8X40"));
  await new Promise((r) => setTimeout(r, 500));
  await step("typed-gate: counter + typed match enables confirm", async () => {
    const s = await evalJs(cdp, `(() => {
      const dlg = document.querySelector('[role="dialog"]');
      const txt = dlg ? dlg.innerText : '';
      const confirm = [...(dlg ? dlg.querySelectorAll('button') : [])].find((b) => (b.innerText ?? '').includes('Confirmer'));
      return { hasReason: txt.includes('Motif de suppression'), hasCounter: txt.includes('/ 3'),
        hasTyped: txt.includes('Tapez le code'), confirmEnabled: confirm ? !confirm.disabled : null, txt: txt.slice(-220) };
    })()`);
    if (!s.hasReason || !s.hasCounter || !s.hasTyped) throw new Error("gate UI missing " + JSON.stringify(s));
    if (s.confirmEnabled !== true) throw new Error("confirm not enabled after valid reason+token");
    return `confirm enabled; tail="${s.txt.slice(-80)}"`;
  });
  await step("shot del-typed-1280x800-fr-light", async () => shot("del-typed-1280x800-fr-light.png",
    `(() => {
      const dlg = document.querySelector('[role="dialog"]');
      const de = document.documentElement;
      const txt = dlg ? dlg.innerText : '';
      const ok = txt.includes('Motif de suppression') && txt.includes('Tapez le code') && txt.includes('VIS-8X40');
      return { ok, why: ok ? '' : 'gate text missing', de: { sw: de.scrollWidth, sh: de.scrollHeight, w: window.innerWidth, h: window.innerHeight }, note: 'typed gate fr/light' };
    })()`));

  // --- Capture 2: blocked depot, ar/dark, 1280x800 ---
  await viewport(1280, 800);
  await setDark(true);
  await setArabic(true);
  await evalJs(cdp, clickTextExpr("nav button", "المخزون والمستودعات"));
  await waitFor(cdp, `[...document.querySelectorAll('button')].some((b) => (b.innerText ?? '').includes('تحويل بين المستودعات'))`, 20000, "transfer btn");
  await evalJs(cdp, clickTextExpr("button", "تحويل بين المستودعات"));
  await waitFor(cdp, `[...document.querySelectorAll('button')].some((b) => (b.innerText ?? '').includes('المستودعات'))`, 10000, "manage tab");
  await evalJs(cdp, clickTextExpr("button", "المستودعات"));
  await waitFor(cdp, `document.body.innerText.includes('مقفل (DEP-01)')`, 10000, "lock badge");
  await step("shot del-blocked-1280x800-ar-dark", async () => shot("del-blocked-1280x800-ar-dark.png",
    `(() => {
      const body = document.body.innerText;
      const hasLock = body.includes('مقفل (DEP-01)');
      const depDel = !!document.querySelector('[aria-label*="DEP-01"]');
      const auxDel = !!document.querySelector('[aria-label*="AUX-01"]');
      const ok = hasLock && !depDel && auxDel;
      const de = document.documentElement;
      return { ok, why: ok ? '' : ('lock=' + hasLock + ' depDel=' + depDel + ' auxDel=' + auxDel), de: { sw: de.scrollWidth, sh: de.scrollHeight, w: window.innerWidth, h: window.innerHeight }, note: 'blocked depot ar/dark' };
    })()`));

  // --- Capture 3: typed-gate, fr/light, 1024x600 ---
  await viewport(1024, 600);
  await evalJs(cdp, clickTextExpr("nav button", "Catalogue"));
  await waitFor(cdp, `!!document.querySelector('button[aria-label="Supprimer"]')`, 20000, "row delete btn");
  await evalJs(cdp, `document.querySelector('button[aria-label="Supprimer"]').click()`);
  await waitFor(cdp, `!!document.querySelector('#safe-delete-reason')`, 8000, "reason textarea");
  await step("shot del-typed-1024x600-fr-light", async () => shot("del-typed-1024x600-fr-light.png",
    `(() => {
      const dlg = document.querySelector('[role="dialog"]');
      const txt = dlg ? dlg.innerText : '';
      const ok = txt.includes('Motif de suppression') && txt.includes('/ 3');
      const de = document.documentElement;
      return { ok, why: ok ? '' : 'modal missing', de: { sw: de.scrollWidth, sh: de.scrollHeight, w: window.innerWidth, h: window.innerHeight }, note: 'typed gate 1024 fr/light' };
    })()`));

  await step("zero console errors across captures", async () => {
    if (CONSOLE_ERRORS.length > 0) throw new Error(CONSOLE_ERRORS.slice(0, 3).join(" || "));
    return "0 console errors";
  });

  fs.writeFileSync(path.join(HERE, "capture-results.json"), JSON.stringify({ results: RESULTS, errors: CONSOLE_ERRORS }, null, 2));
  await cdp.close();
  chrome.kill();
  const failed = RESULTS.filter((r) => !r.ok);
  console.log(`DONE fails=${failed.length}`);
  if (failed.length > 0) process.exit(1);
}

main().catch((err) => { console.error("FATAL", err); process.exit(2); });
