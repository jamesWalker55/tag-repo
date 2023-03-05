// This function can be optimised further, but I'm lazy at the moment:
// https://lise-henry.github.io/articles/optimising_strings.html
// Use the Cow implementation, since most of the input will not contain quotes
/// Escape a string to be used in a FTS5 query. This duplicates all quotes (both single and double).
pub(crate) fn escape_fts5_string(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
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
    result
}

/// Escape a string to be used in a LIKE query. This prefixes any percent ("%"), underscore ("_"),
/// and escape characters with the escape character. This also duplicates any single quotes in the
/// string. The escape character is provided by you in `escape_char`.
///
/// The returned string must be used in conjunction with the given `escape_char` as follows:
///
///     WHERE column LIKE '<returned string>' ESCAPE '<escape char>'
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
