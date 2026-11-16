# Baraka UltraPOS v1.3.1 — Release Notes / مذكرة الإصدار

Date / التاريخ: 2026-11-16 · Baseline: `v1.3.0 (53c50a5)` → `v1.3.1`

## FR — Contenu de la version

**Correctifs R (re-tests terrain, sans défaut reproduit) :**
- R1 — Vente à crédit 586 re-testée en isolation : `Espèces 0 / Crédit 586` →
  `Timbre 0,00` → `TOTAL 586,00` → `TICK-… / Finalisés` + champs d'équation
  (`solde_avant=0 / après=58600`). Aucun défaut → `BRK-118` non requis.
- R2 — Stock initial re-testé en isolation : produit `R2-025` (200/300, +50 %),
  stock initial 25 → ligne `25 PCS / Actif` + mouvement `ADJUSTMENT_POSITIVE | 25`.
  Aucun défaut → `BRK-119` non requis.
- R3 (`DSN-107`) — badges tarifaires `tier_*`, dates `formatInstant`, chargement
  auto, section `actionless`, `customers.limited`.
- R4 (`BRK-120` + `ADR-121`) — seuil de rupture unifié (`UNION ALL`).

**Visuels P (déjà livrés) :** P1 bannière unique + F12 · P2 `bulkPrice` + conversion
Devis→BL · P3 ligne vide unique + colonnes masquées · P4/P5 boutons `h-7` +
montants `dir=ltr`.

**Stratégie S (additif, sans migration sauf 0023) :**
- S1 (`BRK-121a/b` + `ADR-122`) — paliers de quantité (`price_breaks`,
  migration 0023, éditeur d'échelle, use case `ReplacePriceBreaks`
  réservé `MANAGE_CATALOG`).
- S3 — radar de réapprovisionnement (onglet `Radar`, couverture jours,
  dégradation isolée, `REPORTS_VIEW`).
- S4 — répartition `unit-mix` par ligne radar (parts `share_bp` + confiance).
- S5 — `ADR-123` (synchro multi-caisses) : conception PROPOSED, implémentation
  différée après terrain.
- S6 — picking vocal expérimental derrière flag (`baraka.voicePicking`) ;
  texte → recherche uniquement, jamais d'ajout auto.

**Contraintes reportées :** pas de `.sig` (clé absente — installation manuelle) ·
sauvegarde obligatoire avant install (refus de downgrade `v1.2.1` sur base v22/v23) ·
aucune imprimante (reportée avec accord du propriétaire).

## AR — مضمون الإصدار

- R1: أُعيد اختبار البيع الآجل 586 معزولًا — لا عطل (`BRK-118` غير مطلوب).
- R2: أُعيد اختبار المخزون الافتتاحي 25 معزولًا — لا عطل (`BRK-119` غير مطلوب).
- R3/R4: حزمة التسميات والتواريخ + عتبة النفاد الموحدة.
- P5..P1: بصريات الصندوق والكتالوج (لافتة واحدة، F12، `dir=ltr`).
- S6..S1: شرائح الكمية (ترحيل 0023) + رادار التموين + توزيع الوحدات +
  تصميم المزامنة (مؤجل) + الالتقاط الصوتي (خلف علم).
- القيود: بلا توقيع (تثبيت يدوي) · نسخة احتياطية إجبارية · بلا طابعات.

## Vérification / التحقق

- `G1 fmt ✓ · G2 clippy -D warnings ✓ · G3 662 passed / 0 failed / 7 ignored ✓`
- `G4 eslint --max-warnings 0 ✓ · G5 tsc --noEmit ✓`
- `G6 build ✓ · G7 98 fichiers / 983 tests ✓` (+ `reporting 15/15`, `stock 5/36`)
- Terrain isolé R1/R2 : preuves ci-dessus (zéro écriture hors `BARAKA_DATA_DIR`).
