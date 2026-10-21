/**
 * TASK-BRK-079 visual verification: post-checkout failed-print banner + chip.
 * Headless Chrome over CDP against vite :5173, Tauri IPC stubbed in-page
 * (built-ins only). Flow per capture: login → Caisse → search "vis" →
 * click product card → F12 checkout → cash tender → Valider → done screen
 * with stubbed FAILED print status → assert banner + chip + reprint button →
 * screenshot. 3 captures: 1280x800 fr/light + ar/dark, 1024x600 fr/light.
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
      session_token: 'stub-pos', user_id: 'u-pos', username: 'caissier',
      role: 'CAISSIER', permissions: ['POS_SELL', 'CREATE_SALE'],
      expires_at: '2030-01-01T00:00:00',
    });
    if (cmd === 'auth_logout') return respond(null);
    if (cmd === 'warehouses_list') return respond({ warehouses: [{ id: 'dep-01', code: 'DEP-01', name: 'Dépôt Central', is_active: true }] });
    if (cmd === 'category_list') return respond([]);
    if (cmd === 'store_identity_get') return respond({ identity: { store_name: 'Q-Test' }, device_kind: 'physical' });
    if (cmd === 'cash_get_active_session') return respond({ id: 's-1', warehouse_id: 'dep-01', terminal_id: 'T1', status: 'OPEN', opened_by: 'u-pos', opened_by_username: 'caissier', opened_at: '2026-10-21T08:00:00', opening_float_centimes: '0', closed_by: null, closed_by_username: null, closed_at: null, actual_counted_centimes: null, system_expected_centimes: null, variance_centimes: null, closing_notes: null, revision: '1', created_at: '2026-10-21T08:00:00', updated_at: '2026-10-21T08:00:00' });
    if (cmd === 'catalog_metrics' || cmd === 'catalog_get_metrics') return respond({ total_products: 1, in_stock: 1, out_of_stock: 0, inactive: 0 });
    if (cmd === 'catalog_search') return respond([${JSON.stringify(PRODUCT)}]);
    if (cmd === 'stock_get') return respond({ product_id: 'p-vis', warehouse_id: 'dep-01', base_unit_id: 'u-vis', quantity_base_micro: '500000' });
    if (cmd === 'sales_checkout') return respond({
      document_id: 'doc-1', document_number: 'TIC-2026-0455',
      subtotal_ht_centimes: '1500', tva_total_centimes: '285', discount_total_centimes: '0',
      timbre_centimes: '15', total_ttc_centimes: '1800',
      paid_amount_centimes: '2000', change_amount_centimes: '200', print_job_id: 'job-1',
    });
    if (cmd === 'documents_get_print_status') return respond({ job_id: 'job-1', status: 'FAILED', attempt_count: '3', last_error: 'EXHAUSTED_MAX_RETRIES: PRINTER_OFFLINE' });
    if (cmd === 'documents_reprint') return respond({ ok: true });
    if (cmd === 'sales_preview_totals') return respond({
      subtotal_ht_centimes: '1500', tva_total_centimes: '285', global_discount_centimes: '0',
      timbre_centimes: '15', total_ttc_centimes: '1800', paid_amount_centimes: '2000',
      change_amount_centimes: '200', remaining_due_centimes: '0', stock_warnings: [], line_margins: [],
    });
    if (cmd === 'documents_search') return respond({ documents: [] });
    if (cmd === 'cash_get_session_report') return respond({ session: { id: 's-1', warehouse_id: 'dep-01', terminal_id: 'T1', status: 'OPEN', opened_by: 'u-pos', opened_by_username: 'caissier', opened_at: '2026-10-21T08:00:00', opening_float_centimes: '0', closed_by: null, closed_by_username: null, closed_at: null, actual_counted_centimes: null, system_expected_centimes: null, variance_centimes: null, closing_notes: null, revision: '1', created_at: '2026-10-21T08:00:00', updated_at: '2026-10-21T08:00:00' }, movements: [], totals_by_type: [], is_final: false });
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

async function evalJs(cdp, expression, awaitPromise = false) {
  const res = await cdp.send("Runtime.evaluate", { expression, awaitPromise, returnByValue: true });
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
    await evalJs(cdp, fillExpr('input[autocomplete="username"]', "caissier"));
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
  async function toFailedCheckout(posLabel, arabic) {
    const themeLbl = arabic ? "المظهر" : "Thème";
    const prodName = arabic ? "برغي" : "Vis bois";
    await evalJs(cdp, clickTextExpr("nav button", posLabel));
    await waitFor(cdp, `!!document.querySelector('button[aria-label="${themeLbl}"]')`, 15000, "pos topbar");
    // Search the stubbed product, then click its card to add to cart.
    const searchSel = await evalJs(cdp, `(() => {
      const inp = document.querySelector('input[placeholder]');
      return inp ? 'found' : 'MISSING-SEARCH';
    })()`);
    if (searchSel !== "found") throw new Error(searchSel);
    await evalJs(cdp, fillExpr("input[placeholder]", "vis"));
    await waitFor(cdp, `[...document.querySelectorAll('span')].some((e) => (e.innerText ?? '').includes('${prodName}'))`, 15000, "product card");
    await evalJs(cdp, `(() => {
      const span = [...document.querySelectorAll('span')].find((e) => (e.innerText ?? '').includes('${prodName}'));
      if (!span) return 'MISSING-CARD';
      span.click(); return 'ok';
    })()`);
    // Open checkout with F12, tender cash, confirm.
    await new Promise((r) => setTimeout(r, 600));
    await cdp.send("Input.dispatchKeyEvent", { type: "keyDown", key: "F12", code: "F12", windowsVirtualKeyCode: 123 });
    await cdp.send("Input.dispatchKeyEvent", { type: "keyUp", key: "F12", code: "F12", windowsVirtualKeyCode: 123 });
    await waitFor(cdp, `[...document.querySelectorAll('button')].some((b) => (b.innerText ?? '').includes('Valider la vente') || (b.innerText ?? '').includes('تأكيد'))`, 15000, "checkout modal");
  }
  async function capture(name, w, h, arabic, dark, posLabel, confirmLabel, tenderSel) {
    await viewport(w, h);
    await evalJs(cdp, clickTextExpr("nav button", "Caisse"));
    await waitFor(cdp, `!!document.querySelector('button[aria-label="Thème"]')`, 15000, "pos topbar");
    await setDark(dark);
    await setArabic(arabic);
    await toFailedCheckout(posLabel, arabic);
    // Tender enough cash, then confirm the sale.
    const tender = await evalJs(cdp, `(() => {
      const inputs = [...document.querySelectorAll('input')].filter((i) => i.type !== 'hidden');
      return inputs.map((i) => i.placeholder || i.name || i.type).join('|');
    })()`);
    await step(`${name}: tender inputs`, async () => tender);
    // Tender enough cash (first decimal input in the modal), then confirm.
    await evalJs(cdp, `(() => {
      const inp = document.querySelector('input[inputmode="decimal"]');
      if (!inp) return 'MISSING-TENDER';
      const proto = Object.getPrototypeOf(inp);
      const setter = Object.getOwnPropertyDescriptor(proto, 'value')?.set
        ?? Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value')?.set;
      setter.call(inp, '2000');
      inp.dispatchEvent(new Event('input', { bubbles: true }));
      inp.dispatchEvent(new Event('change', { bubbles: true }));
      return 'ok';
    })()`);
    await evalJs(cdp, clickTextExpr("button", confirmLabel));
    await new Promise((r) => setTimeout(r, 3000));
    const diag = await evalJs(cdp, `(() => {
      const calls = JSON.stringify(window.__CALLS__ || {});
      const unm = window.__UNMOCKED__ || 'none';
      const modalText = (document.querySelector('[role="dialog"]') || document.body).innerText.slice(-500);
      return 'CALLS=' + calls + ' UNMOCKED=' + unm + ' MODAL=' + modalText;
    })()`);
    fs.writeFileSync(path.join(HERE, "diag.txt"), diag);
    await step(`${name}: post-confirm diag`, async () => diag);
    await waitFor(cdp, `[...document.querySelectorAll('p')].some((p) => (p.getAttribute('role') === 'alert'))`, 15000, "failed banner");
    await new Promise((r) => setTimeout(r, 800));
    await step(`${name}: failed banner + chip + reprint`, async () => {
      const m = await evalJs(cdp, `(() => {
        const de = document.documentElement;
        const banner = document.querySelector('p[role="alert"]');
        const chip = [...document.querySelectorAll('span')].some((s) => (s.innerText ?? '').includes(${arabic ? "'فشلت الطباعة'" : "'Échec d\\'impression'"}));
        const reprint = [...document.querySelectorAll('button')].some((b) => (b.innerText ?? '').includes(${arabic ? "'إعادة الطباعة'" : "'Réimprimer'"}));
        const live = !!document.querySelector('[aria-live="polite"]');
        return {
          banner: banner ? banner.innerText.slice(0, 80) : null,
          chip, reprint, live,
          sw: de.scrollWidth, sh: de.scrollHeight, w: window.innerWidth, h: window.innerHeight,
          noPageScroll: de.scrollWidth <= window.innerWidth && de.scrollHeight <= window.innerHeight,
        };
      })()`);
      if (!m.banner) throw new Error("guidance banner missing");
      if (!m.chip) throw new Error("FAILED chip missing");
      if (!m.reprint) throw new Error("reprint button missing");
      if (!m.live) throw new Error("aria-live missing");
      if (!m.noPageScroll) throw new Error(`page scroll sw=${m.sw} sh=${m.sh}`);
      return `banner="${m.banner}" chip=true reprint=true live=true noPageScroll=true`;
    });
    const res = await cdp.send("Page.captureScreenshot", { format: "png" });
    const out = path.join(HERE, name);
    fs.writeFileSync(out, Buffer.from(res.data, "base64"));
    console.log(`  SHOT ${name} (${fs.statSync(out).size} bytes)`);
  }

  await capture("pos-failed-1280x800-fr-light.png", 1280, 800, false, false, "Caisse", "Valider la vente (F12)");
  await capture("pos-failed-1280x800-ar-dark.png", 1280, 800, true, true, "الصندوق", "تأكيد البيع (F12)");
  await capture("pos-failed-1024x600-fr-light.png", 1024, 600, false, false, "Caisse", "Valider la vente (F12)");

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
