use regex::Regex;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use tokio;

// --- Data Structures ---

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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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

// --- Helper Functions ---

pub fn is_single_word(text: &str) -> bool {
    let trimmed = text.trim();
    !trimmed.contains(char::is_whitespace) && trimmed.len() < 50
}

// --- Core Translation Logic ---

pub async fn translate_text(text: &str) -> CombinedTranslationData {
    let search_word = text.trim().to_string();

    if is_single_word(&search_word) {
        // For single words, fetch from both Google and Longdo concurrently.
        let (google_res, longdo_res) = tokio::join!(
            google_translate(&search_word, "th", "en"),
            fetch_longdo_translation(&search_word)
        );

        let google_translation =
            google_res.unwrap_or_else(|e| format!("Google Translate Error: {}", e));
        let longdo_data = longdo_res.ok();

        CombinedTranslationData {
            search_word,
            source_lang: "EN".to_string(),
            target_lang: "TH".to_string(),
            google_translation,
            longdo_data,
        }
    } else {
        // For sentences, only use Google Translate.
        let google_translation = google_translate(&search_word, "th", "en")
            .await
            .unwrap_or_else(|e| format!("Google Translate Error: {}", e));

        CombinedTranslationData {
            search_word,
            source_lang: "EN".to_string(),
            target_lang: "TH".to_string(),
            google_translation,
            longdo_data: None,
        }
    }
}

// --- Service-Specific Fetchers ---

async fn google_translate(
    text: &str,
    target_lang: &str,
    source_lang: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let url = format!(
        "https://translate.googleapis.com/translate_a/single?client=gtx&sl={}&tl={}&dt=t&q={}",
        source_lang,
        target_lang,
        urlencoding::encode(text)
    );

    let client = reqwest::Client::new();
    let response = client.get(&url).send().await?;
    let json: serde_json::Value = response.json().await?;

    if let Some(translations) = json.get(0).and_then(|v| v.as_array()) {
        let mut result = String::new();
        for item in translations {
            if let Some(text_part) = item.get(0).and_then(|v| v.as_str()) {
                result.push_str(text_part);
            }
        }
        Ok(result)
    } else {
        Err("Failed to parse Google Translate JSON response".into())
    }
}

async fn fetch_longdo_translation(
    word: &str,
) -> Result<LongdoData, Box<dyn std::error::Error + Send + Sync>> {
    let url = format!("https://dict.longdo.com/mobile.php?search={}", word);
    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .header(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        )
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await?;

    let html = response.text().await?;
    parse_longdo_html(&html)
}

// --- HTML Parsing Logic for Longdo (Adapted from user's working code) ---

fn parse_longdo_html(html: &str) -> Result<LongdoData, Box<dyn std::error::Error + Send + Sync>> {
    let document = Html::parse_document(html);
    let mut data = LongdoData::default();

    let target_dicts = vec![
        "NECTEC Lexitron Dictionary EN-TH",
        "Nontri Dictionary",
        "Hope Dictionary",
    ];
    let b_selector = Selector::parse("b").unwrap();

    // Parse translations by finding the dictionary header first.
    for dict_name in &target_dicts {
        for b_element in document.select(&b_selector) {
            let text = b_element.text().collect::<String>();
            if text.contains(dict_name) {
                let mut next = b_element.next_sibling();
                while let Some(node) = next {
                    if let Some(elem) = scraper::ElementRef::wrap(node) {
                        if elem.value().name() == "table" {
                            if let Some(class) = elem.value().attr("class") {
                                if class.contains("result-table") {
                                    parse_translation_table(&elem, &mut data, dict_name);
                                    break;
                                }
                            }
                        }
                    }
                    next = node.next_sibling();
                }
            }
        }
    }

    parse_examples(&document, &mut data);

    Ok(data)
}

fn parse_translation_table(table: &scraper::ElementRef, data: &mut LongdoData, dict_name: &str) {
    let tr_selector = Selector::parse("tr").unwrap();
    let td_selector = Selector::parse("td").unwrap();

    for row in table.select(&tr_selector) {
        let cells: Vec<_> = row.select(&td_selector).collect();
        if cells.len() == 2 {
            let word = cells[0].text().collect::<String>().trim().to_string();
            let definition = cells[1].text().collect::<String>().trim().to_string();

            if !word.is_empty() && !definition.is_empty() {
                let (pos, translation) = parse_definition(&definition);
                data.translations.push(TranslationItem {
                    word,
                    pos,
                    translation,
                    dictionary: dict_name.to_string(),
                });
            }
        }
    }
}

fn parse_definition(definition: &str) -> (String, String) {
    let re = Regex::new(r"^\s*\((.*?)\)\s*(.*)").unwrap();

    if let Some(caps) = re.captures(definition) {
        let pos = caps.get(1).map_or("N/A", |m| m.as_str()).trim().to_string();
        let translation_text = caps.get(2).map_or("", |m| m.as_str()).trim().to_string();

        let pos_re = Regex::new(r"^(?i)(pron|adj|det|n|v|adv|int|conj)\.?\s*(.*)").unwrap();
        if let Some(caps2) = pos_re.captures(&translation_text) {
            let extracted_pos = caps2.get(1).map_or("", |m| m.as_str());
            let final_translation = caps2.get(2).map_or("", |m| m.as_str()).trim().to_string();
            return (extracted_pos.to_string(), final_translation);
        }
        return (pos, translation_text);
    }
    ("N/A".to_string(), definition.to_string())
}

fn parse_examples(document: &Html, data: &mut LongdoData) {
    let b_selector = Selector::parse("b").unwrap();
    for b_element in document.select(&b_selector) {
        let text = b_element.text().collect::<String>();
        if text.contains("ตัวอย่างประโยค") {
            let mut next = b_element.next_sibling();
            while let Some(node) = next {
                if let Some(elem) = scraper::ElementRef::wrap(node) {
                    if elem.value().name() == "table" {
                        if let Some(class) = elem.value().attr("class") {
                            if class.contains("result-table") {
                                parse_example_table(&elem, data);
                                return;
                            }
                        }
                    }
                }
                next = node.next_sibling();
            }
        }
    }
}

fn parse_example_table(table: &scraper::ElementRef, data: &mut LongdoData) {
    let tr_selector = Selector::parse("tr").unwrap();
    let font_selector = Selector::parse("font[color='black']").unwrap();

    for row in table.select(&tr_selector) {
        let fonts: Vec<_> = row.select(&font_selector).collect();
        if fonts.len() == 2 {
            let en = fonts[0].text().collect::<String>().trim().to_string();
            let th = fonts[1].text().collect::<String>().trim().to_string();
            if !en.is_empty() && !th.is_empty() {
                data.examples.push(ExampleItem { en, th });
            }
        }
    }
}
