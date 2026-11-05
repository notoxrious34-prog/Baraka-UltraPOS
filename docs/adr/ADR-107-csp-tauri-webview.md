# ADR-107 — Content Security Policy for the Tauri webview (TASK-BRK-105)

Status: ACCEPTED. Date: 2026-11-05 (TASK-BRK-105). Authority: Sovereign (self-tasked under v3.1 §1.3).

## Context

`app.security.csp` was `null`: the webview ran with no content policy.
Any injected markup could load remote scripts, frames, or plugins.

## Decision

Enforce (in `src-tauri/tauri.conf.json`):

```
default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline';
img-src 'self' asset: data: blob:; font-src 'self' data:;
connect-src 'self' ipc: http://ipc.localhost;
object-src 'none'; frame-src 'self'; base-uri 'self'; form-action 'self'
```

Per-directive justification (verified statically, see Conformance):

- `script-src 'self'`: zero inline `<script>` in `dist/index.html`, zero
  `eval`/`new Function` in the bundle. No inline event-handler attributes
  anywhere in rendered markup (React props compile to `addEventListener`).
- `style-src 'self' 'unsafe-inline'`: required by 16 React `style={{...}}`
  attribute sites and by the label print document's `<style>` block.
  External Tailwind CSS is same-origin.
- `img-src ... data: blob:`: `bitmap-capture.ts` renders
  `data:image/svg+xml` into an `Image`; downloads use `blob:` URLs.
- `connect-src ... ipc: http://ipc.localhost`: Tauri v2 invoke transport;
  the updater JS API goes through IPC (Rust side performs HTTPS).
- `frame-src 'self'`: the label print path (`printLabelDocument`) creates a
  programmatic `about:blank` iframe + `document.write`; `'none'` could
  block it, `'self'` permits origin-inheriting frames. `object-src 'none'`:
  no plugins exist.

## Conformance

`npm run build` (tsc + vite, 15s) green; `npm run lint` green; static
conformance script 7/7 PASS (no inline scripts/styles, same-origin refs
only, no eval, print template clean, all directives present).

## Consequences / residual risk

- Runtime proof (WebView2 console free of CSP violations, label print,
  updater check) requires opening the built app — assigned to the next
  trial-installer smoke test (owner gate), not claimed here.
- Dev-server workflow is unaffected (Vite serves dev HTML; the policy is
  injected into the production bundle only).
