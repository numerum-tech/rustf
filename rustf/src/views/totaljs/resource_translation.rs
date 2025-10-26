use anyhow::{Context, Result};
use indexmap::IndexMap;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;

/// Resource-based translation system with sectioned support
#[derive(Clone, Debug)]
pub struct ResourceTranslationSystem {
    /// Current language
    current_language: String,

    /// Translations organized by section
    translations: HashMap<String, SectionedTranslations>,

    /// Fallback language when translation not found
    fallback_language: String,

    /// Pre-computed view translations (cached per view)
    view_cache: Arc<dashmap::DashMap<(String, String), Arc<HashMap<String, String>>>>,
}

impl ResourceTranslationSystem {
    /// Create a new resource translation system
    pub fn new() -> Self {
        Self {
            current_language: "en".to_string(),
            translations: HashMap::new(),
            fallback_language: "en".to_string(),
            view_cache: Arc::new(dashmap::DashMap::new()),
        }
    }

    /// Load translations from a resource file
    pub fn load_resource(&mut self, language: &str, path: &Path) -> Result<()> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read translation file: {}", path.display()))?;

        let translations = parse_resource_content(&content)?;
        self.translations.insert(language.to_string(), translations);

        // Clear cache when loading new translations
        self.view_cache.clear();

        Ok(())
    }

    /// Load all resource files from a directory
    pub fn load_resources_dir(&mut self, dir: &Path) -> Result<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension() == Some(std::ffi::OsStr::new("res")) {
                if let Some(filename) = path.file_stem() {
                    let lang = filename.to_str().unwrap();
                    if lang != "default" {
                        self.load_resource(lang, &path)?;
                    }
                }
            }
        }

        // Load default as fallback
        let default_path = dir.join("default.res");
        if default_path.exists() {
            let fallback_lang = self.fallback_language.clone();
            self.load_resource(&fallback_lang, &default_path)?;
        }

        Ok(())
    }

    /// Set the current language
    pub fn set_language(&mut self, language: &str) {
        if self.current_language != language {
            self.current_language = language.to_string();
            self.view_cache.clear();
        }
    }

    /// Set the fallback language
    pub fn set_fallback(&mut self, language: &str) {
        if self.fallback_language != language {
            self.fallback_language = language.to_string();
            self.view_cache.clear();
        }
    }

    /// Get translations for a specific view (with caching)
    pub fn get_view_translations(&self, view_path: &str) -> Arc<HashMap<String, String>> {
        let cache_key = (self.current_language.clone(), view_path.to_string());

        // Check cache first
        if let Some(cached) = self.view_cache.get(&cache_key) {
            return cached.clone();
        }

        // Build merged translations for this view
        let mut merged = HashMap::new();

        // Start with global translations from current language
        if let Some(lang_translations) = self.translations.get(&self.current_language) {
            if let Some(global) = &lang_translations.global {
                for (key, value) in global {
                    merged.insert(key.clone(), value.clone());
                }
            }

            // Override with view-specific translations
            let section = view_to_section_name(view_path);
            if let Some(section_translations) = lang_translations.sections.get(&section) {
                for (key, value) in section_translations {
                    merged.insert(key.clone(), value.clone());
                }
            }
        }

        // Add fallback translations for missing keys
        if self.fallback_language != self.current_language {
            if let Some(fallback_translations) = self.translations.get(&self.fallback_language) {
                // Add global fallbacks
                if let Some(global) = &fallback_translations.global {
                    for (key, value) in global {
                        merged.entry(key.clone()).or_insert_with(|| value.clone());
                    }
                }

                // Add view-specific fallbacks
                let section = view_to_section_name(view_path);
                if let Some(section_translations) = fallback_translations.sections.get(&section) {
                    for (key, value) in section_translations {
                        merged.entry(key.clone()).or_insert_with(|| value.clone());
                    }
                }
            }
        }

        let arc_merged = Arc::new(merged);
        self.view_cache.insert(cache_key, arc_merged.clone());
        arc_merged
    }

    /// Translate text or key for a specific view
    pub fn translate(&self, text: &str, view_path: Option<&str>) -> String {
        // If it's a direct key reference @(#key)
        if text.starts_with('#') {
            return self.translate_key(&text[1..], view_path);
        }

        // Generate key from text and translate
        let key = generate_translation_key(text);
        self.translate_key(&key, view_path)
    }

    /// Translate by key for a specific view
    pub fn translate_key(&self, key: &str, view_path: Option<&str>) -> String {
        if let Some(view) = view_path {
            let translations = self.get_view_translations(view);
            translations
                .get(key)
                .cloned()
                .unwrap_or_else(|| key.to_string())
        } else {
            // No view context - try global only
            self.translations
                .get(&self.current_language)
                .and_then(|t| t.global.as_ref())
                .and_then(|g| g.get(key))
                .cloned()
                .unwrap_or_else(|| key.to_string())
        }
    }
}

impl Default for ResourceTranslationSystem {
    fn default() -> Self {
        Self::new()
    }
}

/// Sectioned translations from a resource file
#[derive(Clone, Debug)]
struct SectionedTranslations {
    global: Option<IndexMap<String, String>>,
    sections: HashMap<String, IndexMap<String, String>>,
}

/// Parse resource file content into sectioned translations
fn parse_resource_content(content: &str) -> Result<SectionedTranslations> {
    let mut sections = HashMap::new();
    let mut current_section = "global".to_string();
    let mut current_translations = IndexMap::new();

    for line in content.lines() {
        let line = line.trim();

        // Skip comments and empty lines
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Section header [section_name]
        if line.starts_with('[') && line.ends_with(']') {
            // Save previous section
            if !current_translations.is_empty() {
                sections.insert(current_section.clone(), current_translations);
                current_translations = IndexMap::new();
            }

            current_section = line[1..line.len() - 1].to_string();
            continue;
        }

        // Translation entry: key : "value"
        if let Some(pos) = line.find(" : ") {
            let key = line[..pos].trim().to_string();
            let value_part = &line[pos + 3..].trim();

            // Parse quoted value
            let value = if value_part.starts_with('"') && value_part.ends_with('"') {
                unescape_value(&value_part[1..value_part.len() - 1])
            } else {
                value_part.to_string()
            };

            current_translations.insert(key, value);
        }
    }

    // Save last section
    if !current_translations.is_empty() {
        sections.insert(current_section, current_translations);
    }

    // Extract global section
    let global = sections.remove("global");

    Ok(SectionedTranslations { global, sections })
}

/// Convert view path to section name
fn view_to_section_name(view_path: &str) -> String {
    // Remove .html extension and prefix with views/
    let path = view_path.trim_end_matches(".html");
    if path.starts_with("views/") {
        path.to_string()
    } else {
        format!("views/{}", path)
    }
}

/// Generate translation key from text
fn generate_translation_key(text: &str) -> String {
    // Simple slug generation (matches CLI scanner logic)
    text.chars()
        .filter_map(|c| match c {
            'a'..='z' | '0'..='9' => Some(c),
            'A'..='Z' => Some(c.to_ascii_lowercase()),
            ' ' | '-' => Some('_'),
            _ => None,
        })
        .take(30)
        .collect::<String>()
        .trim_matches('_')
        .to_string()
}

/// Unescape value from resource file
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_resource_parsing() {
        let content = r#"
[global]
save : "Save"
cancel : "Cancel"

[views/home/index]
welcome : "Welcome"
title : "Home Page"
"#;

        let translations = parse_resource_content(content).unwrap();

        assert!(translations.global.is_some());
        let global = translations.global.unwrap();
        assert_eq!(global.get("save"), Some(&"Save".to_string()));
        assert_eq!(global.get("cancel"), Some(&"Cancel".to_string()));

        assert!(translations.sections.contains_key("views/home/index"));
        let home = translations.sections.get("views/home/index").unwrap();
        assert_eq!(home.get("welcome"), Some(&"Welcome".to_string()));
    }

    #[test]
    fn test_view_translations() {
        let mut system = ResourceTranslationSystem::new();

        // Create temp directory with test files
        let dir = tempdir().unwrap();
        let default_path = dir.path().join("default.res");

        let mut file = fs::File::create(&default_path).unwrap();
        writeln!(file, "[global]").unwrap();
        writeln!(file, "save : \"Save\"").unwrap();
        writeln!(file, "[views/home/index]").unwrap();
        writeln!(file, "welcome : \"Welcome\"").unwrap();

        system.load_resource("en", &default_path).unwrap();

        let translations = system.get_view_translations("home/index");
        assert_eq!(translations.get("save"), Some(&"Save".to_string()));
        assert_eq!(translations.get("welcome"), Some(&"Welcome".to_string()));
    }
}
