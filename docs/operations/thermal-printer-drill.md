# Procédure matérielle — imprimante thermique 80 mm (Baraka UltraPOS)

> Document destiné au propriétaire du magasin. Ton : opérationnel.
> Seuls identifiants techniques admis : noms de réglages et noms de fichiers.

## 1. Portée et limites honnêtes

- La simulation dans l'application (fichier de capture `.escpos` + inspection
  structurelle + aperçu à l'écran) prouve l'exactitude au niveau des octets :
  ce que l'application envoie est correct et déterministe.
- Cette procédure prouve l'imprimante physique : câblage, pilote, papier, coupe.
- Le test d'impression est un **diagnostic synchrone sans effet comptable** :
  aucun document, aucune tâche de file d'impression, aucune écriture comptable.
- L'élément « Real hardware printing verified » (§38 du socle) n'est clôturé
  que par **une ligne signée dans le tableau de validation (§6)**.

## 2. Imprimantes prises en charge (v1)

Catégories compatibles — ESC/POS, thermique 80 mm, adressable en port série
ou COM virtuel :

1. Imprimantes tickets RS-232 série natives.
2. Imprimantes thermiques USB dont le pilote expose un **port COM virtuel**.
3. Imprimantes reliées via un adaptateur USB→série.

**Hors périmètre v1 (limite honnête)** : imprimantes réseau / Wi-Fi et
imprimantes USB-only sans pilote mode COM. Elles ne sont pas testées par
cette procédure.

## 3. Contrôles avant vol

- [ ] Papier thermique 80 mm chargé (côté thermique vers la tête).
- [ ] Imprimante sous tension, câble branché.
- [ ] Port COM identifié (Gestionnaire de périphériques Windows → Ports).
- [ ] Réglages `printer.serial_*` renseignés dans la section imprimante
      des réglages (port + débit) avec le même port COM.

## 4. Étapes de la procédure

### Étape 1 — test en capture seule

1. Ouvrir « Identité & Impression », mode **Capture uniquement**.
2. Choisir la langue (AR ou FR), lancer le test.
3. Vérifier l'aperçu à l'écran :
   - bloc d'identité : nom centré, adresse / téléphone, ligne
     NIF / NIS / RC / AI en abréviations latines ;
   - lignes du ticket, totaux, lignes du pied de ticket ;
   - avance papier (feed), position de coupe.
4. Noter le nom du fichier `.escpos` (historique en ajout uniquement —
   les captures ne sont jamais supprimées).

### Étape 2 — test physique

1. Passer en mode **Impression physique + capture**, relancer le test.
2. Comparer le papier avec le même aperçu, point par point :

| Point | Papier | Aperçu |
|---|---|---|
| En-tête d'identité | | |
| Alignement (centré) | | |
| Langue des libellés (« Tél » vs « هاتف » selon la langue choisie ; NIF/NIS/RC/AI toujours latins) | | |
| Pied de ticket | | |
| Longueur d'avance | | |
| Position de coupe | | |

### Étape 3 — contrôles de tolérance

Pour chaque condition (câble USB débranché / imprimante éteinte /
papier retiré), relancer le test et noter le badge observé
(code d'erreur classé, pas de plantage, application utilisable) :

| Condition | Badge observé | App utilisable |
|---|---|---|
| USB débranché | | |
| Imprimante éteinte | | |
| Papier retiré | | |

Preuve côté moteur (file d'impression de production, indépendante du
diagnostic) — suite `print_worker.rs` :
`transient_failures_retry_with_backoff_then_done`,
`permanent_failure_marks_failed_after_three_sends`,
`failed_print_leaves_accounting_byte_identical`,
`stale_processing_reset_on_startup_cycle`,
`test_print_worker_transient_failure_increments_attempts_and_sets_duplicata`,
`test_print_worker_exhausted_retries_transitions_to_failed`,
`test_stale_pending_jobs_reconciled_on_startup`,
`test_unconfigured_printer_is_bounded_and_never_spins`.

## 5. Dépannage rapide

- Badge `PRINTER_UNAVAILABLE` : vérifier le port COM (§3) et l'alimentation.
- Badge `DEVICE_ERROR` : imprimante injoignable — câble, pilote, rallumer.
- Ticket vide ou tronqué : papier à l'envers (face thermique), puis refaire
  l'étape 1.

## 6. Tableau de validation (signature)

| Date | Opérateur | Marque & Modèle | Port (COMx) | Résultat (OK/Échec) | Observations |
|---|---|---|---|---|---|
| | | | | | |
| | | | | | |

**Au moins une ligne signée clôt l'élément §38.**

## 7. ملخص بالعربية

- الاختبار داخل التطبيق (ملف `.escpos` + معاينة على الشاشة) يثبت صحة
  البايتات المرسلة، وهذا الإجراء يثبت الطابعة الفيزيائية.
- اختبار الطباعة تشخيص متزامن **بدون أي أثر محاسبي**.
- الطابعات المدعومة (v1): طابعات حرارية 80 مم ESC/POS عبر منفذ تسلسلي
  أو COM افتراضي. خارج النطاق: طابعات الشبكة / Wi-Fi وطابعات USB
  بدون وضع COM.
- قبل البدء: الورق محمّل، الطابعة مشغّلة، الكبل موصول، منفذ COM معروف،
  وإعدادات `printer.serial_*` مضبوطة.
- الخطوات: (1) اختبار التقاط فقط ومقارنة المعاينة (الهوية، NIF/NIS/RC/AI
  باللاتينية، التذييل، التقديم، القطع)؛ (2) اختبار فيزيائي ومقارنة
  الورق مع المعاينة؛ (3) اختبارات التحمل (فصل USB / إطفاء / نزع الورق)
  وتسجيل الشارة الملاحظة.
- عنصر §38 (« Real hardware printing verified ») لا يُغلق إلا بسطر
  موقّع في جدول Validation (§6).
