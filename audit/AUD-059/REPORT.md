# AUD-059 — قبول الموجة 1: إثبات تطابق الدفتر-المطبوع

النطاق: `HEAD=d70bf5e` (286 commit) — BRK-108 + BRK-109 + DSN-103 + DSN-104.
المنهج: تتبع السلسلة `DB → wire → أسطح` بالـ `file:line` + إعادة تشغيل البوابات
G1–G7 كاملة على نفس الشجرة + فحص الانضباط (لا floats، لا كتابة في ترحيل قديم).
التخمين محظور — كل رقم أدناه من مخرجات حقيقية.

## 1. الحكم: الموجة 1 مقبولة (ACCEPTED)

| المحور | الدرجة | الدليل |
|---|---|---|
| لقطة المعادلة (تجميد ذري) | A+ | `0020` إلحاقي؛ التجميد داخل `BEGIN IMMEDIATE` قبل أي كتابة (`sqlite_checkout_repository.rs:282-284`)؛ 3 اختبارات احتفاظ (`credit_equation_snapshot.rs`) |
| تطابق الصيغة محرك↔واجهة | A+ | المحرك `ttc − |delta|` (`tspl_printing.rs:523-529`) = الواجهة `montant − magnitude` (`credit-equation.ts:56-57`) + تحقق من الاتجاهين قبل الطباعة (شحن/AVOIR) |
| أسطح المعادلة الأربعة | A | تذكرة V1 (`graphique-ticket.ts:301`) + V2 (`:518`) + A4 (`DocumentDetailDrawer.tsx:241`) + TSPL V2 (`tspl.rs:1651-1668`، شريحة `CreditEquation` في `:651`) — كلها من نفس اللقطة، وتنهار عند `None` |
| تفكيك Colisage | A+ | النطاق `breakdown` (6 اختبارات نطاق، `cargo test -p baraka-domain colisage`: ‏6/0) ≡ المساعد `colisageShares` (6 اختبارات، نفس المتجهات: 150 قطعة → 12 دزينة + 1 نصف) |
| عرض Colisage | A | سلة حية (`CartTable.tsx:170`) + V1 (`graphique-ticket.ts:264`) + V2 (`:406`) + A4 (`DocumentDetailDrawer.tsx:174`) — تمرير حرفي للسلك (`CheckoutModal.tsx:375`، الدرج `:648`) |
| انضباط المال | A+ | صفر `Number(`/`parseFloat` في `colisage.ts` + `credit-equation.ts` + `privacy-shield-store.ts`؛ BigInt/i128 فقط |
| الهجرات | A+ | `0001..0019` بلا مساس + `0020` إلحاقي (أعمدة NULL بلا backfill) |
| البوابات (طازجة) | A+ | G1:0، G2:0، G3: ‏643/0/7 EXIT 0 (75 نتيجة)، G4:0، G5:0 صفر `error TS`، G6:0 (9.37s)، G7: ‏92 ملفًا/948 اختبارًا EXIT 0 |

## 2. ملاحظات (لا شيء يمنع القبول)

- **F1 (مؤجل — منخفض):** نص Colisage في أسطر TSPL خارج الموجة بقرار موثق
  (`ADR-113`): مسار المحرك pinned بالبايت ويحتاج mirror test واعتماد مالك.
- **F2 (نطاق مقصود):** درع الخصوصية `Ctrl+H` يغطي عنوان هامش السلة فقط؛
  شاشات التقارير/الكتالوج للمدير ولا تُقنَّع — موثق في الكود.
- **F3 (بيئة — لا علاقة له بالكود):** قرص `C:` ممتلئ (`ENOSPC`) فيتطلب
  `npm --cache D:/hermes/...`؛ وصفة `hermes verify` تفشل في
  `[bootstrap] pnpm install` (المستودع npm أصلًا).

## 3. ما بعد القبول

الموجة 1 مغلقة. التالي الجاهز حسب الخارطة: `TASK-BRK-110` (مجمّع أرصدة
الزبائن + تنبيهات حدود الائتمان). `NEXT READY: YES`.
