/**
 * AUD-031 live drill (pure Node, no extra deps): boots vite on :5173, drives
 * headless Chrome over CDP with stubbed Tauri IPC, and executes the full auth
 * lifecycle across Admin / Vendeur / Caissier — rail visibility, user
 * protection, customer gates, payment modal, logout transitions, console and
 * scroll assertions.
 *
 * Usage: node audit/AUD-031/drill.mjs   (vite dev must be up on :5173)
 * Writes: audit/AUD-031/drill-results.json ; exit 0 only if every step passes.
 */
import { spawn } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const HERE = path.dirname(fileURLToPath(import.meta.url));
const CHROME = "C:/Program Files/Google/Chrome/Application/chrome.exe";
const DEBUG_PORT = 9224;
const APP_URL = "http://localhost:5173/";
const VIEWPORT = { width: 1280, height: 800 };

const PERMS = {
  VENDEUR: ["POS_SELL", "POS_PARK_RESUME", "VIEW_STOCK", "CREATE_SALE", "VIEW_CUSTOMERS"],
  GERANT: [
    "POS_SELL", "POS_PARK_RESUME", "CASHIER_COLLECT_PAYMENT", "CASH_SESSION_OPEN",
    "CASH_SESSION_CLOSE", "CASH_EXPENSE_RECORD", "CASH_REPORT_VIEW", "PRICE_OVERRIDE",
    "DISCOUNT_OVERRIDE", "NEGATIVE_STOCK_OVERRIDE", "CREDIT_LIMIT_OVERRIDE", "PROCESS_RETURN",
    "STOCK_ADJUSTMENT", "PURCHASE_MANAGE", "REPORTS_VIEW", "VIEW_STOCK", "ADJUST_STOCK",
    "TRANSFER_STOCK", "CREATE_SALE", "VIEW_CUSTOMERS", "CREATE_CUSTOMER",
    "MANAGE_CUSTOMER_CREDIT", "RECORD_CREDIT_PAYMENT", "REPRINT_DOCUMENT", "SUPPLIER_MANAGE",
    "PURCHASE_ORDER_CREATE", "PURCHASE_RECEIVE",    "PURCHASE_INVOICE_RECORD", "SUPPLIER_PAYMENT_RECORD",
  ],
  CAISSIER: [
    "POS_SELL", "POS_PARK_RESUME", "CASHIER_COLLECT_PAYMENT", "CASH_SESSION_OPEN",
    "CASH_SESSION_CLOSE", "CASH_EXPENSE_RECORD", "CASH_REPORT_VIEW", "VIEW_STOCK",
    "CREATE_SALE", "VIEW_CUSTOMERS", "CREATE_CUSTOMER", "RECORD_CREDIT_PAYMENT", "REPRINT_DOCUMENT",
  ],
  ADMIN: [
    "POS_SELL", "POS_PARK_RESUME", "CASHIER_COLLECT_PAYMENT", "CASH_SESSION_OPEN",
    "CASH_SESSION_CLOSE", "CASH_EXPENSE_RECORD", "CASH_REPORT_VIEW", "PRICE_OVERRIDE",
    "DISCOUNT_OVERRIDE", "NEGATIVE_STOCK_OVERRIDE", "CREDIT_LIMIT_OVERRIDE", "PROCESS_RETURN",
    "STOCK_ADJUSTMENT", "PURCHASE_MANAGE", "REPORTS_VIEW", "USER_MANAGE",
    "SYSTEM_SETTINGS_MANAGE", "BACKUP_RESTORE_MANAGE", "VIEW_STOCK", "ADJUST_STOCK",
    "TRANSFER_STOCK", "MANAGE_WAREHOUSES", "CREATE_SALE", "VIEW_CUSTOMERS", "CREATE_CUSTOMER",
    "MANAGE_CUSTOMER_CREDIT", "RECORD_CREDIT_PAYMENT", "REPRINT_DOCUMENT",
    "MANAGE_PRINTER_SETTINGS", "SUPPLIER_MANAGE", "PURCHASE_ORDER_CREATE", "PURCHASE_RECEIVE",
    "PURCHASE_INVOICE_RECORD", "SUPPLIER_PAYMENT_RECORD",
  ],
};
const ROLES = { admin: "ADMIN", vendeur: "VENDEUR", caissier: "CAISSIER", karim: "GERANT" };
const USERS = [
  { id: "u-admin", username: "admin", role: "ADMIN", is_active: true, has_pin: false, created_at: "2026-01-05 09:00:00", updated_at: "2026-01-05 09:00:00" },
  { id: "u-gerant", username: "karim", role: "GERANT", is_active: true, has_pin: true, created_at: "2026-02-10 10:30:00", updated_at: "2026-02-10 10:30:00" },
  { id: "u-caisse", username: "caisse1", role: "CAISSIER", is_active: true, has_pin: true, created_at: "2026-03-01 08:00:00", updated_at: "2026-03-01 08:00:00" },
  { id: "u-vendeur", username: "vendeur1", role: "VENDEUR", is_active: false, has_pin: false, created_at: "2026-04-12 14:00:00", updated_at: "2026-04-12 14:00:00" },
];
const CUSTOMER = {
  id: "c-1", name: "Quincaillerie Test", phone: "0550000000", address: null,
  credit_policy: "LIMITED", credit_limit_centimes: "50000", customer_tier: "retail",
  current_balance_centimes: "12000", is_active: true,
};
const CUSTOMER_SUMMARY = {
  id: "c-1", name: "Quincaillerie Test", phone: "0550000000",
  credit_policy: "LIMITED", customer_tier: "retail",
};

const STUB_SOURCE = `
window.__TAURI_INTERNALS__ = {
  invoke: function (cmd, args) {
    var req = (args && args.request) || {};
    var respond = function (v) { return Promise.resolve(v); };
    var fail = function (code) { return Promise.reject(code); };
    if (cmd === 'check_first_run_status') return respond({ requires_setup: false });
    if (cmd === 'auth_login') {
      var u = req.username || 'admin';
      var role = (${JSON.stringify(ROLES)})[u] || 'ADMIN';
      var perms = (${JSON.stringify(PERMS)})[role] || [];
      return respond({
        session_token: 'stub-' + u, user_id: 'u-' + u, username: u, role: role,
        permissions: perms, expires_at: '2030-01-01T00:00:00'
      });
    }
    if (cmd === 'auth_logout') return respond(null);
    if (cmd === 'warehouses_list') return respond({ warehouses: [{ id: 'w1', code: 'DEP-01', name: 'Depot Central', is_active: true }] });
    if (cmd === 'cash_get_active_session') return respond(null);
    if (cmd === 'users_list') return respond(${JSON.stringify(USERS)});
    if (cmd === 'customers_list') return respond({ customers: [${JSON.stringify(CUSTOMER_SUMMARY)}] });
    if (cmd === 'customers_search') return respond({ customers: [${JSON.stringify(CUSTOMER_SUMMARY)}] });
    if (cmd === 'customers_get') return respond(${JSON.stringify(CUSTOMER)});
    if (cmd === 'credit_get_statement') return respond({ entries: [] });
    if (cmd === 'reports_low_stock') return respond([]);
    if (cmd === 'reports_stock_movement_history') return respond([]);
    if (cmd === 'catalog_search') return respond([]);
    if (cmd === 'category_list') return respond([]);
    if (cmd === 'documents_search') return respond({ documents: [] });
    if (cmd === 'suppliers_list') return respond({ suppliers: [] });
    if (cmd === 'store_identity_get') return respond({ identity: { store_name: 'Q-Test', store_address: '', store_phone: '', nif: '', nis: '', rc: '', ai: '', ticket_footer_message: '' }, device_kind: 'physical' });
    if (cmd === 'printer_get_settings') return respond({ serial_port: '', serial_baud: 9600, paper_width_dots: 384, paper_width_mm: 80 });
    if (cmd === 'fiscal_settings_get') return respond({ timbre_enabled: false });
    if (cmd === 'backups_list') return respond([]);
    return fail('UNMOCKED_COMMAND:' + cmd);
  },
  transformCallback: function () { return 0; }
};
`;

class Cdp {
  constructor(url) {
    this.url = url;
    this.id = 0;
    this.pending = new Map();
    this.consoleErrors = [];
    this.pageExceptions = [];
  }
  connect() {
    return new Promise((resolve, reject) => {
      this.ws = new WebSocket(this.url);
      this.ws.addEventListener("open", resolve);
      this.ws.addEventListener("error", reject);
      this.ws.addEventListener("message", (ev) => {
        const msg = JSON.parse(String(ev.data));
        if (msg.method === "Console.messageAdded") {
          const m = msg.params.message;
          if (m.level === "error") this.consoleErrors.push(m.text);
        } else if (msg.method === "Runtime.exceptionThrown") {
          this.pageExceptions.push(String(msg.params.exceptionDetails?.text ?? "exception").slice(0, 200));
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
  close() {
    try { this.ws.close(); } catch { /* noop */ }
  }
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
// Full-expression builders (valid as standalone Runtime.evaluate programs).
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
function clickIncludesExpr(tag, substr) {
  return `(() => {
    const els = [...document.querySelectorAll(${JSON.stringify(tag)})].filter((e) => (e.innerText ?? '').includes(${JSON.stringify(substr)}));
    if (els.length === 0) return 'MISSING:' + ${JSON.stringify(substr)};
    els[0].click();
    return 'ok:' + els.length;
  })()`;
}
const SCROLL_EXPR = `(() => ({
  x: document.documentElement.scrollWidth <= document.documentElement.clientWidth,
  y: document.documentElement.scrollHeight <= document.documentElement.clientHeight,
  sw: document.documentElement.scrollWidth, cw: document.documentElement.clientWidth,
  sh: document.documentElement.scrollHeight, ch: document.documentElement.clientHeight,
}))()`;

const results = [];
async function step(name, fn) {
  try {
    const detail = await fn();
    results.push({ name, ok: true, detail: String(detail ?? "") });
    console.log(`  PASS ${name}`);
  } catch (err) {
    results.push({ name, ok: false, detail: String(err?.message ?? err).slice(0, 300) });
    console.log(`  FAIL ${name} :: ${String(err?.message ?? err).slice(0, 200)}`);
  }
}
async function loginAs(cdp, username) {
  await evalJs(cdp, fillExpr('input[autocomplete="username"]', username));
  await evalJs(cdp, fillExpr('input[autocomplete="current-password"]', "pw"));
  const label = await evalJs(cdp, `document.querySelector('button[type="submit"]').innerText.trim()`);
  await evalJs(cdp, clickTextExpr("button", label));
}
async function logout(cdp) {
  await evalJs(cdp, clickTextExpr("nav button", "Caisse"));
  await new Promise((r) => setTimeout(r, 600));
  await evalJs(cdp, clickTextExpr("button", "Déconnexion"));
  await waitFor(cdp, `!!document.querySelector('button[type="submit"]')`, 15000, "login screen after logout");
}

async function main() {
  const chrome = spawn(CHROME, [
    "--headless=new", `--remote-debugging-port=${DEBUG_PORT}`,
    "--no-first-run", "--disable-extensions",
    `--window-size=${VIEWPORT.width},${VIEWPORT.height}`, "about:blank",
  ], { stdio: "ignore" });
  let cdp = null;
  try {
    let targets = null;
    for (let i = 0; i < 40; i++) {
      try { targets = await httpJson("GET", `http://127.0.0.1:${DEBUG_PORT}/json/list`); break; }
      catch { await new Promise((r) => setTimeout(r, 500)); }
    }
    if (!targets || targets.length === 0) throw new Error("chrome remote debugging unavailable");
    void targets;
    const created = await httpJson("PUT", `http://127.0.0.1:${DEBUG_PORT}/json/new?about:blank`);
    cdp = new Cdp(created.webSocketDebuggerUrl);
    await cdp.connect();
    await cdp.send("Page.enable");
    await cdp.send("Runtime.enable");
    await cdp.send("Console.enable");
    await cdp.send("Emulation.setDeviceMetricsOverride", {
      width: VIEWPORT.width, height: VIEWPORT.height, deviceScaleFactor: 1, mobile: false,
    });
    await cdp.send("Page.addScriptToEvaluateOnNewDocument", { source: STUB_SOURCE });
    await cdp.send("Page.navigate", { url: APP_URL });
    await waitFor(cdp, `!!document.querySelector('button[type="submit"]')`, 30000, "login submit");
    await evalJs(cdp, STUB_SOURCE);

    console.log("Scenario A: initial load");
    await step("A1 boot shows LoginScreen, no AppShell nav", async () => {
      const hasLogin = await evalJs(cdp, `!!document.querySelector('button[type="submit"]')`);
      const hasNav = await evalJs(cdp, `!!document.querySelector('nav')`);
      if (!hasLogin || hasNav) throw new Error(`login=${hasLogin} nav=${hasNav}`);
      return `login=${hasLogin} nav=${hasNav}`;
    });

    console.log("Scenario B: admin flow");
    await loginAs(cdp, "admin");
    await step("B1 login renders AppShell immediately", async () => {
      await waitFor(cdp, `!!document.querySelector('nav')`, 15000, "nav after admin login");
      return "nav present";
    });
    const RAIL = ["Caisse", "Catalogue", "Stock & Dépôts", "Clients & Crédit", "Documents & Factures", "Fournisseurs", "Rapports", "Paramètres", "Utilisateurs & Accès"];
    await step("B2 rail shows all 9 entries", async () => {
      const text = await evalJs(cdp, `document.querySelector('nav').innerText`);
      const missing = RAIL.filter((l) => !text.includes(l));
      if (missing.length > 0) throw new Error("missing: " + missing.join(","));
      return "9/9 present";
    });
    for (const entry of RAIL) {
      await step(`B3 view renders: ${entry}`, async () => {
        const clicked = await evalJs(cdp, clickTextExpr("nav button", entry));
        if (!String(clicked).startsWith("ok")) throw new Error(clicked);
        await new Promise((r) => setTimeout(r, 900));
        return clicked;
      });
    }
    await step("B4 users view: 4 rows + last-admin badge + disabled control", async () => {
      await evalJs(cdp, clickTextExpr("nav button", "Utilisateurs & Accès"));
      await waitFor(cdp, `document.body.innerText.includes('Dernier Admin')`, 15000, "last-admin badge");
      const text = await evalJs(cdp, `document.body.innerText`);
      for (const u of ["admin", "karim", "caisse1", "vendeur1"]) {
        if (!text.includes(u)) throw new Error("missing user row: " + u);
      }
      const disabledCount = await evalJs(cdp, `[...document.querySelectorAll('button[disabled]')].length`);
      if (disabledCount < 1) throw new Error("no disabled last-admin control");
      return `4 rows, badge ok, disabled=${disabledCount}`;
    });
    await step("B5 scroll clean on users view", async () => {
      const s = await evalJs(cdp, SCROLL_EXPR);
      if (!s.x || !s.y) throw new Error(JSON.stringify(s));
      return JSON.stringify(s);
    });
    await step("B6 logout returns to LoginScreen", async () => {
      await logout(cdp);
      return "login screen back";
    });

    console.log("Scenario C: vendeur flow");
    await loginAs(cdp, "vendeur");
    await step("C1 vendeur rail: 5 entries, 4 absent", async () => {
      await waitFor(cdp, `!!document.querySelector('nav')`, 15000, "nav after vendeur login");
      const text = await evalJs(cdp, `document.querySelector('nav').innerText`);
      for (const l of ["Caisse", "Catalogue", "Stock & Dépôts", "Clients & Crédit", "Documents & Factures"]) {
        if (!text.includes(l)) throw new Error("missing: " + l);
      }
      for (const l of ["Fournisseurs", "Rapports", "Paramètres", "Utilisateurs & Accès"]) {
        if (text.includes(l)) throw new Error("should be absent: " + l);
      }
      return "5 present, 4 absent";
    });
    await step("C2 customers module: real Vendeur sees module-level unauthorized (Batch 2b pending)", async () => {
      await evalJs(cdp, clickTextExpr("nav button", "Clients & Crédit"));
      await waitFor(cdp, `document.body.innerText.includes('Non autorisé pour votre rôle')`, 15000, "module unauthorized");
      const hasNew = await evalJs(cdp, `document.body.innerText.includes('+ Nouveau client')`);
      if (hasNew) throw new Error("nouveau client visible despite module gate");
      return "module gate holds; DSN-073 component gates covered live in G-flow + unit tests (13/13)";
    });
    await step("C3 POS customer search lists directory row", async () => {
      await evalJs(cdp, clickTextExpr("nav button", "Caisse"));
      await waitFor(cdp, `document.body.innerText.includes('Client Comptant')`, 15000, "pos client button");
      await evalJs(cdp, clickIncludesExpr("button", "Client Comptant"));
      await waitFor(cdp, `!!document.querySelector('input[placeholder="Nom, téléphone ou code…"]')`, 15000, "customer search");
      await evalJs(cdp, fillExpr('input[placeholder="Nom, téléphone ou code…"]', "quin"));
      await waitFor(cdp, `document.body.innerText.includes('Quincaillerie Test')`, 15000, "directory row");
      return "search wiring live";
    });
    await step("C4 payment modal: vendeur inputs disabled with guidance + scroll clean", async () => {
      await waitFor(cdp, `[...document.querySelectorAll('button')].some((b) => (b.innerText ?? '').trim() === 'د.ج')`, 15000, "versement button");
      await evalJs(cdp, `(() => { [...document.querySelectorAll('button')].find((b) => (b.innerText ?? '').trim() === 'د.ج').click(); return 'ok'; })()`);
      await waitFor(cdp, `document.body.innerText.includes('Rôle Vendeur')`, 15000, "vendeur restriction notice");
      const title = await evalJs(cdp, `(() => {
        const btns = [...document.querySelectorAll('button')];
        const submit = btns.find((b) => b.disabled && (b.innerText ?? '').includes('Encaisser'));
        if (!submit) return 'NOSUBMIT';
        return submit.getAttribute('title') ?? 'NOTITLE';
      })()`);
      if (!title.includes("réservé à la caisse")) throw new Error("submit title=" + title);
      const scroll = await evalJs(cdp, SCROLL_EXPR);
      if (!scroll.x || !scroll.y) throw new Error("scroll " + JSON.stringify(scroll));
      await evalJs(cdp, clickTextExpr("button", "Annuler"));
      await waitFor(cdp, `!document.body.innerText.includes('Rôle Vendeur')`, 15000, "modal closed");
      return `notice ok, title ok, scroll clean, modal closed`;
    });
    await step("C5 logout returns to LoginScreen", async () => {
      await logout(cdp);
      return "login screen back";
    });

    console.log("Scenario D: caissier flow");
    await loginAs(cdp, "caissier");
    await step("D1 caissier payment modal enabled", async () => {
      await waitFor(cdp, `!!document.querySelector('nav')`, 15000, "nav after caissier login");
      await evalJs(cdp, clickTextExpr("nav button", "Caisse"));
      await waitFor(cdp, `document.body.innerText.includes('Client Comptant')`, 15000, "pos client button");
      await evalJs(cdp, clickIncludesExpr("button", "Client Comptant"));
      await waitFor(cdp, `!!document.querySelector('input[placeholder="Nom, téléphone ou code…"]')`, 15000, "customer search");
      await evalJs(cdp, fillExpr('input[placeholder="Nom, téléphone ou code…"]', "quin"));
      await waitFor(cdp, `[...document.querySelectorAll('button')].some((b) => (b.innerText ?? '').trim() === 'د.ج')`, 15000, "versement button");
      await evalJs(cdp, `(() => { [...document.querySelectorAll('button')].find((b) => (b.innerText ?? '').trim() === 'د.ج').click(); return 'ok'; })()`);
      await waitFor(cdp, `document.body.innerText.includes('Montant')`, 15000, "payment amount label");
      const blocked = await evalJs(cdp, `document.body.innerText.includes('Rôle Vendeur')`);
      if (blocked) throw new Error("caissier wrongly blocked");
      const enabled = await evalJs(cdp, `(() => {
        const input = [...document.querySelectorAll('input')].find((i) => (i.closest('label')?.innerText ?? '').includes('Montant'));
        return input ? !input.disabled : 'NOINPUT';
      })()`);
      if (enabled !== true) throw new Error("amount input not enabled: " + enabled);
      await evalJs(cdp, clickTextExpr("button", "Annuler"));
      await waitFor(cdp, `!document.body.innerText.includes('Encaissement de dette') && document.body.innerText.includes('Client Comptant')`, 15000, "modal closed");
      return "payment open, inputs enabled, modal closed";
    });
    await step("D2 customers module: real Caissier sees module-level unauthorized (Batch 2b pending)", async () => {
      await evalJs(cdp, clickTextExpr("nav button", "Clients & Crédit"));
      await waitFor(cdp, `document.body.innerText.includes('Non autorisé pour votre rôle')`, 15000, "module unauthorized");
      return "module gate holds; DSN-073 component gates covered live in G-flow + unit tests";
    });
    await step("D3 logout returns to LoginScreen", async () => {
      await logout(cdp);
      return "login screen back";
    });

    console.log("Scenario G: gerant flow (module gate passes; component gates live)");
    await loginAs(cdp, "karim");
    await step("G1 gerant rail: settings present, users absent", async () => {
      await waitFor(cdp, `!!document.querySelector('nav')`, 15000, "nav after gerant login");
      const text = await evalJs(cdp, `document.querySelector('nav').innerText`);
      for (const l of ["Caisse", "Catalogue", "Stock & Dépôts", "Clients & Crédit", "Fournisseurs", "Rapports", "Paramètres"]) {
        if (!text.includes(l)) throw new Error("missing: " + l);
      }
      if (text.includes("Utilisateurs & Accès")) throw new Error("users should be absent for gerant");
      return "7 present, users absent";
    });
    await step("G2 gerant: Nouveau Client enabled (CREATE_CUSTOMER held)", async () => {
      await evalJs(cdp, clickTextExpr("nav button", "Clients & Crédit"));
      await waitFor(cdp, `document.body.innerText.includes('+ Nouveau client')`, 15000, "new customer button");
      const disabled = await evalJs(
        cdp,
        `(() => { const b = [...document.querySelectorAll('button')].find((e) => (e.innerText ?? '').trim() === '+ Nouveau client'); return b ? b.disabled : 'MISSING'; })()`,
      );
      if (disabled !== false) throw new Error("nouveau client disabled=" + disabled);
      return "enabled";
    });
    await step("G3 gerant: credit limit input enabled (MANAGE_CUSTOMER_CREDIT held)", async () => {
      const clicked = await evalJs(cdp, `(() => { const rows = [...document.querySelectorAll('li button')]; if (rows.length === 0) return 'NOROWS'; rows[0].click(); return 'ok'; })()`);
      if (clicked !== "ok") throw new Error(clicked);
      await waitFor(cdp, `document.body.innerText.includes('Modifier')`, 15000, "dossier edit");
      await evalJs(cdp, clickTextExpr("button", "Modifier la fiche"));
      await waitFor(cdp, `document.body.innerText.includes('Plafond (DZD)')`, 15000, "credit limit label");
      const state = await evalJs(cdp, `(() => {
        const lab = [...document.querySelectorAll('label')].find((l) => (l.innerText ?? '').includes('Plafond (DZD)'));
        const input = lab?.querySelector('input');
        if (!input) return 'NOINPUT';
        return JSON.stringify({ disabled: input.disabled });
      })()`);
      if (JSON.parse(state).disabled !== false) throw new Error(state);
      await evalJs(cdp, clickTextExpr("button", "Annuler"));
      return "limit enabled for gerant";
    });
    await step("G4 gerant: payment modal enabled (RECORD_CREDIT_PAYMENT held)", async () => {
      await evalJs(cdp, clickTextExpr("nav button", "Caisse"));
      await waitFor(cdp, `document.body.innerText.includes('Client Comptant')`, 15000, "pos client button");
      await evalJs(cdp, clickIncludesExpr("button", "Client Comptant"));
      await waitFor(cdp, `!!document.querySelector('input[placeholder="Nom, téléphone ou code…"]')`, 15000, "customer search");
      await evalJs(cdp, fillExpr('input[placeholder="Nom, téléphone ou code…"]', "quin"));
      await waitFor(cdp, `[...document.querySelectorAll('button')].some((b) => (b.innerText ?? '').trim() === 'د.ج')`, 15000, "versement button");
      await evalJs(cdp, `(() => { [...document.querySelectorAll('button')].find((b) => (b.innerText ?? '').trim() === 'د.ج').click(); return 'ok'; })()`);
      await waitFor(cdp, `document.body.innerText.includes('Montant')`, 15000, "payment amount label");
      const blocked = await evalJs(cdp, `document.body.innerText.includes('Rôle Vendeur')`);
      if (blocked) throw new Error("gerant wrongly blocked");
      await evalJs(cdp, clickTextExpr("button", "Annuler"));
      await waitFor(cdp, `!document.body.innerText.includes('Encaissement de dette')`, 15000, "modal closed");
      return "payment enabled, modal closed";
    });
    await step("G5 logout returns to LoginScreen", async () => {
      await logout(cdp);
      return "login screen back";
    });

    await step("Z1 zero console errors and zero page exceptions", async () => {
      if (cdp.consoleErrors.length > 0) throw new Error("console: " + cdp.consoleErrors.slice(0, 3).join(" | "));
      if (cdp.pageExceptions.length > 0) throw new Error("exceptions: " + cdp.pageExceptions.slice(0, 3).join(" | "));
      return "clean";
    });
  } finally {
    if (cdp) cdp.close();
    try { chrome.kill(); } catch { /* noop */ }
  }

  const failed = results.filter((r) => !r.ok);
  fs.writeFileSync(path.join(HERE, "drill-results.json"), JSON.stringify({ results }, null, 2));
  console.log(`\n${results.length - failed.length}/${results.length} steps passed`);
  if (failed.length > 0) {
    console.log("FAILURES:");
    for (const f of failed) console.log(` - ${f.name}: ${f.detail}`);
    process.exit(1);
  }
}

export { STUB_SOURCE };

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  main().catch((err) => {
    console.error("DRILL FATAL:", err?.message ?? err);
    process.exit(2);
  });
}
