/**
 * TASK-DSN-080 visual verification: warehouse-transfer permission gates.
 * Headless Chrome over CDP against vite :5173, Tauri IPC stubbed in-page
 * (built-ins only). Pattern: audit/DSN-079/capture.mjs.
 *
 * Sessions: ADMIN (TRANSFER_STOCK + MANAGE_WAREHOUSES, full) and LIMITED
 * (Gerant-like: TRANSFER_STOCK present, MANAGE_WAREHOUSES absent — the
 * creation/depot interior gates are UI-reachable; the no-TRANSFER_STOCK
 * execute interior is launcher-unreachable by design and pinned by tests).
 * 3 captures: transfer modal ADMIN fr/light 1280, modal LIMITED ar/dark
 * 1280 (creation gate), modal ADMIN fr/light 1024 spot check.
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
    if (cmd === 'auth_login') {
      if (window.__LIMITED__) return respond({
        session_token: 'stub-gerant', user_id: 'u-gerant', username: 'gerant',
        role: 'GERANT', permissions: ['VIEW_STOCK', 'POS_SELL', 'CREATE_SALE', 'VIEW_CUSTOMERS', 'STOCK_ADJUSTMENT', 'ADJUST_STOCK', 'TRANSFER_STOCK'],
        expires_at: '2030-01-01T00:00:00',
      });
      return respond({
        session_token: 'stub-admin', user_id: 'u-admin', username: 'admin',
        role: 'ADMIN', permissions: ['VIEW_STOCK', 'POS_SELL', 'CREATE_SALE', 'VIEW_CUSTOMERS', 'SYSTEM_SETTINGS_MANAGE', 'STOCK_ADJUSTMENT', 'TRANSFER_STOCK', 'MANAGE_WAREHOUSES', 'ADJUST_STOCK'],
        expires_at: '2030-01-01T00:00:00',
      });
    }
    if (cmd === 'auth_logout') return respond(null);
    if (cmd === 'warehouses_list') return respond({ warehouses: [{ id: 'dep-01', code: 'DEP-01', name: 'Dépôt Central', is_active: true }, { id: 'mag-01', code: 'MAG-01', name: 'Magasin', is_active: true }] });
    if (cmd === 'category_list') return respond([]);
    if (cmd === 'store_identity_get') return respond({ identity: { store_name: 'Q-Test', store_address: '', store_phone: '', nif: '', nis: '', rc: '', ai: '', ticket_footer_message: '' }, device_kind: 'physical' });
    if (cmd === 'cash_get_active_session') return respond(null);
    if (cmd === 'fiscal_settings_get') return respond({ timbreEnabled: true });
    if (cmd === 'backups_list') return respond([]);
    if (cmd === 'catalog_search') return respond([]);
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
    const full = JSON.stringify(res.exceptionDetails);
    console.log("  EVAL-EXPR: " + String(expression).slice(0, 120).replace(/\n/g, "\\n"));
    throw new Error("JS-FULL: " + full.slice(0, 900));
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
    console.log(`  PASS ${name} :: ${String(detail ?? "").slice(0, 220)}`);
  } catch (err) {
    RESULTS.push({ name, ok: false, detail: String(err?.message ?? err).slice(0, 300) });
    console.log(`  FAIL ${name} :: ${String(err?.message ?? err).slice(0, 200)}`);
  }
}

const LAYOUT_EXPR = `(() => {
  const de = document.documentElement;
  return {
    w: window.innerWidth, h: window.innerHeight, dpr: window.devicePixelRatio,
    sw: de.scrollWidth, sh: de.scrollHeight,
    noPageScroll: de.scrollWidth <= window.innerWidth && de.scrollHeight <= window.innerHeight,
  };
})()`;

let currentAr = false;
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

  async function viewport(w, h, limited) {
    await cdp.send("Emulation.setDeviceMetricsOverride", {
      width: w, height: h, deviceScaleFactor: 1, mobile: false,
    });
    await cdp.send("Page.navigate", { url: APP_URL });
    currentAr = false;
    await waitFor(cdp, `!!document.querySelector('button[type="submit"]')`, 30000, "login submit");
    await evalJs(cdp, STUB_SOURCE);
    await evalJs(cdp, `window.__LIMITED__ = ${limited ? "true" : "false"}`);
    await evalJs(cdp, fillExpr('input[autocomplete="username"]', limited ? "gerant" : "admin"));
    await evalJs(cdp, fillExpr('input[autocomplete="current-password"]', "pw"));
    await evalJs(cdp, `(function(){var f=document.querySelector('form');if(!f)return 'NOFORM';f.requestSubmit();return 'requested'})()`);
    await waitFor(cdp, `!!document.querySelector('nav')`, 15000, "nav after login");
    await waitFor(cdp, `!![...document.querySelectorAll('nav button')].find((b) => (b.innerText ?? '').trim() === 'Caisse')`, 15000, "caisse btn");
    const toCaisse = await evalJs(cdp, clickTextExpr("nav button", "Caisse"));
    if (toCaisse !== "ok:1") throw new Error("caisse entry missing: " + toCaisse);
    await waitFor(cdp, `!!document.querySelector('button[aria-label="Thème"]')`, 15000, "pos topbar");
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
        const b = [...document.querySelectorAll('button')].find((x) => (x.innerText ?? '').includes(${on ? "'العربية'" : "'Français'"}));
        if (!b) return 'MISSING-LOCALE-BTN';
        b.click(); return 'ok';
      })()`);
      if (btn !== "ok") throw new Error(btn);
      currentAr = on;
      await new Promise((r) => setTimeout(r, 500));
    }
  }
  async function openTransferModal(stockNav, launcher) {
    let r = await evalJs(cdp, clickTextExpr("nav button", stockNav));
    if (r !== "ok:1") throw new Error("stock nav missing: " + r);
    await waitFor(cdp, `document.body.innerText.includes(${JSON.stringify(launcher)})`, 15000, "stock view");
    r = await evalJs(cdp, clickTextExpr("button", launcher));
    if (!String(r).startsWith("ok:")) throw new Error("transfer launcher missing/disabled: " + r);
  }
  async function openTab(label) {
    const r = await evalJs(cdp, `(() => {
      const scope = document.querySelector('div.fixed');
      if (!scope) return 'MISSING-MODAL';
      const els = [...scope.querySelectorAll('button')].filter((e) => (e.innerText ?? '').trim() === ${JSON.stringify(label)});
      if (els.length === 0) return 'MISSING:' + ${JSON.stringify(label)};
      els[0].click();
      return 'ok:' + els.length;
    })()`);
    if (r !== "ok:1") throw new Error("tab missing: " + r);
    await new Promise((r) => setTimeout(r, 800));
  }
  async function shot(name) {
    const res = await cdp.send("Page.captureScreenshot", { format: "png" });
    const out = path.join(HERE, name);
    fs.writeFileSync(out, Buffer.from(res.data, "base64"));
    console.log(`  SHOT ${name} (${fs.statSync(out).size} bytes)`);
  }

  // C1: ADMIN fr/light — modal opens, transfer tab live, no permission lock.
  await viewport(1280, 800, false);
  await setDark(false);
  await setArabic(false);
  await openTransferModal("Stock & Dépôts", "Transfert Inter-Dépôts");
  await step("C1 authorized modal renders with unlocked execute (fr/light)", async () => {
    await waitFor(cdp, `document.body.innerText.includes('Gestion des Dépôts')`, 15000, "transfer modal");
    await new Promise((r) => setTimeout(r, 800));
    const m = await evalJs(cdp, `(() => {
      const btns = [...document.querySelectorAll('button')];
      const exec = btns.find((b) => (b.innerText ?? '').includes('Valider le Transfert'));
      const lay = ${LAYOUT_EXPR};
      return { found: !!exec, title: exec ? (exec.getAttribute('title') || null) : 'MISSING', ...lay };
    })()`);
    if (!m.found) throw new Error("execute button missing");
    if (m.title !== null) throw new Error("authorized execute should carry no guidance title, got: " + m.title);
    if (!m.noPageScroll) throw new Error(`page scroll sw=${m.sw} sh=${m.sh}`);
    return `execTitle=null ${m.w}x${m.h} dpr=${m.dpr} noPageScroll=true`;
  });
  await shot("transfer-modal-1280x800-fr-light.png");

  // C2: LIMITED ar/dark — create/depot gates disabled with guidance.
  await viewport(1280, 800, true);
  await setDark(true);
  await setArabic(true);
  await openTransferModal("المخزون والمستودعات", "تحويل بين المستودعات");
  await step("C2a depot tab shows management guidance note (ar/dark)", async () => {
    await waitFor(cdp, `document.body.innerText.includes('إدارة المستودعات')`, 15000, "transfer modal ar");
    await openTab("المستودعات");
    await waitFor(cdp, `document.body.innerText.includes('إنشاء المستودعات مخصص')`, 15000, "depot guidance note");
    const lay = await evalJs(cdp, LAYOUT_EXPR);
    if (!lay.noPageScroll) throw new Error(`page scroll sw=${lay.sw} sh=${lay.sh}`);
    return `note present ${lay.w}x${lay.h} dpr=${lay.dpr} noPageScroll=true`;
  });
  await step("C2b create-depot tab button disabled with guidance (ar/dark)", async () => {
    await openTab("تحويل بين المستودعات");
    const m = await evalJs(cdp, `(() => {
      const btn = [...document.querySelectorAll('button')].find((b) => (b.innerText ?? '').trim() === 'إضافة مستودع جديد');
      const lay = ${LAYOUT_EXPR};
      return { found: !!btn, disabled: btn ? btn.disabled : null, title: btn ? (btn.getAttribute('title') || '') : '', ...lay };
    })()`);
    if (!m.found) throw new Error("create tab button missing");
    if (!m.disabled) throw new Error("create tab should be disabled without MANAGE_WAREHOUSES");
    if (!m.title) throw new Error("create tab guidance title missing");
    if (!m.noPageScroll) throw new Error(`page scroll sw=${m.sw} sh=${m.sh}`);
    return `disabled=true title="${m.title.slice(0, 30)}" ${m.w}x${m.h} noPageScroll=true`;
  });
  await shot("transfer-modal-1280x800-ar-dark.png");

  // C3: 1024x600 fr/light ADMIN spot check.
  await viewport(1024, 600, false);
  await setDark(false);
  await setArabic(false);
  await openTransferModal("Stock & Dépôts", "Transfert Inter-Dépôts");
  await step("C3 modal at 1024x600 without page scroll (fr/light)", async () => {
    await waitFor(cdp, `document.body.innerText.includes('Gestion des Dépôts')`, 15000, "transfer modal 1024");
    await new Promise((r) => setTimeout(r, 800));
    const lay = await evalJs(cdp, LAYOUT_EXPR);
    if (!lay.noPageScroll) throw new Error(`page scroll sw=${lay.sw} sh=${lay.sh}`);
    return `${lay.w}x${lay.h} dpr=${lay.dpr} noPageScroll=true`;
  });
  await shot("transfer-modal-1024x600-fr-light.png");

  await step("zero console errors across captures", async () => {
    if (CONSOLE_ERRORS.length > 0) throw new Error(CONSOLE_ERRORS.join(" | ").slice(0, 300));
    return "0 console errors, all stubbed commands responded";
  });

  const failed = RESULTS.filter((r) => !r.ok);
  fs.writeFileSync(path.join(HERE, "capture-results.json"), JSON.stringify({ failed: failed.length, total: RESULTS.length, results: RESULTS }, null, 2));
  console.log(`\nCAPTURE DONE: ${RESULTS.length - failed.length}/${RESULTS.length} steps passed`);
  cdp.close();
  chrome.kill();
  if (failed.length > 0) process.exit(1);
}
main().catch((e) => { console.error("FATAL", e?.message ?? e); process.exit(2); });
