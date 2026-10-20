/**
 * TASK-DSN-079 visual verification: Settings per-tab permission gating.
 * Headless Chrome over CDP against vite :5173, Tauri IPC stubbed in-page
 * (built-ins only). Pattern: audit/DSN-078/capture.mjs.
 *
 * Session is Gerant-like (STOCK_ADJUSTMENT, no SYSTEM_SETTINGS_MANAGE,
 * no MANAGE_PRINTER_SETTINGS): settings route visible, fiscal/backups/
 * printer/identity tabs locked with badges, categories/pack/exchange open.
 * 4 captures: fiscal fr/light 1280, backups ar/dark 1280,
 * settings-nav fr/light 1024, settings-nav ar/dark 1024.
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
        role: 'ADMIN', permissions: ['VIEW_STOCK', 'POS_SELL', 'CREATE_SALE', 'VIEW_CUSTOMERS', 'SYSTEM_SETTINGS_MANAGE', 'STOCK_ADJUSTMENT', 'MANAGE_PRINTER_SETTINGS'],
        expires_at: '2030-01-01T00:00:00',
      });
    }
    if (cmd === 'auth_logout') return respond(null);
    if (cmd === 'warehouses_list') return respond({ warehouses: [{ id: 'dep-01', code: 'DEP-01', name: 'Dépôt Central', is_active: true }] });
    if (cmd === 'category_list') return respond([]);
    if (cmd === 'store_identity_get') return respond({ identity: { store_name: 'Q-Test', store_address: '', store_phone: '', nif: '', nis: '', rc: '', ai: '', ticket_footer_message: '' }, device_kind: 'physical' });
    if (cmd === 'cash_get_active_session') return respond(null);
    if (cmd === 'fiscal_settings_get') return respond({ timbreEnabled: true });
    if (cmd === 'backups_list') return respond([]);
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
    console.log(`  PASS ${name} :: ${String(detail ?? "").slice(0, 200)}`);
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
const NAV_EXPR = "(function(){var nav=document.querySelector('nav[aria-label]');if(!nav){return JSON.stringify({badges:-1,tabs:0});}return JSON.stringify({badges:nav.querySelectorAll('svg.ms-auto').length,tabs:nav.querySelectorAll('button').length});})()";

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
    await evalJs(cdp, fillExpr('input[autocomplete="username"]', "gerant"));
    await evalJs(cdp, fillExpr('input[autocomplete="current-password"]', "pw"));
    const label = await evalJs(cdp, `document.querySelector('button[type="submit"]').innerText.trim()`);
    await evalJs(cdp, clickTextExpr("button", label));
    await waitFor(cdp, `!!document.querySelector('nav')`, 15000, "nav after login");
    await evalJs(cdp, clickTextExpr("nav button", "Caisse"));
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
  async function openSettings(navLabel) {
    const r = await evalJs(cdp, clickTextExpr("nav button", navLabel));
    if (r !== "ok:1") throw new Error("settings nav entry missing: " + r);
    await waitFor(cdp, `!!document.querySelector('nav[aria-label]')`, 15000, "settings nav");
    await new Promise((r) => setTimeout(r, 800));
  }
  async function openTab(tabLabel) {
    const r = await evalJs(cdp, clickTextExpr("nav button", tabLabel));
    if (r !== "ok:1") throw new Error("tab missing: " + r);
    await new Promise((r) => setTimeout(r, 800));
  }
  async function shot(name) {
    const res = await cdp.send("Page.captureScreenshot", { format: "png" });
    const out = path.join(HERE, name);
    fs.writeFileSync(out, Buffer.from(res.data, "base64"));
    console.log(`  SHOT ${name} (${fs.statSync(out).size} bytes)`);
  }

  // C1: fiscal tab, FR/light, limited session → disabled switch + guidance.
  await viewport(1280, 800, true);
  await setDark(false);
  await setArabic(false);
  await openSettings("Paramètres");
  await step("C1 fiscal tab shows disabled switch with guidance (fr/light)", async () => {
    await openTab("Fiscalité");
    await waitFor(cdp, `!!document.querySelector('button[role="switch"]')`, 15000, "fiscal switch");
    await new Promise((r) => setTimeout(r, 500));
    const m = await evalJs(cdp, `(() => {
      const sw = document.querySelector('button[role="switch"]');
      const lay = ${LAYOUT_EXPR};
      return { disabled: sw.disabled, title: sw.getAttribute('title') || '', ...lay };
    })()`);
    if (!m.disabled) throw new Error("switch should be disabled for Gerant-like session");
    if (!m.title) throw new Error("switch guidance title missing");
    if (!m.noPageScroll) throw new Error(`page scroll sw=${m.sw} sh=${m.sh}`);
    return `disabled=true title="${m.title.slice(0, 40)}" ${m.w}x${m.h} dpr=${m.dpr} noPageScroll=true`;
  });
  await shot("settings-fiscal-1280x800-fr-light.png");

  // C2: backups tab, AR/dark, limited session → note + disabled create.
  await viewport(1280, 800, true);
  await setDark(true);
  await setArabic(true);
  await openSettings("الإعدادات");
  await step("C2 backups tab shows note + disabled create with guidance (ar/dark)", async () => {
    await openTab("النسخ الاحتياطي والأمان");
    await waitFor(cdp, `!![...document.querySelectorAll('button[disabled]')].find((b) => (b.getAttribute('title') || '').length > 0)`, 15000, "disabled guided button");
    await new Promise((r) => setTimeout(r, 500));
    const m = await evalJs(cdp, `(() => {
      const btns = [...document.querySelectorAll('button')];
      const create = btns.find((b) => (b.innerText ?? '').includes('إنشاء نسخة'));
      const lay = ${LAYOUT_EXPR};
      return { found: !!create, disabled: create ? create.disabled : null, title: create ? (create.getAttribute('title') || '') : '', ...lay };
    })()`);
    if (!m.found) throw new Error("disabled create button missing");
    if (!m.disabled) throw new Error("create should be disabled");
    if (!m.title) throw new Error("create guidance title missing");
    if (!m.noPageScroll) throw new Error(`page scroll sw=${m.sw} sh=${m.sh}`);
    return `disabled=true title="${m.title.slice(0, 30)}" ${m.w}x${m.h} dpr=${m.dpr} noPageScroll=true`;
  });
  await shot("settings-backups-1280x800-ar-dark.png");

  // C3/C4: settings nav with lock badges, FR/light 1024 + AR/dark 1024.
  for (const [w, h, ar, dark, navLabel] of [[1024, 600, false, false, "Paramètres"], [1024, 600, true, true, "الإعدادات"]]) {
    await viewport(w, h, true);
    await setDark(dark);
    await setArabic(ar);
    await openSettings(navLabel);
    const tag = `${w}x${h}-${ar ? "ar-dark" : "fr-light"}`;
    await step(`C-nav ${tag}: lock badges on restricted tabs, no page scroll`, async () => {
      const n = JSON.parse(await evalJs(cdp, NAV_EXPR));
      const lay = await evalJs(cdp, LAYOUT_EXPR);
      if (n.tabs < 8) throw new Error(`expected 8 tabs, got ${n.tabs}`);
      if (n.badges < 4) throw new Error(`expected >=4 lock badges (identity/printer/fiscal/backups), got ${n.badges}`);
      if (!lay.noPageScroll) throw new Error(`page scroll sw=${lay.sw} sh=${lay.sh}`);
      return `tabs=${n.tabs} badges=${n.badges} ${lay.w}x${lay.h} dpr=${lay.dpr} noPageScroll=true`;
    });
    await shot(`settings-nav-${w}x${h}-${ar ? "ar-dark" : "fr-light"}.png`);
  }

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
