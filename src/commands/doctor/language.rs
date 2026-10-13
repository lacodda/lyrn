//! `language`: what ships is written in English; another language lives
//! where its translations go.
//!
//! Every product of the line speaks English first - its docs, its comments,
//! its messages - and a translation sits where translations go: `docs/ru/`,
//! `locales/ru.json`, `README.ru.md`. What slips past is prose written in the
//! language one thinks in, and the word typed half in one keyboard layout and
//! half in the other, which looks right and is not: a `c` that is Cyrillic.
//!
//! Text in another script is not by itself a finding. A product that handles
//! Russian has Russian in its tests and in the names its examples show, and
//! that is data. Prose is a run of words - four or more in a row - and in a
//! test's material only its comments are read.

use super::{Finding, Outcome, Repo, is_test_material, plural, some_of};

/// Words in a row that make a sentence rather than a name or a value.
const PROSE_WORDS: usize = 4;

/// Characters in a row that make a sentence, in a script written without
/// spaces.
const PROSE_CHARACTERS: usize = 6;

/// A script other than Latin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Script {
    Cyrillic,
    Greek,
    Armenian,
    Hebrew,
    Arabic,
    Devanagari,
    Thai,
    Georgian,
    Hangul,
    Kana,
    Han,
}

impl Script {
    fn of(c: char) -> Option<Script> {
        Some(match c as u32 {
            0x0400..=0x052F | 0x1C80..=0x1C8F | 0x2DE0..=0x2DFF | 0xA640..=0xA69F => Script::Cyrillic,
            0x0370..=0x03FF | 0x1F00..=0x1FFF => Script::Greek,
            0x0530..=0x058F => Script::Armenian,
            0x0590..=0x05FF => Script::Hebrew,
            0x0600..=0x06FF | 0x0750..=0x077F => Script::Arabic,
            0x0900..=0x097F => Script::Devanagari,
            0x0E00..=0x0E7F => Script::Thai,
            0x10A0..=0x10FF => Script::Georgian,
            0x1100..=0x11FF | 0xAC00..=0xD7AF => Script::Hangul,
            0x3040..=0x30FF => Script::Kana,
            0x4E00..=0x9FFF | 0x3400..=0x4DBF => Script::Han,
            _ => return None,
        })
    }

    fn name(self) -> &'static str {
        match self {
            Script::Cyrillic => "Cyrillic",
            Script::Greek => "Greek",
            Script::Armenian => "Armenian",
            Script::Hebrew => "Hebrew",
            Script::Arabic => "Arabic",
            Script::Devanagari => "Devanagari",
            Script::Thai => "Thai",
            Script::Georgian => "Georgian",
            Script::Hangul => "Korean",
            Script::Kana => "Japanese",
            Script::Han => "Chinese or Japanese",
        }
    }

    /// Whether the script puts spaces between its words.
    fn spaced(self) -> bool {
        !matches!(self, Script::Thai | Script::Kana | Script::Han)
    }

    /// The languages written in it, by the codes a translation's path names
    /// them with. Only codes that are not also common words of a path: `be`
    /// and `os` are languages, and also `should-be` and `src/os/`.
    fn languages(self) -> &'static [&'static str] {
        match self {
            Script::Cyrillic => &["ru", "uk", "bg", "sr", "mk", "kk", "ky", "mn", "tg", "rus", "ukr"],
            Script::Greek => &["el"],
            Script::Armenian => &["hy"],
            Script::Hebrew => &["he", "iw", "yi"],
            Script::Arabic => &["ar", "fa", "ur", "ps"],
            Script::Devanagari => &["hi", "mr", "ne"],
            Script::Thai => &["th"],
            Script::Georgian => &["ka"],
            Script::Hangul => &["ko"],
            Script::Kana | Script::Han => &["zh", "ja"],
        }
    }
}

/// Directories that hold translations, whatever their language.
const TRANSLATIONS: &[&str] = &["locales", "locale", "i18n", "l10n", "translations", "lang", "langs"];

/// Whether `path` is where a translation into a language written in `script`
/// lives: a directory of translations, or a part of the path named for the
/// language - `docs/ru/`, `ru.json`, `README.ru.md`, `pack-en-from-ru`.
fn is_translation(path: &str, script: Script) -> bool {
    path.split('/').any(|part| TRANSLATIONS.contains(&part.to_ascii_lowercase().as_str()))
        || path
            .split(['/', '.', '-', '_'])
            .any(|token| script.languages().contains(&token.to_ascii_lowercase().as_str()))
}

/// The prose on one line: the script of the longest run of words, when a run
/// is long enough to be a sentence.
fn prose(line: &str) -> Option<Script> {
    // A run is broken by what a sentence does not contain: quotes, brackets
    // and the marks of code. A list of words in quotes is data, one word each.
    const BREAKS: &[char] = &['"', '\'', '`', '[', ']', '{', '}', '<', '>', '=', '|', '/', '\\', '(', ')', '*'];
    let mut run = Run::default();
    let mut unspaced: Option<(Script, usize)> = None;
    let mut word = String::new();
    for c in line.chars() {
        // Scripts written without spaces are measured in characters.
        match Script::of(c).filter(|s| !s.spaced()) {
            Some(script) => {
                // Japanese runs kana and kanji together in one sentence.
                let japanese = |s: Script| matches!(s, Script::Kana | Script::Han);
                let (script, count) = match unspaced {
                    Some((s, n)) if s == script => (s, n + 1),
                    Some((s, n)) if japanese(s) && japanese(script) => (Script::Kana, n + 1),
                    _ => (script, 1),
                };
                if count >= PROSE_CHARACTERS {
                    return Some(script);
                }
                unspaced = Some((script, count));
            }
            None => unspaced = None,
        }
        if c.is_alphanumeric() || c == '\u{301}' {
            word.push(c);
            continue;
        }
        if let Some(script) = run.end_word(&mut word) {
            return Some(script);
        }
        if BREAKS.contains(&c) {
            run = Run::default();
        }
    }
    run.end_word(&mut word)
}

/// The words in a row in one script, so far on a line.
#[derive(Default)]
struct Run {
    script: Option<Script>,
    words: usize,
    /// Latin words since the last word of the run. One is a term inside a
    /// sentence - "API" - and two are an English sentence the run has left.
    latin: usize,
}

impl Run {
    /// Count a finished word in, and say the run's script once it is long
    /// enough to be prose.
    fn end_word(&mut self, word: &mut String) -> Option<Script> {
        if word.is_empty() {
            return None;
        }
        let script = word.chars().find_map(Script::of).filter(|s| s.spaced());
        let latin = word.chars().any(|c| c.is_ascii_alphabetic());
        word.clear();
        let Some(script) = script else {
            if latin {
                self.latin += 1;
                if self.latin >= 2 {
                    *self = Run::default();
                }
            }
            return None;
        };
        self.words = if self.script == Some(script) { self.words + 1 } else { 1 };
        self.script = Some(script);
        self.latin = 0;
        (self.words >= PROSE_WORDS).then_some(script)
    }
}

/// The words on one line typed partly in each layout: Latin and Cyrillic
/// letters in one word, which reads as a word and is not one.
fn mixed_words(line: &str) -> Vec<String> {
    line.split(|c: char| !c.is_alphanumeric())
        .filter(|word| {
            word.chars().count() >= 2 && word.chars().any(|c| c.is_ascii_alphabetic()) && word.chars().any(|c| Script::of(c) == Some(Script::Cyrillic))
        })
        .map(str::to_string)
        .collect()
}

/// How a file writes its comments.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Syntax {
    /// `//` and `/* */`.
    Slashes,
    /// `//` and `/* */`, where `'` opens a char or names a lifetime rather
    /// than starting a string.
    Rust,
    /// `#` to the end of the line.
    Hash,
    /// `--` to the end of the line.
    Dashes,
    /// `<!-- -->`.
    Markup,
    /// No comments: what is in it is data.
    None,
}

fn syntax_of(path: &str) -> Syntax {
    let name = path.rsplit('/').next().unwrap_or(path);
    let extension = name.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default();
    match extension.as_str() {
        "rs" => Syntax::Rust,
        "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" | "mts" | "cts" | "c" | "h" | "cc" | "cpp" | "hpp" | "go" | "java" | "kt" | "swift" | "cs" | "css"
        | "scss" | "less" | "dart" | "zig" => Syntax::Slashes,
        "py" | "sh" | "bash" | "zsh" | "ps1" | "toml" | "yml" | "yaml" | "rb" | "r" | "pl" | "nix" => Syntax::Hash,
        "sql" | "lua" | "hs" => Syntax::Dashes,
        "md" | "mdx" | "html" | "htm" | "xml" | "svg" | "vue" | "svelte" | "astro" => Syntax::Markup,
        _ if matches!(name, "Dockerfile" | "Makefile" | ".gitignore" | ".gitattributes" | ".editorconfig") => Syntax::Hash,
        _ => Syntax::None,
    }
}

/// The comments of a text, line by line: what a reader of the code reads
/// as prose, with the strings and the code taken out.
fn comments(text: &str, syntax: Syntax) -> Vec<(usize, String)> {
    let slashes = matches!(syntax, Syntax::Slashes | Syntax::Rust);
    let mut out = Vec::new();
    let mut in_block = false;
    for (index, line) in text.lines().enumerate() {
        let mut comment = String::new();
        let chars: Vec<char> = line.chars().collect();
        let at = |i: usize, s: &[char]| chars.get(i..).is_some_and(|rest| rest.starts_with(s));
        let mut i = 0;
        let mut quote: Option<char> = None;
        while i < chars.len() {
            let c = chars[i];
            if in_block {
                let end: &[char] = if slashes { &['*', '/'] } else { &['-', '-', '>'] };
                if at(i, end) {
                    in_block = false;
                    i += end.len();
                } else {
                    comment.push(c);
                    i += 1;
                }
                continue;
            }
            if let Some(q) = quote {
                if c == '\\' {
                    i += 2;
                    continue;
                }
                if c == q {
                    quote = None;
                }
                i += 1;
                continue;
            }
            let rest_is_comment = match syntax {
                Syntax::Slashes | Syntax::Rust => at(i, &['/', '/']),
                Syntax::Hash => c == '#',
                Syntax::Dashes => at(i, &['-', '-']),
                Syntax::Markup | Syntax::None => false,
            };
            if rest_is_comment {
                comment.extend(&chars[i..]);
                break;
            }
            if (slashes && at(i, &['/', '*'])) || (syntax == Syntax::Markup && at(i, &['<', '!', '-', '-'])) {
                in_block = true;
                i += if slashes { 2 } else { 4 };
                continue;
            }
            if syntax == Syntax::Rust && c == '\'' {
                // `'x'` and `'\n'` are chars; anything else is a lifetime.
                i += match (chars.get(i + 1), chars.get(i + 2), chars.get(i + 3)) {
                    (Some('\\'), _, Some('\'')) => 4,
                    (Some(_), Some('\''), _) => 3,
                    _ => 1,
                };
                continue;
            }
            if c == '"' || c == '`' || c == '\'' {
                quote = Some(c);
            }
            i += 1;
        }
        if !comment.trim().is_empty() {
            out.push((index + 1, comment));
        }
    }
    out
}

/// Files that hold values rather than sentences.
fn is_data(path: &str) -> bool {
    const DATA: &[&str] = &["json", "jsonl", "ndjson", "geojson", "csv", "tsv", "po", "ftl", "xliff", "xlf"];
    let extension = path.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default();
    DATA.contains(&extension.as_str())
}

/// The prose of a Markdown page, line by line: what it says in its own voice.
/// Code - fenced, indented by a fence or inline - is shown, not said, and so
/// is a word or a phrase set in emphasis: `*накипь*` in a sentence about
/// search is an example, the way a quotation is.
fn markdown_prose(text: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let mut fence: Option<&str> = None;
    for (index, line) in text.lines().enumerate() {
        let trimmed = line.trim_start();
        let marker = ["```", "~~~"].into_iter().find(|m| trimmed.starts_with(m));
        match (fence, marker) {
            (None, Some(m)) => {
                fence = Some(m);
                continue;
            }
            (Some(open), Some(m)) if open == m => {
                fence = None;
                continue;
            }
            (Some(_), _) => continue,
            (None, None) => {}
        }
        out.push((index + 1, strip_shown(line)));
    }
    out
}

/// A line of Markdown with its inline code and its emphasis taken out.
fn strip_shown(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        let closing = match c {
            '`' => '`',
            '*' => '*',
            '_' if out.chars().last().is_none_or(|p| !p.is_alphanumeric()) => '_',
            _ => {
                out.push(c);
                continue;
            }
        };
        // `**bold**` closes on the second star of its pair; a lone star that
        // never closes was not emphasis, and nothing is lost.
        while chars.peek() == Some(&closing) {
            chars.next();
        }
        let mut inside = String::new();
        let mut closed = false;
        while let Some(d) = chars.next() {
            // An underscore inside a word - snake_case - closes nothing.
            let inside_word = closing == '_' && chars.peek().is_some_and(|n| n.is_alphanumeric());
            if d == closing && !inside_word {
                while chars.peek() == Some(&closing) {
                    chars.next();
                }
                closed = true;
                break;
            }
            inside.push(d);
        }
        out.push(' ');
        if !closed {
            out.push_str(&inside);
        }
    }
    out
}

/// One place another language was found.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Hit {
    file: String,
    line: usize,
}

impl Hit {
    fn at(&self) -> String {
        format!("{}:{}", self.file, self.line)
    }
}

/// What one file says in another language: its prose by script, and the
/// words typed in two layouts.
fn examine(path: &str, text: &str, prose_hits: &mut Vec<(Script, Hit)>, mixed: &mut Vec<(String, Hit)>) {
    if is_data(path) {
        // Values, not sentences: a phrasebook's entries, a seed's rows.
        return;
    }
    let syntax = syntax_of(path);
    let code = matches!(syntax, Syntax::Slashes | Syntax::Rust | Syntax::Hash | Syntax::Dashes);
    let test_file = is_test_material(path);
    let comments = || -> Vec<(usize, String)> { comments(text, syntax).into_iter().map(|(n, c)| (n, strip_cited(&c))).collect() };

    // What the file says in its own voice. In code that is its comments: a
    // string is a value the program holds - a fixture, a translation kept
    // to recognise old data, an example in a prompt - and a quotation inside
    // a comment is a citation of one.
    let said: Vec<(usize, String)> = if code || test_file {
        comments()
    } else if matches!(path.rsplit('.').next(), Some("md" | "mdx")) {
        markdown_prose(text)
    } else {
        text.lines().enumerate().map(|(i, l)| (i + 1, l.to_string())).collect()
    };

    // What was typed: every line, where an identifier with a Cyrillic letter
    // is as wrong as a word - except a test's, whose strings may hold one on
    // purpose. Rust keeps a file's tests at its foot, from `#[cfg(test)]` or
    // the first `#[test]`.
    let tests_from = if path.ends_with(".rs") {
        text.lines().position(|l| matches!(l.trim(), "#[cfg(test)]" | "#[test]")).map(|i| i + 1)
    } else {
        None
    };
    let typed: Vec<(usize, String)> = if test_file {
        comments()
    } else {
        let mut lines: Vec<(usize, String)> = text
            .lines()
            .enumerate()
            .take_while(|(i, _)| tests_from.is_none_or(|from| i + 1 < from))
            .map(|(i, l)| (i + 1, l.to_string()))
            .collect();
        if let Some(from) = tests_from {
            lines.extend(comments().into_iter().filter(|(line, _)| *line >= from));
        }
        lines
    };

    let hit = |line: usize| Hit { file: path.to_string(), line };
    for (line, content) in said.iter().filter(|(_, content)| !content.is_ascii()) {
        if let Some(script) = prose(content)
            && !is_translation(path, script)
        {
            prose_hits.push((script, hit(*line)));
        }
    }
    if !is_translation(path, Script::Cyrillic) {
        for (line, content) in typed.iter().filter(|(_, content)| !content.is_ascii()) {
            for word in mixed_words(content) {
                mixed.push((word, hit(*line)));
            }
        }
    }
}

/// A comment with what it quotes taken out: `code`, "a string", «a phrase».
/// A quotation left open runs to the end of the line - it closes on the next.
fn strip_cited(comment: &str) -> String {
    let mut out = String::with_capacity(comment.len());
    let mut closing: Option<char> = None;
    for c in comment.chars() {
        match closing {
            Some(end) if c == end => closing = None,
            Some(_) => {}
            None => match c {
                '`' => closing = Some('`'),
                '"' => closing = Some('"'),
                '«' => closing = Some('»'),
                '“' => closing = Some('”'),
                _ => out.push(c),
            },
        }
    }
    out
}

pub fn check(repo: &Repo) -> Outcome {
    let mut prose_hits: Vec<(Script, Hit)> = Vec::new();
    let mut mixed: Vec<(String, Hit)> = Vec::new();
    // Another script is never ASCII: most files are passed over at a glance.
    for (path, text) in repo.texts().iter().filter(|(_, text)| !text.is_ascii()) {
        examine(path, text, &mut prose_hits, &mut mixed);
    }

    let mut findings = Vec::new();
    let mut scripts: Vec<Script> = prose_hits.iter().map(|(s, _)| *s).collect();
    scripts.sort();
    scripts.dedup();
    for script in scripts {
        let hits: Vec<&Hit> = prose_hits.iter().filter(|(s, _)| *s == script).map(|(_, h)| h).collect();
        let mut files: Vec<&str> = hits.iter().map(|h| h.file.as_str()).collect();
        files.dedup();
        let places: Vec<String> = hits.iter().map(|h| h.at()).collect();
        findings.push(
            Finding::fail(format!(
                "{} prose in {}: {}",
                script.name(),
                plural(files.len(), "file", "files"),
                some_of(&places, 3)
            ))
            .fix("write it in English; a translation lives where its language is named - docs/ru/, locales/ru.json, README.ru.md"),
        );
    }
    if !mixed.is_empty() {
        let shown: Vec<String> = mixed.iter().map(|(word, hit)| format!("`{word}` ({})", hit.at())).collect();
        findings.push(
            Finding::fail(format!(
                "{} typed partly in a Cyrillic layout: {}",
                plural(mixed.len(), "word", "words"),
                some_of(&shown, 3)
            ))
            .fix("retype each in one layout - the letters look the same and are not, so search and spelling miss them"),
        );
    }

    if let Some(git) = repo.git.as_ref().filter(|g| g.has_commits()) {
        let log = git.run(&["log", "--all", "--format=%h%x1f%s"]).unwrap_or_default();
        let foreign: Vec<String> = log
            .lines()
            .filter_map(|line| line.split_once('\x1f'))
            .filter(|(_, subject)| foreign_subject(subject))
            .map(|(hash, _)| hash.to_string())
            .collect();
        if !foreign.is_empty() {
            findings.push(
                Finding::warn(format!(
                    "{} not written in English: {}",
                    plural(foreign.len(), "commit's subject is", "commits' subjects are"),
                    some_of(&foreign, 4)
                ))
                .fix("the changelog is made of them; `git filter-repo --message-callback` rewrites them - a rewrite, so only before the history is shared"),
            );
        }
    }

    Outcome::from_findings(findings, || {
        format!(
            "{} in English, translations in their places",
            plural(repo.texts().len(), "text file", "text files")
        )
    })
}

/// A subject that is not in English: two words in another script, or a word
/// typed in two layouts. One word is a name.
fn foreign_subject(subject: &str) -> bool {
    let words = subject
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.chars().any(|c| Script::of(c).is_some()))
        .count();
    words >= 2 || !mixed_words(subject).is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sentence_is_prose_and_a_name_is_not() {
        assert_eq!(prose("// Это длинное объяснение на русском языке"), Some(Script::Cyrillic));
        assert_eq!(prose("A Russian \"В работе\" or a custom \"Doing\" works the same"), None);
        assert_eq!(prose("rows(\"Гражданская оборона\\tSigur Rós\\n\")"), None);
        assert_eq!(prose(r#"for bad in ["и", "в", "на", "с", "по"] {"#), None, "a list of words is data");
        // A Latin word inside a sentence does not break it.
        assert_eq!(prose("используем API для этого случая"), Some(Script::Cyrillic));
        assert_eq!(prose("これは日本語の文章です"), Some(Script::Kana));
        assert_eq!(prose("a 5 μs wait, λ and Δ"), None);
    }

    #[test]
    fn two_english_words_end_a_run_and_one_does_not() {
        assert_eq!(prose("один два API три четыре"), Some(Script::Cyrillic));
        assert_eq!(prose("один два three four три четыре"), None);
    }

    #[test]
    fn a_page_says_in_its_own_voice_what_is_not_code_or_emphasis() {
        let page = "Intro.\n\n```console\n$ sefy gen --lang ru\nулей суббота жетон клумба май\n```\n\n*Накипь в чайнике: уксус или лимонная кислота* point the same way, and `ещё одна строка кода тут`.\n\nЭто настоящий абзац на русском языке.\n";
        let said: Vec<usize> = markdown_prose(page).iter().filter(|(_, l)| prose(l).is_some()).map(|(n, _)| *n).collect();
        assert_eq!(said, [10]);
        assert_eq!(strip_shown("a **bold** and _snake_case_ x"), "a   and   x");
    }

    #[test]
    fn in_code_the_comments_speak_and_the_strings_hold_values() {
        let mut prose_hits = Vec::new();
        let mut mixed = Vec::new();
        let source = "const HINT: &str = \"→ негатив и генератор идей\";\n// stage with `### Патч v0.24.1 — в тот же день` and the\n// \"в колоду уходит именно сбой\": the shape came, the word did not\nlet сount = 1;\n// Этот модуль делает очень важную работу\n";
        examine("src/a.rs", source, &mut prose_hits, &mut mixed);
        let lines: Vec<usize> = prose_hits.iter().map(|(_, h)| h.line).collect();
        assert_eq!(lines, [5], "only the comment in Russian is prose");
        assert_eq!(mixed.iter().map(|(w, h)| (w.as_str(), h.line)).collect::<Vec<_>>(), [("сount", 4)]);
        assert_eq!(strip_cited("a `code` and \"str\" and «фраза» left open `x"), "a  and  and  left open ");
    }

    #[test]
    fn values_are_not_sentences() {
        let mut prose_hits = Vec::new();
        let mut mixed = Vec::new();
        examine("demo/learner.json", "{\"gloss\": \"У меня есть машина и дом.\"}\n", &mut prose_hits, &mut mixed);
        assert!(prose_hits.is_empty());
        examine("docs/guide.txt", "У меня есть машина и дом.\n", &mut prose_hits, &mut mixed);
        assert_eq!(prose_hits.len(), 1);
    }

    #[test]
    fn a_word_typed_in_two_layouts_is_found() {
        assert_eq!(mixed_words(r#"- Improved "export" сommand"#), ["сommand"]);
        assert!(mixed_words("API-ключ и IPv4-адрес").is_empty(), "two words joined by a hyphen are two words");
        assert!(mixed_words("`N` arrives as `т`").is_empty());
    }

    #[test]
    fn a_translation_is_where_its_language_is_named() {
        assert!(is_translation("docs/ru/README.md", Script::Cyrillic));
        assert!(is_translation("README.ru.md", Script::Cyrillic));
        assert!(is_translation("web/src/i18n/locales/ru.json", Script::Cyrillic));
        assert!(is_translation("packs/en-from-ru/lesson.md", Script::Cyrillic));
        assert!(is_translation("src/locales/strings.json", Script::Greek));
        assert!(!is_translation("src/rules.rs", Script::Cyrillic));
        assert!(!is_translation("docs/ru/README.md", Script::Han));
    }

    #[test]
    fn comments_are_told_from_code_and_strings() {
        let rust = "let s = \"// not a comment\"; // a comment\n/* block\n still */ let x = 1;\nlet c = '\"'; // after a char\n";
        let found: Vec<(usize, String)> = comments(rust, Syntax::Rust);
        assert_eq!(found[0], (1, "// a comment".to_string()));
        assert_eq!(found[1].0, 2);
        assert_eq!(found[2], (3, " still ".to_string()));
        assert_eq!(found[3], (4, "// after a char".to_string()));
        assert_eq!(comments("key = \"#nope\" # yes\n", Syntax::Hash), [(1, "# yes".to_string())]);
        assert_eq!(comments("text <!-- note --> more\n", Syntax::Markup), [(1, " note ".to_string())]);
    }

    /// A test's strings are data; its comments are read like any others.
    #[test]
    fn in_a_test_only_the_comments_are_read() {
        let mut prose_hits = Vec::new();
        let mut mixed = Vec::new();
        let test =
            "#[test]\nfn x() {\n    // Проверяем, что длинная фраза на русском ловится\n    assert_eq!(f(\"Это длинная фраза на русском языке\"), 1);\n}\n";
        examine("tests/x.rs", test, &mut prose_hits, &mut mixed);
        let lines: Vec<usize> = prose_hits.iter().map(|(_, h)| h.line).collect();
        assert_eq!(lines, [3]);

        // The same past `#[cfg(test)]` at the foot of a source file.
        prose_hits.clear();
        let source = format!("// Модуль делает очень важную вещь сейчас\nfn f() {{}}\n\n#[cfg(test)]\nmod tests {{\n{test}}}\n");
        examine("src/lib.rs", &source, &mut prose_hits, &mut mixed);
        let lines: Vec<usize> = prose_hits.iter().map(|(_, h)| h.line).collect();
        assert_eq!(lines, [1, 8]);
    }

    #[test]
    fn a_subject_in_another_language_is_found() {
        assert!(foreign_subject("исправил сборку"));
        assert!(foreign_subject("feat(export): Improved \"export\" сommand"));
        assert!(!foreign_subject("fix: read «Решена» as done"));
        assert!(!foreign_subject("feat: add the docs site"));
    }
}
