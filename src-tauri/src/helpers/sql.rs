use std::borrow::Cow;

// Cow optimisation from:
// https://lise-henry.github.io/articles/optimising_strings.html
/// Escape a string to be used in a FTS5 query. This duplicates all quotes (both single and double).
pub(crate) fn escape_fts5_string<'a>(text: impl Into<Cow<'a, str>>) -> Cow<'a, str> {
    let text = text.into();

    fn is_quote_char(c: char) -> bool {
        c == '\'' || c == '"'
    }
    if text.contains(is_quote_char) {
        // add 1 to existing length, we already know it contains a quote
        let mut result = String::with_capacity(text.len() + 1);
        for char in text.chars() {
            match char {
                '\'' => {
                    result.push_str("''");
                }
                '"' => {
                    result.push_str(r#""""#);
                }
                _ => {
                    result.push(char);
                }
            }
        }
        result.into()
    } else {
        text.into()
    }
}

/// Escape a string to be used in a LIKE query. This prefixes any percent ("%"), underscore ("_"),
/// and escape characters with the escape character. This also duplicates any single quotes in the
/// string. The escape character is provided by you in `escape_char`.
///
/// The returned string must be used in conjunction with the given `escape_char` as follows:
///
/// ```sql
/// WHERE column LIKE '<returned string>' ESCAPE '<escape char>'
/// ```
pub(crate) fn escape_like_pattern(text: &str, escape_char: char) -> String {
    let mut result = String::with_capacity(text.len());
    for char in text.chars() {
        match char {
            '%' => {
                result.push(escape_char);
                result.push('%');
            }
            '_' => {
                result.push(escape_char);
                result.push('_');
            }
            '\'' => {
                result.push_str("''");
            }
            _ if char == escape_char => {
                result.push(escape_char);
                result.push(escape_char);
            }
            _ => {
                result.push(char);
            }
        }
    }
    result
}

#[cfg(test)]
mod test_fts5 {
    use super::*;

    #[test]
    fn no_quotes() {
        assert_eq!(escape_fts5_string("asd"), "asd",)
    }

    #[test]
    fn single_quotes() {
        assert_eq!(escape_fts5_string("'as'd"), "''as''d",)
    }

    #[test]
    fn double_quotes() {
        assert_eq!(escape_fts5_string(r#""as"d"#), r#"""as""d"#,)
    }

    #[test]
    fn both_quotes() {
        assert_eq!(escape_fts5_string(r#""a's"d'"#), r#"""a''s""d''"#,)
    }
}

#[cfg(test)]
mod test_like {
    use super::*;

    const ESC_CHAR: char = '\\';

    fn assert_escaped(text: &str, expected: &str) {
        assert_eq!(escape_like_pattern(text, ESC_CHAR), expected);
    }

    #[test] fn no_quotes() { assert_escaped("asd", "asd"); }
    #[test] fn single_quotes() { assert_escaped("'as'd", "''as''d"); }
    #[test] fn double_quotes() { assert_escaped(r#""as"d"#, r#""as"d"#); }
    #[test] fn both_quotes() { assert_escaped(r#""a's"d'"#, r#""a''s"d''"#); }
    #[test] fn percent_1() { assert_escaped(r#"100% free range"#, r#"100\% free range"#); }
    #[test] fn percent_2() { assert_escaped(r#"100%% free range%"#, r#"100\%\% free range\%"#); }
    #[test] fn underscore_1() { assert_escaped(r#"foo_bar"#, r#"foo\_bar"#); }
    #[test] fn underscore_2() { assert_escaped(r#"foo__bar"#, r#"foo\_\_bar"#); }
    #[test] fn escape_char_1() { assert_escaped(r#"C:\Program Files"#, r#"C:\\Program Files"#); }
    #[test] fn escape_char_2() { assert_escaped(r#"D:\Audio Samples\Drum Kit"#, r#"D:\\Audio Samples\\Drum Kit"#); }
}
