/**
 * TASK-DSN-093 visual verification: graphique 80mm ticket HTML + settings
 * render-mode control. Headless Chrome over CDP against vite :5173.
 *
 * Shots: (1) ticket fr-light clean sale, (2) ticket ar reprint (RTL mirror +
 * DUPLICATA), (3) settings render-mode control with graphique pressed.
 * Asserts: no page scroll on ticket renders, DUPLICATA visible on the reprint
 * render only, 0 console errors throughout.
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
const SAVED_CALLS = [];

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
    if (cmd === 'store_identity_get') return respond({ identity: { store_name: 'Quincaillerie El Baraka', store_address: 'Rue des Frères, Alger', store_phone: '0555123456', nif: '123456789012345', nis: '', rc: '', ai: '', ticket_footer_message: '' }, device_kind: 'physical' });
    if (cmd === 'printer_get_settings') return respond({ serial_port: 'COM1', serial_baud: 9600, paper_width_dots: 640, paper_width_mm: 80, transport: 'spooler', queue_name: 'Xprinter XP-410B', ticket_render_mode: 'graphique' });
    if (cmd === 'printer_update_settings') {
      window.__SAVED__ = window.__SAVED__ || [];
      window.__SAVED__.push(JSON.parse(JSON.stringify(req)));
      return respond({ serial_port: req.serial_port, serial_baud: req.serial_baud, paper_width_dots: req.paper_width_mm === 58 ? 384 : 640, paper_width_mm: req.paper_width_mm, transport: req.transport || 'serial', queue_name: (req.queue_name || '').trim(), ticket_render_mode: req.ticket_render_mode || 'graphique' });
    }
    if (cmd === 'printing_test_print_fr') return respond({ preview: [], decode_summary: { bands: 0, feed_lines: 0, unknown_spans: 0 }, capture_file_name: 'test-fr.escpos', device_outcome: 'sent' });
    if (cmd === 'printing_test_print_ar') return respond({ preview: [], decode_summary: { bands: 0, feed_lines: 0, unknown_spans: 0 }, capture_file_name: 'test-ar.escpos', device_outcome: 'capture-only' });
    if (cmd === 'cash_get_active_session') return respond(null);
    if (cmd === 'fiscal_settings_get') return respond({ timbreEnabled: true });
    if (cmd === 'backups_list') return respond([]);
    if (cmd === 'catalog_search') return respond([]);
    if (cmd === 'documents_search') return respond({ documents: [] });
    if (cmd === 'sales_get') return respond({
      document: { document_id: 'd-1', number: 'T-2026-000123', doc_type: 'TICKET', status: 'COMMITTED', warehouse_id: 'dep-01', subtotal_ht_centimes: '350000', tva_total_centimes: '10000', total_ttc_centimes: '360000', paid_amount_centimes: '400000', change_amount_centimes: '40000', payment_status: 'PAID' },
      items: [
        { product_id: 'p-1', unit_id: 'u-1', quantity_micro: '20000', unit_price_centimes: '85000', line_tva_centimes: '0', line_ttc_centimes: '170000', product_name_fr: 'Ciment CPJ 50kg', product_name_ar: 'إسمنت CPJ 50كغ', sku: 'CIM-001', unit_name_fr: 'Sac', unit_name_ar: 'كيس', product_missing: false },
        { product_id: 'p-2', unit_id: 'u-2', quantity_micro: '15000', unit_price_centimes: '120000', line_tva_centimes: '10000', line_ttc_centimes: '180000', product_name_fr: 'Fer Ø12', product_name_ar: 'حديد Ø12', sku: 'FER-012', unit_name_fr: 'Barre', unit_name_ar: 'قضيب', product_missing: false },
      ],
      payments: [{ method_code: 'CASH', amount_centimes: '400000' }],
      print_status: 'FRONTEND_RENDERED',
    });
    return fail('UNMOCKED_COMMAND:' + cmd);
  },
  transformCallback: function () { return 0 }
};
`;

const TICKET_BUILD = (lang, reprint) => `
(async () => {
  const m = await import('/src/features/tickets/graphique-ticket.ts');
  const fr = ${JSON.stringify(lang)} === 'FR';
  const html = m.buildTicketPrintHtml({
    storeName: fr ? 'Quincaillerie El Baraka' : 'البركة للمواد الحديدية',
    storeAddress: fr ? 'Rue des Frères, Alger' : 'شارع الإخوة، الجزائر',
    storePhone: '0555123456',
    documentNumber: 'T-2026-000123',
    createdAt: fr ? '24/10/2026 14:03' : '24/10/2026 14:03',
    language: ${JSON.stringify(lang)},
    lines: [
      { designation: fr ? 'Ciment CPJ 50kg' : 'إسمنت CPJ 50كغ', sku: 'CIM-001', quantity: '20000', unit: fr ? 'Sac' : 'كيس', unitPrice: '85000', lineTotal: '170000' },
      { designation: fr ? 'Fer Ø12' : 'حديد Ø12', sku: 'FER-012', quantity: '15000', unit: fr ? 'Barre' : 'قضيب', unitPrice: '120000', lineTotal: '180000' },
    ],
    subtotalHt: '350000',
    tvaTotal: '10000',
    totalTtc: '360000',
    paidAmount: '400000',
    changeAmount: '40000',
    paymentLabel: 'CASH',
    isReprint: ${reprint ? 'true' : 'false'},
    strings: fr
      ? { title: 'Ticket de caisse', reprintMark: 'DUPLICATA', designation: 'Désignation', qty: 'Qté', price: 'P.U', total: 'Total', subtotal: 'Sous-total HT', tva: 'TVA', grandTotal: 'TOTAL TTC', paid: 'Encaissé', change: 'Rendu', payment: 'Paiement', footer: 'Merci de votre visite' }
      : { title: 'تذكرة الصندوق', reprintMark: 'DUPLICATA', designation: 'البيان', qty: 'الكمية', price: 'السعر', total: 'المجموع', subtotal: 'المجموع HT', tva: 'TVA', grandTotal: 'المجموع TTC', paid: 'المقبوض', change: 'الباقي', payment: 'الدفع', footer: 'شكرا لزيارتكم' },
  });
  document.open();
  document.write(html);
  document.close();
  return document.documentElement.outerHTML.length;
})()`;

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
function clickButton(scopeExpr, text) {
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
    noPageScroll: de.scrollWidth <= window.innerWidth + 1,
    scrollW: de.scrollWidth, innerW: window.innerWidth,
  };
})()`;

const hasExactButton = (text) => `[...document.querySelectorAll('button')].some((b) => (b.innerText ?? '').trim() === ${JSON.stringify(text)})`;
const hasText = (t) => `document.body.innerText.includes(${JSON.stringify(t)})`;

async function main() {
  let cdp = null;
  const chrome = spawn(CHROME, [
    "--headless=new", "--disable-gpu", "--no-sandbox",
    "--remote-debugging-port=0", "--window-size=480,900", "about:blank",
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

  const shot = async (name) => {
    const res = await cdp.send("Page.captureScreenshot", { format: "png" });
    fs.writeFileSync(path.join(HERE, name), Buffer.from(res.data, "base64"));
    return name;
  };

  // ---- Shot 1: ticket fr-light, clean sale ----
  await step("ticket-fr-render", async () => {
    await cdp.send("Page.navigate", { url: APP_URL });
    await waitFor(cdp, `document.readyState === 'complete'`, 30000, "app shell");
    const len = await evalJs(cdp, TICKET_BUILD('FR', false), true);
    if (typeof len !== "number" || len < 1000) throw new Error("short html: " + len);
    const layout = await evalJs(cdp, LAYOUT_EXPR);
    if (!layout.noPageScroll) throw new Error(`page scroll fr ${layout.scrollW} > ${layout.innerW}`);
    const dup = await evalJs(cdp, hasText("DUPLICATA"));
    if (dup) throw new Error("clean sale carries DUPLICATA");
    const lines = await evalJs(cdp, `document.querySelectorAll('[data-testid="ticket-line"]').length`);
    if (lines !== 2) throw new Error("lines: " + lines);
    return `html=${len} lines=2 no-duplicata no-scroll`;
  });
  await step("shot-ticket-fr", async () => shot("ticket-80mm-fr-light.png"));

  // ---- Shot 2: ticket ar reprint (RTL mirror + DUPLICATA) ----
  await step("ticket-ar-reprint", async () => {
    const len = await evalJs(cdp, TICKET_BUILD('AR', true), true);
    if (typeof len !== "number" || len < 1000) throw new Error("short html: " + len);
    const dir = await evalJs(cdp, `document.documentElement.getAttribute('dir')`);
    if (dir !== "rtl") throw new Error("root dir not rtl: " + dir);
    const dup = await evalJs(cdp, hasText("DUPLICATA"));
    if (!dup) throw new Error("DUPLICATA missing on reprint render");
    const ltrNums = await evalJs(cdp, `document.querySelectorAll('bdi[dir="ltr"]').length`);
    if (ltrNums < 4) throw new Error("LTR-isolated numbers missing: " + ltrNums);
    const layout = await evalJs(cdp, LAYOUT_EXPR);
    if (!layout.noPageScroll) throw new Error(`page scroll ar ${layout.scrollW} > ${layout.innerW}`);
    return `html=${len} rtl duplicata ltr-nums=${ltrNums} no-scroll`;
  });
  await step("shot-ticket-ar", async () => shot("ticket-80mm-ar-reprint.png"));

  // ---- Shot 3: settings render-mode control ----
  await step("settings-mode-control", async () => {
    await cdp.send("Emulation.setDeviceMetricsOverride", {
      width: 1280, height: 800, deviceScaleFactor: 1, mobile: false,
    });
    await cdp.send("Page.navigate", { url: APP_URL });
    await waitFor(cdp, `!!document.querySelector('button[type="submit"]')`, 60000, "login submit");
    await evalJs(cdp, STUB_SOURCE);
    await evalJs(cdp, `(function(){var el=document.querySelector('input[autocomplete="username"]');var p=Object.getPrototypeOf(el);Object.getOwnPropertyDescriptor(p,'value').set.call(el,'admin');el.dispatchEvent(new Event('input',{bubbles:true}));el.dispatchEvent(new Event('change',{bubbles:true}));return 1})()`);
    await evalJs(cdp, `(function(){var el=document.querySelector('input[autocomplete="current-password"]');var p=Object.getPrototypeOf(el);Object.getOwnPropertyDescriptor(p,'value').set.call(el,'pw');el.dispatchEvent(new Event('input',{bubbles:true}));el.dispatchEvent(new Event('change',{bubbles:true}));return 1})()`);
    await evalJs(cdp, `(function(){var f=document.querySelector('form');if(!f)return 'NOFORM';f.requestSubmit();return 'requested'})()`);
    await waitFor(cdp, `!!document.querySelector('header')`, 15000, "strip after login");
    const nav = await evalJs(cdp, clickButton("document", "Paramètres"));
    if (nav !== "ok:1") throw new Error("nav: " + nav);
    await waitFor(cdp, hasExactButton("Impression & Matériel"), 10000, "printer tab visible");
    const tab = await evalJs(cdp, clickButton("document", "Impression & Matériel"));
    if (tab !== "ok:1") throw new Error("tab: " + tab);
    await waitFor(cdp, hasExactButton("Ticket graphique 80mm"), 8000, "render-mode group");
    const pressed = await evalJs(cdp, `(() => {
      const btns = [...document.querySelectorAll('button')].filter((b) => ['Ticket ESC/POS', 'Ticket graphique 80mm'].includes((b.innerText ?? '').trim()));
      return btns.map((b) => (b.innerText ?? '').trim() + '=' + b.getAttribute('aria-pressed')).join(',');
    })()`);
    if (pressed !== "Ticket ESC/POS=false,Ticket graphique 80mm=true") throw new Error("pressed: " + pressed);
    const help = await evalJs(cdp, hasText("80 mm"));
    if (!help) throw new Error("paper-size help missing");
    const payload = await evalJs(cdp, `(async () => {
      const save = [...document.querySelectorAll('button')].find((b) => (b.innerText ?? '').trim().startsWith('Enregistrer'));
      if (!save) return 'NO-SAVE';
      save.click();
      await new Promise((r) => setTimeout(r, 1500));
      const last = (window.__SAVED__ ?? []).pop();
      return JSON.stringify(last ?? null);
    })()`, true);
    const parsed = JSON.parse(payload);
    if (!parsed || parsed.ticket_render_mode !== "graphique") throw new Error("payload: " + payload);
    return pressed + " help-ok payload-ok";
  });
  await step("shot-settings", async () => shot("settings-render-mode-1280x800-fr-light.png"));

  await step("console-clean", async () => {
    if (CONSOLE_ERRORS.length > 0) throw new Error(CONSOLE_ERRORS.join(" | "));
    SAVED_CALLS.push("none");
    return "0 console errors";
  });

  fs.writeFileSync(
    path.join(HERE, "capture-results.json"),
    JSON.stringify({ results: RESULTS, consoleErrors: CONSOLE_ERRORS, at: new Date().toISOString() }, null, 2),
  );
  const failed = RESULTS.filter((r) => !r.ok);
  console.log(`\nDONE ${RESULTS.length - failed.length}/${RESULTS.length} ok, consoleErrors=${CONSOLE_ERRORS.length}`);
  cdp.close();
  chrome.kill();
  if (failed.length > 0) process.exit(1);
}

main().catch((err) => {
  console.error("CAPTURE-FATAL", err?.message ?? err);
  process.exit(2);
});
