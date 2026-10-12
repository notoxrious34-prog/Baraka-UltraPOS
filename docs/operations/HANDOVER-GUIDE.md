# Baraka UltraPOS v1.0.0 — Guide de remise au gérant / دليل التسليم للتاجر

> Practical handover handbook for the shop owner (FR/AR). Companion to
> `release-notes-v1.0.0.md` (install + safety) and `thermal-printer-drill.md`
> (printer proof). Keep this guide next to the cash register.

---

## Partie 1 — Artefact & Installation / الجزء 1: ملف التثبيت

**Français.**
Le technicien vous remet un seul fichier :

- Nom : `Baraka UltraPOS_1.0.0_x64-setup.exe`
- Taille : environ 211 Mo (221 552 107 octets)
- Empreinte SHA-256 :
  `C608C32A8C96B089CB51E187DE473AD0FCCD3E728E8B1C07D0311C1E65D75934`
  (le technicien peut la revérifier avant d'installer)
- Windows 10 ou 11, **aucune connexion internet requise** — tout le
  nécessaire (y compris le moteur d'affichage WebView2) est dans le fichier.
- Double-cliquez, choisissez le français, installez depuis le compte Windows
  qui tiendra la caisse (installation par utilisateur). Si Windows affiche
  l'écran bleu SmartScreen (« Windows a protégé votre PC »), cliquez
  « Informations complémentaires » puis « Exécuter quand même » : c'est
  attendu, l'installeur n'est pas signé numériquement.

**العربية.**
يُسلّمك التقني ملفًا واحدًا: `Baraka UltraPOS_1.0.0_x64-setup.exe`
(نحو 211 ميغا). يعمل دون إنترنت على Windows 10 أو 11. ثبّته من حساب
Windows الذي سيستعمل الصندوق. إذا ظهر تحذير SmartScreen الأزرق فاضغط
«Informations complémentaires» ثم «Exécuter quand même» — هذا متوقّع.

---

## Partie 2 — Premier démarrage & Configuration / الجزء 2: أول تشغيل وضبط

**Français.**

1. Au premier lancement, créez le compte **ADMIN** (le gérant) : nom
   d'utilisateur + mot de passe sûr. Un seul ADMIN est créé ; conservez le
   mot de passe en lieu sûr.
2. Paramètres → « Identité & Impression » (compte ADMIN) : renseignez le nom
   du magasin, l'adresse, le téléphone et les identifiants fiscaux
   (**RC / NIF / NIS / AI**) — ils apparaissent sur l'en-tête des tickets.
3. Un magasin par défaut existe déjà (créé automatiquement). Ajoutez vos
   autres magasins si besoin depuis le module Stock.
4. En haut, basculez la langue : français ou arabe (l'arabe s'affiche de
   droite à gauche, RTL).

**العربية.**
عند أول تشغيل أنشئ حساب المدير (ADMIN) واحفظ كلمة المرور. من الإعدادات ←
«الهوية والطباعة» أدخل اسم المحل والعنوان والهاتف والمعرفات الجبائية
(RC / NIF / NIS / AI) فتظهر في ترويسة التذاكر. يوجد مخزن افتراضي جاهز،
ويمكن التبديل بين الفرنسية والعربية (العربية من اليمين إلى اليسار).

---

## Partie 3 — Matériel : imprimante & douchette / الجزء 3: الطابعة وقارئ الباركود

**Français.**

- Imprimante recommandée : ticket thermique **80 mm ESC/POS**, branchée en
  port série ou USB avec pilote exposant un **port COM virtuel**. Notez le
  port (ex. COM3) dans le Gestionnaire de périphériques Windows.
- Paramètres → « Identité & Impression » : renseignez le port série et le
  débit, puis utilisez le **bouton d'impression de test** (diagnostic sans
  effet comptable : aperçu à l'écran + fichier de capture `.escpos`).
- Avant la mise en service, faites le « drill » avec le technicien
  (`thermal-printer-drill.md`) : test en capture seule, test physique sur
  papier, essais câble débranché / imprimante éteinte / papier retiré, puis
  **ligne signée** dans le tableau de validation.
- Douchette code-barres : tout modèle USB qui frappe au clavier
  (« plug-and-play ») fonctionne — branchez, cliquez dans le champ de
  recherche, scannez.

**العربية.**
الطابعة الموصى بها: حرارية 80 مم ESC/POS عبر منفذ تسلسلي أو COM افتراضي.
اضبط المنفذ والسرعة من الإعدادات ثم جرّب زر اختبار الطباعة (تشخيص دون أي
أثر محاسبي). قبل التشغيل الحقيقي نفّذ تجربة الطباعة مع التقني ووقّع جدول
التحقق. قارئ الباركود USB يعمل مباشرة: انقر في حقل البحث ثم امسح.

---

## Partie 4 — Cycle quotidien / الجزء 4: العمل اليومي

**Français.**

1. **Ouvrir la session de caisse** : module Sessions → Ouverture (fond de
   caisse compté). Sans session ouverte, pas de vente.
2. **Vendre** : module Caisse — scannez ou recherchez le produit (nom ou
   code-barres), ajustez les quantités, appliquez la remise, encaissez en
   espèces (le rendu est calculé). Vente à crédit pour un client connu :
   choisissez le client, la vente alimente son compte.
3. **Mettre en attente / reprendre** : **F8** parque la vente en cours
   (brouillon), **F6** la reprend — pour servir le client suivant.
4. **Encaisser une dette (سند قبض)** : module Clients → dossier du client →
   encaissement. Le reçu de recouvrement s'imprime ; les trop-perçus sont
   bloqués (on n'encaisse jamais plus que le dû).
5. **Clôturer la journée** : Sessions → Clôture — comptez le tiroir,
   l'écart éventuel est enregistré. C'est votre « rapport Z » quotidien :
   ventes, crédit, écarts.

**العربية.**
1. افتح جلسة الصندوق من الوحدة Sessions (مع عدّ صندوق البداية).
2. بِع من وحدة Caisse: امسح أو ابحث، عدّل الكميات والخصم، اقبض نقدًا (الباقي
   محسوب). البيع بالدين: اختر الزبون فيُقيَّد على حسابه.
3. **F8** يعلّق البيع الجاري (مسودة)، و**F6** يستأنفه.
4. تحصيل دين (**سند قبض**): من ملف الزبون — يُطبع الإيصال، والزيادة عن
   المستحق ممنوعة.
5. أغلق اليوم من Sessions ← Clôture مع عدّ الدرج: المبيعات والديون والفروق
   مسجلة (تقرير Z اليومي).

---

## Partie 5 — Sécurité des données & Sauvegardes / الجزء 5: سلامة البيانات

**Français.**

- Vos données (ventes, stock, clients) vivent **sur votre PC** :
  `%APPDATA%\com.baraka.ultrapos\baraka.db`. **Désinstaller ou réinstaller
  ne les efface jamais.**
- Sauvegarde manuelle : Paramètres → Sauvegardes → Sauvegarder, puis copiez
  le fichier sur une **clé USB gardée hors du magasin**. En cas de panne :
  réinstallez, restaurez la dernière sauvegarde (compte ADMIN), vérifiez une
  vente et le stock avant de reprendre.
- Ne supprimez jamais le dossier `%APPDATA%\com.baraka.ultrapos` vous-même.

**العربية.**
بياناتك على جهازك في `%APPDATA%\com.baraka.ultrapos\baraka.db` — إلغاء
التثبيت أو إعادة التثبيت لا يحذفها أبدًا. انسخ نسخة احتياطية (من الإعدادات ←
النسخ الاحتياطي) دوريًا على USB خارج المحل. عند عطل: أعد التثبيت ثم استعد
آخر نسخة (بحساب المدير) وتحقق من بيع ومخزون قبل استئناف العمل.

---

## Partie 6 — Check-list du drill imprimante / الجزء 6: قائمة تجربة الطباعة

**Français.** Référence complète : `thermal-printer-drill.md`.

- [ ] Papier 80 mm chargé, imprimante allumée, câble branché, port COM noté.
- [ ] Test « capture seule » : aperçu correct (identité, NIF/NIS/RC/AI,
      totaux, pied de ticket, coupe).
- [ ] Test physique : papier identique à l'aperçu, point par point.
- [ ] Tolérances : USB débranché / éteinte / sans papier → badge d'erreur
      propre, application utilisable.
- [ ] **Ligne signée** (date, opérateur, marque/modèle, COMx, OK) — sans
      elle, la mise en service n'est pas clôturée.

**العربية.**
المرجع الكامل: `thermal-printer-drill.md`. الورق محمّل والطابعة مشغلة
والكبل موصول ومنفذ COM معروف ← اختبار التقاط فقط والمعاينة سليمة ← اختبار
فيزيائي مطابق للمعاينة ← اختبارات التحمل (فصل/إطفاء/نزع الورق) بشارة نظيفة ←
**سطر موقّع** في الجدول، وبدونه لا يُغلَق التشغيل.

---

*Fin du guide / نهاية الدليل — Baraka UltraPOS v1.0.0.*
