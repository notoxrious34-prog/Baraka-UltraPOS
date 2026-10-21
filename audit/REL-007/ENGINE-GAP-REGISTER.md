# Registre des manques moteur (ENGINE-GAP) — cycle suivant
<!-- HEAD : 54511f0 — constatés en DSN-084/DSN-085, contournés côté frontal sans invention. -->

| # | Champ manquant | Consommateur | Localisation moteur (si connue) | Taille | Bloque quoi |
|---|----------------|--------------|----------------------------------|--------|-------------|
| G1 | Montant en toutes lettres (FR + AR) | Pied de facture A4 (`DocumentA4Sheet`) | `GetSaleResponse`, `crates/commands/src/dto.rs` — champ à créer (règle de libellé FR/AR à définir) | M | Visa comptable d'une facture réglementaire |
| G2 | Remise par ligne + taux | Tableau des lignes (tiroir + A4) | `DocumentItemDto`, `crates/commands/src/dto.rs` — champ à ajouter (relais S si déjà stocké, calcul M sinon) | S–M | Factures avec remises ; prix affichés sans justification |
| G3 | Ventilation TVA par taux | Section taxe (tiroir + A4) — seul le total est rendu | Ligne de vente (taux appliqué non exposé) | S | Justification du total TVA au comptable |
| G4 | Identifiants fiscaux du client (NIF/NIS/RC/AI) | Bloc client A4, mentions B2B | Vue résumé (`party_id`/`party_name` seuls) — jointure table des parties | S | Factures inter-entreprises réglementaires |
| G5 | Noms d'unités + désignations des lignes | Lisibilité lignes (tiroir + A4 — seuls les IDs sont rendus) | `DocumentItemDto` (`product_id`/`unit_id` seuls) — snapshot ou jointure | S | Lisibilité par un tiers (comptable, client) |
| G6 | Date de création dans la vue détail | En-tête A4 (contourné : date reprise de la ligne résumé) | `GetSaleResponse` — champ à ajouter | S | Rien (contournement actif, documenté en `DocumentA4Sheet`) |

*Sources : audit/DSN-084/REPORT.md (sections « ENGINE-GAP », totaux) ;
audit/DSN-085/REPORT.md (liste ENGINE-GAP) ; formes de réponse :
`GetSaleResponse`/`DocumentItemDto`/`PaymentDto` en
`crates/commands/src/dto.rs:758-793 (constaté en DSN-084).*
*Règle : aucun contournement frontal n'invente ces valeurs (affichées
seulement si le moteur les fournit) — d'où leur inscription ici.*
