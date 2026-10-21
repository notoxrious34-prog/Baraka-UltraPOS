# Notes de version — Baraka UltraPOS 1.0.0-rc.11 (PILOTE)
<!-- Version build : TBD-PACKAGING (renseigné à l'empaquetage, cf. audit/REL-005/PIPELINE.md) -->

## Ce qui change depuis la rc.10
- **Accès par rôle renforcés.** Chaque écran (Clients, Stock, Catalogues,
  Documents, Paramètres, Transferts) ne montre désormais que ce que le rôle
  de l'utilisateur autorise ; les actions interdites s'affichent grisées avec
  une explication, au lieu d'échouer au moment du clic.
  *(Sources : audit/DSN-077/REPORT.md, audit/DSN-078/REPORT.md,
  audit/DSN-079/REPORT.md, audit/DSN-080/REPORT.md, audit/DSN-081/REPORT.md —
  713 tests automatisés au vert.)*
- **Transferts entre dépôts et réglages sécurisés.** Les transferts et les
  réglages sensibles (identité du magasin, imprimante, fiscalité, sauvegardes)
  exigent désormais l'autorisation correspondante ; sans elle, le bouton est
  désactivé avec un message clair, en français comme en arabe.
  *(Sources : audit/DSN-080/REPORT.md, audit/DSN-081/REPORT.md.)*
- **Recherche catalogue : diagnostic chiffré, sans changement visible.**
  Une mesure sur 80 000 articles a confirmé que la recherche actuelle est
  trop lente ; l'index rapide est en préparation mais N'EST PAS encore actif
  dans cette version — le comportement de recherche est inchangé.
  *(Sources : audit/BRK-075/TIMINGS.md, audit/BRK-076/REPORT.md.)*

## Rappel pilote
Cette rc.11 est une version **pilote** : elle sert à tester en conditions
réelles avant toute généralisation. Lisez la lettre des points ouverts
(`OPEN-GATES-FR.md`) avant l'installation.
