use anyhow::Result;
use indexmap::IndexMap;
use std::fs;
use std::path::Path;
use super::parser::{ResourceParser, SectionedTranslations};

pub struct TranslationUpdater {
    default_translations: SectionedTranslations,
}

impl TranslationUpdater {
    pub fn new(default_path: &Path) -> Result<Self> {
        let parser = ResourceParser::new();
        let default_translations = parser.parse_file(default_path)?;
        
        Ok(Self {
            default_translations,
        })
    }
    
    pub fn update_language_file(&self, resources_dir: &str, lang: &str) -> Result<UpdateReport> {
        let file_path = Path::new(resources_dir).join(format!("{}.res", lang));
        
        // Load existing translations if file exists
        let existing = if file_path.exists() {
            let parser = ResourceParser::new();
            Some(parser.parse_file(&file_path)?)
        } else {
            None
        };
        
        let mut report = UpdateReport::new(lang);
        let mut updated_sections = IndexMap::new();
        
        // Process each section from default
        for (section_name, default_entries) in self.default_translations.sections() {
            let mut section_translations = IndexMap::new();
            
            for (key, _default_text) in default_entries {
                // Check if we have an existing translation
                let existing_value = existing.as_ref()
                    .and_then(|e| e.get(section_name, key));
                
                if let Some(value) = existing_value {
                    // Preserve existing translation
                    section_translations.insert(key.clone(), value.to_string());
                    if !value.is_empty() {
                        report.existing += 1;
                    } else {
                        report.new_keys.push(format!("[{}] {}", section_name, key));
                    }
                } else {
                    // New key - needs translation
                    section_translations.insert(key.clone(), String::new());
                    report.new_keys.push(format!("[{}] {}", section_name, key));
                }
            }
            
            if !section_translations.is_empty() {
                updated_sections.insert(section_name.to_string(), section_translations);
            }
        }
        
        // Check for deprecated keys (in existing but not in default)
        if let Some(existing) = &existing {
            for (section_name, existing_entries) in existing.sections() {
                for (key, value) in existing_entries {
                    if !self.default_translations.get(section_name, key).is_some() {
                        // This key no longer exists in default
                        if !value.is_empty() {
                            report.deprecated_keys.push(format!("[{}] {}", section_name, key));
                            
                            // Preserve it as deprecated
                            let section = updated_sections.entry(section_name.to_string())
                                .or_insert_with(IndexMap::new);
                            section.insert(format!("_deprecated_{}", key), value.clone());
                        }
                    }
                }
            }
        }
        
        // Write updated file
        self.write_updated_file(&file_path, &updated_sections, &report)?;
        
        Ok(report)
    }
    
    fn write_updated_file(
        &self,
        path: &Path,
        sections: &IndexMap<String, IndexMap<String, String>>,
        report: &UpdateReport,
    ) -> Result<()> {
        use std::io::Write;
        
        // Backup existing file if it exists
        if path.exists() {
            let backup_path = path.with_extension("res.backup");
            fs::copy(path, backup_path)?;
        }
        
        let mut file = fs::File::create(path)?;
        
        // Write header
        writeln!(file, "# Translation file for: {}", report.language)?;
        writeln!(file, "# Last updated: {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"))?;
        writeln!(file, "# Total keys: {}", count_total_keys(sections))?;
        
        if !report.new_keys.is_empty() {
            writeln!(file, "#")?;
            writeln!(file, "# NEW KEYS (need translation): {}", report.new_keys.len())?;
            for key in report.new_keys.iter().take(10) {
                writeln!(file, "#   - {}", key)?;
            }
            if report.new_keys.len() > 10 {
                writeln!(file, "#   ... and {} more", report.new_keys.len() - 10)?;
            }
        }
        
        if !report.deprecated_keys.is_empty() {
            writeln!(file, "#")?;
            writeln!(file, "# DEPRECATED (removed from views): {}", report.deprecated_keys.len())?;
            for key in report.deprecated_keys.iter().take(5) {
                writeln!(file, "#   - {}", key)?;
            }
            if report.deprecated_keys.len() > 5 {
                writeln!(file, "#   ... and {} more", report.deprecated_keys.len() - 5)?;
            }
        }
        
        writeln!(file)?;
        
        // Write sections
        for (section_name, translations) in sections {
            // Skip empty sections
            let active_keys: Vec<_> = translations.iter()
                .filter(|(k, _)| !k.starts_with("_deprecated_"))
                .collect();
            
            if active_keys.is_empty() && !translations.iter().any(|(k, _)| k.starts_with("_deprecated_")) {
                continue;
            }
            
            writeln!(file, "[{}]", section_name)?;
            
            // Write active translations
            for (key, value) in &active_keys {
                if value.is_empty() {
                    writeln!(file, "{} : \"\" # TODO: needs translation", key)?;
                } else {
                    writeln!(file, "{} : \"{}\"", key, escape_value(value))?;
                }
            }
            
            // Write deprecated keys as comments
            for (key, value) in translations {
                if key.starts_with("_deprecated_") {
                    let original_key = key.strip_prefix("_deprecated_").unwrap();
                    writeln!(file, "# {} : \"{}\" # DEPRECATED", original_key, escape_value(value))?;
                }
            }
            
            writeln!(file)?;
        }
        
        Ok(())
    }
}

#[derive(Debug)]
pub struct UpdateReport {
    pub language: String,
    pub existing: usize,
    pub new_keys: Vec<String>,
    pub deprecated_keys: Vec<String>,
}

impl UpdateReport {
    pub fn new(language: &str) -> Self {
        Self {
            language: language.to_string(),
            existing: 0,
            new_keys: Vec::new(),
            deprecated_keys: Vec::new(),
        }
    }
    
    pub fn print_summary(&self) {
        println!("\n📊 Update Report for {}.res:", self.language);
        println!("  ✓ Existing translations preserved: {}", self.existing);
        
        if !self.new_keys.is_empty() {
            println!("  ⚠️  New keys needing translation: {}", self.new_keys.len());
            for key in self.new_keys.iter().take(5) {
                println!("      - {}", key);
            }
            if self.new_keys.len() > 5 {
                println!("      ... and {} more", self.new_keys.len() - 5);
            }
        }
        
        if !self.deprecated_keys.is_empty() {
            println!("  ℹ️  Deprecated keys (no longer in views): {}", self.deprecated_keys.len());
            for key in self.deprecated_keys.iter().take(3) {
                println!("      - {}", key);
            }
            if self.deprecated_keys.len() > 3 {
                println!("      ... and {} more", self.deprecated_keys.len() - 3);
            }
        }
        
        println!();
    }
}

fn count_total_keys(sections: &IndexMap<String, IndexMap<String, String>>) -> usize {
    sections.values()
        .flat_map(|s| s.keys())
        .filter(|k| !k.starts_with("_deprecated_"))
        .count()
}

fn escape_value(s: &str) -> String {
    s.replace('\\', "\\\\")
     .replace('\"', "\\\"")
     .replace('\n', "\\n")
     .replace('\r', "\\r")
     .replace('\t', "\\t")
}