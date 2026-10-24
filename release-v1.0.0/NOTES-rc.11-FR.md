# Notes de version — Baraka UltraPOS 1.0.0-rc.11 (PILOTE)
<!-- Version build : v1.0.0-rc.11 (renseigné à l'empaquetage, cf. audit/REL-005/PIPELINE.md) -->
<!-- HEAD testé : ea9d7e7 -->

## Ce qui change depuis la rc.10
- **Accès par rôle renforcés.** Chaque écran (Clients, Stock, Catalogues,
  Documents, Paramètres, Transferts) ne montre désormais que ce que le rôle
  de l'utilisateur autorise ; les actions interdites s'affichent grisées avec
  une explication, au lieu d'échouer au moment du clic.
  *(Sources : audit/DSN-077/REPORT.md, audit/DSN-078/REPORT.md,
  audit/DSN-079/REPORT.md, audit/DSN-080/REPORT.md, audit/DSN-081/REPORT.md.)*
- **Suppressions sécurisées.** Chaque suppression exige désormais un motif
  (3 caractères minimum), les suppressions irréversibles demandent de retaper
  le code de l'élément, le dépôt principal ne peut jamais être supprimé, et
  chaque suppression est tracée avec son auteur.
  *(Sources : audit/BRK-080/REPORT.md, audit/DSN-082/REPORT.md — 29/29
  vérifications automatisées au vert.)*
- **Étiquettes code-barres 39×20 mm.** Nouveau format « Mini Étiquette » pour
  les plus petites étiquettes du magasin : le code-barres s'ajuste
  automatiquement pour tenir sur l'étiquette, sans être coupé. Les trois
  anciens formats restent identiques au pixel près (preuve par empreintes).
  Reste à valider : un vrai passage en caisse avec le lecteur du magasin.
  *(Source : audit/DSN-083/REPORT.md — 752 vérifications automatisées au vert
  sur l'ensemble du produit.)*
- **Détails d'un document en un clic.** Cliquer « Détails » sur un ticket ou
  une facture affiche désormais les lignes, les paiements, la TVA et les
  totaux, avec un état de chargement et un bouton « Réessayer » en cas de
  panne. *(Source : audit/DSN-084/REPORT.md.)*
- **Impression A4 des factures et bons de livraison.** Depuis la fiche
  détail, un bouton « Imprimer A4 » produit une facture ou un bon de
  livraison en format A4 (en-tête du magasin, tableau des lignes, totaux),
  en français comme en arabe. Les tickets restent sur l'imprimante thermique.
  *(Source : audit/DSN-085/REPORT.md.)*
- **Recherche catalogue : diagnostic chiffré, sans changement visible.**
  Une mesure sur 80 000 articles a confirmé que la recherche actuelle est
  trop lente ; l'index rapide est en préparation mais N'EST PAS encore actif
  dans cette version — le comportement de recherche est inchangé.
  *(Sources : audit/BRK-075/TIMINGS.md, audit/BRK-076/REPORT.md.)*

## Rappel pilote
Cette rc.11 est une version **pilote** : elle sert à tester en conditions
réelles avant toute généralisation. Lisez la lettre des points ouverts
(`OPEN-GATES-FR.md`) avant l'installation.
