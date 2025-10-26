use anyhow::Result;
use indexmap::IndexMap;
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

pub struct TranslationScanner {
    sections: IndexMap<String, IndexMap<String, String>>,
    key_generator: KeyGenerator,
}

impl TranslationScanner {
    pub fn new() -> Self {
        let mut sections = IndexMap::new();
        sections.insert("global".to_string(), IndexMap::new());
        
        Self {
            sections,
            key_generator: KeyGenerator::new(30),
        }
    }
    
    pub fn scan_directory(&mut self, views_dir: &Path) -> Result<()> {
        let pattern = Regex::new(r"@\(([^)]+)\)").unwrap();
        
        for entry in WalkDir::new(views_dir) {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension() == Some(std::ffi::OsStr::new("html")) {
                self.scan_file(path, views_dir, &pattern)?;
            }
        }
        
        // Extract common translations to global section
        self.extract_common_to_global();
        
        Ok(())
    }
    
    fn scan_file(&mut self, path: &Path, base_dir: &Path, pattern: &Regex) -> Result<()> {
        let content = fs::read_to_string(path)?;
        
        // Generate section name from path
        let relative_path = path.strip_prefix(base_dir)?;
        let section = format!("views/{}", 
            relative_path.to_str().unwrap().trim_end_matches(".html"));
        
        let mut view_translations = IndexMap::new();
        
        for cap in pattern.captures_iter(&content) {
            let text = &cap[1];
            
            if text.starts_with('#') {
                // Custom key reference - just track it exists
                let key = &text[1..];
                view_translations.entry(key.to_string())
                    .or_insert_with(String::new);
            } else {
                // Generate readable key from text
                let key = self.key_generator.generate_key(text);
                view_translations.insert(key, text.to_string());
            }
        }
        
        if !view_translations.is_empty() {
            self.sections.insert(section, view_translations);
        }
        
        Ok(())
    }
    
    fn extract_common_to_global(&mut self) {
        let mut frequency: HashMap<String, (String, usize)> = HashMap::new();
        
        // Count occurrences across sections
        for (section, translations) in &self.sections {
            if section == "global" { continue; }
            
            for (key, text) in translations {
                if !text.is_empty() {  // Skip custom keys without default text
                    frequency.entry(key.clone())
                        .and_modify(|(_, count)| *count += 1)
                        .or_insert((text.clone(), 1));
                }
            }
        }
        
        // Move translations that appear in 3+ views to global
        let threshold = 3;
        
        // Collect keys to move to global
        let mut to_global = Vec::new();
        for (key, (text, count)) in frequency {
            if count >= threshold {
                to_global.push((key, text));
            }
        }
        
        // Add to global section
        let global = self.sections.get_mut("global").unwrap();
        for (key, text) in &to_global {
            global.insert(key.clone(), text.clone());
        }
        
        // Remove from individual sections
        for (key, text) in to_global {
            for (section_name, section) in self.sections.iter_mut() {
                if section_name != "global" && section.get(&key) == Some(&text) {
                    section.shift_remove(&key);
                }
            }
        }
    }
    
    pub fn write_resource_file(&self, path: &Path) -> Result<()> {
        use std::io::Write;
        
        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        let mut file = fs::File::create(path)?;
        
        // Write header
        writeln!(file, "# Translation Resource File")?;
        writeln!(file, "# Generated: {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"))?;
        writeln!(file, "# Sections: {}", self.sections.len())?;
        writeln!(file)?;
        
        // Write global section first if it has content
        if let Some(global) = self.sections.get("global") {
            if !global.is_empty() {
                writeln!(file, "[global]")?;
                writeln!(file, "# Common translations used across multiple views")?;
                for (key, text) in global {
                    writeln!(file, "{} : \"{}\"", key, escape_value(text))?;
                }
                writeln!(file)?;
            }
        }
        
        // Write view-specific sections
        for (section, translations) in &self.sections {
            if section == "global" || translations.is_empty() {
                continue;
            }
            
            writeln!(file, "[{}]", section)?;
            for (key, text) in translations {
                if text.is_empty() {
                    // Custom key without default
                    writeln!(file, "{} : \"\"", key)?;
                } else {
                    writeln!(file, "{} : \"{}\"", key, escape_value(text))?;
                }
            }
            writeln!(file)?;
        }
        
        Ok(())
    }
    
    pub fn total_translations(&self) -> usize {
        self.sections.values().map(|s| s.len()).sum()
    }
}

struct KeyGenerator {
    max_length: usize,
    existing_keys: HashSet<String>,
}

impl KeyGenerator {
    fn new(max_length: usize) -> Self {
        Self {
            max_length,
            existing_keys: HashSet::new(),
        }
    }
    
    fn generate_key(&mut self, text: &str) -> String {
        let base_slug = self.create_slug(text);
        
        // If unique, use it
        if !self.existing_keys.contains(&base_slug) && base_slug.len() <= self.max_length {
            self.existing_keys.insert(base_slug.clone());
            return base_slug;
        }
        
        // Try with incrementing suffix
        for i in 1..=99 {
            let key = format!("{}_{}", 
                &base_slug[..base_slug.len().min(self.max_length - 3)], i);
            if !self.existing_keys.contains(&key) {
                self.existing_keys.insert(key.clone());
                return key;
            }
        }
        
        // Fall back to hash suffix
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        let hash = format!("{:x}", hasher.finish());
        
        let key = format!("{}_{}", 
            &base_slug[..base_slug.len().min(self.max_length - 5)],
            &hash[..4]);
        
        self.existing_keys.insert(key.clone());
        key
    }
    
    fn create_slug(&self, text: &str) -> String {
        text.chars()
            .filter_map(|c| match c {
                'a'..='z' | '0'..='9' => Some(c),
                'A'..='Z' => Some(c.to_ascii_lowercase()),
                ' ' | '-' => Some('_'),
                '.' | '!' | '?' | ',' | ':' | ';' => None,
                '\'' | '"' => None,
                _ => None,
            })
            .take(self.max_length)
            .collect::<String>()
            .trim_matches('_')
            .to_string()
    }
}

fn escape_value(s: &str) -> String {
    s.replace('\\', "\\\\")
     .replace('\"', "\\\"")
     .replace('\n', "\\n")
     .replace('\r', "\\r")
     .replace('\t', "\\t")
}