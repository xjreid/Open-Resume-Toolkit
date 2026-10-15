# Local data and migration implementation

- [`Local_Data_and_Migration_Plan.md`](Local_Data_and_Migration_Plan.md) is the implementation authority for local paths, encrypted persistence, transactions, migrations, backup/restore, retention, deletion, and portable exports.
- Cross-component security and architecture rules are in [`../System Documentation/`](../System%20Documentation/).

Schema SQL and forward migrations are in `crates/ort-storage/src/migrations.rs`;
the current database is schema 9. Profile ownership, portable snapshot collection,
and credential retirement have separate storage modules. Backup 1.9 and historical
vectors live in `ort-backup`; [the format reference](../../docs/backup/backup-v1.md)
and [current implementation map](../../docs/development/current-state.md) describe
the implemented lifecycle. Secrets and real user content are forbidden in fixtures.
