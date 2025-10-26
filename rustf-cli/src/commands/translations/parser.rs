use anyhow::{Result, Context};
use indexmap::IndexMap;
use std::fs;
use std::path::Path;

pub struct ResourceParser;

impl ResourceParser {
    pub fn new() -> Self {
        Self
    }
    
    pub fn parse_file(&self, path: &Path) -> Result<SectionedTranslations> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        
        self.parse_content(&content)
    }
    
    pub fn parse_content(&self, content: &str) -> Result<SectionedTranslations> {
        let mut sections = IndexMap::new();
        let mut current_section = "global".to_string();
        
        sections.insert(current_section.clone(), IndexMap::new());
        
        for (line_num, line) in content.lines().enumerate() {
            let line = line.trim();
            
            // Skip comments and empty lines
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            
            // Section header [section_name]
            if line.starts_with('[') && line.ends_with(']') {
                current_section = line[1..line.len()-1].to_string();
                sections.entry(current_section.clone())
                    .or_insert_with(IndexMap::new);
                continue;
            }
            
            // Translation entry: key : "value"
            if let Some(pos) = line.find(" : ") {
                let key = line[..pos].trim().to_string();
                let value_part = &line[pos + 3..].trim();
                
                // Parse quoted value
                let value = if value_part.starts_with('"') && value_part.ends_with('"') {
                    unescape_value(&value_part[1..value_part.len()-1])
                } else {
                    // Handle unquoted values for backward compatibility
                    value_part.to_string()
                };
                
                sections.get_mut(&current_section)
                    .unwrap()
                    .insert(key, value);
            } else if line.contains(':') {
                // Try to handle malformed lines gracefully
                eprintln!("Warning: Malformed translation at line {}: {}", line_num + 1, line);
            }
        }
        
        Ok(SectionedTranslations { sections })
    }
}

pub struct SectionedTranslations {
    sections: IndexMap<String, IndexMap<String, String>>,
}

impl SectionedTranslations {
    pub fn get(&self, section: &str, key: &str) -> Option<&str> {
        self.sections.get(section)
            .and_then(|s| s.get(key))
            .map(|s| s.as_str())
    }
    
    pub fn get_with_fallback(&self, section: &str, key: &str) -> Option<&str> {
        // Try section-specific first
        if let Some(value) = self.get(section, key) {
            return Some(value);
        }
        
        // Fall back to global
        self.get("global", key)
    }
    
    pub fn sections(&self) -> impl Iterator<Item = (&str, &IndexMap<String, String>)> {
        self.sections.iter().map(|(k, v)| (k.as_str(), v))
    }
    
    pub fn total_keys(&self) -> usize {
        self.sections.values().map(|s| s.len()).sum()
    }
    
    pub fn complete_keys(&self) -> usize {
        self.sections.values()
            .flat_map(|s| s.values())
            .filter(|v| !v.is_empty())
            .count()
    }
    
    pub fn get_section(&self, section: &str) -> Option<&IndexMap<String, String>> {
        self.sections.get(section)
    }
    
    pub fn has_section(&self, section: &str) -> bool {
        self.sections.contains_key(section)
    }
    
    pub fn merge_with(&mut self, other: SectionedTranslations) {
        for (section, translations) in other.sections {
            let target = self.sections.entry(section).or_insert_with(IndexMap::new);
            for (key, value) in translations {
                // Only override if the new value is not empty
                if !value.is_empty() || !target.contains_key(&key) {
                    target.insert(key, value);
                }
            }
        }
    }
}

fn unescape_value(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars();
    
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            if let Some(next) = chars.next() {
                match next {
                    'n' => result.push('\n'),
                    'r' => result.push('\r'),
                    't' => result.push('\t'),
                    '\\' => result.push('\\'),
                    '"' => result.push('"'),
                    _ => {
                        result.push(ch);
                        result.push(next);
                    }
                }
            } else {
                result.push(ch);
            }
        } else {
            result.push(ch);
        }
    }
    
    result
}