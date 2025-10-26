//! Backup analysis and listing command

use anyhow::Result;
use std::path::PathBuf;
use crate::utils::backup::BackupManager;

/// List and analyze project backups
pub async fn run(_project_path: PathBuf, _detailed: bool) -> Result<()> {
    println!("🔍 Analyzing project backups...");
    println!();
    
    let backup_manager = BackupManager::new()?;
    backup_manager.list_backups()?;
    
    Ok(())
}