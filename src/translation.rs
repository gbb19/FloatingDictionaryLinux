use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationItem {
    pub word: String,
    pub pos: String, // Part of speech
    pub translation: String,
    pub dictionary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExampleItem {
    pub en: String, // Source sentence
    pub th: String, // Target sentence
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LongdoData {
    pub translations: Vec<TranslationItem>,
    pub examples: Vec<ExampleItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombinedTranslationData {
    pub search_word: String,
    pub source_lang: String,
    pub target_lang: String,
    pub google_translation: String,
    pub longdo_data: Option<LongdoData>,
}

/// A dummy function to simulate fetching and processing translation data.
/// In a real application, this would involve network requests and parsing.
pub async fn translate_text(text: &str) -> CombinedTranslationData {
    // Simulate network delay
    tokio::time::sleep(Duration::from_secs(2)).await;

    // --- Dummy Data ---
    let search_word = text.trim().to_string();
    let google_translation = if search_word.eq_ignore_ascii_case("hello") {
        "สวัสดี".to_string()
    } else {
        format!("คำแปลของ '{}'", search_word)
    };

    let longdo_data = LongdoData {
        translations: vec![
            TranslationItem {
                word: "hello".to_string(),
                pos: "int".to_string(),
                translation: "สวัสดี (คำทักทาย)".to_string(),
                dictionary: "Nontri Dictionary".to_string(),
            },
            TranslationItem {
                word: "hello".to_string(),
                pos: "n".to_string(),
                translation: "เสียงร้องทักทาย".to_string(),
                dictionary: "Hope Dictionary".to_string(),
            },
        ],
        examples: vec![
            ExampleItem {
                en: "She said hello to me.".to_string(),
                th: "เธอกล่าวสวัสดีกับฉัน".to_string(),
            },
            ExampleItem {
                en: "I heard a hello from the other room.".to_string(),
                th: "ฉันได้ยินเสียงทักทายจากห้องอื่น".to_string(),
            },
        ],
    };
    // --- End of Dummy Data ---

    let use_longdo = search_word.eq_ignore_ascii_case("hello");

    CombinedTranslationData {
        search_word,
        source_lang: "EN".to_string(),
        target_lang: "TH".to_string(),
        google_translation,
        longdo_data: if use_longdo { Some(longdo_data) } else { None },
    }
}
