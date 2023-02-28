// TODO: Make this module be able to handle complicated queries like in src/repo.rs:478

// This function can be optimised further, but I'm lazy at the moment:
// https://lise-henry.github.io/articles/optimising_strings.html
// Use the Cow implementation, since most of the input will not contain quotes
/// Escape a string to be used in a FTS5 query. This duplicates all quotes (both single and double).
fn escape_fts5_string(text: &str) -> String {
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
fn escape_like_pattern(text: &str, escape_char: char) -> String {
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
      escape_char => {
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

#[derive(Debug, Eq, PartialEq)]
enum Symbol {
  /// A tag. E.g. `kick`
  Tag(String),
  // /// A key-value pair. E.g. `inpath:res/audio/`
  // InPath(String),
}

enum QueryConversionError {
  NoAssociatedFTSString,
  NoAssociatedWhereClause,
}

impl Symbol {
  fn to_fts_string(&self) -> Result<String, QueryConversionError> {
    match self {
      Symbol::Tag(name) => Ok(format!(r#"tags:"{}""#, escape_fts5_string(name.as_str()))),
      // Symbol::InPath(_) => Err(QueryConversionError::NoAssociatedFTSString),
    }
  }

  fn to_where_clause(&self) -> Result<String, QueryConversionError> {
    match self {
      Symbol::Tag(_) => Err(QueryConversionError::NoAssociatedWhereClause),
      // Symbol::InPath(path) => {
      //   Ok(format!(r#"path LIKE '{}' ESCAPE '\'"#, escape_like_pattern(path, '\\')))
      // }
    }
  }
}

#[derive(Debug)]
enum Expr {
  /// Represents AND between 2 terms: `a & b`
  And(Box<Expr>, Box<Expr>),
  /// Represents OR between 2 terms: `a | b`
  Or(Box<Expr>, Box<Expr>),
  /// Represents a negation of an expression: `~a`
  Not(Box<Expr>),
  /// Represents a single term
  Term(Symbol),
}

impl Expr {
  fn to_where_clause(&self) -> String {
    todo!();
    // match self {
    //   Symbol::Tag(_) => Err(QueryConversionError::NoAssociatedWhereClause),
    //   Symbol::InPath(path) => {
    //     Ok(format!(r#"path LIKE '{}' ESCAPE '\'"#, escape_like_pattern(path, '\\')))
    //   }
    // }
  }
}

// #[cfg(test)]
// mod tests {
//   use super::*;
//
//   mod escape_test {
//     use super::*;
//
//     #[test]
//     fn no_quotes() {
//       assert_eq!(
//         escape_fts5_string("asd"),
//         "asd",
//       )
//     }
//
//     #[test]
//     fn single_quotes() {
//       assert_eq!(
//         escape_fts5_string("'as'd"),
//         "''as''d",
//       )
//     }
//
//     #[test]
//     fn double_quotes() {
//       assert_eq!(
//         escape_fts5_string(r#""as"d"#),
//         r#"""as""d"#,
//       )
//     }
//
//     #[test]
//     fn both_quotes() {
//       assert_eq!(
//         escape_fts5_string(r#""a's"d'"#),
//         r#"""a''s""d''"#,
//       )
//     }
//   }
//
//   mod symbol_test {
//     use super::*;
//
//     #[test]
//     fn tag_positive() {
//       let sym = Symbol::Tag("hello".into(), true);
//       let expected = r#""hello""#;
//       let rv = String::from(&sym);
//       assert_eq!(rv, expected);
//     }
//
//     #[test]
//     fn tag_positive_with_quote() {
//       let sym = Symbol::Tag("he' llo".into(), true);
//       let expected = r#""he'' llo""#;
//       let rv = String::from(&sym);
//       assert_eq!(rv, expected);
//     }
//
//     #[test]
//     fn tag_negative() {
//       let sym = Symbol::Tag("hello".into(), false);
//       let expected = r#"(("meta_tags": "all") NOT "hello")"#;
//       let rv = String::from(&sym);
//       assert_eq!(rv, expected);
//     }
//
//     #[test]
//     fn tag_negative_with_quote() {
//       let sym = Symbol::Tag(r#"he" llo"#.into(), false);
//       let expected = r#"(("meta_tags": "all") NOT "he"" llo")"#;
//       let rv = String::from(&sym);
//       assert_eq!(rv, expected);
//     }
//   }
//
//   mod expr_test {
//     use crate::query::{Expr, Symbol};
//
//     #[test]
//     fn and_str() {
//       let a = Expr::Term(Symbol::Tag("a".to_string(), true));
//       let b = Expr::Term(Symbol::Tag("b".to_string(), false));
//       let expr = Expr::And(Box::new(a), Box::new(b));
//       let expected = r#"("a" AND (("meta_tags": "all") NOT "b"))"#;
//       assert_eq!(String::from(&expr), expected);
//     }
//
//     #[test]
//     fn or_str() {
//       let a = Expr::Term(Symbol::Tag("a".to_string(), true));
//       let b = Expr::Term(Symbol::Tag("b".to_string(), false));
//       let expr = Expr::Or(Box::new(a), Box::new(b));
//       let expected = r#"("a" OR (("meta_tags": "all") NOT "b"))"#;
//       assert_eq!(String::from(&expr), expected);
//     }
//
//     #[test]
//     fn term_str() {
//       let expr = Expr::Term(Symbol::Tag("a".to_string(), true));
//       let expected = r#""a""#;
//       assert_eq!(String::from(&expr), expected);
//     }
//
//     #[test]
//     fn test1() {
//       let expr =
//         Expr::And(
//           Box::new(Expr::And(
//             Box::new(Expr::Or(
//               Box::new(Expr::Term(
//                 Symbol::Tag("a".to_string(), true)
//               )),
//               Box::new(Expr::Term(
//                 Symbol::Tag("b".to_string(), true)
//               )),
//             )),
//             Box::new(Expr::Term(
//               Symbol::Tag("c".to_string(), false)
//             )),
//           )),
//           Box::new(Expr::Term(
//             Symbol::Tag("d".to_string(), true)
//           )),
//         );
//       let rv = String::from(&expr);
//       let expected = r#"((("a" OR "b") AND (("meta_tags": "all") NOT "c")) AND "d")"#;
//       assert_eq!(rv, expected);
//     }
//   }
// }
