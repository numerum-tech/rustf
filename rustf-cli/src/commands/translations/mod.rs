pub mod scanner;
pub mod parser;
pub mod updater;

use anyhow::Result;
use clap::Subcommand;
use std::path::Path;

#[derive(Subcommand, Debug)]
pub enum TranslationsCommand {
    /// Scan views and generate reference translation file
    Scan {
        /// Views directory to scan
        #[arg(long, default_value = "views")]
        views_dir: String,
        
        /// Output directory for resource files
        #[arg(long, default_value = "resources")]
        output_dir: String,
    },
    
    /// Update existing translation files with new keys
    Update {
        /// Language to update (or all if not specified)
        #[arg(long)]
        lang: Option<String>,
        
        /// Views directory
        #[arg(long, default_value = "views")]
        views_dir: String,
        
        /// Resources directory
        #[arg(long, default_value = "resources")]
        resources_dir: String,
    },
    
    /// Check for missing translations
    Check {
        /// Language to check
        #[arg(long, required = true)]
        lang: String,
        
        /// Resources directory
        #[arg(long, default_value = "resources")]
        resources_dir: String,
    },
    
    /// Show translation statistics
    Stats {
        /// Resources directory
        #[arg(long, default_value = "resources")]
        resources_dir: String,
    },
}

pub fn execute(cmd: &TranslationsCommand) -> Result<()> {
    match cmd {
        TranslationsCommand::Scan { views_dir, output_dir } => {
            println!("🔍 Scanning views for translations...");
            let mut scanner = scanner::TranslationScanner::new();
            scanner.scan_directory(Path::new(views_dir))?;
            
            let output_path = Path::new(output_dir).join("default.res");
            scanner.write_resource_file(&output_path)?;
            
            println!("✅ Generated {} with {} translations", 
                output_path.display(), 
                scanner.total_translations());
            
            Ok(())
        }
        
        TranslationsCommand::Update { lang, views_dir, resources_dir } => {
            // First regenerate default.res
            println!("🔄 Regenerating default.res...");
            let mut scanner = scanner::TranslationScanner::new();
            scanner.scan_directory(Path::new(views_dir))?;
            
            let default_path = Path::new(resources_dir).join("default.res");
            scanner.write_resource_file(&default_path)?;
            
            // Update specified language or all
            let updater = updater::TranslationUpdater::new(&default_path)?;
            
            if let Some(lang) = lang {
                println!("📝 Updating {}.res...", lang);
                let report = updater.update_language_file(resources_dir, lang)?;
                report.print_summary();
            } else {
                // Update all existing language files
                for entry in std::fs::read_dir(resources_dir)? {
                    let entry = entry?;
                    let path = entry.path();
                    if path.extension() == Some(std::ffi::OsStr::new("res")) {
                        let filename = path.file_stem().unwrap().to_str().unwrap();
                        if filename != "default" {
                            println!("📝 Updating {}.res...", filename);
                            let report = updater.update_language_file(resources_dir, filename)?;
                            report.print_summary();
                        }
                    }
                }
            }
            
            Ok(())
        }
        
        TranslationsCommand::Check { lang, resources_dir } => {
            let lang_file = Path::new(resources_dir).join(format!("{}.res", lang));
            if !lang_file.exists() {
                println!("❌ Translation file {}.res not found", lang);
                return Ok(());
            }
            
            let parser = parser::ResourceParser::new();
            let translations = parser.parse_file(&lang_file)?;
            
            let mut missing = Vec::new();
            for (section, entries) in translations.sections() {
                for (key, value) in entries {
                    if value.is_empty() {
                        missing.push(format!("[{}] {}", section, key));
                    }
                }
            }
            
            if missing.is_empty() {
                println!("✅ All translations complete for {}", lang);
            } else {
                println!("⚠️  Missing {} translations in {}:", missing.len(), lang);
                for key in &missing {
                    println!("  - {}", key);
                }
            }
            
            Ok(())
        }
        
        TranslationsCommand::Stats { resources_dir } => {
            println!("📊 Translation Statistics");
            println!("{}", "─".repeat(40));
            
            for entry in std::fs::read_dir(resources_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension() == Some(std::ffi::OsStr::new("res")) {
                    let filename = path.file_stem().unwrap().to_str().unwrap();
                    let parser = parser::ResourceParser::new();
                    let translations = parser.parse_file(&path)?;
                    
                    let total = translations.total_keys();
                    let complete = translations.complete_keys();
                    let percentage = if total > 0 {
                        (complete as f32 / total as f32 * 100.0) as u32
                    } else {
                        0
                    };
                    
                    println!("{:10} : {:4} keys ({:3}% complete)", 
                        filename, total, percentage);
                }
            }
            
            Ok(())
        }
    }
}