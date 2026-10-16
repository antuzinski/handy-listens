//! User dictionary file (`dictionary.txt` in the app data dir).
//!
//! The line format follows Chistovik: `how to write ← how it is heard (note)`.
//! One file feeds three steps:
//! - the Whisper initial prompt: `>` hint lines and the canonical terms;
//! - deterministic whole-word replacements after recognition, for any
//!   script (the fuzzy custom-word matcher only handles ASCII);
//! - the LLM cleanup prompt, which receives every entry line verbatim.
//!
//! A parenthesised note containing `only` or `только` is a condition. Such
//! variants are left to the LLM: a rule cannot tell "яичный салат" from
//! "яичный текст". When the condition quotes «words», only those variants are
//! conditional and the rest of the line is still replaced.

use log::{debug, warn};
use once_cell::sync::Lazy;
use regex::{Captures, Regex};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;
use strsim::levenshtein;
use tauri::AppHandle;

pub const DICTIONARY_FILE: &str = "dictionary.txt";
pub const CLEANUP_PROMPT_FILE: &str = "cleanup_prompt.txt";
const DEFAULT_DICTIONARY: &str = include_str!("default_dictionary.txt");
pub const DEFAULT_CLEANUP_PROMPT: &str = include_str!("default_cleanup_prompt.txt");
/// Id of the built-in post-processing prompt backed by cleanup_prompt.txt.
pub const CLEANUP_PROMPT_ID: &str = "dictionary_cleanup_ru";

/// Characters the dictionary may add to the Whisper prompt. Whisper keeps at
/// most 224 prompt tokens and Cyrillic costs roughly one token per two
/// characters, so the budget leaves room for the preset or custom prompt.
const WHISPER_PROMPT_CHAR_BUDGET: usize = 380;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct DictionaryError {
    pub line: u32,
    pub message: String,
}

#[derive(Debug, Clone)]
struct Replacement {
    pattern: Regex,
    canonical: String,
    wildcard: bool,
    variant_len: usize,
}

#[derive(Debug, Clone, Default)]
pub struct Dictionary {
    replacements: Vec<Replacement>,
    terms: Vec<String>,
    hints: Vec<String>,
    llm_lines: Vec<String>,
    entries: usize,
    conditional: usize,
    errors: Vec<DictionaryError>,
}

impl Dictionary {
    pub fn parse(text: &str) -> Self {
        let mut dictionary = Dictionary::default();
        for (index, raw) in text.lines().enumerate() {
            let line_no = (index + 1) as u32;
            let line = raw.trim().trim_start_matches('\u{feff}').trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some(hint) = line.strip_prefix('>') {
                let hint = hint.trim();
                if !hint.is_empty() {
                    dictionary.hints.push(hint.to_string());
                }
                continue;
            }
            if let Err(message) = dictionary.add_entry(line) {
                dictionary.errors.push(DictionaryError {
                    line: line_no,
                    message,
                });
            }
        }
        dictionary
            .replacements
            .sort_by(|a, b| b.variant_len.cmp(&a.variant_len));
        dictionary
    }

    fn add_entry(&mut self, line: &str) -> Result<(), String> {
        let (left, right) = match line.split_once('←').or_else(|| line.split_once("<-")) {
            Some((left, right)) => (left, Some(right)),
            None => (line, None),
        };
        let mut notes = Vec::new();
        let left = strip_notes(left, &mut notes)?;
        let right = right.map(|r| strip_notes(r, &mut notes)).transpose()?;

        let canonicals = split_list(&left);
        if canonicals.is_empty() {
            return Err("нет слова слева от «←»".to_string());
        }
        let variants = right.as_deref().map(split_list).unwrap_or_default();
        if right.is_some() && variants.is_empty() {
            return Err("нет вариантов справа от «←»".to_string());
        }

        let conditions: Vec<&String> = notes.iter().filter(|n| is_condition(n)).collect();
        let quoted: Vec<String> = conditions.iter().flat_map(|n| quoted_words(n)).collect();

        let mut compiled = Vec::new();
        for variant in &variants {
            let stem = variant.trim_end_matches('*');
            let wildcard = stem.len() != variant.len();
            if stem.is_empty() || stem.contains('*') {
                return Err(format!(
                    "«{variant}»: звёздочка может стоять только в конце слова"
                ));
            }
            let key = stem.to_lowercase();
            let is_conditional = !conditions.is_empty()
                && (quoted.is_empty() || quoted.iter().any(|q| q.trim_end_matches('*') == key));
            if is_conditional {
                continue;
            }
            let canonical = pick_canonical(&canonicals, stem, wildcard);
            if wildcard && !canonical.ends_with('*') {
                return Err(format!(
                    "«{variant}» со звёздочкой, а слева нет слова со звёздочкой"
                ));
            }
            compiled.push(Replacement {
                pattern: variant_pattern(stem, wildcard)?,
                canonical: canonical.to_string(),
                wildcard,
                variant_len: stem.chars().count(),
            });
        }

        self.entries += 1;
        if !conditions.is_empty() {
            self.conditional += 1;
        }
        self.replacements.extend(compiled);
        for canonical in &canonicals {
            let term = canonical.trim_end_matches('*').to_string();
            if !self
                .terms
                .iter()
                .any(|t| t.to_lowercase() == term.to_lowercase())
            {
                self.terms.push(term);
            }
        }
        self.llm_lines.push(line.to_string());
        Ok(())
    }

    /// Replace misheard variants with their canonical spelling.
    pub fn apply(&self, text: &str) -> String {
        let mut out = text.to_string();
        for replacement in &self.replacements {
            out = replacement
                .pattern
                .replace_all(&out, |caps: &Captures| replacement.render(caps))
                .into_owned();
        }
        out
    }

    /// Append `>` hints and canonical terms to the Whisper prompt within the
    /// character budget. Returns the prompt and how many terms it covers.
    pub fn extend_whisper_prompt(&self, base: String) -> (String, usize) {
        let mut parts = Vec::new();
        if !base.is_empty() {
            parts.push(base.clone());
        }
        let mut used = 0;
        for hint in &self.hints {
            let len = hint.chars().count();
            if used + len > WHISPER_PROMPT_CHAR_BUDGET {
                continue;
            }
            used += len;
            parts.push(hint.clone());
        }

        let mentioned = parts.join("\n").to_lowercase();
        let mut covered = 0;
        let mut extra = Vec::new();
        for term in &self.terms {
            if mentioned.contains(&term.to_lowercase()) {
                covered += 1;
                continue;
            }
            let len = term.chars().count() + 2;
            if used + len > WHISPER_PROMPT_CHAR_BUDGET {
                continue;
            }
            used += len;
            covered += 1;
            extra.push(term.clone());
        }
        if !extra.is_empty() {
            parts.push(extra.join(", "));
        }
        (parts.join("\n"), covered)
    }

    /// Entry lines for the LLM cleanup prompt (`${dictionary}`).
    pub fn llm_lines(&self) -> &[String] {
        &self.llm_lines
    }

    pub fn status(&self, path: &Path) -> DictionaryStatus {
        DictionaryStatus {
            path: path.to_string_lossy().into_owned(),
            entries: self.entries as u32,
            conditional: self.conditional as u32,
            replacements: self.replacements.len() as u32,
            terms_total: self.terms.len() as u32,
            terms_in_whisper_prompt: self.extend_whisper_prompt(String::new()).1 as u32,
            hints: self.hints.len() as u32,
            errors: self.errors.clone(),
        }
    }
}

impl Replacement {
    fn render(&self, caps: &Captures) -> String {
        let matched = caps.get(0).map(|m| m.as_str()).unwrap_or_default();
        let base = self.canonical.trim_end_matches('*');
        let suffix = if self.wildcard {
            caps.get(1).map(|m| m.as_str()).unwrap_or_default()
        } else {
            ""
        };
        let mut rendered = format!("{base}{suffix}");
        // Keep a sentence-initial capital for lowercase canonicals
        // ("Беклог" → "Бэклог"); canonicals with capitals are kept as written.
        let starts_upper = matched.chars().next().is_some_and(char::is_uppercase);
        if starts_upper && !base.chars().any(char::is_uppercase) {
            let mut chars = rendered.chars();
            if let Some(first) = chars.next() {
                rendered = first.to_uppercase().chain(chars).collect();
            }
        }
        rendered
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct DictionaryStatus {
    pub path: String,
    pub entries: u32,
    pub conditional: u32,
    pub replacements: u32,
    pub terms_total: u32,
    pub terms_in_whisper_prompt: u32,
    pub hints: u32,
    pub errors: Vec<DictionaryError>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum DictionaryFileKind {
    Dictionary,
    CleanupPrompt,
}

impl DictionaryFileKind {
    fn file_name(self) -> &'static str {
        match self {
            DictionaryFileKind::Dictionary => DICTIONARY_FILE,
            DictionaryFileKind::CleanupPrompt => CLEANUP_PROMPT_FILE,
        }
    }
}

/// Remove every `(...)` group, collecting its text.
fn strip_notes(part: &str, notes: &mut Vec<String>) -> Result<String, String> {
    let mut out = String::new();
    let mut depth = 0usize;
    let mut note = String::new();
    for c in part.chars() {
        match c {
            '(' => {
                if depth > 0 {
                    note.push(c);
                }
                depth += 1;
            }
            ')' if depth > 0 => {
                depth -= 1;
                if depth == 0 {
                    notes.push(note.trim().to_string());
                    note.clear();
                } else {
                    note.push(c);
                }
            }
            ')' => return Err("лишняя закрывающая скобка".to_string()),
            _ if depth > 0 => note.push(c),
            _ => out.push(c),
        }
    }
    if depth > 0 {
        return Err("скобка не закрыта".to_string());
    }
    Ok(out)
}

fn split_list(part: &str) -> Vec<String> {
    part.split(',')
        .map(|s| s.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|s| !s.is_empty())
        .collect()
}

fn is_condition(note: &str) -> bool {
    let lower = note.to_lowercase();
    lower
        .split(|c: char| !c.is_alphanumeric())
        .any(|w| w == "only" || w == "только")
}

fn quoted_words(note: &str) -> Vec<String> {
    static QUOTED: Lazy<Regex> = Lazy::new(|| Regex::new(r"«([^»]+)»").unwrap());
    QUOTED
        .captures_iter(note)
        .flat_map(|caps| {
            caps[1]
                .split(['/', ','])
                .map(|w| w.trim().to_lowercase())
                .filter(|w| !w.is_empty())
                .collect::<Vec<_>>()
        })
        .collect()
}

/// Closest canonical spelling for a variant: starred variants prefer starred
/// canonicals, then the smallest case-insensitive edit distance wins
/// ("иишные" → "ИИшные" rather than "ИИшный").
fn pick_canonical<'a>(canonicals: &'a [String], stem: &str, wildcard: bool) -> &'a str {
    let same_kind: Vec<&String> = canonicals
        .iter()
        .filter(|c| c.ends_with('*') == wildcard)
        .collect();
    let pool: Vec<&String> = if same_kind.is_empty() {
        canonicals.iter().collect()
    } else {
        same_kind
    };
    let stem = stem.to_lowercase();
    pool.into_iter()
        .min_by_key(|c| levenshtein(&stem, &c.trim_end_matches('*').to_lowercase()))
        .map(String::as_str)
        .unwrap_or(canonicals[0].as_str())
}

fn variant_pattern(stem: &str, wildcard: bool) -> Result<Regex, String> {
    let body = stem
        .split_whitespace()
        .map(regex::escape)
        .collect::<Vec<_>>()
        .join(r"\s+");
    let pattern = if wildcard {
        format!(r"(?i)\b{body}(\w*)")
    } else {
        format!(r"(?i)\b{body}\b")
    };
    Regex::new(&pattern).map_err(|e| format!("«{stem}»: {e}"))
}

struct Cache {
    path: Option<PathBuf>,
    modified: Option<SystemTime>,
    dictionary: Arc<Dictionary>,
}

static CACHE: Lazy<Mutex<Cache>> = Lazy::new(|| {
    Mutex::new(Cache {
        path: None,
        modified: None,
        dictionary: Arc::new(Dictionary::default()),
    })
});

/// The dictionary loaded by the last [`refresh`]. Empty until then.
pub fn current() -> Arc<Dictionary> {
    CACHE
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .dictionary
        .clone()
}

pub fn file_path(app: &AppHandle, kind: DictionaryFileKind) -> Result<PathBuf, String> {
    crate::portable::app_data_dir(app)
        .map(|dir| dir.join(kind.file_name()))
        .map_err(|e| format!("Failed to get app data directory: {e}"))
}

/// Re-read `dictionary.txt` when it changed (creating it on first use) and
/// return the current dictionary. A read failure keeps the previous one.
pub fn refresh(app: &AppHandle) -> Arc<Dictionary> {
    let path = match ensure_file(app, DictionaryFileKind::Dictionary) {
        Ok(path) => path,
        Err(e) => {
            warn!("Dictionary unavailable: {e}");
            return current();
        }
    };
    let modified = fs::metadata(&path).and_then(|m| m.modified()).ok();
    let mut cache = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    if cache.path.as_ref() == Some(&path) && modified.is_some() && cache.modified == modified {
        return cache.dictionary.clone();
    }
    match fs::read(&path) {
        Ok(bytes) => {
            let dictionary = Dictionary::parse(&String::from_utf8_lossy(&bytes));
            debug!(
                "Dictionary loaded: {} entries, {} replacements, {} errors",
                dictionary.entries,
                dictionary.replacements.len(),
                dictionary.errors.len()
            );
            cache.path = Some(path);
            cache.modified = modified;
            cache.dictionary = Arc::new(dictionary);
        }
        Err(e) => warn!("Failed to read {}: {e}", path.display()),
    }
    cache.dictionary.clone()
}

/// Create the file with its default content if it does not exist yet.
pub fn ensure_file(app: &AppHandle, kind: DictionaryFileKind) -> Result<PathBuf, String> {
    let path = file_path(app, kind)?;
    if !path.exists() {
        write_default(app, kind, &path)?;
    }
    Ok(path)
}

/// Back up the current file as `<name>.bak` and restore the default.
pub fn reset_file(app: &AppHandle, kind: DictionaryFileKind) -> Result<PathBuf, String> {
    let path = file_path(app, kind)?;
    if path.exists() {
        let backup = path.with_extension("txt.bak");
        fs::copy(&path, &backup)
            .map_err(|e| format!("Failed to back up {}: {e}", path.display()))?;
    }
    write_default(app, kind, &path)?;
    Ok(path)
}

fn write_default(app: &AppHandle, kind: DictionaryFileKind, path: &Path) -> Result<(), String> {
    let content = match kind {
        DictionaryFileKind::Dictionary => {
            let settings = crate::settings::get_settings(app);
            default_dictionary_with(&settings.custom_words)
        }
        DictionaryFileKind::CleanupPrompt => DEFAULT_CLEANUP_PROMPT.to_string(),
    };
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create {}: {e}", parent.display()))?;
    }
    fs::write(path, content).map_err(|e| format!("Failed to write {}: {e}", path.display()))
}

/// Default dictionary plus the user's existing custom words, so switching to
/// the file loses nothing.
fn default_dictionary_with(custom_words: &[String]) -> String {
    let defaults = Dictionary::parse(DEFAULT_DICTIONARY);
    let missing: Vec<&String> = custom_words
        .iter()
        .filter(|w| !w.trim().is_empty())
        .filter(|w| {
            let w = w.trim().to_lowercase();
            !defaults.terms.iter().any(|t| t.to_lowercase() == w)
        })
        .collect();
    let mut content = DEFAULT_DICTIONARY.to_string();
    if !missing.is_empty() {
        if !content.ends_with('\n') {
            content.push('\n');
        }
        content.push_str("\n# Перенесено из старого словаря\n");
        for word in missing {
            content.push_str(word.trim());
            content.push('\n');
        }
    }
    content
}

/// Prompt file content without `#` comment lines.
pub fn strip_comments(text: &str) -> String {
    text.lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

/// Final text of a post-processing prompt. The built-in cleanup prompt is
/// read from cleanup_prompt.txt (next to the dictionary loaded by the last
/// [`refresh`]) on every call, so edits apply without a restart. In any
/// prompt, `${dictionary}` becomes the dictionary entry lines.
pub fn resolve_llm_prompt(prompt_id: &str, stored_prompt: String) -> String {
    let template = if prompt_id == CLEANUP_PROMPT_ID {
        let path = CACHE
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .path
            .as_ref()
            .and_then(|p| p.parent().map(|dir| dir.join(CLEANUP_PROMPT_FILE)));
        let text = path
            .and_then(|p| fs::read(&p).ok())
            .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
            .unwrap_or_else(|| DEFAULT_CLEANUP_PROMPT.to_string());
        strip_comments(&text)
    } else {
        stored_prompt
    };
    if !template.contains("${dictionary}") {
        return template;
    }
    let dictionary = current();
    let lines = if dictionary.llm_lines().is_empty() {
        "(empty)".to_string()
    } else {
        dictionary.llm_lines().join("\n")
    };
    template.replace("${dictionary}", &lines)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_dictionary() -> Dictionary {
        Dictionary::parse(DEFAULT_DICTIONARY)
    }

    #[test]
    fn default_file_parses_without_errors() {
        let d = default_dictionary();
        assert!(d.errors.is_empty(), "{:?}", d.errors);
        assert!(d.entries > 30);
        assert_eq!(d.hints.len(), 1);
    }

    #[test]
    fn replaces_cyrillic_words_and_keeps_sentence_capital() {
        let d = default_dictionary();
        assert_eq!(d.apply("это иишный текст"), "это ИИшный текст");
        assert_eq!(d.apply("Иишные штуки"), "ИИшные штуки");
        assert_eq!(d.apply("закинь в беклог"), "закинь в бэклог");
        assert_eq!(d.apply("Беклог пустой"), "Бэклог пустой");
        assert_eq!(d.apply("в беклоге"), "в бэклоге");
        assert_eq!(d.apply("запустим абтест"), "запустим A/B-тест");
        assert_eq!(d.apply("два аб теста"), "два A/B-теста");
        assert_eq!(d.apply("нового чат бота"), "нового чат-бота");
    }

    #[test]
    fn real_dictation_2026_10_16() {
        // Whisper output from the first practical test. «яичные» and
        // «кодексом» are conditional and stay for the LLM cleanup.
        let d = default_dictionary();
        let text = "Не писать сразу в джиру. Короче, всякие яичные штуки. \
                    Потом добавляю в общий конфлюенс и с Клодом или с кодексом.";
        assert_eq!(
            d.apply(text),
            "Не писать сразу в джиру. Короче, всякие яичные штуки. \
             Потом добавляю в общий Confluence и с Claude или с кодексом."
        );
    }

    #[test]
    fn conditional_variants_are_left_for_the_llm() {
        let d = default_dictionary();
        assert_eq!(d.apply("рецепт яичного салата"), "рецепт яичного салата");
        assert_eq!(d.apply("какие-то яичные штуки"), "какие-то яичные штуки");
        assert_eq!(
            d.apply("наведи курсор на кнопку"),
            "наведи курсор на кнопку"
        );
        assert_eq!(
            d.apply("нужен пиар перед запуском"),
            "нужен пиар перед запуском"
        );
        assert_eq!(d.apply("с кодексом"), "с кодексом");
        assert_eq!(d.apply("задача в Jira"), "задача в Jira");
    }

    #[test]
    fn longer_variants_win_and_names_are_latin() {
        let d = default_dictionary();
        assert_eq!(
            d.apply("Клод Код написал тесты"),
            "Claude Code написал тесты"
        );
        assert_eq!(d.apply("спроси у Клода"), "спроси у Claude");
        assert_eq!(
            d.apply("гляну пулреквест на гитхабе"),
            "гляну пул-реквест на GitHub"
        );
        assert_eq!(d.apply("переписать промт"), "переписать промпт");
        assert_eq!(d.apply("макет из фигмы"), "макет из Figma");
    }

    #[test]
    fn words_inside_other_words_are_not_replaced() {
        let d = Dictionary::parse("API ← апи");
        assert_eq!(d.apply("апи и капитал"), "API и капитал");
    }

    #[test]
    fn quoted_condition_only_affects_quoted_words() {
        let d = Dictionary::parse(
            "ИИшный ← иишный, яичный (replace «яичный» only when not about food)",
        );
        assert_eq!(d.apply("иишный и яичный"), "ИИшный и яичный");
        assert_eq!(d.conditional, 1);
    }

    #[test]
    fn notes_without_only_do_not_block_replacement() {
        let d = Dictionary::parse("Claude ← Клод (always written Claude, in any grammatical case)");
        assert_eq!(d.apply("Клод ответил"), "Claude ответил");
        assert_eq!(d.conditional, 0);
    }

    #[test]
    fn reports_errors_with_line_numbers() {
        let d = Dictionary::parse(
            "# comment\nбэклог ← беклог*\n ← пусто\nслово ← (только тут\nок ← окей",
        );
        let lines: Vec<u32> = d.errors.iter().map(|e| e.line).collect();
        assert_eq!(lines, vec![2, 3, 4]);
        assert_eq!(d.apply("окей"), "ок");
    }

    #[test]
    fn terms_and_hints_extend_whisper_prompt_once() {
        let d = Dictionary::parse("> Обсуждаем бэклог.\nбэклог* ← беклог*\nAlina\nClaude ← Клод");
        let (prompt, covered) = d.extend_whisper_prompt("Base prompt".to_string());
        assert_eq!(prompt, "Base prompt\nОбсуждаем бэклог.\nAlina, Claude");
        assert_eq!(covered, 3);
    }

    #[test]
    fn whisper_additions_respect_the_budget() {
        let long_terms: String = (0..200).map(|i| format!("термин{i}\n")).collect();
        let d = Dictionary::parse(&long_terms);
        let (prompt, covered) = d.extend_whisper_prompt(String::new());
        assert!(prompt.chars().count() <= WHISPER_PROMPT_CHAR_BUDGET);
        assert!(covered < 200);
    }

    #[test]
    fn migrated_custom_words_are_appended_once() {
        let content = default_dictionary_with(&["вайб-кодинг".to_string(), "Алина".to_string()]);
        assert!(content.ends_with("# Перенесено из старого словаря\nАлина\n"));
        assert!(Dictionary::parse(&content).errors.is_empty());
        assert_eq!(default_dictionary_with(&[]), DEFAULT_DICTIONARY);
    }

    #[test]
    fn default_cleanup_prompt_has_both_placeholders() {
        let prompt = strip_comments(DEFAULT_CLEANUP_PROMPT);
        assert!(prompt.contains("${dictionary}"));
        assert!(prompt.contains("<dictation>\n${output}\n</dictation>"));
        assert!(!prompt.contains("# Промпт"));
    }

    #[test]
    fn custom_prompts_keep_their_text_and_get_the_dictionary() {
        assert_eq!(
            resolve_llm_prompt("custom", "Fix ${output}".into()),
            "Fix ${output}"
        );
        let resolved = resolve_llm_prompt("custom", "${dictionary}\n${output}".into());
        assert!(!resolved.contains("${dictionary}"));
        assert!(resolved.ends_with("\n${output}"));
    }

    #[test]
    fn strip_comments_removes_header_lines() {
        assert_eq!(
            strip_comments("# a\nline\n  # b\n${output}\n"),
            "line\n${output}"
        );
    }
}
