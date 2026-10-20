/**
 * TASK-DSN-081 visual verification: residual write-button gates.
 * Headless Chrome over CDP against vite :5173, Tauri IPC stubbed in-page
 * (built-ins only). Pattern: audit/DSN-080/capture.mjs (requestSubmit
 * login, bare-array catalog_search, scoped tab clicks).
 *
 * Sessions: ADMIN (full) and LIMITED (SA-only Gerant-like: STOCK_ADJUSTMENT
 * without SYSTEM_SETTINGS_MANAGE / MANAGE_PRINTER_SETTINGS).
 * 4 captures: identity fr/light 1280 (disabled+guidance), printer ar/dark
 * 1280 (disabled+guidance), category fr/light 1280 (authorized-enabled —
 * the disabled interior is route-unreachable by design and test-pinned),
 * settings nav ar/dark 1024 (badges).
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
        role: 'ADMIN', permissions: ['VIEW_STOCK', 'POS_SELL', 'CREATE_SALE', 'VIEW_CUSTOMERS', 'SYSTEM_SETTINGS_MANAGE', 'STOCK_ADJUSTMENT', 'MANAGE_WAREHOUSES', 'MANAGE_PRINTER_SETTINGS', 'ADJUST_STOCK', 'TRANSFER_STOCK'],
        expires_at: '2030-01-01T00:00:00',
      });
    }
    if (cmd === 'auth_logout') return respond(null);
    if (cmd === 'warehouses_list') return respond({ warehouses: [{ id: 'dep-01', code: 'DEP-01', name: 'Dépôt Central', is_active: true }, { id: 'mag-01', code: 'MAG-01', name: 'Magasin', is_active: true }] });
    if (cmd === 'category_list') return respond([]);
    if (cmd === 'store_identity_get') return respond({ identity: { store_name: 'Q-Test', store_address: '', store_phone: '0555123456', nif: '123456789012345', nis: '', rc: '', ai: '', ticket_footer_message: '' }, device_kind: 'physical' });
    if (cmd === 'printer_get_settings') return respond({ serial_port: 'COM1', serial_baud: 9600, paper_width_dots: 640, paper_width_mm: 80 });
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
    w: window.innerWidth, h: window.innerHeight, dpr: window.devicePixelRatio,
    sw: de.scrollWidth, sh: de.scrollHeight,
    noPageScroll: de.scrollWidth <= window.innerWidth && de.scrollHeight <= window.innerHeight,
  };
})()`;
const SETTINGS_NAV = `document.querySelector('nav[aria-label]')`;

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
  const page = tabs.filter((t) => t.type === "page").pop();
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
    await evalJs(cdp, `(function(){var el=document.querySelector('input[autocomplete="username"]');var p=Object.getPrototypeOf(el);Object.getOwnPropertyDescriptor(p,'value').set.call(el,'${limited ? "gerant" : "admin"}');el.dispatchEvent(new Event('input',{bubbles:true}));el.dispatchEvent(new Event('change',{bubbles:true}));return 1})()`);
    await evalJs(cdp, `(function(){var el=document.querySelector('input[autocomplete="current-password"]');var p=Object.getPrototypeOf(el);Object.getOwnPropertyDescriptor(p,'value').set.call(el,'pw');el.dispatchEvent(new Event('input',{bubbles:true}));el.dispatchEvent(new Event('change',{bubbles:true}));return 1})()`);
    await evalJs(cdp, `(function(){var f=document.querySelector('form');if(!f)return 'NOFORM';f.requestSubmit();return 'requested'})()`);
    await waitFor(cdp, `!!document.querySelector('nav')`, 15000, "nav after login");
    await waitFor(cdp, `!![...document.querySelectorAll('nav button')].find((b) => (b.innerText ?? '').trim() === 'Caisse')`, 15000, "caisse btn");
    const toCaisse = await evalJs(cdp, clickIn("document.querySelector('nav')", "Caisse"));
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
  async function setArabic(navLabel) {
    const btn = await evalJs(cdp, `(() => {
      const b = [...document.querySelectorAll('button')].find((x) => (x.innerText ?? '').includes('العربية'));
      if (!b) return 'MISSING-LOCALE-BTN';
      b.click(); return 'ok';
    })()`);
    if (btn !== "ok") throw new Error(btn);
    currentAr = true;
    await waitFor(cdp, `[...document.querySelectorAll('nav button')].some((b) => (b.innerText ?? '').trim() === ${JSON.stringify(navLabel)})`, 10000, "locale nav");
    await new Promise((r) => setTimeout(r, 500));
  }
  async function openSettings(navLabel) {
    const r = await evalJs(cdp, clickIn("document.querySelector('nav:not([aria-label])') || document", navLabel));
    if (r !== "ok:1") throw new Error("settings nav entry missing: " + r);
    await waitFor(cdp, `!!${SETTINGS_NAV}`, 15000, "settings nav");
    await new Promise((r) => setTimeout(r, 800));
  }
  async function openTab(label) {
    const r = await evalJs(cdp, clickIn(SETTINGS_NAV, label));
    if (r !== "ok:1") throw new Error("tab missing: " + r);
    await new Promise((r) => setTimeout(r, 800));
  }
  async function shot(name) {
    const res = await cdp.send("Page.captureScreenshot", { format: "png" });
    const out = path.join(HERE, name);
    fs.writeFileSync(out, Buffer.from(res.data, "base64"));
    console.log(`  SHOT ${name} (${fs.statSync(out).size} bytes)`);
  }
  async function guidedButton(label) {
    return evalJs(cdp, `(() => {
      const main = document.querySelector('main') || document.body;
      const btn = [...main.querySelectorAll('button')].find((b) => (b.innerText ?? '').trim() === ${JSON.stringify(label)});
      const lay = ${LAYOUT_EXPR};
      return { found: !!btn, disabled: btn ? btn.disabled : null, title: btn ? (btn.getAttribute('title') || '') : '', ...lay };
    })()`);
  }

  // C1: identity fr/light 1280 LIMITED — save disabled + FR guidance.
  await viewport(1280, 800, true);
  await setDark(false);
  await openSettings("Paramètres");
  await step("C1 identity save disabled with FR guidance (fr/light LIMITED)", async () => {
    await waitFor(cdp, `document.body.innerText.includes('Identité du Magasin')`, 15000, "identity tab");
    await new Promise((r) => setTimeout(r, 800));
    const m = await guidedButton("Enregistrer les modifications");
    if (!m.found) throw new Error("identity save button missing");
    if (!m.disabled) throw new Error("identity save should be disabled without SYSTEM_SETTINGS_MANAGE");
    if (!m.title) throw new Error("identity guidance title missing");
    if (!m.noPageScroll) throw new Error(`page scroll sw=${m.sw} sh=${m.sh}`);
    return `disabled=true title="${m.title.slice(0, 40)}" ${m.w}x${m.h} dpr=${m.dpr} noPageScroll=true`;
  });
  await shot("identity-1280x800-fr-light.png");

  // C2: printer ar/dark 1280 LIMITED — save + test disabled + AR guidance.
  await viewport(1280, 800, true);
  await setDark(true);
  await setArabic("الإعدادات");
  await openSettings("الإعدادات");
  await step("C2 printer save disabled with AR guidance (ar/dark LIMITED)", async () => {
    await openTab("الطباعة والملحقات");
    await waitFor(cdp, `document.body.innerText.includes('اختبار الطباعة')`, 15000, "printer tab");
    await new Promise((r) => setTimeout(r, 500));
    const m = await guidedButton("حفظ التغييرات");
    if (!m.found) throw new Error("printer save button missing");
    if (!m.disabled) throw new Error("printer save should be disabled without MANAGE_PRINTER_SETTINGS");
    if (!m.title) throw new Error("printer guidance title missing");
    if (!m.noPageScroll) throw new Error(`page scroll sw=${m.sw} sh=${m.sh}`);
    return `disabled=true title="${m.title.slice(0, 30)}" ${m.w}x${m.h} noPageScroll=true`;
  });
  await shot("printer-1280x800-ar-dark.png");

  // C3: category fr/light 1280 ADMIN — authorized unchanged (enabled, no title).
  await viewport(1280, 800, false);
  await setDark(false);
  await openSettings("Paramètres");
  await step("C3 category submit enabled without guidance (fr/light ADMIN)", async () => {
    await openTab("Rayons & Catégories");
    await waitFor(cdp, `document.body.innerText.includes('Enregistrer') || document.body.innerText.includes('Installer')`, 15000, "category panel");
    await new Promise((r) => setTimeout(r, 500));
    const m = await evalJs(cdp, `(() => {
      const main = document.querySelector('main') || document.body;
      const btn = [...main.querySelectorAll('button')].find((b) => (b.innerText ?? '').trim() === 'Enregistrer');
      const lay = ${LAYOUT_EXPR};
      return { found: !!btn, disabled: btn ? btn.disabled : null, title: btn ? (btn.getAttribute('title') || null) : 'MISSING', ...lay };
    })()`);
    if (!m.found) throw new Error("category submit button missing");
    if (m.disabled) throw new Error("authorized category submit should be enabled");
    if (m.title !== null) throw new Error("authorized submit should carry no guidance title, got: " + m.title);
    if (!m.noPageScroll) throw new Error(`page scroll sw=${m.sw} sh=${m.sh}`);
    return `enabled title=null ${m.w}x${m.h} noPageScroll=true`;
  });
  await shot("category-1280x800-fr-light.png");

  // C4: settings nav ar/dark 1024 LIMITED — badges + no scroll.
  await viewport(1024, 600, true);
  await setDark(true);
  await setArabic("الإعدادات");
  await openSettings("الإعدادات");
  await step("C4 nav badges ar/dark 1024 (LIMITED)", async () => {
    const n = JSON.parse(await evalJs(cdp, `(function(){var nav=${SETTINGS_NAV};if(!nav){return JSON.stringify({badges:-1,tabs:0});}return JSON.stringify({badges:nav.querySelectorAll('svg.ms-auto').length,tabs:nav.querySelectorAll('button').length});})()`));
    const lay = await evalJs(cdp, LAYOUT_EXPR);
    if (n.tabs < 8) throw new Error(`expected 8 tabs, got ${n.tabs}`);
    if (n.badges < 4) throw new Error(`expected >=4 lock badges, got ${n.badges}`);
    if (!lay.noPageScroll) throw new Error(`page scroll sw=${lay.sw} sh=${lay.sh}`);
    return `tabs=${n.tabs} badges=${n.badges} ${lay.w}x${lay.h} dpr=${lay.dpr} noPageScroll=true`;
  });
  await shot("settings-nav-1024x600-ar-dark.png");

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
