use std::collections::HashMap;

/// Translation system for Total.js templates
#[derive(Clone, Debug)]
pub struct TranslationSystem {
    /// Current language
    current_language: String,

    /// Translations: language -> key -> translated text
    translations: HashMap<String, HashMap<String, String>>,

    /// Fallback language when translation not found
    fallback_language: String,
}

impl TranslationSystem {
    /// Create a new translation system
    pub fn new() -> Self {
        Self {
            current_language: "en".to_string(),
            translations: HashMap::new(),
            fallback_language: "en".to_string(),
        }
    }

    /// Set the current language
    pub fn set_language(&mut self, language: &str) {
        self.current_language = language.to_string();
    }

    /// Set the fallback language
    pub fn set_fallback(&mut self, language: &str) {
        self.fallback_language = language.to_string();
    }

    /// Add translations for a language
    pub fn add_translations(&mut self, language: &str, translations: HashMap<String, String>) {
        self.translations.insert(language.to_string(), translations);
    }

    /// Load translations from a JSON value
    pub fn load_from_json(&mut self, language: &str, json: &serde_json::Value) {
        if let serde_json::Value::Object(map) = json {
            let mut translations = HashMap::new();
            for (key, value) in map {
                if let serde_json::Value::String(text) = value {
                    translations.insert(key.clone(), text.clone());
                }
            }
            self.add_translations(language, translations);
        }
    }

    /// Translate a key
    pub fn translate_key(&self, key: &str) -> String {
        // Try current language
        if let Some(lang_translations) = self.translations.get(&self.current_language) {
            if let Some(translation) = lang_translations.get(key) {
                return translation.clone();
            }
        }

        // Try fallback language
        if self.fallback_language != self.current_language {
            if let Some(lang_translations) = self.translations.get(&self.fallback_language) {
                if let Some(translation) = lang_translations.get(key) {
                    return translation.clone();
                }
            }
        }

        // Return the key itself if no translation found
        format!("[{}]", key)
    }

    /// Translate text (simple text-based translation)
    pub fn translate_text(&self, text: &str) -> String {
        // For text translation, we use the text itself as the key
        // This is a simplified approach - in production you might want
        // to use more sophisticated text matching

        // Try to find exact match in current language
        if let Some(lang_translations) = self.translations.get(&self.current_language) {
            for (key, translation) in lang_translations {
                if key == text {
                    return translation.clone();
                }
            }
        }

        // Try fallback language
        if self.fallback_language != self.current_language {
            if let Some(lang_translations) = self.translations.get(&self.fallback_language) {
                for (key, translation) in lang_translations {
                    if key == text {
                        return translation.clone();
                    }
                }
            }
        }

        // Return original text if no translation found
        text.to_string()
    }

    /// Check if a language is available
    pub fn has_language(&self, language: &str) -> bool {
        self.translations.contains_key(language)
    }

    /// Get available languages
    pub fn available_languages(&self) -> Vec<String> {
        self.translations.keys().cloned().collect()
    }
}

impl Default for TranslationSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_translation_key() {
        let mut translator = TranslationSystem::new();

        let mut en_translations = HashMap::new();
        en_translations.insert("welcome.message".to_string(), "Welcome!".to_string());
        en_translations.insert("goodbye".to_string(), "Goodbye!".to_string());
        translator.add_translations("en", en_translations);

        let mut fr_translations = HashMap::new();
        fr_translations.insert("welcome.message".to_string(), "Bienvenue!".to_string());
        fr_translations.insert("goodbye".to_string(), "Au revoir!".to_string());
        translator.add_translations("fr", fr_translations);

        // Test English
        translator.set_language("en");
        assert_eq!(translator.translate_key("welcome.message"), "Welcome!");
        assert_eq!(translator.translate_key("goodbye"), "Goodbye!");

        // Test French
        translator.set_language("fr");
        assert_eq!(translator.translate_key("welcome.message"), "Bienvenue!");
        assert_eq!(translator.translate_key("goodbye"), "Au revoir!");

        // Test missing key
        assert_eq!(translator.translate_key("missing.key"), "[missing.key]");
    }

    #[test]
    fn test_fallback_language() {
        let mut translator = TranslationSystem::new();
        translator.set_fallback("en");

        let mut en_translations = HashMap::new();
        en_translations.insert("only_in_english".to_string(), "English only".to_string());
        translator.add_translations("en", en_translations);

        // Set to a language without this translation
        translator.set_language("fr");

        // Should fall back to English
        assert_eq!(translator.translate_key("only_in_english"), "English only");
    }

    #[test]
    fn test_load_from_json() {
        let mut translator = TranslationSystem::new();

        let json = json!({
            "hello": "Hello",
            "world": "World",
            "app.name": "My App"
        });

        translator.load_from_json("en", &json);
        translator.set_language("en");

        assert_eq!(translator.translate_key("hello"), "Hello");
        assert_eq!(translator.translate_key("world"), "World");
        assert_eq!(translator.translate_key("app.name"), "My App");
    }
}
