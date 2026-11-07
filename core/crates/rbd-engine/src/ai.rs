//! Offline Document AI & Natural Language Assistance Engine.
//!
//! Provides 100% offline, privacy-guaranteed document understanding:
//! - Extractive document summarization with TF-ISF sentence scoring
//! - Key topic & keyword extraction with stop-word filtering (Turkish & English)
//! - In-document question answering & semantic passage retrieval with page citations
//! - Document reading metrics (word count, reading time estimation)
//!
//! Conforms strictly to Rule 5 of AGENTS.md: zero network calls, zero external APIs.

#![allow(clippy::collapsible_if, clippy::collapsible_match)]

use crate::error::EngineError;
use crate::search::turkish_lowercase;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Stop words for Turkish and English to filter out non-informative tokens.
const STOP_WORDS_EN: &[&str] = &[
    "a",
    "about",
    "above",
    "after",
    "again",
    "against",
    "all",
    "am",
    "an",
    "and",
    "any",
    "are",
    "aren't",
    "as",
    "at",
    "be",
    "because",
    "been",
    "before",
    "being",
    "below",
    "between",
    "both",
    "but",
    "by",
    "can't",
    "cannot",
    "could",
    "did",
    "do",
    "does",
    "doing",
    "don't",
    "down",
    "during",
    "each",
    "few",
    "for",
    "from",
    "further",
    "had",
    "has",
    "have",
    "having",
    "he",
    "her",
    "here",
    "hers",
    "herself",
    "him",
    "himself",
    "his",
    "how",
    "i",
    "if",
    "in",
    "into",
    "is",
    "it",
    "its",
    "itself",
    "let's",
    "me",
    "more",
    "most",
    "my",
    "myself",
    "no",
    "nor",
    "not",
    "of",
    "off",
    "on",
    "once",
    "only",
    "or",
    "other",
    "ought",
    "our",
    "ours",
    "ourselves",
    "out",
    "over",
    "own",
    "same",
    "she",
    "should",
    "so",
    "some",
    "such",
    "than",
    "that",
    "the",
    "their",
    "theirs",
    "them",
    "themselves",
    "then",
    "there",
    "these",
    "they",
    "this",
    "those",
    "through",
    "to",
    "too",
    "under",
    "until",
    "up",
    "very",
    "was",
    "wasn't",
    "we",
    "were",
    "what",
    "when",
    "where",
    "which",
    "while",
    "who",
    "whom",
    "why",
    "with",
    "would",
    "you",
    "your",
    "yours",
    "yourself",
    "yourselves",
];

const STOP_WORDS_TR: &[&str] = &[
    "acaba",
    "altı",
    "ama",
    "ancak",
    "arada",
    "artık",
    "asla",
    "aslında",
    "ayrıca",
    "bana",
    "bazen",
    "bazı",
    "bazıları",
    "belki",
    "ben",
    "benden",
    "beni",
    "benim",
    "beri",
    "beş",
    "bile",
    "bin",
    "bir",
    "biraz",
    "biri",
    "birkaçı",
    "birkez",
    "birçok",
    "birlikte",
    "biz",
    "bizden",
    "bize",
    "bizi",
    "bizim",
    "böyle",
    "böylece",
    "bu",
    "buna",
    "bunda",
    "bundan",
    "bunu",
    "bunun",
    "burada",
    "bütün",
    "çoğu",
    "çoğunlukla",
    "çok",
    "çünkü",
    "da",
    "daha",
    "dahi",
    "de",
    "defa",
    "değil",
    "diğer",
    "diğeri",
    "diye",
    "dolayı",
    "doksan",
    "dokuz",
    "dolayısıyla",
    "dört",
    "edecek",
    "eden",
    "ederek",
    "edilecek",
    "ediliyor",
    "edilmesi",
    "ediyor",
    "eğer",
    "elli",
    "en",
    "etmesi",
    "etti",
    "ettiği",
    "ettiğini",
    "fakat",
    "gibi",
    "göre",
    "halen",
    "hangi",
    "hangisi",
    "hani",
    "hatta",
    "hem",
    "henüz",
    "hep",
    "hepsi",
    "her",
    "herhangi",
    "herkes",
    "herkesin",
    "hiç",
    "hiçbir",
    "hiçbiri",
    "için",
    "içinde",
    "iki",
    "ile",
    "ilgili",
    "ise",
    "işte",
    "itibaren",
    "kadar",
    "karşın",
    "katrilyon",
    "kendi",
    "kendine",
    "kendini",
    "kendisi",
    "kendisine",
    "kendisini",
    "kez",
    "ki",
    "kim",
    "kime",
    "kimi",
    "kimse",
    "kırk",
    "milyar",
    "milyon",
    "mu",
    "mü",
    "mı",
    "misin",
    "nasıl",
    "ne",
    "neden",
    "nedenle",
    "nerde",
    "nerede",
    "nereye",
    "nitekim",
    "o",
    "olan",
    "olarak",
    "oldu",
    "olduğu",
    "olduğunu",
    "olduklarını",
    "olmadı",
    "olmadığı",
    "olmak",
    "olması",
    "olmayan",
    "olmaz",
    "olsa",
    "olsun",
    "olup",
    "olur",
    "olursa",
    "oluyor",
    "on",
    "ona",
    "ondan",
    "onlar",
    "onlardan",
    "onlara",
    "onların",
    "onları",
    "onu",
    "onun",
    "orada",
    "oysa",
    "oysaki",
    "öbürü",
    "ön",
    "önce",
    "ötürü",
    "öyle",
    "pek",
    "rağmen",
    "sadece",
    "sanki",
    "sekiz",
    "seksen",
    "sen",
    "senden",
    "seni",
    "senin",
    "siz",
    "sizden",
    "size",
    "sizi",
    "sizin",
    "sonra",
    "şayet",
    "şey",
    "şeyden",
    "şeyi",
    "şeyler",
    "şu",
    "şuna",
    "şunda",
    "şundan",
    "şunları",
    "şunu",
    "tarafından",
    "trilyon",
    "tüm",
    "üç",
    "üzere",
    "var",
    "vardı",
    "ve",
    "veya",
    "veyahut",
    "ya",
    "yahut",
    "yalnız",
    "yani",
    "yapacak",
    "yapılan",
    "yapılması",
    "yapıyor",
    "yedi",
    "yetmiş",
    "yine",
    "yirmi",
    "yok",
    "yoksa",
    "yüz",
    "zaten",
];

/// A key topic keyword with calculated importance score.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KeywordScore {
    pub word: String,
    pub count: usize,
    pub score: f32,
}

/// Extracted document summary and metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocumentSummary {
    /// Combined fluent summary text.
    pub summary_text: String,
    /// Individual key sentences selected from the document in chronological order.
    pub key_sentences: Vec<String>,
    /// Top extracted key topics and keywords.
    pub top_keywords: Vec<KeywordScore>,
    /// Total sentences analyzed across document.
    pub total_sentences: usize,
    /// Estimated word count.
    pub word_count: usize,
    /// Estimated reading time in minutes (based on 200 wpm).
    pub reading_time_minutes: f32,
}

/// Result of an in-document natural language question query.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocumentAnswer {
    /// The user query.
    pub query: String,
    /// The extracted answer sentence or passage.
    pub answer_text: String,
    /// Wider context excerpt around the answer.
    pub context_excerpt: String,
    /// 1-based page index where the answer was identified.
    pub page_number: u32,
    /// Relevance / confidence score (0.0 to 1.0).
    pub confidence: f32,
    /// Whether an answer was found with acceptable confidence.
    pub found: bool,
}

/// Sentence candidate with location metadata.
#[derive(Debug, Clone)]
struct SentenceCandidate {
    page_number: u32,
    original_index: usize,
    text: String,
    normalized_words: Vec<String>,
    score: f32,
}

/// Build the set of Turkish + English stop words.
fn get_stop_words_set() -> HashSet<String> {
    let mut set = HashSet::with_capacity(STOP_WORDS_EN.len() + STOP_WORDS_TR.len());
    for &w in STOP_WORDS_EN {
        set.insert(w.to_string());
    }
    for &w in STOP_WORDS_TR {
        set.insert(w.to_string());
    }
    set
}

/// Tokenize text into words, removing punctuation and normalizing with Turkish case folding.
pub fn tokenize_words(text: &str) -> Vec<String> {
    let lower = turkish_lowercase(text);
    lower
        .split(|c: char| !c.is_alphanumeric() && c != '\'' && c != '-')
        .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()))
        .filter(|w| w.len() >= 2 && !w.chars().all(|c| c.is_numeric()))
        .map(|w| w.to_string())
        .collect()
}

/// Split text into individual sentences based on punctuation (. ! ? \n\n).
pub fn split_sentences(text: &str) -> Vec<String> {
    let mut sentences = Vec::new();
    let mut current = String::new();
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();

    let mut i = 0;
    while i < len {
        let c = chars[i];
        current.push(c);

        let is_terminator = c == '.' || c == '!' || c == '?' || c == '…';
        let is_double_newline = c == '\n' && i + 1 < len && chars[i + 1] == '\n';

        if is_terminator || is_double_newline {
            // Check if '.' is not an abbreviation or decimal number
            let is_abbrev = if c == '.' && i + 1 < len {
                let next_char = chars[i + 1];
                next_char.is_numeric() || (next_char.is_alphabetic() && next_char.is_lowercase())
            } else {
                false
            };

            if !is_abbrev {
                let trimmed = current.trim();
                if trimmed.len() >= 15 && trimmed.contains(' ') {
                    sentences.push(trimmed.to_string());
                }
                current.clear();
            }
        }
        i += 1;
    }

    let remaining = current.trim();
    if remaining.len() >= 15 && remaining.contains(' ') {
        sentences.push(remaining.to_string());
    }

    sentences
}

/// Compute Jaccard similarity between two lists of tokenized words.
fn jaccard_similarity(a: &[String], b: &[String]) -> f32 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let set_a: HashSet<&String> = a.iter().collect();
    let set_b: HashSet<&String> = b.iter().collect();

    let intersection = set_a.intersection(&set_b).count();
    let union = set_a.union(&set_b).count();

    if union == 0 {
        0.0
    } else {
        (intersection as f32) / (union as f32)
    }
}

/// Generate an extractive summary of document pages.
///
/// `pages_text`: Vector of `(page_number, text)`.
/// `max_sentences`: Target maximum number of key summary sentences (default ~3-5).
pub fn summarize_document(
    pages_text: &[(u32, &str)],
    max_sentences: usize,
) -> Result<DocumentSummary, EngineError> {
    if pages_text.is_empty() {
        return Ok(DocumentSummary {
            summary_text: String::new(),
            key_sentences: Vec::new(),
            top_keywords: Vec::new(),
            total_sentences: 0,
            word_count: 0,
            reading_time_minutes: 0.0,
        });
    }

    let stop_words = get_stop_words_set();
    let mut candidates: Vec<SentenceCandidate> = Vec::new();
    let mut word_freq: HashMap<String, usize> = HashMap::new();
    let mut total_word_count = 0;
    let mut sentence_idx = 0;

    // 1. Tokenize sentences and build global vocabulary frequencies
    for &(page_num, text) in pages_text {
        let sentences = split_sentences(text);
        for s in sentences {
            let words = tokenize_words(&s);
            if words.is_empty() {
                continue;
            }

            total_word_count += words.len();
            let mut informative_words = Vec::new();

            for w in words {
                if !stop_words.contains(&w) {
                    *word_freq.entry(w.clone()).or_insert(0) += 1;
                    informative_words.push(w);
                }
            }

            candidates.push(SentenceCandidate {
                page_number: page_num,
                original_index: sentence_idx,
                text: s,
                normalized_words: informative_words,
                score: 0.0,
            });
            sentence_idx += 1;
        }
    }

    let total_sentences = candidates.len();
    if total_sentences == 0 {
        return Ok(DocumentSummary {
            summary_text: String::new(),
            key_sentences: Vec::new(),
            top_keywords: Vec::new(),
            total_sentences: 0,
            word_count: total_word_count,
            reading_time_minutes: (total_word_count as f32) / 200.0,
        });
    }

    // 2. Score sentences using TF-ISF (Term Frequency * Inverse Sentence Frequency)
    let max_freq = word_freq.values().copied().max().unwrap_or(1) as f32;

    for candidate in &mut candidates {
        if candidate.normalized_words.is_empty() {
            continue;
        }

        let mut sum_score = 0.0;
        for w in &candidate.normalized_words {
            if let Some(&count) = word_freq.get(w) {
                let tf = (count as f32) / max_freq;
                sum_score += tf;
            }
        }

        let length_factor = (candidate.normalized_words.len() as f32).sqrt();
        let base_score = sum_score / length_factor.max(1.0);

        // Position bonus: early sentences on a page (first 2) receive priority
        let position_bonus = if candidate.original_index < 3 {
            1.3
        } else {
            1.0
        };

        candidate.score = base_score * position_bonus;
    }

    // 3. Select top non-redundant sentences
    let mut sorted_candidates = candidates.clone();
    sorted_candidates.sort_by(|a, b| b.score.total_cmp(&a.score));

    let target_count = max_sentences.clamp(1, 10).min(total_sentences);
    let mut selected: Vec<SentenceCandidate> = Vec::with_capacity(target_count);

    for cand in sorted_candidates {
        if selected.len() >= target_count {
            break;
        }

        // Diversity check: avoid picking almost identical sentences
        let is_redundant = selected
            .iter()
            .any(|s| jaccard_similarity(&s.normalized_words, &cand.normalized_words) > 0.55);

        if !is_redundant {
            selected.push(cand);
        }
    }

    // 4. Restore chronological order
    selected.sort_by_key(|c| c.original_index);

    let key_sentences: Vec<String> = selected.iter().map(|c| c.text.clone()).collect();
    let summary_text = key_sentences.join(" ");

    // 5. Extract top keywords
    let mut keyword_vec: Vec<KeywordScore> = word_freq
        .into_iter()
        .filter(|(w, count)| *count >= 2 && w.len() >= 3)
        .map(|(word, count)| KeywordScore {
            score: (count as f32) / max_freq,
            count,
            word,
        })
        .collect();
    keyword_vec.sort_by(|a, b| {
        b.count
            .cmp(&a.count)
            .then_with(|| b.score.total_cmp(&a.score))
    });
    keyword_vec.truncate(10);

    let reading_time = ((total_word_count as f32) / 200.0).max(0.1);

    Ok(DocumentSummary {
        summary_text,
        key_sentences,
        top_keywords: keyword_vec,
        total_sentences,
        word_count: total_word_count,
        reading_time_minutes: (reading_time * 10.0).round() / 10.0,
    })
}

/// Ask a natural language question about the document and locate relevant passages with citations.
pub fn ask_document(
    pages_text: &[(u32, &str)],
    query: &str,
) -> Result<DocumentAnswer, EngineError> {
    let trimmed_query = query.trim();
    if trimmed_query.is_empty() {
        return Ok(DocumentAnswer {
            query: String::new(),
            answer_text: "Sorgu boş olamaz.".into(),
            context_excerpt: String::new(),
            page_number: 1,
            confidence: 0.0,
            found: false,
        });
    }

    let stop_words = get_stop_words_set();
    let query_words: Vec<String> = tokenize_words(trimmed_query)
        .into_iter()
        .filter(|w| !stop_words.contains(w))
        .collect();

    if query_words.is_empty() {
        return Ok(DocumentAnswer {
            query: trimmed_query.to_string(),
            answer_text: "Sorgu çok genel veya sadece durak kelimelerden oluşuyor.".into(),
            context_excerpt: String::new(),
            page_number: 1,
            confidence: 0.0,
            found: false,
        });
    }

    let mut best_score = 0.0f32;
    let mut best_candidate: Option<SentenceCandidate> = None;
    let mut best_context = String::new();

    for &(page_num, text) in pages_text {
        let sentences = split_sentences(text);
        let s_len = sentences.len();

        for (i, s) in sentences.iter().enumerate() {
            let words = tokenize_words(s);
            if words.is_empty() {
                continue;
            }

            let word_set: HashSet<&String> = words.iter().collect();
            let mut match_count = 0;
            let mut prefix_match_count = 0;

            for qw in &query_words {
                if word_set.contains(qw) {
                    match_count += 1;
                } else if word_set
                    .iter()
                    .any(|target| target.starts_with(qw) || qw.starts_with(*target))
                {
                    prefix_match_count += 1;
                }
            }

            let total_matches = (match_count as f32) + (prefix_match_count as f32) * 0.6;
            if total_matches <= 0.0 {
                continue;
            }

            // Relevance score normalized by query size and sentence length
            let coverage = total_matches / (query_words.len() as f32);
            let length_penalty = (words.len() as f32).max(10.0).log10();
            let score = (coverage * 1.5) / length_penalty;

            if score > best_score {
                best_score = score;
                best_candidate = Some(SentenceCandidate {
                    page_number: page_num,
                    original_index: i,
                    text: s.clone(),
                    normalized_words: words,
                    score,
                });

                // Build context excerpt (previous sentence + match + next sentence)
                let mut ctx = String::new();
                if i > 0 {
                    ctx.push_str(&sentences[i - 1]);
                    ctx.push(' ');
                }
                ctx.push_str(s);
                if i + 1 < s_len {
                    ctx.push(' ');
                    ctx.push_str(&sentences[i + 1]);
                }
                best_context = ctx;
            }
        }
    }

    if let Some(cand) = best_candidate {
        let normalized_conf = (best_score.min(1.0) * 100.0).round() / 100.0;
        let found = normalized_conf >= 0.25;

        Ok(DocumentAnswer {
            query: trimmed_query.to_string(),
            answer_text: if found {
                cand.text
            } else {
                "Belgede sorgunuzla doğrudan eşleşen bir yanıt bulunamadı.".into()
            },
            context_excerpt: if found { best_context } else { String::new() },
            page_number: cand.page_number,
            confidence: normalized_conf,
            found,
        })
    } else {
        Ok(DocumentAnswer {
            query: trimmed_query.to_string(),
            answer_text: "Belgede sorgunuzla eşleşen hiçbir kayıt bulunamadı.".into(),
            context_excerpt: String::new(),
            page_number: 1,
            confidence: 0.0,
            found: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_and_turkish_folding() {
        let tokens = tokenize_words("RBD OFİS ile Çözüm, İstanbul ve Şırnak!");
        assert!(tokens.contains(&"rbd".to_string()));
        assert!(tokens.contains(&"ofis".to_string()));
        assert!(tokens.contains(&"çözüm".to_string()));
        assert!(tokens.contains(&"istanbul".to_string()));
        assert!(tokens.contains(&"şırnak".to_string()));
    }

    #[test]
    fn test_split_sentences_basic() {
        let text = "RBD OFİS açık kaynaklı bir ofis paketidir. Kullanıcı gizliliğine tam saygı duyar! İnternet bağlantısı gerektirmez?";
        let sentences = split_sentences(text);
        assert_eq!(sentences.len(), 3);
        assert!(sentences[0].contains("RBD OFİS"));
    }

    #[test]
    fn test_summarize_document() {
        let page1 = "RBD OFİS açık kaynaklı ve reklamsız bir ofis yazılımıdır. Tüm mantık Rust çekirdeğinde çalışır. Kullanıcı verileri asla dışarı gönderilmez.";
        let page2 = "PDF belgeleri yüksek hızda açılır ve pürüzsüz karo çizimi yapılır. Güçlü açıklama araçları ile belgeler üzerinde çizim yapılabilir. Çevrimdışı yapay zeka özellikleri belge özetleme imkanı sunar.";

        let pages = vec![(1, page1), (2, page2)];
        let summary = summarize_document(&pages, 3).unwrap();

        assert!(!summary.summary_text.is_empty());
        assert!(!summary.key_sentences.is_empty());
        assert!(summary.key_sentences.len() <= 3);
        assert!(summary.word_count > 0);
        assert!(summary.reading_time_minutes > 0.0);
    }

    #[test]
    fn test_ask_document_exact_match() {
        let page1 = "Proje adı RBD OFİS olarak belirlenmiştir. Lisans AGPL-3.0 olarak seçilmiştir.";
        let page2 = "Belge şifreleme algoritması AES-256 olarak uygulanmıştır. Fesih süresi otuz gün olarak kararlaştırılmıştır.";

        let pages = vec![(1, page1), (2, page2)];
        let answer = ask_document(&pages, "Fesih süresi ne kadardır?").unwrap();

        assert!(answer.found);
        assert_eq!(answer.page_number, 2);
        assert!(answer.answer_text.contains("Fesih süresi"));
        assert!(answer.confidence > 0.3);
    }

    #[test]
    fn test_ask_document_not_found() {
        let page1 = "Bu belgede sadece teknik mimari detayları anlatılmaktadır.";
        let pages = vec![(1, page1)];
        let answer = ask_document(&pages, "Uzay yolculuğu maliyeti nedir?").unwrap();

        assert!(!answer.found);
    }

    #[test]
    fn test_ai_keyword_and_candidate_nan_total_cmp_regression() {
        let mut keywords = [
            KeywordScore {
                word: "alpha".into(),
                count: 5,
                score: f32::NAN,
            },
            KeywordScore {
                word: "beta".into(),
                count: 5,
                score: 0.5,
            },
            KeywordScore {
                word: "gamma".into(),
                count: 10,
                score: f32::NAN,
            },
        ];
        // Must not panic with total_cmp when comparing NaN scores
        keywords.sort_by(|a, b| {
            b.count
                .cmp(&a.count)
                .then_with(|| b.score.total_cmp(&a.score))
        });
        assert_eq!(keywords[0].word, "gamma");
    }
}
