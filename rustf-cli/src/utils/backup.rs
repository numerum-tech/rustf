use anyhow::{anyhow, Result};
use std::fs;
use std::path::{Path, PathBuf};
use chrono::Utc;
use walkdir::WalkDir;

pub struct BackupManager {
    project_root: PathBuf,
    rustf_dir: PathBuf,
}

impl BackupManager {
    /// Create a new BackupManager for the current project
    pub fn new() -> Result<Self> {
        let project_root = find_project_root()?;
        let rustf_dir = project_root.join(".rustf");
        
        // Create .rustf directory if it doesn't exist
        if !rustf_dir.exists() {
            fs::create_dir_all(&rustf_dir)?;
            Self::create_readme(&rustf_dir)?;
        }
        
        Ok(Self { project_root, rustf_dir })
    }
    
    /// Backup a directory before overwriting with --force
    pub fn backup_directory(&self, source: &Path, backup_type: &str) -> Result<String> {
        // Check if source exists and has content
        if !source.exists() {
            println!("📝 No existing {} to backup", backup_type);
            return Ok(String::new());
        }
        
        // Check if directory is empty
        if source.is_dir() {
            let is_empty = fs::read_dir(source)?.next().is_none();
            if is_empty {
                println!("📝 {} directory is empty, no backup needed", backup_type);
                return Ok(String::new());
            }
        }
        
        // Create timestamp for backup
        let timestamp = Utc::now().format("%Y-%m-%dT%H-%M-%SZ").to_string();
        let backup_base = self.rustf_dir.join("backups").join(backup_type);
        let backup_path = backup_base.join(&timestamp);
        
        // Create backup directory
        fs::create_dir_all(&backup_path)?;
        
        // Copy directory recursively
        let file_count = copy_dir_recursive(source, &backup_path)?;
        let size = calculate_dir_size(&backup_path)?;
        
        // Create or update latest symlink (for reference only)
        let latest_link = backup_base.join("latest");
        if latest_link.exists() {
            fs::remove_file(&latest_link).ok(); // Ignore errors on Windows
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            symlink(&backup_path, &latest_link).ok(); // Best effort
        }
        
        // Show user feedback
        println!("📦 Creating safety backup before overwriting...");
        println!("✅ Backed up to: .rustf/backups/{}/{}/", backup_type, timestamp);
        println!("📁 {} files backed up ({})", file_count, format_size(size));
        println!();
        println!("⚠️  Backups are manual-restore only for safety");
        println!("💡 To restore: cp -r .rustf/backups/{}/{}/* {}/", 
                 backup_type, timestamp, source.display());
        println!();
        
        Ok(timestamp)
    }
    
    /// List all backups in the project
    pub fn list_backups(&self) -> Result<()> {
        let backups_dir = self.rustf_dir.join("backups");
        
        if !backups_dir.exists() {
            println!("📭 No backups found");
            return Ok(());
        }
        
        println!("📦 Backups in .rustf/backups/:");
        println!();
        
        // Iterate through backup types
        for entry in fs::read_dir(&backups_dir)? {
            let entry = entry?;
            let backup_type = entry.file_name();
            let backup_type_str = backup_type.to_string_lossy();
            
            if entry.path().is_dir() {
                println!("{}:", backup_type_str);
                
                // List timestamps for this type
                let mut timestamps = Vec::new();
                for timestamp_entry in fs::read_dir(entry.path())? {
                    let timestamp_entry = timestamp_entry?;
                    let name = timestamp_entry.file_name().to_string_lossy().to_string();
                    
                    // Skip "latest" symlink
                    if name == "latest" {
                        continue;
                    }
                    
                    let path = timestamp_entry.path();
                    let metadata = fs::metadata(&path)?;
                    let modified = metadata.modified()?;
                    let elapsed = modified.elapsed().unwrap_or_default();
                    let age = format_duration(elapsed);
                    
                    let size = calculate_dir_size(&path)?;
                    let file_count = count_files(&path)?;
                    
                    timestamps.push((name, file_count, size, age));
                }
                
                // Sort by timestamp (newest first)
                timestamps.sort_by(|a, b| b.0.cmp(&a.0));
                
                for (timestamp, files, size, age) in timestamps {
                    println!("  {} ({} files, {}) - {}", 
                             timestamp, files, format_size(size), age);
                }
                println!();
            }
        }
        
        println!("💡 To restore manually, see .rustf/README.md");
        
        Ok(())
    }
    
    /// Create README in .rustf directory
    fn create_readme(rustf_dir: &Path) -> Result<()> {
        let readme_path = rustf_dir.join("README.md");
        let content = r#"# RustF Backups Directory

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
"#;
        fs::write(readme_path, content)?;
        Ok(())
    }
}

/// Find the project root (directory containing Cargo.toml)
fn find_project_root() -> Result<PathBuf> {
    let current_dir = std::env::current_dir()?;
    let mut path = current_dir.as_path();
    
    loop {
        if path.join("Cargo.toml").exists() {
            return Ok(path.to_path_buf());
        }
        
        match path.parent() {
            Some(parent) => path = parent,
            None => return Err(anyhow!("Could not find project root (no Cargo.toml found)")),
        }
    }
}

/// Recursively copy a directory
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<usize> {
    let mut file_count = 0;
    
    for entry in WalkDir::new(src) {
        let entry = entry?;
        let path = entry.path();
        let relative = path.strip_prefix(src)?;
        let target = dst.join(relative);
        
        if entry.file_type().is_dir() {
            fs::create_dir_all(&target)?;
        } else {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(path, target)?;
            file_count += 1;
        }
    }
    
    Ok(file_count)
}

/// Calculate total size of a directory
fn calculate_dir_size(path: &Path) -> Result<u64> {
    let mut total_size = 0;
    
    for entry in WalkDir::new(path) {
        let entry = entry?;
        if entry.file_type().is_file() {
            let metadata = entry.metadata()?;
            total_size += metadata.len();
        }
    }
    
    Ok(total_size)
}

/// Count files in a directory
fn count_files(path: &Path) -> Result<usize> {
    let mut count = 0;
    
    for entry in WalkDir::new(path) {
        let entry = entry?;
        if entry.file_type().is_file() {
            count += 1;
        }
    }
    
    Ok(count)
}

/// Format file size for display
fn format_size(size: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
    let mut size = size as f64;
    let mut unit_index = 0;
    
    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }
    
    if unit_index == 0 {
        format!("{} {}", size as u64, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}

/// Format duration for display
fn format_duration(duration: std::time::Duration) -> String {
    let secs = duration.as_secs();
    
    if secs < 60 {
        "just now".to_string()
    } else if secs < 3600 {
        format!("{} minutes ago", secs / 60)
    } else if secs < 86400 {
        format!("{} hours ago", secs / 3600)
    } else {
        format!("{} days ago", secs / 86400)
    }
}

/// Check if a directory is empty (contains no files or only hidden files)
pub fn is_empty_directory(path: &Path) -> Result<bool> {
    if !path.exists() {
        return Ok(true);
    }
    
    if !path.is_dir() {
        return Ok(false);
    }
    
    let mut entries = fs::read_dir(path)?;
    Ok(entries.next().is_none())
}