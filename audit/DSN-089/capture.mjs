/**
 * TASK-DSN-089 visual verification: drawer product designation.
 * Headless Chrome over CDP against vite :5173, Tauri IPC stubbed in-page
 * (built-ins only). Pattern: audit/DSN-081/capture.mjs.
 *
 * Flow: ADMIN login -> Documents & Factures -> click first row ->
 * details drawer (sales_get stubbed 3-line ticket: 2 named lines + 1
 * orphan) -> screenshot fr-light; switch ar-DZ + dark -> screenshot.
 * 1280x800, no page scroll, names visible, badge visible, zero UUID text
 * in the drawer DOM, zero console errors.
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

const TICKET_ROW = {
  id: "doc-drawer-1",
  document_number: "T-2026-0007",
  document_type: "TICKET",
  status: "FINALIZED",
  party_id: null,
  party_name: null,
  total_ttc_centimes: "3025",
  items_count: 3,
  created_at: "2026-10-24T10:00:00",
};

const DETAILS = {
  document: {
    document_id: "doc-drawer-1",
    number: "T-2026-0007",
    doc_type: "TICKET",
    status: "FINALIZED",
    warehouse_id: "dep-01",
    subtotal_ht_centimes: "2542",
    tva_total_centimes: "483",
    total_ttc_centimes: "3025",
    paid_amount_centimes: "5000",
    change_amount_centimes: "1975",
    payment_status: "PAID",
  },
  items: [
    {
      product_id: "3f4a1c2e-9b5d-4a1f-8c3e-7d2e6a5b4c1d",
      unit_id: "a1b2c3d4-e5f6-4a7b-8c9d-e0f1a2b3c4d5",
      quantity_micro: "20000",
      unit_price_centimes: "1271",
      line_tva_centimes: "483",
      line_ttc_centimes: "3025",
      product_name_fr: "Vis TR 4x40",
      product_name_ar: "برغي 4×40",
      sku: "VIS-440",
      unit_name_fr: "Pièce",
      unit_name_ar: "قطعة",
      product_missing: false,
    },
    {
      product_id: "11111111-2222-4a3b-8c4d-555555555555",
      unit_id: "66666666-7777-4a3b-8c4d-999999999999",
      quantity_micro: "10000",
      unit_price_centimes: "1000",
      line_tva_centimes: "190",
      line_ttc_centimes: "1190",
      product_name_fr: "Café Arabica 250g",
      product_name_ar: "قهوة عربية 250غ",
      sku: "CAFE-250",
      unit_name_fr: "Paquet",
      unit_name_ar: "علبة",
      product_missing: false,
    },
    {
      product_id: "aaaaaaaa-bbbb-4c5d-8e6f-000000000000",
      unit_id: "cccccccc-dddd-4e5f-8a6b-111111111111",
      quantity_micro: "30000",
      unit_price_centimes: "800",
      line_tva_centimes: "0",
      line_ttc_centimes: "2400",
      product_name_fr: null,
      product_name_ar: null,
      sku: "SUCRE-1KG",
      unit_name_fr: null,
      unit_name_ar: null,
      product_missing: true,
    },
  ],
  payments: [{ method_code: "CASH", amount_centimes: "5000" }],
  print_status: "DONE",
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
      role: 'ADMIN', permissions: ['VIEW_STOCK', 'POS_SELL', 'CREATE_SALE', 'VIEW_CUSTOMERS', 'VIEW_DOCUMENTS', 'SYSTEM_SETTINGS_MANAGE', 'STOCK_ADJUSTMENT', 'ADJUST_STOCK', 'TRANSFER_STOCK', 'MANAGE_PRINTER_SETTINGS', 'MANAGE_WAREHOUSES'],
      expires_at: '2030-01-01T00:00:00',
    });
    if (cmd === 'auth_logout') return respond(null);
    if (cmd === 'warehouses_list') return respond({ warehouses: [{ id: 'dep-01', code: 'DEP-01', name: 'Dépôt Central', is_active: true }] });
    if (cmd === 'category_list') return respond([]);
    if (cmd === 'store_identity_get') return respond({ identity: { storeName: 'Q-Test', storeAddress: '', storePhone: '0555123456', nif: '123456789012345', nis: '', rc: '', ai: '', ticketFooterMessage: '' }, deviceKind: 'physical' });
    if (cmd === 'printer_get_settings') return respond({ serial_port: 'COM1', serial_baud: 9600, paper_width_dots: 640, paper_width_mm: 80 });
    if (cmd === 'cash_get_active_session') return respond(null);
    if (cmd === 'fiscal_settings_get') return respond({ timbreEnabled: true });
    if (cmd === 'backups_list') return respond([]);
    if (cmd === 'catalog_search') return respond([]);
    if (cmd === 'documents_search') return respond({ documents: [${JSON.stringify(TICKET_ROW)}] });
    if (cmd === 'sales_get') return respond(${JSON.stringify(DETAILS)});
    return fail('UNMOCKED_COMMAND:' + cmd);
  },
  transformCallback: function () { return 0 }
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
          if (msg.error) reject(new Error(JSON.stringify(msg.error)));
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
  if (res.exceptionDetails) {
    const desc = res.exceptionDetails?.exception?.description ?? res.exceptionDetails?.text;
    throw new Error("JS-DESC: " + String(desc).slice(0, 400));
  }
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
function clickIn(scopeExpr, text) {
  return `(() => {
    const scope = ${scopeExpr};
    if (!scope) return 'MISSING-SCOPE';
    const els = [...scope.querySelectorAll('button')].filter((e) => (e.innerText ?? '').trim() === ${JSON.stringify(text)});
    if (els.length === 0) return 'MISSING:' + ${JSON.stringify(text)};
    els[0].click();
    return 'ok:' + els.length;
  })()`;
}
async function step(name, fn) {
  try {
    const detail = await fn();
    RESULTS.push({ name, ok: true, detail: String(detail ?? "") });
    console.log(`  PASS ${name} :: ${String(detail ?? "").slice(0, 220)}`);
  } catch (err) {
    RESULTS.push({ name, ok: false, detail: String(err?.message ?? err).slice(0, 300) });
    console.log(`  FAIL ${name} :: ${String(err?.message ?? err).slice(0, 200)}`);
  }
}

const LAYOUT_EXPR = `(() => {
  const de = document.documentElement;
  return {
    noPageScroll: de.scrollWidth <= window.innerWidth && de.scrollHeight <= window.innerHeight,
  };
})()`;
const DRAWER_HTML_EXPR = `(() => {
  const el = document.querySelector('[data-testid="details-lines"]');
  return el ? el.closest('aside, [role="dialog"], div[class*="fixed"], section')?.outerHTML?.slice(0, 6000) ?? el.outerHTML.slice(0, 6000) : 'NO-DRAWER-LINES';
})()`;
const UUID_IN_DRAWER_EXPR = `(() => {
  const root = document.querySelector('[data-testid="details-lines"]')?.closest('aside') ?? document.body;
  const html = root.outerHTML;
  const m = html.match(/[0-9a-f]{8}-[0-9a-f]{4}-/);
  return m ? ('UUID-FOUND:' + m[0]) : 'UUID-CLEAN';
})()`;

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
  const page = tabs.filter((t) => t.type === "page").pop();
  cdp = new Cdp(page.webSocketDebuggerUrl);
  await cdp.connect();
  await cdp.send("Console.enable", {});
  await cdp.send("Page.enable", {});
  await cdp.send("Runtime.enable", {});

  await cdp.send("Emulation.setDeviceMetricsOverride", {
    width: 1280, height: 800, deviceScaleFactor: 1, mobile: false,
  });
  await cdp.send("Page.navigate", { url: APP_URL });
  await waitFor(cdp, `!!document.querySelector('button[type="submit"]')`, 60000, "login submit");
  await evalJs(cdp, STUB_SOURCE);
  // ADMIN login.
  await evalJs(cdp, `(function(){var el=document.querySelector('input[autocomplete="username"]');var p=Object.getPrototypeOf(el);Object.getOwnPropertyDescriptor(p,'value').set.call(el,'admin');el.dispatchEvent(new Event('input',{bubbles:true}));el.dispatchEvent(new Event('change',{bubbles:true}));return 1})()`);
  await evalJs(cdp, `(function(){var el=document.querySelector('input[autocomplete="current-password"]');var p=Object.getPrototypeOf(el);Object.getOwnPropertyDescriptor(p,'value').set.call(el,'pw');el.dispatchEvent(new Event('input',{bubbles:true}));el.dispatchEvent(new Event('change',{bubbles:true}));return 1})()`);
  await evalJs(cdp, `(function(){var f=document.querySelector('form');if(!f)return 'NOFORM';f.requestSubmit();return 'requested'})()`);
  await waitFor(cdp, `!!document.querySelector('nav')`, 15000, "nav after login");

  // Open Documents & Factures.
  await step("open-documents", async () => {
    const r = await evalJs(cdp, clickIn("document.querySelector('nav')", "Documents & Factures"));
    if (r !== "ok:1") throw new Error("nav entry: " + r);
    await waitFor(cdp, `!!document.querySelector('tbody tr')`, 15000, "doc rows");
    return r;
  });

  // Click first row -> drawer.
  await step("open-drawer", async () => {
    const r = await evalJs(cdp, `(() => {
      const row = document.querySelector('tbody tr');
      if (!row) return 'NO-ROWS';
      row.click();
      return 'clicked';
    })()`);
    await waitFor(cdp, `!!document.querySelector('[data-testid="details-lines"]')`, 15000, "drawer lines");
    return r;
  });

  // FR + light asserts + shot.
  await step("fr-light-asserts", async () => {
    const html = await evalJs(cdp, DRAWER_HTML_EXPR);
    const checks = [
      ["Vis TR 4x40", html.includes("Vis TR 4x40")],
      ["Café Arabica 250g", html.includes("Café Arabica 250g")],
      ["SUCRE-1KG", html.includes("SUCRE-1KG")],
      ["badge Produit archivé", html.includes("Produit archivé")],
    ];
    const bad = checks.filter(([, ok]) => !ok).map(([n]) => n);
    if (bad.length) throw new Error("missing in drawer: " + bad.join(",") + " :: " + html.slice(0, 800));
    const uuid = await evalJs(cdp, UUID_IN_DRAWER_EXPR);
    if (uuid !== "UUID-CLEAN") throw new Error(uuid);
    const layout = await evalJs(cdp, LAYOUT_EXPR);
    if (!layout.noPageScroll) throw new Error("page scroll at 1280x800");
    return "names+badge visible, " + uuid + ", noPageScroll";
  });
  await step("shot-fr-light", async () => {
    const shot = await cdp.send("Page.captureScreenshot", { format: "png" });
    fs.writeFileSync(path.join(HERE, "drawer-1280x800-fr-light.png"), Buffer.from(shot.data, "base64"));
    return "drawer-1280x800-fr-light.png";
  });

  // Switch ar-DZ + dark via the Caisse topbar (locale/theme toggles live
  // only there), then return and reopen the drawer.
  await step("switch-ar-dark", async () => {
    const caisse = await evalJs(cdp, clickIn("document.querySelector('nav')", "Caisse"));
    if (caisse !== "ok:1") throw new Error("caisse nav: " + caisse);
    await waitFor(cdp, `!!document.querySelector('button[aria-label="Thème"]')`, 15000, "pos topbar");
    // Theme first (its aria-label localizes after the switch).
    await evalJs(cdp, `document.querySelector('button[aria-label="Thème"]').click()`);
    await waitFor(cdp, `document.documentElement.classList.contains('dark') === true`, 8000, "dark");
    const loc = await evalJs(cdp, `(() => {
      const b = [...document.querySelectorAll('button')].find((x) => (x.textContent ?? '').trim() === 'العربية');
      if (!b) return 'MISSING-LOCALE-BTN';
      b.click(); return 'ok';
    })()`);
    if (loc !== "ok") throw new Error(loc);
    await waitFor(
      cdp,
      `[...document.querySelectorAll('nav button')].some((b) => (b.innerText ?? '').trim() === 'الوثائق والفواتير')`,
      10000, "ar nav",
    );
    const docs = await evalJs(cdp, clickIn("document", "الوثائق والفواتير"));
    if (docs !== "ok:1") throw new Error("docs nav: " + docs);
    await waitFor(cdp, `!!document.querySelector('tbody tr')`, 15000, "doc rows ar");
    await evalJs(cdp, `document.querySelector('tbody tr').click()`);
    await waitFor(cdp, `!!document.querySelector('[data-testid="details-lines"]')`, 15000, "drawer persists");
    await new Promise((r) => setTimeout(r, 600));
    return "ar-DZ + dark, drawer reopened";
  });
  await step("ar-dark-asserts", async () => {
    const html = await evalJs(cdp, DRAWER_HTML_EXPR);
    const checks = [
      ["برغي 4×40", html.includes("برغي 4×40")],
      ["قهوة عربية 250غ", html.includes("قهوة عربية 250غ")],
      ["badge منتج مؤرشف", html.includes("منتج مؤرشف")],
    ];
    const bad = checks.filter(([, ok]) => !ok).map(([n]) => n);
    if (bad.length) throw new Error("missing in drawer: " + bad.join(",") + " :: " + html.slice(0, 800));
    const uuid = await evalJs(cdp, UUID_IN_DRAWER_EXPR);
    if (uuid !== "UUID-CLEAN") throw new Error(uuid);
    const layout = await evalJs(cdp, LAYOUT_EXPR);
    if (!layout.noPageScroll) throw new Error("page scroll at 1280x800 ar-dark");
    const rtl = await evalJs(cdp, `document.documentElement.getAttribute('dir')`);
    return "ar names+badge visible, " + uuid + ", noPageScroll, dir=" + rtl;
  });
  await step("shot-ar-dark", async () => {
    const shot = await cdp.send("Page.captureScreenshot", { format: "png" });
    fs.writeFileSync(path.join(HERE, "drawer-1280x800-ar-dark.png"), Buffer.from(shot.data, "base64"));
    return "drawer-1280x800-ar-dark.png";
  });

  await step("zero-console-errors", async () => {
    if (CONSOLE_ERRORS.length) throw new Error(CONSOLE_ERRORS.join(" | ").slice(0, 300));
    return "0 console errors";
  });

  fs.writeFileSync(path.join(HERE, "capture-results.json"), JSON.stringify({ results: RESULTS, consoleErrors: CONSOLE_ERRORS }, null, 2));
  const failed = RESULTS.filter((r) => !r.ok);
  console.log(`\nCAPTURE ${failed.length === 0 ? "GREEN" : "RED"}: ${RESULTS.length - failed.length}/${RESULTS.length} steps ok`);
  chrome.kill();
  cdp.close();
  if (failed.length) process.exit(1);
}

main().catch((err) => { console.error("CAPTURE-FATAL:", err?.message ?? err); process.exit(2); });
