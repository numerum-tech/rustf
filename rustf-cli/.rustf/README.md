# RustF Backups Directory

This directory contains automatic backups created when using --force flags.

## Manual Restoration

Backups are NOT automatically restorable to ensure you know what you're doing.

To manually restore a backup:

### For Models:
```bash
# First, review what's in the backup
ls -la .rustf/backups/models/[timestamp]/

# Restore entire backup
cp -r .rustf/backups/models/[timestamp]/* src/models/

# Or restore specific files
cp .rustf/backups/models/[timestamp]/user.rs src/models/
```

### For Schemas:
```bash
cp -r .rustf/backups/schemas/[timestamp]/* schemas/
```

### For Projects:
```bash
# Review carefully before restoring entire project
cp -r .rustf/backups/project/[timestamp]/* ./
```

## Latest Backup
The 'latest' symlink points to the most recent backup for easy reference.

## Cleanup
Old backups are kept for safety. Delete manually when no longer needed:
```bash
rm -rf .rustf/backups/models/[old-timestamp]/
```
