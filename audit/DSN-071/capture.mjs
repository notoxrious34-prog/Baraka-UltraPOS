/**
 * DSN-071 headless visual gate (pure Node, no extra deps).
 * Boots vite (already running on localhost:5173), drives headless Chrome over
 * CDP with a stubbed Tauri IPC layer (ADMIN login + 4-user directory), and
 * captures the four required screenshots plus WCAG AA contrast measurements.
 *
 * Usage: node audit/DSN-071/capture.mjs   (vite dev must be up on :5173)
 */
import { spawn } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const HERE = path.dirname(fileURLToPath(import.meta.url));
const CHROME =
  "C:/Program Files/Google/Chrome/Application/chrome.exe";
const DEBUG_PORT = 9223;
const APP_URL = "http://localhost:5173/";
const VIEWPORT = { width: 1280, height: 800 };

const ADMIN_PERMS = [
  "USER_MANAGE",
  "CREATE_SALE",
  "VIEW_STOCK",
  "CREATE_CUSTOMER",
  "VIEW_CUSTOMERS",
  "ADJUST_STOCK",
  "TRANSFER_STOCK",
  "PROCESS_RETURN",
  "VIEW_REPORTS",
  "SYSTEM_SETTINGS_MANAGE",
  "BACKUP_RESTORE_MANAGE",
  "MANAGE_WAREHOUSES",
];
const USERS = [
  { id: "u-admin", username: "admin", role: "ADMIN", is_active: true, has_pin: false, created_at: "2026-01-05 09:00:00", updated_at: "2026-01-05 09:00:00" },
  { id: "u-gerant", username: "karim", role: "GERANT", is_active: true, has_pin: true, created_at: "2026-02-10 10:30:00", updated_at: "2026-02-10 10:30:00" },
  { id: "u-caisse", username: "caisse1", role: "CAISSIER", is_active: true, has_pin: true, created_at: "2026-03-01 08:00:00", updated_at: "2026-03-01 08:00:00" },
  { id: "u-vendeur", username: "vendeur1", role: "VENDEUR", is_active: false, has_pin: false, created_at: "2026-04-12 14:00:00", updated_at: "2026-04-12 14:00:00" },
];

const STUB_SOURCE = `
window.__TAURI_INTERNALS__ = {
  invoke: function (cmd, args) {
    var req = (args && args.request) || {};
    var respond = function (v) { return Promise.resolve(v); };
    var fail = function (code) { return Promise.reject(code); };
    if (cmd === 'check_first_run_status') return respond({ requires_setup: false });
    if (cmd === 'auth_login') return respond({
      session_token: 'stub-session-token',
      user_id: 'u-admin',
      username: (req && req.username) || 'admin',
      role: 'ADMIN',
      permissions: ${JSON.stringify(ADMIN_PERMS)},
      expires_at: '2030-01-01T00:00:00'
    });
    if (cmd === 'auth_logout') return respond(null);
    if (cmd === 'warehouses_list') return respond({ warehouses: [{ id: 'w1', code: 'DEP-01', name: 'Depot Central', is_active: true }] });
    if (cmd === 'cash_get_active_session') return respond(null);
    if (cmd === 'users_list') return respond(${JSON.stringify(USERS)});
    return fail('UNMOCKED_COMMAND:' + cmd);
  },
  transformCallback: function () { return 0; }
};
`;

const MEAS_FN = `(() => {
  const out = [];
  const isTransparent = (c) => c === "transparent" || c.replace(/\\s/g, "") === "rgba(0,0,0,0)";
  function effectiveBg(el) {
    let node = el;
    while (node && node !== document.documentElement) {
      const bg = getComputedStyle(node).backgroundColor;
      if (bg && !isTransparent(bg)) return bg;
      node = node.parentElement;
    }
    return getComputedStyle(document.body).backgroundColor;
  }
  function byText(tag, text) {
    const els = [...document.querySelectorAll(tag)];
    return els.find((e) => (e.textContent || "").trim() === text) || null;
  }
  const targets = [
    ["title-h1", () => document.querySelector("section h1")],
    ["subtitle", () => document.querySelector("section header p")],
    ["table-th", () => document.querySelector("thead th")],
    ["username-cell", () => [...document.querySelectorAll("tbody td")].find((e) => e.textContent.trim() === "caisse1") || null],
    ["badge-admin", () => byText("tbody span", "Administrateur")],
    ["badge-gerant", () => byText("tbody span", "Gérant")],
    ["badge-caissier", () => byText("tbody span", "Caissier")],
    ["badge-vendeur", () => byText("tbody span", "Vendeur")],
    ["badge-lastadmin", () => byText("tbody span", "Dernier Admin")],
    ["pill-active", () => byText("tbody span", "Actif")],
    ["pill-inactive", () => byText("tbody span", "Inactif")],
    ["pin-pill", () => byText("tbody span", "Défini")],
    ["created-cell", () => [...document.querySelectorAll("tbody td")].find((e) => e.textContent.includes("2026-04-12")) || null],
    ["hud-value", () => document.querySelector("section div.grid p.text-xl")],
    ["btn-primary-new", () => byText("section header button", "+ Nouvel Utilisateur")],
    ["btn-edit", () => byText("tbody button", "Modifier")],
    ["btn-credentials", () => byText("tbody button", "Identifiants")],
    ["input-first", () => document.querySelector("input[type=password]")],
    ["modal-save", () => [...document.querySelectorAll("button")].find((e) => (e.textContent || "").trim() === "Enregistrer") || null],
    ["modal-cancel", () => [...document.querySelectorAll("button")].find((e) => (e.textContent || "").trim() === "Annuler") || null],
  ];
  for (const [key, find] of targets) {
    let el = null;
    try { el = find(); } catch { el = null; }
    if (!el) { out.push({ key, found: false }); continue; }
    const cs = getComputedStyle(el);
    out.push({ key, found: true, fg: cs.color, bg: effectiveBg(el) });
  }
  const de = document.documentElement;
  out.push({ key: "__dims", scrollW: de.scrollWidth, clientW: de.clientWidth, scrollH: de.scrollHeight, clientH: de.clientHeight, dark: de.classList.contains("dark") });
  return out;
})()`;

function lum(rgb) {
  const m = rgb.match(/rgba?\(([^)]+)\)/);
  if (!m) return null;
  const [r, g, b] = m[1].split(",").slice(0, 3).map((v) => parseFloat(v) / 255);
  const f = (v) => (v <= 0.03928 ? v / 12.92 : Math.pow((v + 0.055) / 1.055, 2.4));
  return 0.2126 * f(r) + 0.7152 * f(g) + 0.0722 * f(b);
}
function ratio(fg, bg) {
  const a = lum(fg);
  const b = lum(bg);
  if (a === null || b === null) return null;
  return (Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05);
}

class Cdp {
  constructor(url) {
    this.url = url;
    this.id = 0;
    this.pending = new Map();
  }
  connect() {
    return new Promise((resolve, reject) => {
      this.ws = new WebSocket(this.url);
      this.ws.addEventListener("open", resolve);
      this.ws.addEventListener("error", reject);
      this.ws.addEventListener("message", (ev) => {
        const msg = JSON.parse(String(ev.data));
        if (msg.id && this.pending.has(msg.id)) {
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
  const res = await cdp.send("Runtime.evaluate", {
    expression,
    awaitPromise,
    returnByValue: true,
  });
  if (res.exceptionDetails) {
    throw new Error(`eval failed: ${JSON.stringify(res.exceptionDetails).slice(0, 400)}`);
  }
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

async function main() {
  const chrome = spawn(CHROME, [
    "--headless=new",
    `--remote-debugging-port=${DEBUG_PORT}`,
    "--no-first-run",
    "--disable-extensions",
    `--window-size=${VIEWPORT.width},${VIEWPORT.height}`,
    "about:blank",
  ], { stdio: "ignore" });
  const shot = (name) => path.join(HERE, name);
  let cdp = null;
  try {
    let targets = null;
    for (let i = 0; i < 30; i++) {
      try {
        targets = await httpJson("GET", `http://127.0.0.1:${DEBUG_PORT}/json/list`);
        break;
      } catch { await new Promise((r) => setTimeout(r, 500)); }
    }
    if (!targets) throw new Error("chrome remote debugging unavailable");
    const created = await httpJson("PUT", `http://127.0.0.1:${DEBUG_PORT}/json/new?about:blank`);
    cdp = new Cdp(created.webSocketDebuggerUrl);
    await cdp.connect();
    await cdp.send("Page.enable");
    await cdp.send("Runtime.enable");
    await cdp.send("Emulation.setDeviceMetricsOverride", {
      width: VIEWPORT.width,
      height: VIEWPORT.height,
      deviceScaleFactor: 1,
      mobile: false,
    });
    await cdp.send("Page.addScriptToEvaluateOnNewDocument", { source: STUB_SOURCE });
    await cdp.send("Page.navigate", { url: APP_URL });
    await waitFor(cdp, `!!document.querySelector('button[type="submit"]')`, 30000, "login submit");
    await evalJs(cdp, STUB_SOURCE);

    // Fill login form (native setter so React sees the change) and submit.
    await evalJs(cdp, `(() => {
      const inputs = [...document.querySelectorAll('form input')];
      const set = (el, v) => {
        const setter = Object.getOwnPropertyDescriptor(el.__proto__, "value")?.set
          ?? Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value").set;
        setter.call(el, v);
        el.dispatchEvent(new Event("input", { bubbles: true }));
        el.dispatchEvent(new Event("change", { bubbles: true }));
      };
      set(inputs[0], "admin");
      set(inputs[1], "Admin123!");
      document.querySelector('button[type="submit"]').click();
      return true;
    })()`);
    await waitFor(
      cdp,
      `[...document.querySelectorAll('nav button')].some((b) => (b.textContent || "").includes("Utilisateurs"))`,
      30000,
      "users nav",
    );
    await evalJs(cdp, `[...document.querySelectorAll('nav button')].find((b) => (b.textContent || "").includes("Utilisateurs")).click()`);
    await waitFor(cdp, `document.body.textContent.includes("Dernier Admin")`, 20000, "user table + last-admin badge");

    const results = {};
    async function snap(key, file) {
      const { data } = await cdp.send("Page.captureScreenshot", { format: "png" });
      fs.writeFileSync(shot(file), Buffer.from(data, "base64"));
      results[key] = { shot: file, measures: await evalJs(cdp, MEAS_FN) };
      console.log(`captured ${file}`);
    }

    await snap("light", "users-module-light.png");
    // Topbar (which hosts the theme toggle) is not mounted on the users view;
    // drive the Tailwind dark class directly (same class the store toggles).
    await evalJs(cdp, `document.documentElement.classList.add("dark")`);
    await waitFor(cdp, `document.documentElement.classList.contains("dark")`, 10000, "dark mode");
    await snap("dark", "users-module-dark.png");

    // Create modal (dark).
    await evalJs(cdp, `[...document.querySelectorAll('section header button')].find((b) => (b.textContent || "").includes("Nouvel Utilisateur")).click()`);
    await waitFor(cdp, `[...document.querySelectorAll('button')].some((b) => (b.textContent || "").trim() === "Enregistrer")`, 10000, "create modal");
    await snap("createDark", "users-modal-create-dark.png");

    // Back to light, then credential-reset modal.
    await evalJs(cdp, `[...document.querySelectorAll('button[aria-label="Close"]')].pop().click()`);
    await evalJs(cdp, `document.documentElement.classList.remove("dark")`);
    await waitFor(cdp, `!document.documentElement.classList.contains("dark")`, 10000, "light mode");
    await evalJs(cdp, `[...document.querySelectorAll('tbody button')].find((b) => (b.textContent || "").trim() === "Identifiants").click()`);
    await waitFor(cdp, `document.body.textContent.includes("Compte concerné")`, 10000, "reset modal");
    await snap("resetLight", "users-modal-reset-light.png");

    // Contrast table.
    console.log("\nkey | theme | fg | bg | ratio");
    for (const [theme, entry] of Object.entries(results)) {
      for (const m of entry.measures) {
        if (m.key === "__dims") {
          console.log(`${theme} | dims scroll=${m.scrollW}x${m.scrollH} client=${m.clientW}x${m.clientH} dark=${m.dark}`);
          continue;
        }
        if (!m.found) { console.log(`${theme} | ${m.key} NOT-FOUND`); continue; }
        const r = ratio(m.fg, m.bg);
        console.log(`${theme} | ${m.key} | ${m.fg} | ${m.bg} | ${r === null ? "n/a" : r.toFixed(2)}`);
      }
    }
    fs.writeFileSync(shot("contrast.json"), JSON.stringify(results, null, 2));
    console.log("\nwrote contrast.json");
  } finally {
    if (cdp) cdp.close();
    try { chrome.kill(); } catch { /* noop */ }
  }
}

main().catch((err) => {
  console.error(`CAPTURE FAILED: ${err?.stack ?? err}`);
  process.exitCode = 1;
});
