# Glossary Arabic-French (UI terms)

Status: v1.0 RATIFIED. Date: 2026-10-11 (REL-047, under the owner's directive for maximum delivery pace). Baseline: a374cf10c6b04d33605c1c293c2fb726df842775. All Arabic terms below are the authoritative bilingual lexicon for Baraka UltraPOS v1.0.0. ARC-007 decisions (glossary-decisions-arc-007.md) are merged: rows 3, 11, 12, 19, 35, 57. Four additional formal terms (rows 113-116) are ratified by the same owner directive; their provenance is recorded per row.

## 1. Rules

- R1 Arabic means Modern Standard Arabic, no dialect. Digits are always Latin (ADR-069).
- R2 One term per concept, used identically on every screen. No synonyms in the UI.
- R3 French is the source key. Every Arabic term maps to exactly one French term in the table below.
- R4 Money format (RATIFIED): amounts always show two decimals, a comma as decimal separator and a narrow no-break space as thousands separator. French shows the suffix DA, Arabic shows the suffix دج.
- R5 Product name display (RATIFIED): French mode shows the French name. Arabic mode shows the Arabic alias if one exists, otherwise the French name.
- R6 Credit HUD for the Caissier uses one word plus an icon: rows 53 and 54. It never shows an amount or a reason.
- R7 Owner-review flags — RESOLVED at ratification. Rows 3, 11, 12, 19, 35, 57 were decided in glossary-decisions-arc-007.md and are merged unchanged. Row 24 (Duplicata): the printed mark stays DUPLICATA in both languages (row note); the UI label نسخة مكررة is standard MSA. Row 58 (dialog title ترخيص المسيّر): standard MSA, consistent with rows 12 and 59.

## 2. Section map (review coverage)

Rows are grouped by the six review sections; every row was checked against
standard professional software terminology (Odoo/SAP Arabic, Algerian
commercial law) with no colloquialisms:

| Section | Rows |
|---|---|
| Commercial Documents | 15–24, 38–44 |
| Inventory & Warehouse | 61–74, 113, 115 |
| Partners & Credit | 3, 45–60, 75–77, 116 |
| Fiscal & Tax | 30, 44 |
| Roles & POS Actions | 11–14, 25–29, 31–37, 78–83 |
| System & Security | 1–2, 4–10, 84–112, 114 |

## 3. Terms

| # | Français | العربية | Note |
|---|---|---|---|
| 1 | Vente | البيع | - |
| 2 | Documents | الوثائق | - |
| 3 | Clients & Crédit | العملاء والائتمان | Decided (glossary-decisions-arc-007.md). Standard international/academic Arabic. |
| 4 | Produits & Codes-barres | المنتجات والرموز الشريطية | - |
| 5 | Stock | المخزون | - |
| 6 | Fournisseurs & Achats | الموردون والمشتريات | - |
| 7 | Caisse & Clôture | الصندوق والإقفال | - |
| 8 | Rapports | التقارير | - |
| 9 | Utilisateurs | المستخدمون | - |
| 10 | Paramètres & Sauvegarde | الإعدادات والنسخ الاحتياطي | - |
| 11 | Administrateur | مسؤول النظام | System administrator role (glossary-decisions-arc-007.md). |
| 12 | Gérant | المدير / المسيّر | Shop manager (glossary-decisions-arc-007.md). Approves returns and debt collection by PIN (ADR-079). |
| 13 | Caissier | أمين الصندوق | The employee role. |
| 14 | Vendeur | البائع | - |
| 15 | Ticket de caisse | وصل الصندوق | - |
| 16 | Facture | فاتورة | - |
| 17 | Bon de livraison | وصل التسليم | - |
| 18 | Devis | عرض سعر | - |
| 19 | Avoir | إشعار دائن | Ratified (glossary-decisions-arc-007.md). Standard commercial credit memo. |
| 20 | Bon de commande | طلب شراء | - |
| 21 | Bon de réception | وصل الاستلام | - |
| 22 | Facture fournisseur | فاتورة المورد | - |
| 23 | Brouillon | مسودة | - |
| 24 | Duplicata | نسخة مكررة | The printed mark is DUPLICATA in both languages today. This is the UI label only. |
| 25 | Panier | السلة | - |
| 26 | Quantité | الكمية | - |
| 27 | Prix unitaire | سعر الوحدة | - |
| 28 | Remise | تخفيض | - |
| 29 | Total | المجموع | - |
| 30 | Timbre | الطابع | The Arabic receipt label is الطابع (confirmed in code). |
| 31 | Encaisser | تحصيل | - |
| 32 | Espèces | نقدًا | - |
| 33 | Chèque | شيك | - |
| 34 | Carte | بطاقة | - |
| 35 | À crédit | بالأجل | Payment-method label (glossary-decisions-arc-007.md). Standard academic spelling with hamza. |
| 36 | Montant reçu | المبلغ المقبوض | - |
| 37 | Monnaie à rendre | الباقي | Change given back to the customer. |
| 38 | Nouvelle vente | بيع جديد | - |
| 39 | Mettre en attente | تعليق | - |
| 40 | Reprendre | استئناف | - |
| 41 | Annuler la vente | إلغاء البيع | - |
| 42 | Retour de marchandise | إرجاع بضاعة | Not the navigation back button (row 91). |
| 43 | Réimprimer | إعادة طباعة | - |
| 44 | Dinar algérien (DA) | دينار جزائري (دج) | Printed as DA in French and دج in Arabic. |
| 45 | Client | زبون | - |
| 46 | Nouveau client | زبون جديد | - |
| 47 | Nom | الاسم | - |
| 48 | Téléphone | الهاتف | - |
| 49 | Adresse | العنوان | - |
| 50 | Solde | الرصيد | - |
| 51 | Dette | الدين | - |
| 52 | Plafond de crédit | سقف الدين | - |
| 53 | Autorisé | مسموح | Credit HUD word for the Caissier: one word plus an icon, no amount, no reason. |
| 54 | Refusé | مرفوض | Credit HUD word for the Caissier: one word plus an icon, no amount, no reason. |
| 55 | Relevé de compte | كشف الحساب | - |
| 56 | Encaissement de dette | تسديد دين | Collection form, opened only after manager approval. |
| 57 | Preuve d'encaissement | سند قبض | Printed proof handed to the customer (ADR-082, ADR-083, glossary-decisions-arc-007.md). |
| 58 | Autorisation du gérant | ترخيص المسيّر | Title of the approval dialog. |
| 59 | Code PIN du gérant | الرمز السري للمسيّر | - |
| 60 | Valider | اعتماد | - |
| 61 | Produit | منتج | - |
| 62 | Catégorie | فئة | - |
| 63 | Code-barres | رمز شريطي | - |
| 64 | Référence | المرجع | - |
| 65 | Nom en français | الاسم بالفرنسية | - |
| 66 | Nom en arabe (alias) | الاسم بالعربية | Optional. The French name is mandatory. |
| 67 | Prix de vente | سعر البيع | - |
| 68 | Prix d'achat | سعر الشراء | - |
| 69 | Unité | الوحدة | - |
| 70 | Quantité en stock | الكمية المتوفرة | - |
| 71 | Stock minimum | الحد الأدنى للمخزون | - |
| 72 | Rupture de stock | نفاد المخزون | - |
| 73 | Kit | مجموعة منتجات | - |
| 74 | Inventaire | الجرد | - |
| 75 | Fournisseur | المورد | - |
| 76 | Achat | شراء | - |
| 77 | Réception | استلام | - |
| 78 | Ouvrir la caisse | فتح الصندوق | - |
| 79 | Fond de caisse | مبلغ الافتتاح | - |
| 80 | Clôture de caisse | إقفال الصندوق | - |
| 81 | Écart | الفارق | - |
| 82 | Mouvement de caisse | حركة الصندوق | - |
| 83 | Rapport de clôture | تقرير الإقفال | - |
| 84 | Rechercher | بحث | - |
| 85 | Enregistrer | حفظ | - |
| 86 | Annuler | إلغاء | - |
| 87 | Modifier | تعديل | - |
| 88 | Supprimer | حذف | - |
| 89 | Confirmer | تأكيد | - |
| 90 | Fermer | إغلاق | - |
| 91 | Retour (navigation) | رجوع | Navigation back, not a goods return (row 42). |
| 92 | Imprimer | طباعة | - |
| 93 | Aperçu | معاينة | - |
| 94 | Importer | استيراد | - |
| 95 | Exporter | تصدير | - |
| 96 | Sauvegarde | نسخة احتياطية | - |
| 97 | Restaurer | استعادة | - |
| 98 | Imprimante | طابعة | - |
| 99 | Langue | اللغة | - |
| 100 | Thème | السمة | - |
| 101 | Clair | فاتح | - |
| 102 | Sombre | داكن | - |
| 103 | Utilisateur | مستخدم | - |
| 104 | Mot de passe | كلمة المرور | - |
| 105 | Se déconnecter | تسجيل الخروج | - |
| 106 | Journal d'audit | سجل التدقيق | - |
| 107 | Oui | نعم | - |
| 108 | Non | لا | - |
| 109 | Chargement en cours | جارٍ التحميل | - |
| 110 | Aucun résultat | لا توجد نتائج | - |
| 111 | Erreur | خطأ | - |
| 112 | Opération réussie | تمت العملية بنجاح | - |
| 113 | Alerte de stock bas | تنبيه انخفاض المخزون | Formal term, RATIFIED REL-047 (not from ARC-007). No v1 UI binding: the shipped low-stock report title uses المخزون النافد (`rep.lowStock`) — no UI change in v1. |
| 114 | Code QR | رمز الاستجابة السريعة | Formal term, RATIFIED REL-047 (not from ARC-007). No v1 feature binding. |
| 115 | Ajustement manuel du stock | تعديل يدوي للمخزون | Formal term, RATIFIED REL-047 (not from ARC-007). No v1 UI binding: shipped stock-adjust labels use تسوية مخزون (`stock.adjust`) — no UI change in v1. |
| 116 | Relevé de compte client | كشف حساب العميل | Formal long form, RATIFIED REL-047 (not from ARC-007). Short form in row 55 (كشف الحساب) matches the shipped `customers.statement` label. |
| 117 | Historique de crédit (registre « crédi ») | سجل الكريدي | Teaching-panel copy (`customers.emptyBody`), author-approved verbatim TASK-DSN-054. Colloquial-register transliteration of « crédit », used only inside this panel; formal UI term for credit stays الائتمان/الدين (rows 3, 35, 51–52). |

## 4. Ratification record (open points closed)

- O1 Avoir: CLOSED (Ratified as إشعار دائن per glossary-decisions-arc-007.md).
- O2 الباقي for change given back: CLOSED — ratified as standard MSA (row 37). No competing term in the shipped UI.
- O3 Owner review with prototypes: CLOSED — prototypes delivered, UI shipped, and ratification ordered under the REL-047 owner directive.
- Provenance correction (REL-047): glossary-decisions-arc-007.md settles Clients & Crédit, Administrateur/Gérant, Avoir, À crédit, and Preuve d'encaissement (rows 3, 11, 12, 19, 35, 57). Rows 113–116 are new formal terms ratified by the REL-047 directive itself, not by ARC-007; each row says so.
