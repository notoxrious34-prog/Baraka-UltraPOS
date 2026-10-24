# Lettre pilote — points OUVERTS rc.11 (à lire avant installation)
<!-- Version build : v1.0.0-rc.11 — HEAD testé : ea9d7e7 -->

Baraka UltraPOS rc.11 est une version **pilote** : ce qui suit n'est PAS
encore vérifié ou validé, et reste sous votre responsabilité de tester.
*(État des lieux : audit/REL-005/PIPELINE.md.)*

## Non vérifié / non validé à ce jour — la liste exacte
1. **Impression ticket réelle (action : propriétaire).** Les impressions
   n'ont été vérifiées qu'en simulation, y compris le format A4 et les
   étiquettes 39×20. *(Sources : audit/BRK-079/REPORT.md,
   audit/DSN-083/REPORT.md, audit/DSN-085/REPORT.md. À faire : ticket d'essai
   FR + AR sur l'imprimante du magasin.)*
2. **Test de lecture des étiquettes 39×20 (action : propriétaire).** Le
   dimensionnement est prouvé sur écran, pas la lecture optique : la finesse
   des barres (~1,4–2 points par module) doit passer le lecteur du magasin.
   *(Source : audit/DSN-083/REPORT.md.)*
3. **Conformité fiscale (action : professionnel).** Aucune validation par un
   professionnel qualifié (comptable algérien) n'a eu lieu : mentions, timbre,
   numérotation. *(À faire : visa du comptable avant usage commercial.)*
4. **Chiffrement de la base (action : décision).** La base locale N'EST PAS
   chiffrée (SQLCipher absent) : quiconque accède au fichier lit les données.
   *(À faire : décision explicite — poste verrouillé Windows + sauvegardes.)*
5. **Recherche rapide (état : livré).** L'index FTS5 est ACTIF depuis le
   cutover BRK-077 (991251b) : recherche instantanée jusqu'à ~5 000 articles
   (17–27 ms mesurés), marge jusqu'à 20 000 ; 80 000 surveillés (ADR-102).
   *(Source : audit/BRK-087/REPORT.md.)*

## Check-list propriétaire avant mise en service
- [ ] Ticket d'essai imprimé et lisible (FR + AR) sur l'imprimante du magasin.
- [ ] Étiquette 39×20 lue au premier passage par le lecteur du magasin.
- [ ] Facture A4 imprimée : en-tête, lignes et totaux conformes au visa.
- [ ] Comptable : visa écrit sur mentions fiscales et TVA/timbre.
- [ ] Copie de sauvegarde testée : restaurée sur un second poste, ouverte, chiffres concordants.
- [ ] Poste de caisse verrouillé (session Windows, pas de partage du fichier de données).
- [ ] Retour pilote consigné (bugs, lenteurs, souhaits) avant toute généralisation.
