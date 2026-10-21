# Lettre pilote — points OUVERTS rc.11 (à lire avant installation)
<!-- Version build : TBD-PACKAGING -->

Baraka UltraPOS rc.11 est une version **pilote** : ce qui suit n'est PAS
encore vérifié ou validé, et reste sous votre responsabilité de tester.
*(État des lieux : audit/REL-005/PIPELINE.md.)*

## Non vérifié / non validé à ce jour
1. **Impression ticket réelle.** Les impressions n'ont été vérifiées qu'en
   simulation. *(À faire : ticket d'essai FR + AR sur l'imprimante du magasin.)*
2. **Conformité fiscale.** Aucune validation par un professionnel qualifié
   (comptable algérien) n'a eu lieu : mentions, timbre, numérotation.
   *(À faire : visa du comptable avant usage commercial.)*
3. **Chiffrement de la base.** La base locale N'EST PAS chiffrée (SQLCipher
   absent) : quiconque accède au fichier lit les données.
   *(À faire : décision explicite — poste verrouillé Windows + sauvegardes.)*
4. **Recherche rapide.** L'index FTS5 est préparé mais NON activé
   (cutover BRK-077 en attente) ; la recherche reste lente sur gros catalogues.
   *(Source : audit/BRK-076/REPORT.md.)*
5. **Polices embarquées.** En attente ; l'affichage dépend des polices du poste.

## Check-list propriétaire avant mise en service
- [ ] Ticket d'essai imprimé et lisible (FR + AR) sur l'imprimante du magasin.
- [ ] Comptable : visa écrit sur mentions fiscales et TVA/timbre.
- [ ] Copie de sauvegarde testée : restaurée sur un second poste, ouverte, chiffres concordants.
- [ ] Poste de caisse verrouillé (session Windows, pas de partage du fichier de données).
- [ ] Retour pilote consigné (bugs, lenteurs, souhaits) avant toute généralisation.
