/**
 * TASK-DSN-094 visual verification: factory-reset danger-zone section.
 * Headless Chrome CDP vs vite :5173, TAURI_INTERNALS stubbed.
 *
 * Shots: (1) danger zone fr-light, eligible precheck, token empty
 * (execute disabled); (2) precheck-blocked fr-light (open session +
 * unwritable dir, reasons verbatim, execute disabled); (3) confirm-token
 * ar-light (token typed, execute enabled, RTL root).
 * Asserts: key texts, input/button states, no horizontal page scroll,
 * 0 console errors throughout.
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

const STUB_SOURCE = `
window.__TAURI_INTERNALS__ = {
  invoke: function (cmd, args) {
    var req = (args && args.request) || {};
    var respond = function (v) { return Promise.resolve(v); };
    var fail = function (code) { return Promise.reject(code); };
    if (cmd === 'check_first_run_status') return respond({ requires_setup: false });
    if (cmd === 'auth_login') return respond({
      session_token: 'stub-admin', user_id: 'u-admin', username: 'admin',
      role: 'ADMIN', permissions: ['CREATE_SALE', 'VIEW_STOCK', 'POS_SELL', 'VIEW_CUSTOMERS', 'VIEW_DOCUMENTS', 'SYSTEM_SETTINGS_MANAGE', 'STOCK_ADJUSTMENT', 'ADJUST_STOCK', 'TRANSFER_STOCK', 'MANAGE_PRINTER_SETTINGS', 'MANAGE_WAREHOUSES', 'SUPPLIER_MANAGE', 'REPORTS_VIEW', 'CASH_REPORT_VIEW', 'USER_MANAGE'],
      expires_at: '2030-01-01T00:00:00',
    });
    if (cmd === 'auth_logout') return respond(null);
    if (cmd === 'warehouses_list') return respond({ warehouses: [{ id: 'dep-01', code: 'DEP-01', name: 'Dépôt Central', is_active: true }] });
    if (cmd === 'category_list') return respond([]);
    if (cmd === 'store_identity_get') return respond({ identity: { store_name: 'Quincaillerie El Baraka', store_address: 'Rue des Frères, Alger', store_phone: '0555123456', nif: '', nis: '', rc: '', ai: '', ticket_footer_message: '' }, device_kind: 'physical' });
    if (cmd === 'printer_get_settings') return respond({ serial_port: 'COM1', serial_baud: 9600, paper_width_dots: 640, paper_width_mm: 80, transport: 'spooler', queue_name: 'Xprinter XP-410B', ticket_render_mode: 'graphique' });
    if (cmd === 'fiscal_settings_get') return respond({ timbreEnabled: true });
    if (cmd === 'backups_list') return respond([]);
    if (cmd === 'backups_create') return respond({ id: 'b-1', file_name: 'baraka_pos_backup_20261025T120000_a1b2c3d4.db', file_size_bytes: 102400, sha256_checksum: 'ab'.repeat(32), trigger_type: 'MANUAL', status: 'SUCCESS', schema_version: 18, error_message: null, created_at: '2026-10-25T12:00:00.000Z' });
    if (cmd === 'catalog_search') return respond([]);
    if (cmd === 'documents_search') return respond({ documents: [] });
    if (cmd === 'cash_get_active_session') return respond(null);
    if (cmd === 'factory_reset_precheck') {
      if (window.__RESET_MODE === 'blocked') return respond({ open_session: true, backup_dir_writable: false, newest_backup: null, eligible: false, reasons: ['RESET_OPEN_SESSION', 'RESET_BACKUP_UNWRITABLE'] });
      return respond({ open_session: false, backup_dir_writable: true, newest_backup: { file_name: 'baraka_pos_backup_20261025T120000_a1b2c3d4.db', created_at: '2026-10-25T12:00:00.000Z', status: 'SUCCESS' }, eligible: true, reasons: [] });
    }
    if (cmd === 'factory_reset_execute') return respond({ reset_backup_file_name: 'baraka_pos_backup_20261025T120000_a1b2c3d4.db', quarantined_to: 'active-20251025-120000Z.db', reset_at: '2026-10-25T12:05:00.000Z', restart_required: true });
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
// Substring variant: the topbar locale toggle carries an icon next to the
// label, so exact-match is brittle across renders. Matches by containment.
function clickButtonContaining(scopeExpr, text) {
  return `(() => {
    const scope = ${scopeExpr};
    if (!scope) return 'MISSING-SCOPE';
    const els = [...scope.querySelectorAll('button')].filter((e) => (e.innerText ?? '').includes(${JSON.stringify(text)}));
    if (els.length === 0) return 'MISSING~:' + ${JSON.stringify(text)};
    els[0].click();
    return 'ok:' + els.length;
  })()`;
}
// The backups page renders several "Actualiser" buttons (catalog + reset);
// scope the reset refresh to its own danger-zone section.
const RESET_SECTION_SCOPE = `document.querySelector('section[aria-label^="Zone"]') || document.querySelector('section[aria-label^="منطقة"]')`;
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

const NO_H_SCROLL = `document.documentElement.scrollWidth <= window.innerWidth + 1`;
const hasText = (t) => `document.body.innerText.includes(${JSON.stringify(t)})`;
const hasExactButton = (text) => `[...document.querySelectorAll('button')].some((b) => (b.innerText ?? '').trim() === ${JSON.stringify(text)})`;
const execDisabled = `(() => {
  const btns = [...document.querySelectorAll('button')];
  const b = btns.find((x) => (x.innerText ?? '').trim() === 'Effacer tout et réinitialiser' || (x.innerText ?? '').trim() === 'مسح الكل وإعادة الضبط');
  if (!b) return 'NO-BTN';
  return b.disabled ? 'disabled' : 'enabled';
})()`;
const typeToken = (value) => `(() => {
  const el = document.querySelector('#factory-reset-token');
  if (!el) return 'NO-INPUT';
  const p = Object.getPrototypeOf(el);
  Object.getOwnPropertyDescriptor(p, 'value').set.call(el, ${JSON.stringify(value)});
  el.dispatchEvent(new Event('input', { bubbles: true }));
  el.dispatchEvent(new Event('change', { bubbles: true }));
  return 'typed';
})()`;

async function loginAndOpenBackups(cdp) {
  await cdp.send("Page.navigate", { url: APP_URL });
  await waitFor(cdp, `!!document.querySelector('button[type="submit"]')`, 60000, "login submit");
  await evalJs(cdp, STUB_SOURCE);
  await evalJs(cdp, `(function(){var el=document.querySelector('input[autocomplete="username"]');var p=Object.getPrototypeOf(el);Object.getOwnPropertyDescriptor(p,'value').set.call(el,'admin');el.dispatchEvent(new Event('input',{bubbles:true}));el.dispatchEvent(new Event('change',{bubbles:true}));return 1})()`);
  await evalJs(cdp, `(function(){var el=document.querySelector('input[autocomplete="current-password"]');var p=Object.getPrototypeOf(el);Object.getOwnPropertyDescriptor(p,'value').set.call(el,'pw');el.dispatchEvent(new Event('input',{bubbles:true}));el.dispatchEvent(new Event('change',{bubbles:true}));return 1})()`);
  await evalJs(cdp, `(function(){var f=document.querySelector('form');if(!f)return 'NOFORM';f.requestSubmit();return 'requested'})()`);
  await waitFor(cdp, `!!document.querySelector('header')`, 15000, "strip after login");
  const nav = await evalJs(cdp, clickButton("document", "Paramètres"));
  if (nav !== "ok:1") throw new Error("nav: " + nav);
  await waitFor(cdp, hasExactButton("Sauvegardes & Sécurité"), 10000, "backups tab visible");
  const tab = await evalJs(cdp, clickButton("document", "Sauvegardes & Sécurité"));
  if (tab !== "ok:1") throw new Error("tab: " + tab);
  await waitFor(cdp, hasText("Zone dangereuse"), 10000, "danger zone visible");
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

  // ---- Shot 1: danger zone fr-light, eligible precheck, token empty ----
  await step("reset-danger-fr", async () => {
    await loginAndOpenBackups(cdp);
    const newest = await evalJs(cdp, hasText("baraka_pos_backup_20261025T120000_a1b2c3d4.db"));
    if (!newest) throw new Error("newest backup row missing");
    const st = await evalJs(cdp, execDisabled);
    if (st !== "disabled") throw new Error("execute must be disabled with empty token: " + st);
    const hscroll = await evalJs(cdp, NO_H_SCROLL);
    if (!hscroll) throw new Error("horizontal page scroll");
    const createBtn = await evalJs(cdp, hasExactButton("Créer une sauvegarde vérifiée"));
    if (!createBtn) throw new Error("backup entry point missing");
    return `eligible newest-ok exec-disabled no-hscroll`;
  });
  await step("shot-reset-danger-fr", async () => shot("reset-danger-fr-light.png"));

  // ---- Shot 2: precheck blocked (open session + unwritable dir) ----
  await step("reset-blocked-fr", async () => {
    await evalJs(cdp, `window.__RESET_MODE = 'blocked'`);
    const refr = await evalJs(cdp, clickButton(RESET_SECTION_SCOPE, "Actualiser"));
    if (refr !== "ok:1") throw new Error("refresh: " + refr);
    await waitFor(cdp, hasText("RESET_OPEN_SESSION"), 8000, "open-session reason");
    const unwr = await evalJs(cdp, hasText("RESET_BACKUP_UNWRITABLE"));
    if (!unwr) throw new Error("unwritable reason missing");
    const blocked = await evalJs(cdp, hasText("Caisse ouverte"));
    if (!blocked) throw new Error("open-session guidance missing");
    const st = await evalJs(cdp, execDisabled);
    if (st !== "disabled") throw new Error("execute must stay disabled: " + st);
    return `reasons-verbatim guidance-ok exec-disabled`;
  });
  await step("shot-reset-blocked-fr", async () => shot("reset-blocked-fr-light.png"));

  // ---- Shot 3: confirm-token ar (token typed, execute enabled, RTL) ----
  await step("reset-confirm-ar", async () => {
    await evalJs(cdp, `window.__RESET_MODE = 'ok'`);
    // The locale toggle lives in the POS Topbar (absent from settings
    // views): switch locale on the POS view, then return to the tab.
    const pos = await evalJs(cdp, clickButton("document", "Caisse"));
    if (pos !== "ok:1") throw new Error("pos nav: " + pos);
    await waitFor(cdp, hasText("Ouvrir la session"), 10000, "pos view");
    const lang = await evalJs(cdp, clickButtonContaining("document", "العربية"));
    if (lang !== "ok:1") throw new Error("lang toggle: " + lang);
    const back = await evalJs(cdp, clickButton("document", "الإعدادات"));
    if (back !== "ok:1") throw new Error("settings nav ar: " + back);
    const tabAr = await evalJs(cdp, clickButton("document", "النسخ الاحتياطي والأمان"));
    if (tabAr !== "ok:1") throw new Error("backups tab ar: " + tabAr);
    await waitFor(cdp, hasText("منطقة خطرة"), 8000, "ar danger title");
    const typed = await evalJs(cdp, typeToken("Quincaillerie El Baraka"));
    if (typed !== "typed") throw new Error("token input: " + typed);
    await waitFor(cdp, `(${execDisabled}) === 'enabled'`, 5000, "execute enabled");
    const dir = await evalJs(cdp, `(() => { const el = document.querySelector('[dir="rtl"]'); return el ? 'rtl' : 'none'; })()`);
    if (dir !== "rtl") throw new Error("no rtl shell container");
    const hscroll = await evalJs(cdp, NO_H_SCROLL);
    if (!hscroll) throw new Error("horizontal page scroll ar");
    return `ar token-typed exec-enabled rtl no-hscroll`;
  });
  await step("shot-reset-confirm-ar", async () => shot("reset-confirm-ar-light.png"));

  await step("console-clean", async () => {
    if (CONSOLE_ERRORS.length > 0) throw new Error(CONSOLE_ERRORS.join(" | "));
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
