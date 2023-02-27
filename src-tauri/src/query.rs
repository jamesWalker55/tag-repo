use std::fmt::Formatter;

// This function can be optimised further, but I'm lazy at the moment:
// https://lise-henry.github.io/articles/optimising_strings.html
// Use the Cow implementation, since most of the input will not contain quotes
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

#[derive(Debug)]
enum Symbol {
  /// A tag. The boolean indicates whether this singleton is positive:
  /// `true = a`, and `false = ~a`.
  Tag(String, bool),
}

impl From<&Symbol> for String {
  fn from(sym: &Symbol) -> String {
    match sym {
      Symbol::Tag(name, positive) => {
        if *positive {
          format!(r#""{}""#, escape_fts5_string(name.as_str()))
        } else {
          format!(r#"(("meta_tags": "all") NOT "{}")"#, escape_fts5_string(name.as_str()))
        }
      }
    }
  }
}

#[derive(Debug)]
enum Expr {
  /// Represents AND between 2 terms: `a & b`
  And(Box<Expr>, Box<Expr>),
  /// Represents OR between 2 terms: `a | b`
  Or(Box<Expr>, Box<Expr>),
  /// Represents a singleton. ("Term" is short for "terminal")
  Term(Symbol),
}

impl From<&Expr> for String {
  fn from(expr: &Expr) -> String {
    match expr {
      Expr::And(a, b) => {
        let a = String::from(&**a);
        let b = String::from(&**b);
        format!("({} AND {})", a, b)
      }
      Expr::Or(a, b) => {
        let a = String::from(&**a);
        let b = String::from(&**b);
        format!("({} OR {})", a, b)
      }
      Expr::Term(sym) => String::from(sym)
    }
  }
}

impl std::fmt::Display for Expr {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", String::from(self))
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  mod escape_test {
    use super::*;

    #[test]
    fn no_quotes() {
      assert_eq!(
        escape_fts5_string("asd"),
        "asd",
      )
    }

    #[test]
    fn single_quotes() {
      assert_eq!(
        escape_fts5_string("'as'd"),
        "''as''d",
      )
    }

    #[test]
    fn double_quotes() {
      assert_eq!(
        escape_fts5_string(r#""as"d"#),
        r#"""as""d"#,
      )
    }

    #[test]
    fn both_quotes() {
      assert_eq!(
        escape_fts5_string(r#""a's"d'"#),
        r#"""a''s""d''"#,
      )
    }
  }

  mod symbol_test {
    use super::*;

    #[test]
    fn tag_positive() {
      let sym = Symbol::Tag("hello".into(), true);
      let expected = r#""hello""#;
      let rv = String::from(&sym);
      assert_eq!(rv, expected);
    }

    #[test]
    fn tag_positive_with_quote() {
      let sym = Symbol::Tag("he' llo".into(), true);
      let expected = r#""he'' llo""#;
      let rv = String::from(&sym);
      assert_eq!(rv, expected);
    }

    #[test]
    fn tag_negative() {
      let sym = Symbol::Tag("hello".into(), false);
      let expected = r#"(("meta_tags": "all") NOT "hello")"#;
      let rv = String::from(&sym);
      assert_eq!(rv, expected);
    }

    #[test]
    fn tag_negative_with_quote() {
      let sym = Symbol::Tag(r#"he" llo"#.into(), false);
      let expected = r#"(("meta_tags": "all") NOT "he"" llo")"#;
      let rv = String::from(&sym);
      assert_eq!(rv, expected);
    }
  }

  mod expr_test {
    use crate::query::{Expr, Symbol};

    #[test]
    fn and_str() {
      let a = Expr::Term(Symbol::Tag("a".to_string(), true));
      let b = Expr::Term(Symbol::Tag("b".to_string(), false));
      let expr = Expr::And(Box::new(a), Box::new(b));
      let expected = r#"("a" AND (("meta_tags": "all") NOT "b"))"#;
      assert_eq!(String::from(&expr), expected);
    }

    #[test]
    fn or_str() {
      let a = Expr::Term(Symbol::Tag("a".to_string(), true));
      let b = Expr::Term(Symbol::Tag("b".to_string(), false));
      let expr = Expr::Or(Box::new(a), Box::new(b));
      let expected = r#"("a" OR (("meta_tags": "all") NOT "b"))"#;
      assert_eq!(String::from(&expr), expected);
    }

    #[test]
    fn term_str() {
      let expr = Expr::Term(Symbol::Tag("a".to_string(), true));
      let expected = r#""a""#;
      assert_eq!(String::from(&expr), expected);
    }

    #[test]
    fn test1() {
      let expr =
        Expr::And(
          Box::new(Expr::And(
            Box::new(Expr::Or(
              Box::new(Expr::Term(
                Symbol::Tag("a".to_string(), true)
              )),
              Box::new(Expr::Term(
                Symbol::Tag("b".to_string(), true)
              )),
            )),
            Box::new(Expr::Term(
              Symbol::Tag("c".to_string(), false)
            )),
          )),
          Box::new(Expr::Term(
            Symbol::Tag("d".to_string(), true)
          )),
        );
      let rv = String::from(&expr);
      let expected = r#"((("a" OR "b") AND (("meta_tags": "all") NOT "c")) AND "d")"#;
      assert_eq!(rv, expected);
    }
  }
}
