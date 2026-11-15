# Baraka UltraPOS v2.0 — Guide de l'opérateur / دليل المشغّل

> Practical counter handbook for the shop owner and cashiers (FR/AR).
> Covers what changed from v1.2.x to v2.0: credit equation, colisage,
> credit radar, commercial profile, Super-Gros tier, last-price recall.
> Nothing below changes the ledger: it stays the single truth.

---

## 1 — Bloc d'équation crédit / كتلة معادلة الآجل

**Français.**
Sur chaque ticket, A4, BL et facture d'une vente à crédit, un bloc imprimé
montre : **Ancien Solde + Montant Document − Versement = Nouveau Solde**.
Le `Versement` affiché est calculé (`Montant − |delta|`), jamais saisi.
Si un avoir (AVOIR) est émis, le bloc applique la forme miroir
(`Ancien − Montant + Versement = Nouveau`). L'écran du caissier et le
tiroir `Détails document` montrent le même bloc. Raccourci vie privée :
`Ctrl+H` (ou le bouton `[H]`) masque les marges aux yeux du comptoir.

**العربية.**
كل تذكرة وA4 ووصل تسليم وفاتورة لبيع آجل تطبع كتلة: **الرصيد السابق +
مبلغ الوثيقة − الدفعة = الرصيد الجديد**. الدفعة محسوبة لا مُدخلة. وثيقة
الاسترجاع (AVOIR) تطبع الصورة المعكوسة. شاشة الصندوق وتفاصيل الوثيقة
تعرضان نفس الكتلة. زر `[H]` (أو `Ctrl+H`) يخفي الهوامش عن أعين الكونطوار.

---

## 2 — Affichage colisage / عرض التعبئة

**Français.**
Sous chaque ligne du panier, du ticket et du A4, la décomposition
carton/pièce s'affiche quand elle existe (ex. `1 Carton + 4 Pièce`).
Elle est calculée en direct depuis les unités du produit — jamais stockée.
L'impression thermique des étiquettes reste inchangée (format 40×20).

**العربية.**
تحت كل سطر في السلة والتذكرة وA4 يظهر تفكيك الكرتون/القطعة عند وجوده
(مثال: `1 كرتون + 4 قطع`). يُحسب لحظيًا من وحدات المنتج ولا يُخزَّن.
طباعة الملصقات الحرارية لم تتغير (40×20).

---

## 3 — Radar crédit / رادار الآجل

**Français.**
Le module Clients affiche un bandeau global : encours total, nombre de
débiteurs, dépassements, clients sous surveillance (80 % du plafond).
La fiche client affiche une bannière rouge (plafond dépassé — crédit
bloqué) ou ambre (presque atteint). Règle : un client sans plafond ne
monte jamais en alerte ; un solde nul ou négatif reste `OK`.

**العربية.**
وحدة الزبائن تعرض شريطًا إجماليًا: مجموع المستحقات وعدد المدينين
والتجاوزات ومن هم تحت المراقبة (80٪ من السقف). ملف الزبون يعرض لافتة
حمراء (تجاوز السقف — الائتمان محظور) أو كهرمانية (شارف على النفاد).
القاعدة: من لا سقف له لا يُنذَر أبدًا؛ الرصيد الصفري أو السالب سليم.

---

## 4 — Profil commercial / النشاط التجاري

**Français.**
`Paramètres → Identité` propose le **profil** du commerce :
*Sous-vêtements Détail & Gros* (`Détail / Gros / Super-Gros`),
*Quincaillerie* (`Détail / Artisan / Gros`) ou *Hybride* (toutes les
niveaux). Le profil ne change que le vocabulaire du comptoir (filtres,
création client, badges `PRO / GROS / Super-Gros`) — prix, stock et
grand livre inchangés. Un niveau hors profil est signalé en ambre et
forcé au niveau par défaut du profil à l'enregistrement.

**العربية.**
`الإعدادات → الهوية` تعرض **النشاط التجاري**: ملابس داخلية
(تجزئة/جملة/سوبر-جملة)، خردوات (تجزئة/حرفي/جملة)، أو مختلط (كل الفئات).
النشاط يغيّر مفردات الكونطوار فقط — الأسعار والمخزون والدفتر ثابتة.
الفئة الخارجة عن النشاط تُنبَّه بالكهرماني وتُجبَر على الافتراضية عند الحفظ.

---

## 5 — Tarif Super-Gros / سعر السوبر-جملة

**Français.**
Chaque unité peut porter un 4ᵉ prix : **Super-Gros** (vide = repli sur
Gros, puis Détail). Il se saisit dans `Catalogue → Modifier` (champ
`Prix super-gros`, vide = effacer) et s'applique automatiquement aux
clients `Super-Gros` au comptoir. Les autres niveaux ne voient jamais
cette jambe de prix.

**العربية.**
كل وحدة تحمل سعرًا رابعًا: **السوبر-جملة** (الفارغ = سقوط للجملة ثم
التجزئة). يُدخَل من `الكتالوج → تعديل` (حقل `سعر السوبر-جملة`، والفارغ
يمسح). يُطبَّق تلقائيًا على زبائن السوبر-جملة في الصندوق. بقية الفئات
لا ترى هذا السعر أبدًا.

---

## 6 — Dernier prix / آخر سعر

**Français.**
Pour un client nommé, chaque ligne du panier peut afficher
`Dernier prix : 95,00` — le prix exact transigé la dernière fois
(remise incluse dans le calcul), avec la date en survol. C'est un
**rappel seul** : le caissier décide, le moteur ne réapplique jamais.
Aucune puce pour les invités ni les articles jamais vendus à ce client.

**العربية.**
للزبون المسمى قد يعرض كل سطر `آخر سعر: 95.00` — السعر المتعامَل به آخر
مرة (بعد التخفيض) مع التاريخ عند التحويم. هذا **تذكير فقط**: الصندوق
يقرر والمحرك لا يعيد التطبيق أبدًا. لا شريحة للضيوف ولا للأصناف التي
لم تُبع لهذا الزبون.

---

## 7 — Ce qui n'a pas changé / ما لم يتغير

**Français.**
Caisse 100 % hors-ligne ; prix en centimes entiers ; remises et TVA
calculées par le moteur ; sauvegardes et restauration identiques ;
rôles et mots de passe inchangés.

**العربية.**
الصندوق دون إنترنت تمامًا؛ الأسعار بسنتيمات صحيحة؛ التخفيضات والضريبة
يحسبها المحرك؛ النسخ الاحتياطي والاستعادة كما هما؛ الأدوار وكلمات
المرور لم تتغير.
