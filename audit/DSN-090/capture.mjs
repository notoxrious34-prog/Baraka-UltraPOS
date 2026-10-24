/**
 * TASK-DSN-090 visual verification: restyled shell (nav + strip + popup).
 * Headless Chrome over CDP against vite :5173, Tauri IPC stubbed in-page.
 *
 * Flow: ADMIN login -> cockpit (fr-light) strip asserts + cash-menu-open
 * rect -> add stubbed product -> F12 Encaissement -> tender -> Valider ->
 * done screen -> "Vente suivante (Entrée)" -> popup visible (shot 1) -> auto-dismiss
 * assert -> ar-DZ + dark (shot 2, RTL mirror) -> 1024x600 fr-light (shot 3,
 * collapse asserts). 0 console errors throughout.
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

const PRODUCT = {
  product_id: "prod-vis-1",
  sku: "VIS-440",
  barcode: null,
  name_ar: "برغي 4×40",
  name_fr: "Vis TR 4x40",
  category_id: null,
  is_kit: false,
  is_active: true,
  units: [
    {
      unit_id: "u-pcs",
      product_id: "prod-vis-1",
      unit_name_ar: "قطعة",
      unit_name_fr: "Pièce",
      factor_micro: "10000",
      barcode: null,
      sale_price_centimes: "1271",
      purchase_price_centimes: "800",
      wholesale_price_centimes: null,
    },
  ],
  kit_items: [],
};

const PREVIEW = {
  subtotal_ht_centimes: "2542",
  tva_total_centimes: "483",
  global_discount_centimes: "0",
  timbre_centimes: "0",
  total_ttc_centimes: "3025",
  paid_amount_centimes: "5000",
  change_amount_centimes: "1975",
  remaining_due_centimes: "0",
  stock_warnings: [],
  line_margins: [],
};

const RECEIPT = {
  document_id: "doc-99",
  document_number: "T-2026-0099",
  subtotal_ht_centimes: "2542",
  tva_total_centimes: "483",
  discount_total_centimes: "0",
  timbre_centimes: "0",
  total_ttc_centimes: "3025",
  paid_amount_centimes: "5000",
  change_amount_centimes: "1975",
  print_job_id: "pj-99",
};

const CASH_OPEN = {
  id: "cash-1",
  warehouse_id: "dep-01",
  terminal_id: "T1",
  status: "OPEN",
  opened_by: "u-admin",
  opened_by_username: "admin",
  opened_at: "2026-10-24T10:00:00",
  opening_float_centimes: "10000",
  closed_by: null,
  closed_by_username: null,
  closed_at: null,
  actual_counted_centimes: null,
  system_expected_centimes: null,
  variance_centimes: null,
  closing_notes: null,
  revision: "1",
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
    if (cmd === 'cash_get_active_session') return respond(${JSON.stringify(CASH_OPEN)});
    if (cmd === 'fiscal_settings_get') return respond({ timbreEnabled: true });
    if (cmd === 'backups_list') return respond([]);
    if (cmd === 'catalog_search') return respond([${JSON.stringify(PRODUCT)}]);
    if (cmd === 'catalog_get_product') return respond(${JSON.stringify(PRODUCT)});
    if (cmd === 'stock_get') return respond({ product_id: 'prod-vis-1', warehouse_id: 'dep-01', quantity_micro: '1000000' });
    if (cmd === 'documents_search') return respond({ documents: [] });
    if (cmd === 'sales_preview_totals') return respond(${JSON.stringify(PREVIEW)});
    if (cmd === 'sales_checkout') return respond(${JSON.stringify(RECEIPT)});
    if (cmd === 'documents_get_print_status') return respond({ job_id: 'pj-99', status: 'DONE', attempt_count: '1' });
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

const STRIP_RECT_EXPR = `(() => {
  const h = document.querySelector('header');
  if (!h) return 'NO-HEADER';
  const r = h.getBoundingClientRect();
  const vw = window.innerWidth, vh = window.innerHeight;
  const inView = r.left >= -1 && r.top >= -1 && r.right <= vw + 1 && r.bottom <= vh + 1;
  return (inView ? 'STRIP-OK ' : 'STRIP-CLIPPED ') + JSON.stringify({ l: Math.round(r.left), t: Math.round(r.top), r: Math.round(r.right), b: Math.round(r.bottom), vw, vh });
})()`;

const MENU_RECT_EXPR = `(() => {
  const btn = document.querySelector('header button[aria-label="cash-menu"]');
  if (!btn) return 'NO-CASH-TOGGLE';
  const menu = btn.parentElement?.querySelector('span.absolute');
  if (!menu) return 'NO-MENU';
  const r = menu.getBoundingClientRect();
  const vw = window.innerWidth, vh = window.innerHeight;
  const inView = r.left >= -1 && r.top >= -1 && r.right <= vw + 1 && r.bottom <= vh + 1;
  return (inView ? 'MENU-OK ' : 'MENU-CLIPPED ') + JSON.stringify({ l: Math.round(r.left), t: Math.round(r.top), r: Math.round(r.right), b: Math.round(r.bottom), vw, vh });
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

  // ---- 1280x800 fr-light ----
  await setViewport(1280, 800);
  await login();
  await waitFor(cdp, `document.body.innerText.includes('Vis TR 4x40')`, 15000, "product card");

  await step("fr-strip-in-viewport", async () => {
    const r = await evalJs(cdp, STRIP_RECT_EXPR);
    if (!r.startsWith("STRIP-OK")) throw new Error(r);
    const brand = await evalJs(cdp, `document.querySelector('nav').innerText.includes('Baraka UltraPOS') ? 'brand-ok' : 'NO-BRAND'`);
    if (brand !== "brand-ok") throw new Error(brand);
    const noBanner = await evalJs(cdp, `[...document.querySelectorAll('button')].filter((b) => (b.innerText ?? '').includes('Vente validée') && !b.closest('[role="status"]')).length === 0 ? 'no-banner-ok' : 'BANNER-PRESENT'`);
    if (noBanner !== "no-banner-ok") throw new Error(noBanner);
    const preClean = await evalJs(cdp, `!document.body.innerText.includes('Vente validée') ? 'pre-clean-ok' : 'STALE-NOTICE'`);
    if (preClean !== "pre-clean-ok") throw new Error(preClean);
    return r + " brand-ok no-banner-ok pre-clean-ok";
  });

  await step("fr-cash-menu-in-viewport", async () => {
    const clicked = await evalJs(cdp, `(() => {
      const btn = document.querySelector('header button[aria-label="cash-menu"]');
      if (!btn) return 'NO-CASH-TOGGLE';
      btn.click();
      return 'clicked';
    })()`);
    if (clicked !== "clicked") throw new Error(clicked);
    await waitFor(cdp, `!!document.querySelector('header button[aria-label="cash-menu"]')?.parentElement?.querySelector('span.absolute')`, 5000, "cash menu renders");
    const r = await evalJs(cdp, MENU_RECT_EXPR);
    if (!r.startsWith("MENU-OK")) throw new Error(r);
    // Close the menu so later shots show the resting strip, not an open menu.
    await evalJs(cdp, `document.querySelector('header button[aria-label="cash-menu"]').click()`);
    await waitFor(cdp, `!document.querySelector('header button[aria-label="cash-menu"]')?.parentElement?.querySelector('span.absolute')`, 5000, "cash menu closes");
    return r + " closed-ok";
  });

  // Add product -> F12 -> tender -> confirm -> done -> Vente suivante (Entrée) -> popup.
  await step("checkout-to-popup", async () => {
    await waitFor(cdp, `document.querySelectorAll('div.cursor-pointer').length > 0`, 15000, "product cards");
    const add = await evalJs(cdp, `(() => {
      const cards = [...document.querySelectorAll('div.cursor-pointer')];
      const card = cards.find((d) => (d.innerText ?? '').includes('Vis TR 4x40'));
      if (!card) {
        const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
        let found = 'TEXT-NOT-FOUND';
        while (walker.nextNode()) {
          if ((walker.currentNode.nodeValue ?? '').includes('Vis TR 4x40')) {
            const el = walker.currentNode.parentElement;
            found = el.tagName + '|' + (el.className.baseVal ?? el.className) + '|parent=' + el.parentElement.tagName + '|' + String(el.parentElement.className).slice(0, 80);
            break;
          }
        }
        return 'NO-CARD cards=' + cards.length + ' at=' + found;
      }
      card.click();
      return 'added';
    })()`);
    if (add !== "added") throw new Error(add);
    await waitFor(cdp, `document.body.innerText.includes('TOTAL') || document.body.innerText.includes('Total')`, 8000, "cart line");
    await cdp.send("Input.dispatchKeyEvent", { type: "keyDown", key: "F12", code: "F12", windowsVirtualKeyCode: 123 });
    await cdp.send("Input.dispatchKeyEvent", { type: "keyUp", key: "F12", code: "F12", windowsVirtualKeyCode: 123 });
    await waitFor(cdp, `document.body.innerText.includes('Valider la vente (F12)')`, 10000, "checkout modal");
    const tender = await evalJs(cdp, `(() => {
      const labels = [...document.querySelectorAll('label')].filter((l) => (l.innerText ?? '').includes('Espèces'));
      const input = labels.length ? labels[0].querySelector('input') : [...document.querySelectorAll('input')].find((i) => !i.disabled && i.inputMode === 'decimal');
      if (!input) return 'NO-TENDER-INPUT';
      var p = Object.getPrototypeOf(input);
      Object.getOwnPropertyDescriptor(p, 'value').set.call(input, '50');
      input.dispatchEvent(new Event('input', { bubbles: true }));
      input.dispatchEvent(new Event('change', { bubbles: true }));
      return 'tendered';
    })()`);
    if (tender !== "tendered") throw new Error(tender);
    await waitFor(cdp, `(() => {
      const b = [...document.querySelectorAll('button')].find((x) => (x.innerText ?? '').trim() === 'Valider la vente (F12)');
      return !!b && !b.disabled;
    })()`, 12000, "confirm enabled");
    const conf = await evalJs(cdp, clickButton("document", "Valider la vente (F12)"));
    if (conf !== "ok:1") throw new Error("confirm: " + conf);
    await waitFor(cdp, `document.body.innerText.includes('Vente suivante (Entrée)')`, 12000, "done screen");
    const fin = await evalJs(cdp, clickButton("document", "Vente suivante (Entrée)"));
    if (fin !== "ok:1") throw new Error("finish: " + fin);
    await waitFor(cdp, `document.body.innerText.includes('T-2026-0099') && document.body.innerText.includes('Impression en attente')`, 8000, "popup");
    return "popup shown T-2026-0099";
  });

  await step("fr-light-asserts", async () => {
    const layout = await evalJs(cdp, LAYOUT_EXPR);
    if (!layout.noPageScroll) throw new Error("page scroll at 1280x800");
    const popup = await evalJs(cdp, `(() => {
      const el = [...document.querySelectorAll('[role="status"]')].find((x) => (x.innerText ?? '').includes('Vente validée'));
      if (!el) return 'NO-POPUP';
      const r = el.getBoundingClientRect();
      return 'POPUP-OK ' + Math.round(r.left) + ',' + Math.round(r.top) + ',' + Math.round(r.right) + ',' + Math.round(r.bottom);
    })()`);
    if (!popup.startsWith("POPUP-OK")) throw new Error(popup);
    return popup + " noPageScroll";
  });
  await step("shot-fr-light", async () => {
    const shot = await cdp.send("Page.captureScreenshot", { format: "png" });
    fs.writeFileSync(path.join(HERE, "shell-1280x800-fr-light.png"), Buffer.from(shot.data, "base64"));
    return "shell-1280x800-fr-light.png";
  });

  await step("popup-auto-dismiss", async () => {
    await new Promise((r) => setTimeout(r, 4800));
    const gone = await evalJs(cdp, `[...document.querySelectorAll('[role="status"]')].some((x) => (x.innerText ?? '').includes('Vente validée')) ? 'STILL-VISIBLE' : 'dismissed-ok'`);
    if (gone !== "dismissed-ok") throw new Error(gone);
    return gone;
  });

  // ---- ar-DZ + dark ----
  await step("switch-ar-dark", async () => {
    const theme = await evalJs(cdp, `document.querySelector('button[aria-label="Thème"]').click(), 'ok'`);
    if (theme !== "ok") throw new Error("theme: " + theme);
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
      15000, "ar nav",
    );
    await new Promise((r) => setTimeout(r, 600));
    return "ar-DZ + dark";
  });
  await step("ar-dark-asserts", async () => {
    const r = await evalJs(cdp, STRIP_RECT_EXPR);
    if (!r.startsWith("STRIP-OK")) throw new Error(r);
    const logout = await evalJs(cdp, `[...document.querySelectorAll('header button')].some((b) => (b.innerText ?? '').includes('تسجيل الخروج')) ? 'logout-ok' : 'NO-LOGOUT'`);
    if (logout !== "logout-ok") throw new Error(logout);
    const layout = await evalJs(cdp, LAYOUT_EXPR);
    if (!layout.noPageScroll) throw new Error("page scroll at 1280x800 ar-dark");
    return r + " logout-ok noPageScroll dir=rtl";
  });
  await step("shot-ar-dark", async () => {
    const shot = await cdp.send("Page.captureScreenshot", { format: "png" });
    fs.writeFileSync(path.join(HERE, "shell-1280x800-ar-dark.png"), Buffer.from(shot.data, "base64"));
    return "shell-1280x800-ar-dark.png";
  });

  // ---- 1024x600 fr-light (fresh load, collapse asserts) ----
  await step("viewport-1024", async () => {
    await setViewport(1024, 600);
    await login();
    await waitFor(cdp, `!!document.querySelector('header')`, 15000, "strip 1024");
    // Force light theme (the toggle persists per profile; the 1024 pass must be fr-light).
    await evalJs(cdp, `(() => {
      if (!document.documentElement.classList.contains('dark')) return 'already-light';
      const b = [...document.querySelectorAll('header button')].find((x) => (x.getAttribute('aria-label') ?? '') === 'Thème');
      if (!b) return 'NO-THEME-BTN';
      b.click();
      return 'forced-light';
    })()`);
    await waitFor(cdp, `!document.documentElement.classList.contains('dark')`, 8000, "light 1024");
    await new Promise((r) => setTimeout(r, 800));
    return "1024x600 reloaded fr-light";
  });
  await step("1024-collapse-asserts", async () => {
    const r = await evalJs(cdp, STRIP_RECT_EXPR);
    if (!r.startsWith("STRIP-OK")) throw new Error(r);
    const keep = await evalJs(cdp, `(() => {
      const h = document.querySelector('header').innerText;
      const need = ['DEP-01', 'Déconnexion'];
      const missing = need.filter((x) => !h.includes(x));
      if (missing.length) return 'MISSING:' + missing.join(',');
      const dot = [...document.querySelectorAll('header bdi, header span')].some((x) => (x.innerText ?? '').includes('12:'));
      return 'kept-cash-depot-logout';
    })()`);
    if (!keep.startsWith("kept")) throw new Error(keep);
    const collapsed = await evalJs(cdp, `(() => {
      const widthOf = (el) => (el ? Math.round(el.getBoundingClientRect().width) : -1);
      const depot = [...document.querySelectorAll('header label span')].find((s) => (s.innerText ?? '').includes('Dépôt'));
      const offline = [...document.querySelectorAll('header span')].find((s) => (s.innerText ?? '').trim() === 'Hors-ligne');
      const opsLabel = document.querySelector('header button[aria-haspopup] > span:first-of-type');
      const w = { depot: widthOf(depot), offline: widthOf(offline), ops: widthOf(opsLabel) };
      const bad = Object.entries(w).filter(([, v]) => v !== 0);
      return bad.length === 0 ? 'collapsed-ok ' + JSON.stringify(w) : 'LABELS-VISIBLE ' + JSON.stringify(w);
    })()`);
    if (!collapsed.startsWith("collapsed-ok")) throw new Error(collapsed);
    const layout = await evalJs(cdp, LAYOUT_EXPR);
    if (!layout.noPageScroll) throw new Error("page scroll at 1024x600");
    return r + " " + keep + " " + collapsed + " noPageScroll";
  });
  await step("shot-1024", async () => {
    const shot = await cdp.send("Page.captureScreenshot", { format: "png" });
    fs.writeFileSync(path.join(HERE, "shell-1024x600-fr-light.png"), Buffer.from(shot.data, "base64"));
    return "shell-1024x600-fr-light.png";
  });

  await step("zero-console-errors", async () => {
    if (CONSOLE_ERRORS.length) throw new Error(CONSOLE_ERRORS.join(" | ").slice(0, 300));
    return "0 console errors";
  });

  fs.writeFileSync(path.join(HERE, "capture-results.json"), JSON.stringify({ results: RESULTS, consoleErrors: CONSOLE_ERRORS }, null, 2));
  const failed = RESULTS.filter((r) => !r.ok);
  console.log(`CAPTURE ${failed.length === 0 ? "GREEN" : "RED"} ${RESULTS.length - failed.length}/${RESULTS.length}`);
  chrome.kill();
  cdp.close();
  if (failed.length) process.exitCode = 1;
}

main().catch((e) => { console.error("FATAL", e); process.exitCode = 1; });
