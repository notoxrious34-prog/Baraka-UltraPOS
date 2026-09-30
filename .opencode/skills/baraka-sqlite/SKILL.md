# Baraka SQLite Skill
Use for SQLite, migrations, WAL, integrity, repositories, and recovery.

- Rust owns runtime SQLite access.
- Use rusqlite/SQL migrations according to the Master Architecture.
- Enable required PRAGMAs at connection open.
- Financial writes use BEGIN IMMEDIATE and explicit transaction boundaries.
- Use file-backed temporary databases for WAL/crash integration tests.
- Never modify production baraka.db in development verification unless the Task Brief explicitly authorizes it.
