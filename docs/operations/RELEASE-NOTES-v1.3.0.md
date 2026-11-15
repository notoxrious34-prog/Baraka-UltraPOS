# Baraka UltraPOS v1.3.0 — Notes de version test / مذكرة إصدار التجريب

> Field-test build for real shop users (FR/AR). Install over v1.2.1.
> Offline-first: no internet needed on the shop machine.

---

## Français — Quoi tester

1. **Bloc crédit** : vente à crédit → le ticket/A4 montre
   `Ancien + Montant − Versement = Nouveau`. Bouton `[H]` masque les marges.
2. **Colisage** : lignes en carton affichent `1 Carton + 4 Pièce`.
3. **Radar crédit** : module Clients → bandeau encours/débiteurs/surveillance ;
   fiche client → bannières rouge/ambre.
4. **Profil** : `Paramètres → Identité` → choisissez votre commerce
   (Sous-vêtements / Quincaillerie / Hybride) → les niveaux proposés changent.
5. **Super-Gros** : `Catalogue → Modifier` → renseignez `Prix super-gros` →
   un client Super-Gros est facturé à ce prix au comptoir.
6. **Dernier prix** : avec un client nommé, la ligne affiche
   `Dernier prix : …` (rappel seul — le moteur ne réapplique jamais).

## العربية — ماذا تُجرِّب

1. **كتلة الآجل**: بيع آجل → التذكرة تعرض `السابق + المبلغ − الدفعة = الجديد`. زر `[H]` يخفي الهوامش.
2. **التعبئة**: سطور الكرتون تعرض `1 كرتون + 4 قطع`.
3. **رادار الآجل**: وحدة الزبائن → الشريط الإجمالي؛ ملف الزبون → اللافتات.
4. **النشاط**: `الإعدادات → الهوية` → اختر نشاطك → تتغير الفئات المقترحة.
5. **السوبر-جملة**: `الكتالوج → تعديل` → أدخل `سعر السوبر-جملة` → زبون السوبر-جملة يُحاسَب به.
6. **آخر سعر**: مع زبون مسمى، السطر يعرض `آخر سعر: …` (تذكير فقط).

---

## ⚠️ Avant d'installer / قبل التثبيت — OBLIGATOIRE

**Français.**
1. Dans l'application actuelle : `Sauvegarde` → conservez le fichier `.db`
   sur une clé USB. **Sans cette sauvegarde, aucun retour en arrière.**
2. Cette version fait migrer la base vers le schéma **v22** (ajouts seuls,
   aucune donnée touchée). Par sécurité, l'ancien v1.2.1 **refusera**
   d'ouvrir une base v22 — le retour exige la restauration de la sauvegarde.
3. Signalez tout comportement anormal avec : l'écran, l'heure, et ce que
   vous faisiez (photo du ticket bienvenue).

**العربية.**
1. من النسخة الحالية: `نسخ احتياطي` → احتفظ بملف `.db` على USB. **بدونها لا رجوع.**
2. هذه النسخة ترقّي القاعدة إلى **v22** (إضافات فقط، لا تمس البيانات). النسخة القديمة **ترفض** فتح قاعدة v22 — الرجوع يتطلب استعادة النسخة الاحتياطية.
3. بلّغ عن أي خلل مع: الشاشة والوقت وماذا كنت تفعل (صورة التذكرة مرحب بها).
