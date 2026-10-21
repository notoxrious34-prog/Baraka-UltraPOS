# rc.11 — contenu exact de livraison (d'après audit/AUD-037/REPORT.md)

## Fichiers obligatoires
1. `Baraka UltraPOS_1.0.0_x64-setup.exe` — installeur NSIS (modèle rc.10 : ~222 Mo).
2. `Baraka-UltraPOS-v1.0.0-rc.11-Delivery.zip` — installeur + documents ci-dessous.
3. `SHA256SUMS.txt` — sommes de **tous** les fichiers livrés (leçon AUD-037 :
   en rc.10, seuls l'exe et le zip rc.10 étaient couverts ; les zips
   rc.2/4/5/6/7/8/9 n'avaient aucune somme et les sommes rc.5/rc.6 ne
   couvraient que l'exe).
4. `NOTES-rc.11-FR.md` + `NOTES-rc.11-AR.md` (ce dossier) — nouveautés pilote.
5. `OPEN-GATES-FR.md` + `OPEN-GATES-AR.md` (ce dossier) — lettre pilote,
   remise OBLIGATOIRE avec chaque livraison.
6. `LISEZMOI-README.txt` (existe : 2036 o, 12 oct.).
7. `GUIDE-UTILISATION.html` (existe : 33 825 o) +
   `QUESTIONNAIRE-COMPTABLE-FR.html` (existe : 32 995 o).

## Règle anti-régression packaging (verdict AUD-037)
- Chaque zip livré a sa somme dans `SHA256SUMS.txt` ; aucune somme partielle.
- Chaque rc a ses notes (`RELEASE-NOTES-v1.0.0-rc.11.md` à rédiger à
  l'empaquetage depuis les NOTES ci-dessus ; en rc.10 les notes
  s'arrêtaient à rc.9).
- Gabarits confirmés présents dans `release-v1.0.0/` (tous existent).
