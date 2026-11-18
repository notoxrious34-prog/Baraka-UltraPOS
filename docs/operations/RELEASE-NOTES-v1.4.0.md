# Notes de version — Baraka UltraPOS v1.4.0 / مذكرة الإصدار

## FR — Porte visuelle complète (9 modules)
- Matrice visuelle sur isolation `vis14` (compte `vis14audit`, zéro touche prod) :
  Caisse / Clients / Documents / Paramètres × FR clair + AR clair + Caisse AR sombre ✓,
  puis Stock × FR clair + AR clair + AR sombre ✓,
  Catalogue × FR clair (11 colonnes) + AR clair ✓,
  Fournisseurs × AR sombre + FR clair ✓,
  Rapports × AR sombre + FR clair (ligne réelle `V14-100 | 1 | 250,00 DZD`) ✓,
  Utilisateurs × FR clair ✓ — zéro overflow/troncature.
- Correctif i18n : `totals.tva` AR `الضريبة TVA` → `الضريبة` (vérifié en direct en Caisse AR).
- R1b clôturée : relevé `Client V14` (`Solde dû 250,00 / Plafond 100 000,00 / Disponible 99 750,00`)
  + stock visuel `20→19` après `TICK-2026-000001`.
- Portes sur l'arbre de release : `G1 0 / G2 0 / G3 662-0 / G4 0 / G5 0 / G6 OK / G7 98-983`.
- Limites connues (inchangées) : pas de `.sig` (sans `TAURI_SIGNING_PRIVATE_KEY`) ;
  sauvegarde obligatoire avant installation (refus `v22/v23` par `v1.2.1`) ;
  aperçu papier reporté (aucune imprimante).

## AR — البوابة البصرية الكاملة (9 وحدات)
- مصفوفة حية على عزل `vis14` (حساب `vis14audit`، صفر مساس بالإنتاج):
  الصندوق/العملاء/المستندات/الإعدادات × فرنسي فاتح + عربي فاتح + صندوق عربي داكن ✓،
  ثم المخزون × فرنسي فاتح + عربي فاتح + عربي داكن ✓،
  الكتالوج × فرنسي فاتح (11 عمودًا) + عربي فاتح ✓،
  الموردون × عربي داكن + فرنسي فاتح ✓،
  التقارير × عربي داكن + فرنسي فاتح (صف حي) ✓،
  المستخدمون × فرنسي فاتح ✓ — صفر فيض/قصّ.
- إصلاح لغوي: `الضريبة TVA` → `الضريبة` (مُتحقق حيًا).
- R1b مغلقة: كشف `Client V14` + مخزون `20→19`.
- البوابات: `G1 0 / G2 0 / G3 662-0 / G4 0 / G5 0 / G6 OK / G7 98-983`.
- قيود معلنة: بلا `.sig`؛ النسخ الاحتياطي إجباري؛ الورق مؤجل لغياب الطابعات.

## Livrable
- `target/release/bundle/nsis/Baraka UltraPOS_1.4.0_x64-setup.exe`
- (taille + SHA-256 ci-dessous après build / الحجم والبصمة بعد البناء)
