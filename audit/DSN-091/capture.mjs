/**
 * TASK-DSN-091 visual verification: printer transport selection UI.
 * Headless Chrome over CDP against vite :5173, Tauri IPC stubbed in-page.
 *
 * Flow fr-light: ADMIN login -> Paramètres nav -> "Impression & Matériel"
 * tab -> segmented asserts (serial pressed, COM visible, queue hidden) ->
 * click "USB direct" -> COM hidden, queue visible -> empty save blocked
 * (inline message, no IPC) -> fill "Xprinter XP-410B" -> save (payload
 * assert) -> shot 1. ar-dark: locale+theme switch -> printer tab ->
 * serial selected asserts -> shot 2. 0 console errors throughout.
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
    if (cmd === 'store_identity_get') return respond({ identity: { store_name: 'Q-Test', store_address: '', store_phone: '0555123456', nif: '123456789012345', nis: '', rc: '', ai: '', ticket_footer_message: '' }, device_kind: 'physical' });
    if (cmd === 'printer_get_settings') return respond({ serial_port: 'COM1', serial_baud: 9600, paper_width_dots: 640, paper_width_mm: 80, transport: 'serial', queue_name: '' });
    if (cmd === 'printer_update_settings') {
      window.__SAVED__ = window.__SAVED__ || [];
      window.__SAVED__.push(JSON.parse(JSON.stringify(req)));
      return respond({ serial_port: req.serial_port, serial_baud: req.serial_baud, paper_width_dots: req.paper_width_mm === 58 ? 384 : 640, paper_width_mm: req.paper_width_mm, transport: req.transport || 'serial', queue_name: (req.queue_name || '').trim() });
    }
    if (cmd === 'printing_test_print_fr') return respond({ deviceOutcome: 'CAPTURED', captureFileName: 'stub-fr.png' });
    if (cmd === 'printing_test_print_ar') return respond({ deviceOutcome: 'CAPTURED', captureFileName: 'stub-ar.png' });
    if (cmd === 'cash_get_active_session') return respond(null);
    if (cmd === 'fiscal_settings_get') return respond({ timbreEnabled: true });
    if (cmd === 'backups_list') return respond([]);
    if (cmd === 'catalog_search') return respond([]);
    if (cmd === 'documents_search') return respond({ documents: [] });
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

const rectOf = (label, selector) => `(() => {
  const el = document.querySelector(${JSON.stringify(selector)});
  if (!el) return 'MISSING:${label}';
  const r = el.getBoundingClientRect();
  const vw = window.innerWidth, vh = window.innerHeight;
  const inView = r.left >= -1 && r.top >= -1 && r.right <= vw + 1 && r.bottom <= vh + 1 && r.width > 0 && r.height > 0;
  return (inView ? 'RECT-OK:${label} ' : 'RECT-CLIPPED:${label} ') + JSON.stringify({ l: Math.round(r.left), t: Math.round(r.top), r: Math.round(r.right), b: Math.round(r.bottom) });
})()`;

const hasExactButton = (text) => `[...document.querySelectorAll('button')].some((b) => (b.innerText ?? '').trim() === ${JSON.stringify(text)})`;
const hasText = (t) => `document.body.innerText.includes(${JSON.stringify(t)})`;

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

  const setViewport = async (w, h) => {
    await cdp.send("Emulation.setDeviceMetricsOverride", {
      width: w, height: h, deviceScaleFactor: 1, mobile: false,
    });
  };
  const login = async () => {
    await cdp.send("Page.navigate", { url: APP_URL });
    await waitFor(cdp, `!!document.querySelector('button[type="submit"]')`, 60000, "login submit");
    await evalJs(cdp, STUB_SOURCE);
    await evalJs(cdp, `(function(){var el=document.querySelector('input[autocomplete="username"]');var p=Object.getPrototypeOf(el);Object.getOwnPropertyDescriptor(p,'value').set.call(el,'admin');el.dispatchEvent(new Event('input',{bubbles:true}));el.dispatchEvent(new Event('change',{bubbles:true}));return 1})()`);
    await evalJs(cdp, `(function(){var el=document.querySelector('input[autocomplete="current-password"]');var p=Object.getPrototypeOf(el);Object.getOwnPropertyDescriptor(p,'value').set.call(el,'pw');el.dispatchEvent(new Event('input',{bubbles:true}));el.dispatchEvent(new Event('change',{bubbles:true}));return 1})()`);
    await evalJs(cdp, `(function(){var f=document.querySelector('form');if(!f)return 'NOFORM';f.requestSubmit();return 'requested'})()`);
    await waitFor(cdp, `!!document.querySelector('header')`, 15000, "strip after login");
  };
  const gotoPrinterTab = async (navLabel, tabLabel) => {
    const nav = await evalJs(cdp, clickButton("document", navLabel));
    if (nav !== "ok:1") throw new Error("nav: " + nav);
    await waitFor(cdp, hasExactButton(tabLabel), 10000, "printer tab visible");
    const tab = await evalJs(cdp, clickButton("document", tabLabel));
    if (tab !== "ok:1") throw new Error("tab: " + tab);
    await waitFor(cdp, hasText("Transport d") + " || " + hasText("النقل"), 8000, "transport group");
  };

  // ---- 1280x800 fr-light (spooler arm) ----
  await setViewport(1280, 800);
  await login();
  await gotoPrinterTab("Paramètres", "Impression & Matériel");

  await step("fr-serial-default", async () => {
    await waitFor(cdp, hasExactButton("Série COM"), 8000, "segmented control");
    const pressed = await evalJs(cdp, `(() => {
      const btns = [...document.querySelectorAll('button')].filter((b) => ['Série COM', 'USB direct'].includes((b.innerText ?? '').trim()));
      return btns.map((b) => (b.innerText ?? '').trim() + '=' + b.getAttribute('aria-pressed')).join(',');
    })()`);
    if (pressed !== "Série COM=true,USB direct=false") throw new Error("pressed: " + pressed);
    const comVisible = await evalJs(cdp, `(() => {
      const input = [...document.querySelectorAll('input')].find((i) => i.value === 'COM1');
      return input ? 'com-ok' : 'NO-COM-VALUE';
    })()`);
    if (comVisible !== "com-ok") throw new Error("COM field not visible on serial arm");
    const queueAbsent = await evalJs(cdp, `[...document.querySelectorAll('input')].some((i) => (i.placeholder ?? '').includes('Xprinter')) ? 'QUEUE-LEAK' : 'queue-hidden-ok'`);
    if (queueAbsent !== "queue-hidden-ok") throw new Error("queue field leaks onto serial arm");
    return pressed + " com-visible queue-hidden";
  });

  await step("fr-switch-spooler", async () => {
    const clicked = await evalJs(cdp, clickButton("document", "USB direct"));
    if (clicked !== "ok:1") throw new Error("click: " + clicked);
    await waitFor(cdp, hasText("File d’attente"), 5000, "queue field appears");
    const pressed = await evalJs(cdp, `(() => {
      const btns = [...document.querySelectorAll('button')].filter((b) => ['Série COM', 'USB direct'].includes((b.innerText ?? '').trim()));
      return btns.map((b) => (b.innerText ?? '').trim() + '=' + b.getAttribute('aria-pressed')).join(',');
    })()`);
    if (pressed !== "Série COM=false,USB direct=true") throw new Error("pressed: " + pressed);
    return pressed + " queue-visible";
  });

  await step("fr-empty-save-blocked", async () => {
    const before = await evalJs(cdp, `(window.__SAVED__ ?? []).length`);
    const saved = await evalJs(cdp, clickButton("document", "Enregistrer les modifications"));
    if (saved !== "ok:1") throw new Error("save click: " + saved);
    await waitFor(cdp, hasText("Nom de file requis"), 5000, "inline required message");
    const after = await evalJs(cdp, `(window.__SAVED__ ?? []).length`);
    if (after !== before) throw new Error("IPC fired on empty queue");
    return "blocked-ok no-ipc";
  });

  await step("fr-fill-and-save", async () => {
    const typed = await evalJs(cdp, `(() => {
      const input = [...document.querySelectorAll('input')].find((i) => (i.placeholder ?? '').includes('Xprinter'));
      if (!input) return 'NO-QUEUE-INPUT';
      var p = Object.getPrototypeOf(input);
      Object.getOwnPropertyDescriptor(p, 'value').set.call(input, 'Xprinter XP-410B');
      input.dispatchEvent(new Event('input', { bubbles: true }));
      input.dispatchEvent(new Event('change', { bubbles: true }));
      return 'typed';
    })()`);
    if (typed !== "typed") throw new Error(typed);
    const saved = await evalJs(cdp, clickButton("document", "Enregistrer les modifications"));
    if (saved !== "ok:1") throw new Error("save click: " + saved);
    await waitFor(cdp, hasText("Paramètres enregistrés"), 8000, "saved notice");
    const payload = await evalJs(cdp, `JSON.stringify((window.__SAVED__ ?? []).pop() ?? null)`);
    const parsed = JSON.parse(payload);
    if (!parsed || parsed.transport !== "spooler" || parsed.queue_name !== "Xprinter XP-410B") {
      throw new Error("payload: " + payload);
    }
    return "payload-ok " + payload;
  });

  await step("fr-light-asserts", async () => {
    const layout = await evalJs(cdp, LAYOUT_EXPR);
    if (!layout.noPageScroll) throw new Error("page scroll at 1280x800");
    const group = await evalJs(cdp, rectOf("transport-group", '[role="group"]'));
    if (!group.startsWith("RECT-OK")) throw new Error(group);
    const queue = await evalJs(cdp, `(() => {
      const input = [...document.querySelectorAll('input')].find((i) => (i.placeholder ?? '').includes('Xprinter'));
      if (!input) return 'MISSING:queue-input';
      const r = input.getBoundingClientRect();
      return 'RECT-OK:queue-input ' + JSON.stringify({ l: Math.round(r.left), t: Math.round(r.top), r: Math.round(r.right), b: Math.round(r.bottom) });
    })()`);
    if (!queue.startsWith("RECT-OK")) throw new Error(queue);
    return group + " " + queue + " noPageScroll";
  });
  await step("shot-fr-light", async () => {
    const shot = await cdp.send("Page.captureScreenshot", { format: "png" });
    fs.writeFileSync(path.join(HERE, "printer-1280x800-fr-light.png"), Buffer.from(shot.data, "base64"));
    return "printer-1280x800-fr-light.png";
  });

  // ---- ar-dark (serial arm) ----
  await step("switch-ar-dark", async () => {
    // Theme/lang controls live in the POS Topbar header (absent from the
    // settings view), so detour through Caisse first (theme-first order).
    const pos = await evalJs(cdp, clickButton("document", "Caisse"));
    if (pos !== "ok:1") throw new Error("pos nav: " + pos);
    await waitFor(cdp, `!!document.querySelector('header')`, 10000, "pos strip");
    const theme = await evalJs(cdp, `document.querySelector('button[aria-label="Thème"]') ? (document.querySelector('button[aria-label="Thème"]').click(), 'ok') : 'NO-THEME'`);
    if (theme !== "ok") throw new Error("theme: " + theme);
    await waitFor(cdp, `document.documentElement.classList.contains('dark') === true`, 8000, "dark");
    const lang = await evalJs(cdp, `(() => {
      const b = [...document.querySelectorAll('button')].find((x) => (x.textContent ?? '').trim() === 'العربية');
      if (!b) return 'MISSING-LOCALE-BTN';
      b.click(); return 'ok';
    })()`);
    if (lang !== "ok") throw new Error("lang: " + lang);
    await waitFor(cdp, hasText("الإعدادات"), 8000, "arabic nav");
    await new Promise((r) => setTimeout(r, 600));
    return "ar-dark-ok";
  });
  await gotoPrinterTab("الإعدادات", "الطباعة والملحقات");

  await step("ar-serial-asserts", async () => {
    const pressed = await evalJs(cdp, `(() => {
      const btns = [...document.querySelectorAll('button')].filter((b) => ['سلسلة COM', 'USB مباشر'].includes((b.innerText ?? '').trim()));
      return btns.map((b) => (b.innerText ?? '').trim() + '=' + b.getAttribute('aria-pressed')).join(',');
    })()`);
    if (pressed !== "سلسلة COM=true,USB مباشر=false") throw new Error("pressed: " + pressed);
    const dir = await evalJs(cdp, `(() => {
      const root = document.querySelector('div[lang]');
      const d = root ? root.getAttribute('dir') : document.documentElement.dir;
      return d === 'rtl' ? 'rtl-ok' : 'NOT-RTL:' + d;
    })()`);
    if (dir !== "rtl-ok") throw new Error(dir);
    const layout = await evalJs(cdp, LAYOUT_EXPR);
    if (!layout.noPageScroll) throw new Error("page scroll ar-dark");
    const group = await evalJs(cdp, rectOf("transport-group", '[role="group"]'));
    if (!group.startsWith("RECT-OK")) throw new Error(group);
    return pressed + " rtl-ok noPageScroll " + group;
  });
  await step("shot-ar-dark", async () => {
    const shot = await cdp.send("Page.captureScreenshot", { format: "png" });
    fs.writeFileSync(path.join(HERE, "printer-1280x800-ar-dark.png"), Buffer.from(shot.data, "base64"));
    return "printer-1280x800-ar-dark.png";
  });

  const failed = RESULTS.filter((r) => !r.ok);
  const summary = {
    green: failed.length === 0 && CONSOLE_ERRORS.length === 0,
    results: RESULTS,
    consoleErrors: CONSOLE_ERRORS,
  };
  fs.writeFileSync(path.join(HERE, "capture-results.json"), JSON.stringify(summary, null, 2));
  console.log(`\nCAPTURE ${summary.green ? "GREEN" : "RED"} ${RESULTS.length - failed.length}/${RESULTS.length} consoleErrors=${CONSOLE_ERRORS.length}`);
  try { cdp.close(); } catch { /* noop */ }
  try { chrome.kill(); } catch { /* noop */ }
  process.exit(summary.green ? 0 : 1);
}

main().catch((e) => { console.error("FATAL", e); process.exit(2); });
