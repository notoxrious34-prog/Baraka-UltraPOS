# ADR-071 - Deployment topology: single device, single instance
Status: ACCEPTED (SESSION-ARC-001)
Context: The owner has one employee and one computer today, with growth expected later.
Decision: Version 1 targets one Windows PC, one data directory, one running application instance (single-instance lock). Multiple named users with roles (Admin, Gerant, Caissier, Vendeur) are supported and a users-management screen is provided (engine extension per Charter section 11). Multi-PC synchronization is out of scope for v1; the existing future-sync metadata stays untouched. Backups: local rotation plus a copy on an external drive; the drive location is an owner choice.
Consequences: No network features, no server, no shared database. Each cash session belongs to one user and one terminal.
Validation gate: none.
