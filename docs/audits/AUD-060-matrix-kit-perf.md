# AUD-060 — Matrix/Kit Atomicity + FTS5-Recall Audit with G3+ Perf Gate

**الحالة:** مغلق-مشروط (بوابة G3+ مشروطة بإصلاح F-1 قبل Wave 5).
**النطاق:** قراءة فقط — صفر تعديلات إنتاجية. كل القياسات على قواعد scratch مؤقتة.
**الجهاز:** Windows 10 Pro | Intel i5-3450 @ 3.10GHz | 16GiB RAM | SATA SSD.
**نمط البناء للقياسات:** `debug` (يُصرَّح به — أرقام `release` أعلى، لا أدنى).

## 1. الذرّية (مراجعة كود، file:line)

| المسار | الحكم | الدليل |
|---|---|---|
| `generate_matrix` (BRK-114) | ذري ✅ | `sqlite_variant_repository.rs:79` — `run_transaction` يلف `variant_models` + منتجات الخلايا + `variant_cells` |
| `define_kit` (BRK-115) | ذري ✅ | `sqlite_catalog_repository.rs:772` — `DELETE + INSERT`s داخل `run_transaction` |
| `persist_return` (BRK-096) | ذري ✅ | `sqlite_return_repository.rs` — `run_transaction` واحدة (AVOIR + restock + تسوية + idempotency) |
| `persist_exchange` (BRK-125) | ذري ✅ | `persist_exchange_atomic` — حزمة AVOIR+TICKET+أسهم+تسوية واحدة |
| `ComposeSerieUseCase` | ذري بشروط ⚠️ | فحص التوفر قراءة فقط (`variant_matrix.rs:860`) ثم `define_kit` الذرية — نافذة TOCTOU بين الفحص والتعريف، لكن التعريف نفسه لا يخصم مخزونًا (الخصم عند البيع عبر `kit expansion`) فلا فساد حالة |

## 2. بوابة الأداء G3+ (10k SKUs / 1M movements)

### 2.1 مسبارات الإصدار الموجودة (10k منتجات، debug)

```
BENCH atomic_sale n=50 p50=7.876ms p95=18.058ms p99=42.971ms
BENCH print_job_insert n=50 p50=0.706ms p95=1.183ms p99=1.252ms
BENCH product_search n=50 p50=0.296ms p95=27.939ms p99=29.252ms
```

المصدر: `D:/hermes/Home/cache/scratch/aud060-bench.log` — `release_benchmarks -- --ignored` أخضر (3/3).

### 2.2 مسبار审计 الخاص (1M حركة، مخطط وأعمدة مطابقة للإنتاج)

```
Q1 cumulative-return  | n=50 | p50=0.04ms | p95=0.07ms  | max=0.23ms
Q2 stock-level-point  | n=50 | p50=0.03ms | p95=0.13ms  | max=0.37ms
Q3 matrix-view-24cell | n=50 | p50=26.05ms| p95=29.24ms | max=43.25ms
Q4 ledger-tail-50     | n=50 | p50=0.39ms | p95=0.65ms  | max=2.56ms
Q5 movements-tail-100 | n=50 | p50=1.11ms | p95=2.77ms  | max=3.76ms
```

### 2.3 الاستنتاج F-1 (حرج): `matrix_view` يمسح `stock_levels` كاملة لكل خلية

```
EQP Q3:
  SEARCH c USING INDEX idx_variant_cells_model (model_id=?)
  CORRELATED SCALAR SUBQUERY 1
  SCAN s                              <-- مسح كامل لكل خلية
```

السبب: الاستعلام الفرعي `WHERE s.product_id = p.id` لا يستطيع استخدام المفتاح
`(warehouse_id, product_id)` (العمود الأول غير مقيد). النتيجة خطية مزدوجة
(خلايا × مخزون): نموذج 500 خلية على 10k صنف = **p50 ‏536.7ms** (مقاس فعليًا).
لوحة DSN-108 تفتح هذا الاستعلام لكل موديل — نصف ثانية تجميد كاونتر.

**التوصية (ليست تنفيذًا — تحتاج أمر مالك كـ BRK):** فهرس تغطية
`stock_levels(product_id, quantity_base_micro)` أو إعادة الصياغة كـ `GROUP JOIN`
وحيد. صفر تغييرات في هذا التدقيق.

## 3. FTS5 recall (مكتمل)

- البنية: `products_fts` ‏(0016) + محفزات `ai/ad/au` للمنتجات والوحدات + `ai/ad` للأسماء المستعارة.
- `product_aliases` تُقرأ فقط (`sqlite_catalog_repository.rs:224`) ولا مسار `UPDATE` — غياب محفز `au` للأسماء غير مؤثر.
- مسبار 80k (`search_perf_80k -- --ignored` أخضر 1/1، ‏68 دقيقة زرع + قياس، debug):

```
QUERY | IT | p50_ms | p95_ms | max_ms | hits | verdict_vs_50ms
q1-fr vis            | 50 | 204.70 | 233.82 | 245.83 | 100 | OVER-TARGET
q2-ar فيس            | 50 | 202.06 | 218.29 | 229.67 | 100 | OVER-TARGET
q3-fr boulon         | 50 | 201.47 | 216.10 | 265.20 | 100 | OVER-TARGET
q4-infix oul         | 50 |   1.31 |   5.19 |   8.11 |   0 | UNDER-TARGET
q5-barcode 80K00042424|50 |   0.26 |   0.60 |   1.47 |   1 | UNDER-TARGET
q6-multi cadenas 40mm | 50 |  61.77 |  70.56 |  80.31 | 100 | OVER-TARGET
```

المصدر: `D:/hermes/Home/cache/scratch/aud060-search80k.log`.

**الاستنتاج F-2 (تحفظ، لا حجب):** الاستدعاء صحيح حتميًا (عدد النتائج مطابق
للمتوقع في كل استعلام) لكن زمن FTS5 ‏MATCH عند 80k صنف يتجاوز عتبة 50ms في
نمط debug (‏≈200ms للاستعلامات الشائعة). عند مقياس البوابة (10k) مسبار M1
يُظهر p95 ‏28ms في debug — داخل العتبة. التوصية: إعادة القياس بنمط release
قبل Wave 5 + النظر في تضييق أعمدة FTS5 عند الحاجة. لا يُحجب Wave 5 على F-2.

## 4. حكم البوابة (نهائي)

- Q1/Q2/Q4/Q5: تمر بفارق مريح (أقل من 3ms عند p95).
- Q3: **يفشل** عند المصفوفات الكبيرة (536ms > عتبة 50ms التفاعلية) — إصلاح F-1 شرط Wave 5.
- M1/M2/M3: تمر (M1 ثنائي النمط p95 ‏28ms في debug — يُراقَب في release).
- FTS5 ‏80k: استدعاء صحيح؛ الزمن فوق العتبة في debug عند 8× مقياس البوابة — تحفظ F-2 لا حجب.

**الخلاصة:** البوابة خضراء مشروطة — Wave 5 ينطلق بعد تسجيل إصلاح F-1 كمهمة BRK.

💡 **ابتكارات وأفكار تطويرية مقترحة:**
1. فهرس `stock_levels(product_id)` التغطوي يخدم أيضًا `Q5` وتقارير المخزون (Business: فتح فوري لشبكة المقاسات على أي موديل؛ Path: ترحيل `0025` إلحاقي + قياس).
2. تخزين مؤقت لـ `matrix_view` لكل موديل مع إبطال عند حركة مخزون (UX: شبكة فورية؛ Path: `moka`-free — خريطة ذاكرة بسيطة خلف المنفذ).
3. عتبة تحذير في التشخيص الذاتي عند تجاوز `matrix_view` ‏100ms (Business: كشف مبكر قبل الشكوى؛ Path: قياس `Instant` حول الاستعلام + عدّاد).
