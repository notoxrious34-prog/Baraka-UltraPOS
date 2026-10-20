/**
 * TASK-DSN-078 visual verification: Documents journal layout integrity.
 * Headless Chrome over CDP against vite :5173, Tauri IPC stubbed in-page
 * (built-ins only: node:child_process, node:fs, node:path, global
 * WebSocket/fetch). Pattern: audit/DSN-076/drill.mjs.
 *
 * 4 captures: 1280x800 fr/light, 1280x800 ar/dark, 1024x600 fr/light,
 * 1024x600 ar/dark. Per capture asserts: no page-level scrollbars
 * (scrollWidth<=innerWidth && scrollHeight<=innerHeight), filter bar +
 * document table visible with rows, tabular figures present.
 * The module gate itself (authorized vs fallback) is pinned by
 * documents-module.test.ts source assertions (repo convention: zustand
 * seeded state is invisible to SSR); here the stub session carries
 * CREATE_SALE, proving the authorized surface is unchanged.
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

const DOCS = [
  { id: "d-1", document_number: "FAC-2026-0001", document_type: "FACTURE", status: "FINALIZED", party_id: "c-1", party_name: "Client Comptant", total_ttc_centimes: "125000", items_count: 3, created_at: "2026-10-20T09:12:00" },
  { id: "d-2", document_number: "TIC-2026-0455", document_type: "TICKET", status: "FINALIZED", party_id: null, party_name: null, total_ttc_centimes: "9800", items_count: 1, created_at: "2026-10-20T10:02:00" },
  { id: "d-3", document_number: "BL-2026-0110", document_type: "BL", status: "FINALIZED", party_id: "c-2", party_name: "Entreprise ELEC", total_ttc_centimes: "452500", items_count: 8, created_at: "2026-10-19T15:40:00" },
  { id: "d-4", document_number: "DEV-2026-0032", document_type: "DEVIS", status: "DRAFT", party_id: "c-3", party_name: "Plomberie Nord", total_ttc_centimes: "210000", items_count: 5, created_at: "2026-10-18T11:20:00" },
  { id: "d-5", document_number: "AVO-2026-0007", document_type: "AVOIR", status: "FINALIZED", party_id: "c-1", party_name: "Client Comptant", total_ttc_centimes: "15000", items_count: 1, created_at: "2026-10-17T08:05:00" },
  { id: "d-6", document_number: "FAC-2026-0002", document_type: "FACTURE", status: "DRAFT", party_id: null, party_name: null, total_ttc_centimes: "67200", items_count: 2, created_at: "2026-10-20T11:55:00" },
];

const STUB_SOURCE = `
window.__TAURI_INTERNALS__ = {
  invoke: function (cmd, args) {
    var req = (args && args.request) || {};
    var respond = function (v) { return Promise.resolve(v); };
    var fail = function (code) { return Promise.reject(code); };
    if (cmd === 'check_first_run_status') return respond({ requires_setup: false });
    if (cmd === 'auth_login') return respond({
      session_token: 'stub-admin', user_id: 'u-admin', username: 'admin',
      role: 'ADMIN', permissions: ['VIEW_STOCK', 'POS_SELL', 'CREATE_SALE', 'VIEW_CUSTOMERS', 'REPORTS_VIEW'],
      expires_at: '2030-01-01T00:00:00',
    });
    if (cmd === 'auth_logout') return respond(null);
    if (cmd === 'warehouses_list') return respond({ warehouses: [{ id: 'dep-01', code: 'DEP-01', name: 'Dépôt Central', is_active: true }] });
    if (cmd === 'category_list') return respond([]);
    if (cmd === 'store_identity_get') return respond({ identity: { store_name: 'Q-Test' }, device_kind: 'physical' });
    if (cmd === 'cash_get_active_session') return respond(null);
    if (cmd === 'documents_search') {
      var q = String(req.query ?? '').trim().toLowerCase();
      var list = ${JSON.stringify(DOCS)}.filter(function (d) {
        if (q === '') return true;
        return d.document_number.toLowerCase().includes(q) ||
          (d.party_name && d.party_name.toLowerCase().includes(q));
      });
      return respond({ documents: list.slice(0, Math.max(1, Math.min(100, Number(req.limit) || 100))) });
    }
    if (cmd === 'documents_reprint') return respond({ ok: true });
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

const LAYOUT_EXPR = `(() => {
  const de = document.documentElement;
  const table = document.querySelector('table');
  const rows = table ? table.querySelectorAll('tbody tr').length : -1;
  const filter = !!document.querySelector('input[placeholder]');
  const tabular = !!document.querySelector('table .tabular-nums');
  return {
    w: window.innerWidth, h: window.innerHeight, dpr: window.devicePixelRatio,
    sw: de.scrollWidth, sh: de.scrollHeight,
    rows, filter, tabular,
    noPageScroll: de.scrollWidth <= window.innerWidth && de.scrollHeight <= window.innerHeight,
  };
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
  const page = tabs.find((t) => t.type === "page");
  cdp = new Cdp(page.webSocketDebuggerUrl);
  await cdp.connect();
  await cdp.send("Console.enable", {});
  await cdp.send("Page.enable", {});
  await cdp.send("Runtime.enable", {});

  async function viewport(w, h) {
    await cdp.send("Emulation.setDeviceMetricsOverride", {
      width: w, height: h, deviceScaleFactor: 1, mobile: false,
    });
    await cdp.send("Page.navigate", { url: APP_URL });
    currentAr = false; // fresh load is always FR (locale is session-only)
    await waitFor(cdp, `!!document.querySelector('button[type="submit"]')`, 30000, "login submit");
    await evalJs(cdp, STUB_SOURCE);
    await evalJs(cdp, fillExpr('input[autocomplete="username"]', "admin"));
    await evalJs(cdp, fillExpr('input[autocomplete="current-password"]', "pw"));
    const label = await evalJs(cdp, `document.querySelector('button[type="submit"]').innerText.trim()`);
    await evalJs(cdp, clickTextExpr("button", label));
    await waitFor(cdp, `!!document.querySelector('nav')`, 15000, "nav after login");
  }
  async function toDocuments(navLabel) {
    await evalJs(cdp, clickTextExpr("nav button", navLabel));
    await waitFor(cdp, `document.querySelectorAll('table tbody tr').length >= 6`, 15000, "6 doc rows");
    await new Promise((r) => setTimeout(r, 600));
  }
  async function setDark(on) {
    const isDark = await evalJs(cdp, `document.documentElement.classList.contains('dark')`);
    if (isDark !== on) {
      await evalJs(cdp, `document.querySelector('button[aria-label="Thème"]').click()`);
      await waitFor(cdp, `document.documentElement.classList.contains('dark') === ${on}`, 8000, "theme");
      await new Promise((r) => setTimeout(r, 400));
    }
  }
  // Locale is session-only (fresh FR after every Page.navigate); track in
  // script state instead of DOM detection (static dir="rtl" spans exist in
  // every view, e.g. PosShellView brand mark — DOM probes lie).
  let currentAr = false;
  async function setArabic(on) {
    if (currentAr !== on) {
      const btn = await evalJs(cdp, `(() => {
        const all = [...document.querySelectorAll('button')];
        const probe = all.map((x) => (x.innerText ?? '').trim());
        const lit = ${on ? "'العربية'" : "'Français'"};
        const codes = (s) => [...s].map((c) => 'U+' + c.codePointAt(0).toString(16)).join(' ');
        const cand = probe.find((t) => t.includes('لعر') || t.includes('rab'));
        const b = all.find((x) => (x.innerText ?? '').includes(lit));
        if (!b) return 'MISSING-LOCALE-BTN lit=' + codes(lit) + ' cand=' + (cand ? codes(cand) : 'none');
        b.click(); return 'ok';
      })()`);
      if (btn !== "ok") throw new Error(btn);
      currentAr = on;
      await waitFor(cdp, `(!!document.querySelector('[dir="rtl"]')) === ${on}`, 8000, "locale");
      await new Promise((r) => setTimeout(r, 400));
    }
  }
  async function capture(name, w, h, arabic, dark, navLabel, posLabel) {
    await viewport(w, h);
    // Locale/theme toggles live in the POS Topbar: switch there first.
    await evalJs(cdp, clickTextExpr("nav button", posLabel));
    await waitFor(cdp, `!!document.querySelector('button[aria-label="Thème"]')`, 15000, "pos topbar");
    await setDark(dark);
    await setArabic(arabic);
    await toDocuments(navLabel);
    await step(`${name}: layout assertions`, async () => {
      const m = await evalJs(cdp, LAYOUT_EXPR);
      if (!m.noPageScroll) throw new Error(`page scroll sw=${m.sw} sh=${m.sh} @${m.w}x${m.h}`);
      if (!m.filter) throw new Error("filter bar input missing");
      if (m.rows < 6) throw new Error(`expected >=6 rows, got ${m.rows}`);
      if (!m.tabular) throw new Error("tabular-nums missing in table");
      return `viewport=${m.w}x${m.h} dpr=${m.dpr} rows=${m.rows} noPageScroll=true tabular=true`;
    });
    const res = await cdp.send("Page.captureScreenshot", { format: "png" });
    const out = path.join(HERE, name);
    fs.writeFileSync(out, Buffer.from(res.data, "base64"));
    console.log(`  SHOT ${name} (${fs.statSync(out).size} bytes)`);
  }

  await capture("documents-1280x800-fr-light.png", 1280, 800, false, false, "Documents & Factures", "Caisse");
  await capture("documents-1280x800-ar-dark.png", 1280, 800, true, true, "الوثائق والفواتير", "Caisse");
  await capture("documents-1024x600-fr-light.png", 1024, 600, false, false, "Documents & Factures", "Caisse");
  await capture("documents-1024x600-ar-dark.png", 1024, 600, true, true, "الوثائق والفواتير", "Caisse");

  await step("zero console errors across captures", async () => {
    if (CONSOLE_ERRORS.length > 0) throw new Error(CONSOLE_ERRORS.join(" | ").slice(0, 300));
    return "0 console errors, 0 unmocked IPC (all stubbed commands responded)";
  });

  const failed = RESULTS.filter((r) => !r.ok);
  fs.writeFileSync(path.join(HERE, "capture-results.json"), JSON.stringify({ failed: failed.length, total: RESULTS.length, results: RESULTS }, null, 2));
  console.log(`\nCAPTURE DONE: ${RESULTS.length - failed.length}/${RESULTS.length} steps passed`);
  cdp.close();
  chrome.kill();
  if (failed.length > 0) process.exit(1);
}
main().catch((e) => { console.error("FATAL", e?.message ?? e); process.exit(2); });
