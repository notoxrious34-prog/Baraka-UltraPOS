# ADR-127 — Wave 4 prioritization, owner votes on proposals 1–11, six new owner proposals 12–17, and BRK-114 launch order

- **Status:** ACCEPTED (sovereign owner decision, 2026-11-19)
- **Authority:** owner directive answering `EVALUATION-v2.0-AND-BEYOND-2026-11-19.md` + full audit `PROJECT-FULL-AUDIT-2026-11-19.md`
- **Scope:** documentation + Wave 4/5 scheduling + BRK-114 launch. No production code in this ADR.

## 1. Owner votes on evaluation proposals 1–11 (binding)

| # | Proposal | Verdict | Binding order |
|---|---|---|---|
| W4 | Full Wave 4 (`BRK-114` → `BRK-115` → `DSN-108` → `DSN-109` → `AUD-060`) | **APPROVED, top priority #1, immediate execution** | First executable unit: `BRK-114` |
| 3 | `PRT-06` studio hardware verification | **APPROVED as non-blocking owner hardware gate** (`OPEN HARDWARE OWNER GATE`): executed on-site once the `XP-410B` is connected; MUST NOT block Wave 4 or Wave 5 | Field session |
| 7 | Supplier price-deviation alert | **APPROVED, merged into `DSN-109`** (compare vs last price + 90-day average, margin protection at source) | Inside `DSN-109` |
| 8 | Wave 5 (`BRK-116` + `DSN-110` with per-profile thresholds) | **APPROVED for execution immediately after Wave 4 closes** | After `AUD-060` |
| 5 | Scanner stocktake | **APPROVED** (scan → count → variance report shortage/over/damaged → single reversible atomic adjustment) | After Waves 4–5 |
| 6+11+9+10 | One-click diagnostics bundle (customer-PII-free field list in ADR) + training Sandbox + idle fiscal-compliance seam (`XS`) + perf gate `G3+` (10k SKUs / 1M movements) | **ALL APPROVED**; perf gate merges into `AUD-060`; seam + sandbox ship in the readiness wave before `REL-063`; five `CRLF` `.rs` files normalized to `LF` | With `AUD-060` / readiness |
| 1 | `BRK-124` counter sync per `ADR-123` | **APPROVED, `ADR-123` flips `PROPOSED` → `ACCEPTED`**; architectural decision: execute AFTER Wave 4 migrations (`0024+`) so sync covers matrix/série tables from day one without rewrite | After Wave 4 schema settles |
| 2 | Local `.sig` signing + one-click updater | **APPROVED**; private key stays exclusively on the owner device, outside the repo (documented in `DRILL`, never pushed to Git); public key pinned in `tauri.conf.json` to close the `.sig` gap before `REL-063` | Before `REL-063` |
| 4 | Owner reading companion, no cloud | **APPROVED dual-mode: self-contained encrypted nightly digest + near-live read over local `Wi-Fi` (LAN only, zero cloud)** | After Wave 5 closes |
| NG 1–4 | Non-goals (no cloud backend, no native selling app, no rewrite, no new deps without name+version) | **FULLY RATIFIED** | Standing |

## 2. Six new owner proposals 12–17 (added to roadmap, dependency-ordered)

- **12 — Atomic size/color exchange engine (*Échange Express*) → `BRK-125` (Wave 4, after `BRK-114`):** exchange returned sizes/colors for alternates in ONE atomic operation reusing last-price memory (`0022`); money effect `0 DZD` at equal historic price, stock-cell adjustment, collect net only on difference, customer statement never distorted.
- **13 — Bargaining-cap guard with silent light signal (*Plancher de Négociation*) → `DSN-115` (Wave 4, with `DSN-108`):** per-line coded signal (`green` safe / `amber` near floor / `red` below cost or floor) without exposing purchase price; `red` locks the sale unless `PIN Gérant`.
- **14 — Priceless warehouse prep sheet (*Bon de Préparation*) → `BRK-126` (Wave 4, after `DSN-108`):** pre-finalization sheet from the cart ordered by model/color/size, exploded by *Colisage*, checkbox cells, zero prices/debts, so the storekeeper verifies physical goods before financial posting.
- **15 — Instant payment behavior + standalone `Versement Seul` receipt → `BRK-127` (Wave 4 tail):** days-since-last-payment + invoice-coverage ratio on customer select in POS, plus a fast standalone payment path printing a `Reçu de Versement` with the frozen four-term equation.
- **16 — Available-assortment catalog for WhatsApp/PDF, offline → `DSN-117` (Wave 5, needs `BRK-114/115` live availability + *Série* fill):** one-click local visual bulletin, in-stock models only with live sizes/colors and *Série* fill, price mode (wholesale / super-gros / none). `DSN-116` stays reserved.
- **17 — Dual automatic backup to owner USB on cash close → `BRK-128` (readiness wave before `REL-063`):** bind an external path (owner USB key): every cash-session close copies the SHA-256-verified encrypted database to local disk AND external USB together, with an alert past 48h without external copy. Closes audit gap `2.1 #7` with zero cloud.

## 3. Launch order (binding)

`BRK-114` (now) → `BRK-115` → `DSN-108` (+`DSN-115` alongside) → `BRK-126` → `DSN-109` (with proposal 7) → `BRK-127` → `BRK-125` → `AUD-060` (with perf gate) → Wave 5 (`BRK-116`, `DSN-110`, `DSN-117`) → readiness (`BRK-128`, sandbox, fiscal seam, `.sig`) → `REL-063` (`G8`).
`BRK-124` after Wave 4 schema settles; proposal 4 after Wave 5; proposal 5 after Waves 4–5; `PRT-06` on-site, non-blocking throughout.

## 4. Consequences

- `ADR-123` status becomes `ACCEPTED` (implementation deferred per §3, design frozen).
- Next free ids after this ADR: `ADR-128`, `BRK-129`, `DSN-118` (`DSN-116` reserved).
- `ROADMAP-v2.0-DUAL-VERTICAL.md` Wave 4/5 sections carry the new task ids; Wave 3 notes `PRT-06` as the single remaining open gate (non-blocking).
