// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! Ported from `utils.ts`: `format`, `camelToTitleCase`, `firstUpper`,
//! `getExtension`.

use std::collections::HashMap;

/// `{key}`-style template substitution, used for the cue-name-format
/// settings (e.g. `"Playing {filename}"`). `{{`/`}}` escape to a literal
/// `{`/`}`. A `{key}` with no matching entry in `values` (or an empty
/// `{}`) is left in the output exactly as written, matching the original
/// (it returns the unmodified regex match rather than erroring or
/// dropping it).
///
/// One intentional deviation from the original: a key is `\w` (ASCII
/// letters/digits/underscore) or whitespace in both, but the original's
/// `\w` is Unicode-aware (JS regex without the `u` flag still treats `\w`
/// as ASCII-only in practice for this character class, so this is actually
/// not a deviation -- noted here because it was worth checking, not
/// because it needed fixing).
pub fn format_template(template: &str, values: &HashMap<String, String>) -> String {
    let chars: Vec<char> = template.chars().collect();
    let mut result = String::with_capacity(template.len());
    let mut i = 0;

    while i < chars.len() {
        match chars[i] {
            '{' if chars.get(i + 1) == Some(&'{') => {
                result.push('{');
                i += 2;
            }
            '}' if chars.get(i + 1) == Some(&'}') => {
                result.push('}');
                i += 2;
            }
            '{' => match find_key_end(&chars, i + 1) {
                Some(end) => {
                    let key: String = chars[i + 1..end].iter().collect();
                    match values.get(key.trim()) {
                        Some(value) => result.push_str(value),
                        None => result.extend(&chars[i..=end]),
                    }
                    i = end + 1;
                }
                None => {
                    result.push('{');
                    i += 1;
                }
            },
            c => {
                result.push(c);
                i += 1;
            }
        }
    }

    result
}

/// Finds the index of the `}` closing a `{key}` starting at `start`
/// (just after the `{`). Returns `None` if the key is empty or the run of
/// word/whitespace characters isn't immediately followed by `}` -- in
/// either case there is no match here at all, not a replacement with an
/// empty value.
fn find_key_end(chars: &[char], start: usize) -> Option<usize> {
    let mut i = start;
    while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_' || chars[i].is_whitespace())
    {
        i += 1;
    }

    (i > start && chars.get(i) == Some(&'}')).then_some(i)
}

/// `"crossFadeOut"` -> `"Cross Fade Out"`: splits before each ASCII
/// uppercase letter, then uppercases the first character of the result.
pub fn camel_to_title_case(input: &str) -> String {
    let mut spaced = String::with_capacity(input.len() + 4);
    for c in input.chars() {
        if c.is_ascii_uppercase() {
            spaced.push(' ');
        }
        spaced.push(c);
    }

    let mut chars = spaced.chars();
    let titled = match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    };

    titled.trim().to_string()
}

/// Uppercases only the first character; the rest is left exactly as-is.
pub fn first_upper(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// Extracts a lowercased file extension from a path, or an empty string if
/// there is none (no dot, a leading dot with nothing before it, or a
/// trailing dot with nothing after it).
pub fn get_extension(path: &str) -> String {
    let file_name = path.rsplit(['/', '\\']).next().unwrap_or(path);

    match file_name.rfind('.') {
        Some(dot_index) if dot_index > 0 && dot_index != file_name.len() - 1 => {
            file_name[dot_index + 1..].to_lowercase()
        }
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn values(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    #[test]
    fn substitutes_known_keys() {
        let v = values(&[("filename", "kick.wav")]);
        assert_eq!(format_template("Playing {filename}", &v), "Playing kick.wav");
    }

    #[test]
    fn trims_key_whitespace() {
        let v = values(&[("name", "X")]);
        assert_eq!(format_template("{ name }", &v), "X");
    }

    #[test]
    fn escapes_double_braces() {
        let v = values(&[]);
        assert_eq!(format_template("{{literal}}", &v), "{literal}");
    }

    #[test]
    fn unknown_key_passes_through_unchanged() {
        let v = values(&[]);
        assert_eq!(format_template("{missing}", &v), "{missing}");
    }

    #[test]
    fn empty_braces_pass_through() {
        let v = values(&[]);
        assert_eq!(format_template("a{}b", &v), "a{}b");
    }

    #[test]
    fn title_case_splits_before_uppercase() {
        assert_eq!(camel_to_title_case("crossFadeOut"), "Cross Fade Out");
        assert_eq!(camel_to_title_case("linear"), "Linear");
    }

    #[test]
    fn first_upper_only_touches_first_char() {
        assert_eq!(first_upper("hello world"), "Hello world");
        assert_eq!(first_upper(""), "");
    }

    #[test]
    fn extension_basic() {
        assert_eq!(get_extension("/a/b/kick.WAV"), "wav");
        assert_eq!(get_extension(r"C:\music\kick.wav"), "wav");
    }

    #[test]
    fn extension_edge_cases() {
        assert_eq!(get_extension("noext"), "");
        assert_eq!(get_extension(".gitignore"), "");
        assert_eq!(get_extension("trailing."), "");
        assert_eq!(get_extension("a/b/"), "");
    }
}
