# Local data and migration implementation plans — reserved

Future plans in this folder should cover:

- Database/library selection and directory layout
- Versioned schemas for profile, master draft, published snapshot, workspace, tracker, artifacts, settings, and operation metadata
- Atomic writes, concurrency, journaling, crash recovery, indexes, and local search
- File permissions and evaluated database-encryption/key-vault design
- Temporary import lifecycle and secure cleanup limitations
- Schema migration, compatibility window, rollback, and safety backups
- Versioned encrypted `.ort-backup` format, cryptography, restore/merge, corruption tests, and device migration
- Full portable export and tracker CSV
- Low-disk-space behavior, storage reporting, deletion, uninstall, and orphan cleanup
- Fuzzing and hostile archive/import validation

