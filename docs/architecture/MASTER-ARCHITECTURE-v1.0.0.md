# Baraka UltraPOS — Master Sovereign Architecture & Product Design
## الإصدار النهائي المعتمد للتنفيذ — v1.0.0

**المشروع:** Baraka UltraPOS — Quincaillerie Générale — الجزائر  
**حالة الوثيقة:** Baseline معماري سيادي للتنفيذ  
**تاريخ الاعتماد:** 25 سبتمبر 2026  
**المرجع:** Master Principal Software Architect  

---

# 1. القرار التنفيذي الأعلى

Baraka UltraPOS هو تطبيق Desktop Offline-First محلي، وليس Web POS يعتمد على خادم. المعمارية النهائية هي:

```text
React + TypeScript
        │
        │ typed IPC فقط
        ▼
Tauri v2 / Commands
        │
        ▼
Rust Application
        │
        ▼
Rust Domain
        │
        ▼
Rust Infrastructure Adapters
        │
        ├── rusqlite / SQLite(SQLCipher في إصدار الإنتاج)
        ├── FTS5
        ├── ESC/POS
        ├── Barcode
        ├── Backup / Restore
        └── OS Secure Storage
```

## القرارات الحاكمة

1. **Tauri v2 + Rust Core** هو Runtime المعتمد.
2. **React + TypeScript strict** هي طبقة العرض فقط.
3. **Rust هو المالك الوحيد لمنطق المجال والبيانات والمعاملات.**
4. **لا ORM في Runtime DB path.** لا Drizzle ولا Prisma في الوصول الأساسي إلى قاعدة البيانات.
5. **SQLite هو مصدر الحقيقة المحلي الوحيد.**
6. **rusqlite + SQL مكتوب صراحة + migrations مضمنة في Rust** هو مسار البيانات الرسمي.
7. القيم المالية في Domain/Application/Persistence هي `Money(i64)` بالسنتيم، ولا تستخدم `f32/f64` للحسابات المالية.
8. الكميات ومعاملات التحويل تستخدم Fixed-Point Integers بدقة 4 خانات عشرية.
9. واجهة IPC لا تنقل المال أو الكمية كـ JavaScript `number`؛ تنتقل كسلاسل عشرية canonical strings، ثم تحول الواجهة إلى `bigint`، ويحول Rust إلى Newtypes داخل المجال.
10. البيع النهائي Atomic بالكامل، ويشمل إنشاء أمر الطباعة داخل المعاملة قبل `COMMIT`.
11. Retry يقتصر على الأخطاء العابرة المعلومة فقط مثل `BUSY/LOCKED`؛ لا يعاد تنفيذ أخطاء الأعمال.
12. الاختبارات المتعلقة بـWAL والطاقة وانقطاع العملية تستخدم قاعدة ملفية مؤقتة، وليس `:memory:`.
13. الفواتير النهائية غير قابلة للتعديل؛ التصحيح يتم عبر AVOIR/RETURN أو قيد عكسي مصرح به حسب نوع العملية.
14. السجلات المالية والسجلات المحاسبية التشغيلية Append-Only مفروضة على مستوى قاعدة البيانات أيضاً.
15. التشفير الإنتاجي لقاعدة البيانات والنسخ الاحتياطية جزء من Release Contract، مع مفاتيح مخزنة في OS Secure Storage وآلية Recovery Key منفصلة.
16. الضرائب والطابع قواعد قابلة للإصدار والتاريخ، ولا يوجد أي ادعاء امتثال قانوني نهائي قبل اعتماد القواعد بواسطة مختص محاسبي/جبائي على إصدار 2026 الفعلي.

---

# 2. نطاق المنتج النهائي

## 2.1 الوحدات الأساسية

| Module | الوظيفة |
|---|---|
| POS / Vente | البيع السريع بالباركود أو البحث، كاش، كريدي، دفع مختلط |
| Products / Catalog | المنتجات، المراجع، الأسعار، وحدات البيع، الباركود، Kits |
| Stock / Inventaire | المخزون، الحركات، الجرد، التسويات، المستودعات |
| Customers / Crédit | الحسابات، سقوف الائتمان، دفتر الكريدي، التسديدات، كشف الحساب |
| Suppliers / Achats | الموردون، المشتريات، الديون للموردين، المرتجعات |
| Documents | Ticket, BL, Facture, Devis, Avoir, BC, BR, FF |
| Cash / Clôture | الوردية، الحركات، المصاريف، الفارق، Z-Report |
| Reports | المبيعات، الأرباح، الديون، النقدية، المخزون، النشاط |
| Users / Permissions | Vendeur, Caissier, Gérant, Admin |
| Settings / Backup | إعدادات المتجر، الضرائب، الطابعات، النسخ، الاستعادة |

## 2.2 حدود النسخة الأولى

البيع، المخزون، العملاء، الكريدي، الصندوق، المستندات الأساسية، الطباعة، الباركود، المستخدمون، النسخ الاحتياطية والتقارير الأساسية هي الوظائف الجوهرية.

المزامنة بين الأجهزة والـCloud ليست شرطاً للعمل الأساسي، لكنها مدعومة معمارياً عبر حقول metadata مستقبلية.

---

# 3. مبادئ المجال Domain Invariants

هذه القواعد أعلى من أي UI أو Repository أو ORM.

### المال

```text
Money = i64 centimes
1 DZD = 100 centimes
```

لا يوجد Float في Domain/Application/Financial Persistence.

### الكمية

```text
QtyMicro = integer × 10^4
FactorMicro = integer × 10^4
```

المخزون محفوظ دائماً بالوحدة القاعدية.

### المنتج والوحدة

لكل منتج:
- Base Unit واحدة.
- عدة Selling Units اختيارية.
- كل Unit لها `factor_micro` بالنسبة للوحدة القاعدية.
- كل Barcode يشير إلى Product Unit واحدة فقط.

### المنتج المركب Kit

Kit هو BOM قابل للتوسع إلى مكونات.

يحظر:
- Kit يحتوي نفسه.
- دورة A→B→A.
- مكون غير فعّال.
- مكون بوحدة غير موجودة.

### المخزون

الحركة هي Source of Truth، و`stock_levels.quantity_micro` هو الرصيد التشغيلي المحسن للأداء.

لا يجوز أن تصبح الكمية سالبة إلا عند وجود صلاحية صريحة وAudit Event مناسب.

### الكريدي

الرصيد المحسوب:

```text
balance = SUM(DEBIT) - SUM(CREDIT)
```

ويجب أن يساوي `customers.current_balance_centimes` دائماً بعد كل معاملة مالية ملتزمة.

### سياسة السقف

لا يستخدم `0` كمعنى غامض.

```text
CreditPolicy =
  DISABLED
  LIMITED(limit_centimes)
  UNLIMITED
```

### الفاتورة

بعد `COMPLETED` تصبح immutable.

التصحيح يتم عبر:
- AVOIR / Return.
- أو reversal transaction عند العمليات غير البيعية التي يسمح المجال بتصحيحها.

### الترقيم

Gapless يعني: **لا توجد فجوة بين أرقام الوثائق الملتزمة (Committed)** ضمن النوع والسنة.

الأرقام لا تمنح للمسودات. الرقم النهائي يحجز داخل المعاملة نفسها، وإذا حدث rollback يعود العداد إلى حالته السابقة.

---

# 4. المعمارية الطبقية

## 4.1 Presentation

React مسؤول عن:
- العرض.
- التنقل.
- إدخال المستخدم.
- حالة السلة المحلية المؤقتة.
- الاختصارات.
- i18n.
- الوصولية.
- عرض أخطاء الأعمال القادمة من Rust.

ممنوع:
- SQL.
- SQLite.
- File I/O للأعمال.
- حساب إجماليات مالية مستقلة عن Domain.
- تعديل مخزون مباشرة.

## 4.2 Application

Rust Use Cases:
- ExecuteSale.
- ParkSale.
- ResumeParkedSale.
- RecordCreditPayment.
- AdjustStock.
- TransferStock.
- OpenCashSession.
- CloseCashSession.
- CreateReturn.
- CreatePurchase.
- GenerateReports.

Application يملك Orchestration وAuthorization وIdempotency وAudit decisions، لكنه لا يعرف تفاصيل SQLite.

## 4.3 Domain

Domain هو النواة المستقلة:

```text
Money
Qty
Factor
Percentage
DocNumber
Barcode
TaxSnapshot
Product
ProductUnit
Kit
Sale
SaleLine
Customer
CustomerLedgerEntry
CashSession
CashMovement
Document
Return
Supplier
```

Domain لا يعرف React أو Tauri أو SQLite أو ESC/POS.

## 4.4 Infrastructure

Adapters:
- SQLite.
- FTS5.
- Backup.
- Restore.
- Printer.
- Barcode.
- Secure Storage.
- Clock.
- File System.

الاعتماديات تتجه إلى الداخل فقط.

---

# 5. Repository and Transaction Model

## 5.1 قاعدة إزالة التعارض

لا يوجد Drizzle Repository في المسار:

```text
React → Tauri → Rust Application → Rust Domain → Infrastructure → SQLite
```

هذا يلغي التعارض الذي ظهر في التصميم السابق بين Rust-owned architecture وDrizzle-owned database access.

## 5.2 Atomic Sale Unit of Work

البيع يستخدم Transaction Boundary واحدة يديرها Infrastructure، لكن orchestration والقرارات التجارية تصاغ في Application/Domain.

المسار:

```text
Validate DTO
→ Authorize
→ Idempotency Claim
→ BEGIN IMMEDIATE
→ Validate Customer/Credit
→ Validate Product/Price Snapshot
→ Validate Stock / Expand Kit
→ Allocate Document Number
→ Persist Document + Items
→ Persist Stock Movements
→ Update Stock Levels
→ Persist Payment rows
→ Persist Credit Ledger when applicable
→ Persist Cash Movement when applicable
→ Persist Audit Event
→ Persist Print Job PENDING
→ COMMIT
→ Background Print Worker
```

## 5.3 Retry

يعاد تنفيذ المعاملة فقط عند تصنيف الخطأ كـtransient:

- `SQLITE_BUSY`
- `SQLITE_BUSY_TIMEOUT`
- `SQLITE_LOCKED` إذا كان السياق آمناً لإعادة المحاولة

ولا يعاد تنفيذ:

- InvalidCustomer
- CreditLimitExceeded
- InsufficientStock
- InvalidPrice
- Unauthorized
- ConstraintViolation
- ValidationError
- DuplicateDocument

الحد الأعلى 3 محاولات مع Backoff قصير deterministic.

---

# 6. Financial Engine

## 6.1 Line Calculation

```text
line_gross = ROUND_HALF_UP(qty_micro × unit_price_centimes / 10_000)
line_after_line_discount = line_gross - line_discount
```

## 6.2 Global Discount

الخصم العام يوزع بطريقة deterministic على خطوط الفاتورة باستعمال Largest Remainder / proportional allocation.

يجب أن يكون:

```text
SUM(allocated_global_discount) == global_discount_centimes
```

ولا يوجد rounding drift.

## 6.3 TVA

TVA تحسب لكل خط بعد الخصم المخصص للخط:

```text
line_tva = ROUND_HALF_UP(line_taxable_ht × rate / 100)
```

ثم تجمع.

## 6.4 Timbre

الصيغة الدائرية السابقة مرفوضة.

يحسب الطابع من:

```text
base_ttc_before_timbre = HT + TVA - discounts
```

ثم:

```text
timbre = tax_rule(base_ttc_before_timbre, payment_method, document_type)
total_ttc = base_ttc_before_timbre + timbre
```

القيم الابتدائية للطابع تبقى Configuration Seed قابلة للتعديل، ولا تعتمد Production Legal Compliance إلا بعد توقيع القواعد الضريبية الحالية.

## 6.5 Safety

كل عمليات الضرب التي قد تفوق i64 مؤقتاً تستخدم i128.

إذا تجاوزت القيمة مجال Money أو Qty:

```text
OverflowError
```

ولا يتم wraparound.

---

# 7. IPC Contract

## 7.1 Wire Types

القيم التالية تعبر IPC كسلاسل:

```text
moneyCents: string
qtyMicro: string
factorMicro: string
```

الواجهة تحولها إلى `bigint` عند الحاجة.

Rust يحولها إلى:

```text
Money(i64)
Qty(i64)
Factor(i64)
```

## 7.2 Canonical Rules

- لا `number` للأموال.
- لا `parseFloat` للمال.
- لا JSON floating point للمبالغ.
- لا تقريب في UI.
- كل rounding يحدث في Domain.

## 7.3 Typed Commands

Commands عالية المستوى، وليست CRUD خام:

```text
auth.login
products.search
products.create
products.update
products.get
sales.createDraft
sales.park
sales.resume
sales.checkout
sales.get
sales.reprint
credit.getAccount
credit.recordPayment
stock.get
stock.adjust
stock.transfer
cash.open
cash.close
cash.recordExpense
documents.get
documents.convert
documents.return
reports.dashboard
reports.sales
reports.stock
reports.credit
backups.create
backups.restore
```

العقود تولد TypeScript من Rust باستخدام typed IPC tooling. يجب تثبيت الإصدار المدقق في `Cargo.lock` ولا يسمح بتحديث تلقائي غير مراجَع.

---

# 8. قاعدة البيانات — Canonical Logical Schema

## 8.1 System

1. `users`
2. `settings`
3. `audit_log`
4. `backups_log`
5. `document_sequences`
6. `print_jobs`
7. `command_log`

## 8.2 Catalog / Stock

8. `categories`
9. `products`
10. `product_units`
11. `product_kit_items`
12. `warehouses`
13. `stock_levels`
14. `stock_movements`

## 8.3 Documents / Sales

15. `documents`
16. `document_items`
17. `payments`

## 8.4 Customers / Credit

18. `customers`
19. `customer_ledger`

## 8.5 Cash

20. `cash_sessions`
21. `cash_movements`

## 8.6 Suppliers / Purchases

22. `suppliers`
23. `supplier_ledger`

## 8.7 Tax

24. `tax_rules`

The original v3 classification described 26 logical tables; the corrected final baseline treats the above as the canonical transactional schema and permits derived reporting read-model tables only in later reporting migrations when justified by profiling. No report table is ever a source of truth.

### Critical columns to finalize

`documents` must include at minimum:

```text
id
number
type
party_type
party_id
source_document_id
warehouse_id
document_language
subtotal_ht
vat_total
global_discount
timbre
rounding
TTC
paid_amount
change_amount
payment_method
status
created_at
created_by
revision
origin_device_id
```

`document_items` must include:

```text
id
document_id
product_id
unit_id
source_item_id
quantity_micro
quantity_base_micro
unit_price_centimes
discount_centimes
tva_rate
tax_exemption_code
line_ht_centimes
line_tva_centimes
line_ttc_centimes
cost_price_centimes
sort_order
```

`customers` must include:

```text
credit_policy
credit_limit_centimes
current_balance_centimes
```

`cash_sessions` must include a clear warehouse/terminal context for each session.

## 8.8 Append-Only Tables

Database triggers must prohibit UPDATE/DELETE on:

```text
customer_ledger
supplier_ledger
stock_movements
cash_movements
payments
audit_log
```

For corrections use compensating entries.

## 8.9 Finalized Document Immutability

Database triggers reject mutation of `documents` and `document_items` when the document is in a finalized state.

Draft documents remain editable.

---

# 9. Idempotency

`command_log.command_id` is the idempotency key.

For financial commands:

```text
same command_id + same request_hash
    → return cached result
```

```text
same command_id + different request_hash
    → IdempotencyConflict
```

Financial command logs are not TTL-deleted after one hour. Retention is operationally managed, but the identity needed to prevent a replay of a financial command must remain available for the defined audit horizon.

---

# 10. Audit and Tamper Evidence

Every critical operation emits an audit record.

Audit ordering uses a monotonic `sequence_no`.

Hash formula is conceptually:

```text
self_hash = SHA256(
    sequence_no
    + prev_hash
    + timestamp
    + user_id
    + action
    + entity_type
    + entity_id
    + canonical(old_value)
    + canonical(new_value)
)
```

`prev_hash` must match the previous committed audit record.

A periodic integrity job validates the chain.

The hash chain is tamper-evidence, not encryption.

---

# 11. SQLite Configuration

At connection initialization:

```sql
PRAGMA journal_mode = WAL;
PRAGMA synchronous = FULL;
PRAGMA foreign_keys = ON;
PRAGMA busy_timeout = 5000;
PRAGMA wal_autocheckpoint = 1000;
```

The exact checkpoint cadence may be tuned after profiling, but no tuning may weaken durability without a new ADR.

The database uses `STRICT` tables wherever practical.

The active database resides in the application data directory, never in the source repository.

The product must never depend on copying only the `.db` file while a WAL is active. Backup uses a SQLite-consistent snapshot mechanism.

---

# 12. Crash / Power-Failure Model

Required test matrix:

1. Kill before `BEGIN`.
2. Kill after idempotency claim but before transaction body.
3. Kill after document insert.
4. Kill after stock update.
5. Kill after ledger update.
6. Kill after print job creation but before COMMIT.
7. Kill immediately after COMMIT and before command response.
8. Printer process failure after COMMIT.

Expected guarantees:

- No partial sale.
- No duplicated financial effect.
- No missing stock movement for a committed sale.
- No missing credit movement for a committed credit sale.
- No missing cash effect for a committed cash sale.
- Every committed sale has either a printable `PENDING/DONE/FAILED` job state.

Integration tests use temporary file-backed SQLite databases. `:memory:` is reserved for pure query/domain tests that do not claim WAL/crash semantics.

---

# 13. Transactional Outbox and Printing

`print_jobs` is created inside the same sale transaction.

After COMMIT:

```text
Print Worker → PENDING job → renderer → device → status update
```

A printer failure never rolls back a committed sale.

A retry after the first physical print attempt is marked `DUPLICATA` to avoid visually presenting an uncertain retry as the original print.

The system must tolerate:

- USB disconnect.
- Printer powered off.
- Wrong port.
- Busy printer.
- Paper-out.
- Encoding mismatch.

The accounting state remains independent of printer state.

---

# 14. POS UX — Sovereign Cockpit

## 14.1 Shell

```text
TOPBAR 56px
├── Logo
├── Ctrl+K Command Palette
├── User
├── Cash Session
├── DB / Backup status
├── Clock
├── Language
└── Theme

RAIL 72px outside POS
hidden inside POS
```

## 14.2 POS Grid

```text
┌─────────────────────────────────────────────────────────────┐
│ Search / Barcode                                             │
├──────────────────────────────┬──────────────────────────────┤
│ Product Discovery 7 columns  │ Active Sale 5 columns       │
│                              │                              │
│ Quick Palette                │ Customer / Credit HUD       │
│ Instant Search               │ Cart                         │
│ Results                      │ Sticky Totals                │
│                              │                              │
├──────────────────────────────┴──────────────────────────────┤
│ Function Key Footer 56px                                    │
└─────────────────────────────────────────────────────────────┘
```

## 14.3 Design Rules

- `h-screen`, لا Page Scroll داخل POS.
- التمرير داخل الجداول فقط.
- Sticky Totals.
- F12 دائماً مرئي.
- Inline quantity editing.
- النوافذ المتعددة ممنوعة ما لم تكن العملية حساسة.
- لا Layout Shift في Hero Total.

## 14.4 Credit HUD

```text
UNLIMITED → Green
< 75%      → Green
75–100%    → Amber
> 100%     → Red + Block Checkout
```

عند التجاوز:

```text
F12 disabled
→ Manager Override
→ Authorization
→ Audit
→ Continue or Reject
```

---

# 15. Keyboard Contract

| Key | Action |
|---|---|
| F1 | Global Search |
| F2 | Customer / Cash-Credit |
| F3 | Cycle Unit |
| F4 | Edit Quantity |
| F5 | Discount |
| F6 | Resume Parked Sale |
| F7 | Manager Price/Discount Override |
| F8 | Park Sale |
| F9 | Open Cash Drawer |
| F10 | Cycle Document Type |
| F11 | Close Cash Session |
| F12 | Checkout / Payment |
| Esc | Close modal / clear search |
| Delete | Remove selected line + undo toast |
| Alt+1…8 | Quick products |
| ↑ ↓ | Cart row navigation |
| Enter | Confirm cell and advance |
| Ctrl+K | Command Palette |
| Ctrl+S | Save draft |

داخل text inputs لا تعترض الاختصارات إلا عندما تكون قاعدة السياق صريحة.

---

# 16. Visual Design System

## 16.1 Industrial Trust Palette

### Light

```text
primary-900 #0F3D5C
primary-600 #1E6091
primary-100 #DCEEF9
accent-500  #F59E0B
accent-600  #D97706
accent-700  #92400E
success     #16803C
warning     #B45309
danger      #C62828
info        #2563EB
bg          #F5F7FA
surface     #FFFFFF
border      #E5E7EB
text        #1A1D23
muted       #6B7280
```

### Dark

```text
primary-900 #1E3A5F
primary-600 #2E7BC4
primary-100 #132C45
accent-500  #FBBF24
accent-600  #F59E0B
success     #10B981
warning     #F59E0B
danger      #EF4444
info        #60A5FA
bg          #0B0F17
surface     #111827
border      #1F293D
text        #F9FAFB
muted       #9CA3AF
```

لا تستخدم الحالة باللون وحده؛ كل حالة تحتوي نصاً أو أيقونة.

## 16.2 Typography

```text
Arabic: Cairo Variable
French/Latin: Inter
Money/Data: JetBrains Mono
```

كل الخطوط محلية داخل التطبيق.

## 16.3 Sizes

```text
hero-total 40–48 / 800 Mono
display    32 / 700
pos-price  28 / 800
h1         24 / 700
h2         18 / 600
body       14 / 400
caption    12 / 400
kbd        11–12 / 600 Mono
```

## 16.4 Accessibility

- Touch target ≥ 44×44.
- Primary confirmation ≥ 56px.
- Focus-visible ring واضح.
- `aria-live` للحالات الديناميكية.
- `prefers-reduced-motion` يلغي الحركة غير الضرورية.
- WCAG AA كحد أدنى للواجهة.

---

# 17. Languages

UI locales:

```text
ar-DZ
fr-DZ
en
```

العربية RTL، الفرنسية/الإنجليزية LTR.

المستندات التجارية:

```text
AR / FR
```

`document_language` يحفظ اللغة لحظة إنشاء الوثيقة ولا يعاد تفسيرها لاحقاً من لغة الواجهة.

النصوص الحساسة للأعمال تستخدم i18n keys مرتبطة بأكواد DomainError، وليس رسائل نصية مبعثرة داخل Rust أو React.

---

# 18. Barcode and Product Discovery

## Barcode

- Code128 للباركودات الداخلية.
- EAN13 للمنتجات ذات الباركود التجاري.
- Unique barcode index على `product_units`.

## Scanner

Keyboard-Wedge.

منطق المسح لا يعتمد على driver خاص.

## Search

SQLite FTS5.

Normalization قبل الفهرسة.

المجال:

```text
name_ar_normalized
name_fr_normalized
sku
barcode
```

البحث بالاسم يعمل حتى عند اختلافات الكتابة العربية/الفرنسية.

---

# 19. Inventory Architecture

المخزون Source of Truth:

```text
stock_movements
```

والرصيد التشغيلي:

```text
stock_levels
```

## Sale Out

```text
validate stock
→ decrement stock_levels
→ insert SALE_OUT movement
```

## Return In

```text
validate return against original item quantity
→ increment stock_levels
→ insert RETURN_IN movement
→ create AVOIR
```

لا يسمح بأن تتجاوز الكميات المرتجعة إجمالي الكمية المباعة من المصدر.

الجرد لا يعدل الحركة القديمة؛ ينشئ `ADJUSTMENT_POSITIVE` أو `ADJUSTMENT_NEGATIVE`.

---

# 20. Credit Architecture

الحساب العميل يحتوي:

```text
Customer Master Data
+
Customer Ledger
+
Current Cached Balance
```

## Credit Sale

داخل نفس المعاملة:

```text
validate policy
validate limit
insert DEBIT
update cached balance
```

## Payment

```text
insert payment
insert CREDIT ledger
update cached balance
```

كل تسديد يحتفظ بمرجع واضح للوصل/المستند عند وجوده.

لا حذف لحركة قديمة.

---

# 21. Cash Architecture

Cash Session lifecycle:

```text
CLOSED
  ↓
OPEN
  ↓
CLOSING
  ↓
CLOSED
```

لا توجد جلستان مفتوحتان لنفس Terminal في آن واحد.

`system_expected_centimes` هو مجموع الحركة الموقعة خلال الوردية مع الفكة الافتتاحية.

الفارق:

```text
variance = actual_counted - system_expected
```

Z-Report هو Snapshot غير قابل للتغيير بعد الإغلاق.

---

# 22. Supplier / Purchase

المشتريات تعيد استخدام Documents Engine بدلاً من إنشاء محرك فاتورة منفصل.

الدورة:

```text
Supplier
→ BC
→ BR
→ FF
→ Supplier Ledger
→ Payment
```

`FF` يثبت تكلفة الشراء، و`stock_movements.PURCHASE_IN` يزيد المخزون.

قيد المورد:

```text
PURCHASE = DEBIT
PAYMENT = CREDIT
RETURN = CREDIT/compensating بحسب الاتجاه المحاسبي
```

القواعد النهائية للمورد تُغطى باختبارات المجال قبل تفعيلها.

---

# 23. Documents Engine

Document Types:

```text
TICKET
BL
FACTURE
DEVIS
AVOIR
BC
BR
FF
```

التحويلات:

```text
DEVIS → BL → FACTURE
```

والعكس المالي:

```text
FACTURE → AVOIR
```

لا يوجد حذف للمستندات النهائية.

كل تحويل يحتفظ بـ `source_document_id`.

---

# 24. Tax Engine

Tax Engine مستقل عن UI.

`tax_rules` مؤرخ:

```text
effective_from
effective_to
```

ويصدر `TaxSnapshot` داخل المستند.

أنواع المعالجة:

```text
STANDARD_19
REDUCED_9
EXEMPT_0
```

لا تستخدم `0%` وحدها لتمثيل كل الإعفاءات؛ تحفظ حالة الإعفاء وسببها عند الحاجة.

قواعد TVA والطابع قابلة لتحديث الإصدار الجبائي دون تغيير بنية الكود الأساسية.

---

# 25. Security Architecture

## Authentication

- Argon2id password hash.
- PIN hash مستقل للصندوق.
- Lockout بعد 5 محاولات فاشلة لمدة 5 دقائق.
- الجلسة محفوظة في الذاكرة ولا تحفظ كلمة المرور.

## Authorization

### VENDEUR

- POS.
- البحث.
- بيع نقدي.
- Park/Resume.
- لا صلاحيات مالية حساسة.

### CAISSIER

- كل صلاحيات Vendeur.
- قبض.
- كريدي collection.
- Cash session.
- Close shift.

### GERANT

- كل صلاحيات Caissier.
- Price/Discount override.
- Negative-stock exception.
- Credit-limit override.
- Returns.
- Stock adjustments.
- Purchases.
- Reports.

### ADMIN

- Users.
- System settings.
- Backup/restore.
- Device/security configuration.
- لا يمثل bypass مطلقاً لقواعد المجال؛ التجاوزات المالية تبقى Authorization + Audit.

---

# 26. Encryption and Key Management

Production DB uses an SQLCipher-compatible SQLite build when enabled in the release profile.

The encryption key:

- is not stored in `settings`.
- is not hard-coded.
- is retrieved from OS secure storage.
- is distinct from ordinary user passwords.

Backups use AEAD encryption such as XChaCha20-Poly1305.

SHA-256 may be used as an integrity checksum, but is never treated as authentication.

A Recovery Key can be exported once under authorized owner control and stored offline.

---

# 27. Backup and Disaster Recovery

## Backup events

- Z-Report / close shift.
- Every 24 hours while application is active.
- Manual backup.

## Backup content

Consistent snapshot + manifest:

```text
backup_id
app_version
schema_version
created_at
source_device_id
database_hash
cipher_version
```

## Rotation

Default 30 local copies, plus optional external/USB copy.

## Restore

```text
quarantine active DB
→ verify backup authenticity
→ decrypt
→ open using isolated recovery connection
→ integrity_check
→ foreign_key_check
→ audit chain validation
→ ledger reconciliation
→ activate restored DB
```

The system never silently overwrites a live healthy database from an unverified backup.

---

# 28. Future Sync Readiness

All primary mutable entities carry:

```text
updated_at
revision
origin_device_id
```

The architecture may additionally reserve `hlc` from day one if the sync roadmap is activated.

Conflict model:

- Financial ledgers: append-only events, not LWW.
- Master data: deterministic conflict policy may use revision/HLC/LWW only after explicit synchronization architecture is introduced.

No cloud connectivity is a prerequisite to local operation.

---

# 29. Reporting Architecture

Operational truth remains transactional tables.

Reports use:

```text
indexed SQL views / queries
→ profiling
→ derived aggregates only when justified
```

If materialized aggregates are introduced, they are rebuildable and never authoritative.

Required reports:

- Daily sales.
- Sales by product.
- Sales by category.
- Gross margin.
- Credit outstanding.
- Customer statement.
- Supplier balance.
- Stock movement.
- Low stock.
- Cash Z-Report.
- User activity.
- Returns.

---

# 30. Project Structure

```text
baraka-ultra-pos/
├── Cargo.toml
├── crates/
│   ├── domain/
│   ├── application/
│   ├── infrastructure/
│   └── commands/
├── src-tauri/
│   ├── src/main.rs
│   └── tauri.conf.json
├── src/
│   ├── app/
│   ├── components/
│   ├── features/
│   ├── hooks/
│   ├── stores/
│   ├── i18n/
│   ├── generated/
│   ├── lib/
│   └── App.tsx
├── tests/
│   ├── e2e/
│   ├── power-cut/
│   └── fixtures/
├── docs/
│   ├── adr/
│   ├── architecture/
│   └── operations/
└── Cargo.lock
```

Migrations live only inside the Infrastructure DB module and are embedded in the Rust binary. لا نسختين متوازيتين من migrations.

---

# 31. Testing Strategy

## Domain

- 100% line/branch coverage على domain-critical logic.
- Property-based tests للمال، TVA، rounding، ledger، units، numbering.

## Application

- كل Use Case.
- Authorization matrix.
- Idempotency matrix.
- Edge cases.

## Infrastructure

- Temporary file-backed SQLite.
- WAL.
- Foreign keys.
- Triggers.
- Backup/restore.
- Printer adapter.

## Power Cut

Child process + deterministic kill points.

## E2E

Playwright + Tauri driver.

## Static Quality Gates

```text
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --workspace
npm run lint
npm run typecheck
npm run build
```

CI architectural scans:

- no `f32/f64` in financial domain/application.
- no money `number` in TS financial modules.
- no `any`.
- no TODO.
- no direct SQLite import in React.
- no `document.querySelector` hacks where React refs are required.

---

# 32. Performance Contract

Targets:

| Metric | Target |
|---|---:|
| Cold boot | p95 < 2 s on reference profile |
| POS open | p95 < 200 ms |
| Product search | p95 < 50 ms at 80k products |
| Atomic sale transaction | p95 < 150 ms excluding physical printing |
| Print submission | p95 < 50 ms to queue |
| Arabic render | p95 < 250 ms target |
| Idle RAM | < 150 MB target |

Performance tests must declare:

- OS.
- CPU class.
- RAM.
- SSD type.
- Dataset size.
- warm/cold state.
- iteration count.
- p50/p95/p99.

The metric is an engineering target, not a legal or hardware-independent guarantee.

---

# 33. Observability Without Cloud Dependency

Local operational telemetry only:

- DB health.
- WAL/checkpoint state.
- Backup state.
- Print queue state.
- Last successful reconciliation.
- Current shift.
- Disk free space.

No telemetry endpoint is required for operation.

---

# 34. Startup Protocol

```text
Acquire single-instance lock
→ resolve DB path
→ open DB
→ apply required pragmas
→ verify schema version
→ run quick_check
→ run foreign_key_check when policy requires
→ verify audit chain tail
→ reconcile sentinel ledgers / cached balances
→ initialize print worker
→ initialize backup scheduler
→ initialize UI
```

Corruption enters Recovery Mode rather than destructive self-healing.

---

# 35. Shutdown Protocol

```text
stop accepting new commands
→ finish/abort active UI drafts
→ flush print worker state
→ optimize safely
→ checkpoint according to policy
→ backup if scheduled
→ close DB cleanly
→ release application lock
```

إغلاق التطبيق لا ينشئ Z-Report تلقائياً ما لم تكن سياسة المتجر محددة لذلك؛ الإغلاق الفيزيائي ليس بديلاً عن إغلاق الوردية.

---

# 36. ADRs Required Before Implementation

1. `ADR-001` — Tauri v2 + Rust.
2. `ADR-002` — SQLite + rusqlite, no runtime ORM.
3. `ADR-003` — Money/Qty/Factor fixed-point model.
4. `ADR-004` — Typed IPC wire representation.
5. `ADR-005` — Atomic Sale / Unit of Work.
6. `ADR-006` — Transactional Outbox for printing.
7. `ADR-007` — Append-only ledger enforcement.
8. `ADR-008` — Gapless committed document numbering.
9. `ADR-009` — SQLCipher production profile.
10. `ADR-010` — Backup / Recovery protocol.
11. `ADR-011` — FTS5 and normalization.
12. `ADR-012` — POS keyboard contract.
13. `ADR-013` — Tax rule versioning and legal validation gate.
14. `ADR-014` — Future synchronization metadata.
15. `ADR-015` — Performance benchmark methodology.

---

# 37. Implementation Roadmap

## P0 — Foundation

`TASK-BRK-001` — Repository / Tauri / Workspace skeleton.

## P1 — Sovereign Core

`TASK-BRK-002` — Database schema + domain primitives + transaction infrastructure.

## P1 — Auth

`TASK-BRK-003` — Local authentication + RBAC + session lifecycle.

## P2 — Catalog

`TASK-BRK-004` — Products + categories + units + barcode + kits.

`TASK-BRK-005` — Stock + warehouses + stock movements + inventory adjustment.

## P3 — Sales

`TASK-BRK-006` — Atomic sale engine.

`TASK-BRK-007` — POS Cockpit UI.

## P4 — Documents / Printer

`TASK-BRK-008` — Documents + ESC/POS + Arabic rendering + print outbox.

## P5 — Credit / Cash

`TASK-BRK-009` — Customer credit + ledger + statements.

`TASK-BRK-010` — Cash sessions + Z-Report.

## P6 — Suppliers / Purchases

`TASK-BRK-011` — Supplier + purchase lifecycle.

## P7 — Reports

`TASK-BRK-012` — Reports + derived aggregates after profiling.

## P8 — Hardening

`TASK-BRK-013` — Backup + restore + encryption + performance/security hardening.

---

# 38. Definition of Production Readiness

Production release requires all of the following:

```text
[ ] Architecture baseline frozen
[ ] Canonical schema frozen
[ ] Migration checksum verification
[ ] Financial property tests pass
[ ] Atomic sale crash matrix passes
[ ] Idempotency concurrency tests pass
[ ] Ledger reconciliation passes
[ ] No unauthorized negative stock
[ ] Finalized document immutability tests pass
[ ] Print outbox reliability tests pass
[ ] Backup restore drill passes
[ ] Encryption key recovery drill passes
[ ] RBAC matrix passes
[ ] E2E critical flows pass
[ ] Performance benchmarks recorded
[ ] No any / no TODO / no warnings
[ ] Tax/legal rules verified for the current release year
[ ] Real hardware printing verified
[ ] Data migration/upgrade drill passes
```

---

# 39. Final Architecture Verdict

## Architectural status

**GO — READY FOR IMPLEMENTATION**

The design is now implementation-ready because the major contradictions identified in the previous blueprint have been resolved at the architectural level:

- Drizzle/Rust ownership contradiction removed.
- Circular timbre formula removed.
- In-memory/WAL test mismatch removed.
- Retry policy narrowed to transient failures.
- Print outbox moved inside the transaction before commit.
- Credit limit semantics made explicit.
- IPC money representation made exact and language-safe.
- Append-only enforcement widened to all critical ledgers.
- Backup/restore hardened into a verifiable recovery protocol.
- Performance claims converted to measurable benchmark contracts.

## Important gate

The architecture is ready for coding, but legal/tax compliance is a separate controlled gate. The Algerian Directorate General of Taxes currently states a normal TVA rate of 19% and a reduced rate of 9%, and its 2026 publications include updated tax-code materials; therefore tax rules must be versioned and validated against the applicable 2026 legal text before a release is presented as compliant. 

## Sovereign rule

This document is the single architectural baseline. Any deviation requires a new ADR and explicit architectural authorization before implementation.