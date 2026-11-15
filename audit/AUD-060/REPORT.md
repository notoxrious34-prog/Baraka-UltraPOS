# AUD-060 — التحقق من نطاق Devis→BL القائم (V0-1 من المخطط الشامل)
**النطاق:** هل مقترح S2 (Quote-to-Picklist) موجود أصلًا؟ قراءة فقط، صفر كود.
**المنهج:** تتبع السلسلة `DB → use case → IPC → UI` بالـ `file:line` + مواضع الدخول.

## 1. الحكم: المحرك مكتمل — الدلتا UX فقط (صغيرة، frontend)
| المحور | الحالة | الدليل |
|---|---|---|
| ذرية التحويل | ✅ موجود | `ports.rs:789-797` (`convert_devis_to_bl` داخل `run_transaction`)؛ idempotency + قفل المصدر + ترقيم BL بلا فجوات + ختم CONVERTED + outbox طباعة |
| تجميد الأسعار | ✅ بالبناء | نسخ حرفي للسطور: `unit_price/discount/tva/cost/sort_order` (`sqlite_conversion_repository.rs:378-397`) — السعر المجمد هو سعر الـDevis نفسه |
| الخصم المخزني | ✅ | `SALE_OUT` للأوراق + `authorize_negative_stock` (`ports.rs:865-867`) |
| BL→Facture | ✅ | `ports.rs:799-808` + تسوية دقيقة + صفر كتابة مخزنية (ADR-046) |
| سلك IPC | ✅ | `crates/commands/src/ipc/sales.rs` + `src/lib/ipc.ts` + `ipc-conversion.test.ts` |
| المودال | ✅ | `DocumentConversionModal.tsx` (377 سطرًا): تبويبا devis_to_bl/bl_to_facture، بحث→اختيار→تحويل→نجاح برقم + تحذيرات؛ `conversion-modal.test.ts` (373) |
| نقطة الدخول | ⚠️ موجودة لكن عميقة | Caisse → `Opérations` → مجموعة documents → `conversion` (`OperationsMenu.tsx:53,84-86,138-139`) → بحث يدوي → اختيار → تحويل (4–5 نقرات) |
| زر一键 من الـDevis نفسه | ❌ غائب | صفر إشارة `Convert` في `src/features/documents/` — لا زر في جدول الوثائق ولا في الدرج |

## 2. الدلتا الدقيقة (DSN-113 — frontend فقط، صفر محرك/ترحيل)
1. زر `تحويل إلى BL` على صف الـDevis وفي درج التفاصيل يفتح المودال على
   `devis_to_bl` والمستند محدد مسبقًا (نفس الـstore والـIPC — إعادة استعمال).
2. (اختياري) عرض قائمة تعبئة مرتبة (colisage ظاهر أصلًا على A4 عبر DSN-104؛
   الترتيب حسب الرف يُدرس في DSN-113).
3. إثبات بصري لطباعة BL A4 يبقى معلقًا على بيئة طباعة (نفس قيد الطابعة العام).

## 3. الأثر على المخطط
- S2 يُغلق كعمل محرك؛ يُستبدل بـ`DSN-113` (نطاقها أعلاه) في Phase S.
- لا تغيير على G-D4: التوصيف الجديد «تحقق UX + دلتا» تأكد.
- التالي: `AUD-061` (الجذر التقني F-B4/F-B1) — يُنفَّذ الآن.
