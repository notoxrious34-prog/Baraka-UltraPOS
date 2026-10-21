/**
 * TASK-DSN-084 visual verification: document detail drawer full view.
 * Headless Chrome over CDP against vite :5173, Tauri IPC stubbed in-page
 * (built-ins only). Flows: login (ADMIN) → Documents → click row → drawer
 * shows fetched lines/payments/totals → shot. 3 captures: TICKET fr/light,
 * FACTURE ar/dark, error state (sales_get rejects).
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

const TICKET_SUMMARY = {
  id: "doc-ticket-1", document_number: "T-2026-0007", document_type: "TICKET",
  status: "FINALIZED", party_id: null, party_name: null,
  total_ttc_centimes: "3025", items_count: 1, created_at: "2026-10-21T09:12:00",
};
const FACTURE_SUMMARY = {
  id: "doc-fact-1", document_number: "F-2026-0042", document_type: "FACTURE",
  status: "FINALIZED", party_id: "cust-1", party_name: "SARL Atlas",
  total_ttc_centimes: "119000", items_count: 1, created_at: "2026-10-21T10:02:00",
};
const TICKET_DETAILS = {
  document: {
    document_id: "doc-ticket-1", number: "T-2026-0007", doc_type: "TICKET",
    status: "FINALIZED", warehouse_id: "dep-01", subtotal_ht_centimes: "2542",
    tva_total_centimes: "483", total_ttc_centimes: "3025",
    paid_amount_centimes: "5000", change_amount_centimes: "1975", payment_status: "PAID",
  },
  items: [{
    product_id: "prod-vis", unit_id: "unit-pcs", quantity_micro: "20000",
    unit_price_centimes: "1271", line_tva_centimes: "483", line_ttc_centimes: "3025",
  }],
  payments: [{ method_code: "CASH", amount_centimes: "5000" }],
  print_status: "DONE",
};
const FACTURE_DETAILS = {
  document: {
    document_id: "doc-fact-1", number: "F-2026-0042", doc_type: "FACTURE",
    status: "FINALIZED", warehouse_id: "dep-01", subtotal_ht_centimes: "100000",
    tva_total_centimes: "19000", total_ttc_centimes: "119000",
    paid_amount_centimes: "119000", change_amount_centimes: "0", payment_status: "PAID",
  },
  items: [{
    product_id: "prod-ciment", unit_id: "unit-sac", quantity_micro: "40000",
    unit_price_centimes: "25000", line_tva_centimes: "19000", line_ttc_centimes: "119000",
  }],
  payments: [{ method_code: "VIREMENT", amount_centimes: "119000" }],
  print_status: "NONE",
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
      role: 'ADMIN', permissions: ['POS_SELL', 'CREATE_SALE', 'VIEW_STOCK', 'STOCK_ADJUSTMENT', 'SYSTEM_SETTINGS_MANAGE', 'MANAGE_WAREHOUSES'],
      expires_at: '2030-01-01T00:00:00',
    });
    if (cmd === 'auth_logout') return respond(null);
    if (cmd === 'warehouses_list') return respond({ warehouses: [{ id: 'dep-01', code: 'DEP-01', name: 'Dépôt Central', is_active: true }] });
    if (cmd === 'category_list') return respond([]);
    if (cmd === 'store_identity_get') return respond({ identity: { store_name: 'Q-Test' }, device_kind: 'physical' });
    if (cmd === 'cash_get_active_session') return respond({ id: 's-1', warehouse_id: 'dep-01', status: 'OPEN', opened_by: 'u-admin', opened_by_username: 'admin', opened_at: '2026-10-21T08:00:00', opening_float_centimes: '0', created_at: '2026-10-21T08:00:00', updated_at: '2026-10-21T08:00:00' });
    if (cmd === 'documents_search') {
      if (window.__DOCS_MODE__ === 'error') return respond({ documents: [${JSON.stringify(TICKET_SUMMARY)}] });
      return respond({ documents: [${JSON.stringify(TICKET_SUMMARY)}, ${JSON.stringify(FACTURE_SUMMARY)}] });
    }
    if (cmd === 'sales_get') {
      const docId = args.document_id ?? ((args && args.request) || {}).document_id;
      if (window.__DOCS_MODE__ === 'error') return fail('SIMULATED_ENGINE_OUTAGE');
      if (docId === 'doc-ticket-1') return respond(${JSON.stringify(TICKET_DETAILS)});
      if (docId === 'doc-fact-1') return respond(${JSON.stringify(FACTURE_DETAILS)});
      return fail('NOT_FOUND');
    }
    if (cmd === 'documents_get_print_status') return respond({ jobId: '', status: 'NONE', attemptCount: '0' });
    window.__UNMOCKED__ = (window.__UNMOCKED__ || '') + cmd + ';';
    return fail('UNMOCKED_COMMAND:' + cmd);
  },
  transformCallback: function () { return 0; }
};
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
  async function viewport(w, h, dark = false, docsMode = null) {
    await cdp.send("Emulation.setDeviceMetricsOverride", {
      width: w, height: h, deviceScaleFactor: 1, mobile: false,
    });
    await cdp.send("Page.navigate", { url: APP_URL });
    await waitFor(cdp, `!!document.querySelector('button[type="submit"]')`, 30000, "login submit");
    if (dark) {
      await evalJs(cdp, `localStorage.setItem('baraka-theme', 'dark')`);
      await cdp.send("Page.navigate", { url: APP_URL });
      await waitFor(cdp, `!!document.querySelector('button[type="submit"]')`, 30000, "login after theme seed");
    }
    currentAr = false;
    if (docsMode) await evalJs(cdp, `window.__DOCS_MODE__ = ${JSON.stringify(docsMode)}`);
  }
  async function loginAsAdmin() {
    await waitFor(cdp, `!!document.querySelector('button[type="submit"]')`, 30000, "login submit");
    await evalJs(cdp, STUB_SOURCE);
    await evalJs(cdp, fillExpr('input[autocomplete="username"]', "admin"));
    await evalJs(cdp, fillExpr('input[autocomplete="current-password"]', "pw"));
    const label = await evalJs(cdp, `document.querySelector('button[type="submit"]').innerText.trim()`);
    await evalJs(cdp, clickTextExpr("button", label));
    await waitFor(cdp, `!!document.querySelector('nav')`, 15000, "nav after login");
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
      await new Promise((r) => setTimeout(r, 600));
    }
  }
  async function openDrawer(navLabel, rowNumber, detailsLabel) {
    const navs = await evalJs(cdp, `[...document.querySelectorAll('nav button')].map((b) => (b.innerText ?? '').trim()).join(' | ')`);
    console.log(`  NAVS: ${navs.slice(0, 300)}`);
    await evalJs(cdp, clickTextExpr("nav button", navLabel));
    await new Promise((r) => setTimeout(r, 2000));
    const body = await evalJs(cdp, `document.body.innerText.slice(0, 400)`);
    console.log(`  BODY: ${body}`);
    console.log(`  UNMOCKED: ${await evalJs(cdp, `window.__UNMOCKED__ ?? '(none)'`)}`);
    await waitFor(cdp, `document.body.innerText.includes(${JSON.stringify(rowNumber)})`, 20000, "row " + rowNumber);
    const clicked = await evalJs(cdp, `(() => {
      const rows = [...document.querySelectorAll('tbody tr')];
      const row = rows.find((r) => (r.innerText ?? '').includes(${JSON.stringify(rowNumber)}));
      if (!row) return 'MISSING-ROW';
      const btn = [...row.querySelectorAll('button')].find((b) => (b.innerText ?? '').trim() === ${JSON.stringify(detailsLabel)});
      if (!btn) return 'MISSING-DETAILS-BTN';
      btn.click(); return 'ok';
    })()`);
    if (clicked !== "ok") throw new Error(clicked);
  }
  async function shot(name, waitExpr, note) {
    await waitFor(cdp, waitExpr, 15000, "drawer content " + name);
    await new Promise((r) => setTimeout(r, 800));
    const m = await evalJs(cdp, `(() => {
      const de = document.documentElement;
      return { sw: de.scrollWidth, sh: de.scrollHeight, w: window.innerWidth, h: window.innerHeight };
    })()`);
    if (!(m.sw <= m.w && m.sh <= m.h)) throw new Error(`page scroll sw=${m.sw} sh=${m.sh}`);
    const res = await cdp.send("Page.captureScreenshot", { format: "png" });
    const out = path.join(HERE, name);
    fs.writeFileSync(out, Buffer.from(res.data, "base64"));
    console.log(`  SHOT ${name} (${fs.statSync(out).size} bytes)`);
    return note;
  }

  // --- Capture 1: TICKET fr/light 1280x800 ---
  await viewport(1280, 800);
  await loginAsAdmin();
  await openDrawer("Documents & Factures", "T-2026-0007", "Détails");
  await step("shot doc-ticket-1280x800-fr-light", async () =>
    shot("doc-ticket-1280x800-fr-light.png", `!!document.querySelector('[data-testid="details-lines"]')`, "TICKET fr/light, lines rendered"));

  // --- Capture 2: FACTURE ar/dark 1280x800 ---
  await viewport(1280, 800, true);
  await setArabic(true);
  await loginAsAdmin();
  await openDrawer("الوثائق والفواتير", "F-2026-0042", "التفاصيل");
  await step("shot doc-facture-1280x800-ar-dark", async () =>
    shot("doc-facture-1280x800-ar-dark.png", `!!document.querySelector('[data-testid="details-lines"]')`, "FACTURE ar/dark, lines rendered"));

  // --- Capture 3: error state fr/light 1024x600 ---
  await viewport(1024, 600, false, "error");
  await loginAsAdmin();
  await openDrawer("Documents & Factures", "T-2026-0007", "Détails");
  await step("shot doc-error-1024x600-fr-light", async () =>
    shot("doc-error-1024x600-fr-light.png", `document.body.innerText.includes("indisponibles")`, "error state + retry"));

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
