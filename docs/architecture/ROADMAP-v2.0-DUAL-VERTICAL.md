# ROADMAP v2.0 — Polymorphic Dual-Vertical Enterprise
# (Men's Underwear Wholesale/Retail + General Hardware & Trade)

**Authority:** TASK-REL-020 · Constitution v4.0 (`SOVEREIGN_CONSTITUTION_v4.0.md`)
· Architecture `ADR-110` · Ground truth `TASK-AUD-049-R1` (`v1.2.1`).
**ID discipline (verified 2026-11-12 from `git log`):** max in history
`BRK-107`, `DSN-102`, `AUD-058`, `REL-062`, `DOC-047`, `PRT-05` — every ID below
is above-max and collision-free. No `BRK-108+` / `DSN-103+` exists.
**Laws:** additive migrations `0020+` with scratch-DB retention proof (Pillar 4);
no floats; atomic sale untouched; Gates G1–G7 per wave; innovation section
(Pillar 2) mandatory at every wave boundary.

---

## WAVE 1 — Wholesale Credit Truth on Paper (v2.0 foundation)
**Goal:** the printed document becomes a legal-grade credit witness.
- `TASK-BRK-108` — Credit-equation snapshot engine: atomically persist
  `ancien_solde + montant − versement = nouveau_solde` at finalization
  (migration `0020`, append-only; snapshot frozen even if later payments land).
- `TASK-BRK-109` — *Colisage* totals engine: carton/dozen/piece breakdown lines
  per document from the `product_units` ladder (pure domain, `i128`, no floats).
- `TASK-DSN-103` — Equation print UX: `Ancien Solde / Montant / Versement /
  Nouveau Solde` block on 80/58mm, Graphique/TSPL, A4 `BL`/`Facture` + Cost/Margin
  Privacy Shield (`Ctrl+H`) for counter eyes.
- `TASK-DSN-104` — *Colisage* display: carton/douzaine totals in cart, ticket,
  and drawer detail.
- `TASK-AUD-059` — Wave acceptance audit (ledger-print parity proof).

## WAVE 2 — Profiles, Super-Gros & Price Memory
**Goal:** the system speaks each trade's language and remembers its prices.
- `TASK-BRK-110` — Vertical profiles (`apparel_wholesale_retail` |
  `hardware_general` | `hybrid_enterprise`) + `Super-Gros`/quantity-break tiers
  extending `0015` semantics (new additive migration; `price_for_tier`
  untouched in shape).
- `TASK-BRK-111` — Customer last-price memory: per-customer×product last
  invoiced PU (read-only recall, never auto-override without manager key).
- `TASK-DSN-105` — Profile switch UX in Settings + tier badges/safeguards in POS.
- `TASK-DSN-106` — Last-price chip in cart (`Dernier prix: …`) with override audit.
- `TASK-DOC-048` — v2.0 Wave 1–2 operator guide (FR/AR).

## WAVE 3 — Print Powerhouse & Code Health
**Goal:** two printers, two label studios, zero monoliths.
- `TASK-BRK-112` — Split `tspl.rs` (4,036 LOC) into domain submodules
  (ADR-108/109 standard: pure motion, path-stable, proof-pinned).
- `TASK-BRK-113` — Dual-printer routing: independent **Counter Receipt** +
  **Dedicated `XP-410B` Label** roles with per-role fallback queues.
- `TASK-DSN-107` — Label Studio: Textile template (giant Size badge, Model,
  Color, Pack badge, wholesale price-hide toggle) + Hardware template
  (barcode-forward, unit + length/Ø line).
- `TASK-PRT-06` — On-ruler verification pack for both studios (40×20 liner).
- Frontend twin (same wave): split `src/lib/ipc.ts` (3,170 LOC) per-domain.

## WAVE 4 — Matrix Engine & Série Builder
**Goal:** order 200 variants as fast as one barcode scan.
- `TASK-BRK-114` — Variant-matrix engine (`Parent Model/Ref × Size ×
  Color/Finish`, migration `0021+`): 1-click variant/SKU/barcode generation,
  per-variant stock, atomic multi-line deduction.
- `TASK-BRK-115` — Assorted *Série* builder on `product_kit_items`
  (`1 Série = 2M + 4L + 4XL + 2XXL`) with component-availability precheck.
- `TASK-DSN-108` — Rapid 2D grid order entry in POS (keyboard-first, FR/AR).
- `TASK-DSN-109` — Purchases matrix receiving + Série composer.
- `TASK-AUD-060` — Matrix/kit atomicity + FTS5-recall audit.

## WAVE 5 — Commercial Intelligence
**Goal:** the system warns before the shelf (or the ledger) bleeds.
- `TASK-BRK-116` — Signals engine: `Rupture de Série`/broken size-run alerts,
  seasonality & dead-stock radar, model profitability, credit-aging buckets.
- `TASK-DSN-110` — Intelligence cockpit (alerts + radar + aging, FR/AR, dark-ready).
- `TASK-REL-063` — v2.0 release bundle (G8) with trial-installer evidence chain.

---

## 💡 ابتكارات وأفكار تطويرية مقترحة (Proactive Innovations — من تصميم النموذج)

سبعة ابتكارات أصلية عالية العائد، تعرض على المالك للاعتماد قبل أي تنفيذ
(بوابة الإبداع المنضبط — Pillar 2.3):

1. **فكّ الكرتون الذكي (Smart Carton Break-down)** — عند بيع التجزئة من مخزون
   الجملة، يقترح النظام تلقائيًا فكّ `Balle ×120` إلى دستينات ثم قطعًا مع تتبع
   الأثر، بضغطة واحدة بدل حسابات يدوية خاطئة. *(القيمة: صفر ضياع قطع؛ UX: زر
   «فكّ كرتون» في السلة؛ معماريًا: عملية مخزون ذرية على `product_units`.)*
2. **مبادلة المقاس في ضربتين (2-Keystroke Size-Swap)** — إرجاع/استبدال الملابس
   (L→XL لنفس الموديل واللون) يُنجز بإدخال المقاس الجديد فقط: السطر القديم
   يُعكس والجديد يُنشأ بسعر الطبقة نفسه مع أثر تدقيق واحد. *(يقضي على طوابير
   الاستبدال في الموسم.)*
3. **عرض الجملة → قائمة التعبئة (Quote-to-Packing-List)** — عرض سعر تاجر الجملة
   يتحول بضغطة إلى قائمة تعبئة مرتبة حسب الرف/الكرتون مع عدّاد تحقق بالسكانر،
   فيُغلق الفرق بين «ما اتُّفق عليه» و«ما خُزّم» نهائيًا.
4. **رادار التموين الموسمي (Seasonal Restock Radar)** — يتعلم منحنى كل موديل
   (رمضان/الدخول المدرسي/الشتاء) ويقترح كميات الشراء قبل 3 أسابيع من النفاد
   المتوقع، مع تنبيه `Rupture de Série` عند انكسار تشكيلة المقاسات.
5. **مساعد القصّ والتحويل (Cutting & Conversion Helper)** — للخردوات: إدخال
   «12.5m من لفة 100m» أو «3.2kg من كيس 25kg» يحسب الباقي والسعر فورًا
   بالحساب الصحيح 10⁴، مع تحذير هامش الخسارة قبل الالتزام.
6. **الحسابات الفرعية للأوراش (Chantier Sub-Accounts)** — حساب مقاول أم
   (Tier = Artisan) تنبثق منه حسابات فرعية لكل ورشة، بسقوف منفصلة وكشف
   مستقل، ثم دمجها في فاتورة واحدة نهاية الشهر.
7. **التعلّم الصوتي التجاري (Trade-Slang FTS Learning)** — عدّاد استخدام مجهول
   محليًا يرقّي تلقائيًا الأسماء الدارجة التي يكتبها الباعة (`دريل`، `بولة`،
   `سيرية`) إلى مرادفات دائمة في `product_aliases` بعد عتبة تكرار — البحث
   يزداد ذكاءً كل أسبوع دون أي سحابة.

---

## Wave-execution order & gates

Waves run 1→5 in dependency order; each wave ends with G1–G7 green +
regression/visual proof + Arabic evidence report before the next wave starts.
First executable unit on approval: **`TASK-BRK-108`** (credit-equation snapshot,
migration `0020` + retention test).
