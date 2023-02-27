enum Expr {
  /// Represents AND between 2 terms: `a & b`
  And(Box<Expr>, Box<Expr>),
  /// Represents OR between 2 terms: `a | b`
  Or(Box<Expr>, Box<Expr>),
  /// Represents a singleton. ("Term" is short for "terminal")
  Term(Symbol),
}

enum Symbol {
  /// A tag. The boolean indicates whether this singleton is positive:
  /// `true = a`, and `false = ~a`.
  Tag(String, bool),
}

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

impl From<Symbol> for String {
  fn from(sym: Symbol) -> String {
    match sym {
      Symbol::Tag(name, positive) => {
        if positive {
          format!(r#""{}""#, escape_fts5_string(name.as_str()))
        } else {
          format!(r#"("meta_tags": "all") NOT "{}""#, escape_fts5_string(name.as_str()))
        }
      }
    }
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
      let rv = String::from(sym);
      assert_eq!(rv, expected);
    }

    #[test]
    fn tag_positive_with_quote() {
      let sym = Symbol::Tag("he' llo".into(), true);
      let expected = r#""he'' llo""#;
      let rv = String::from(sym);
      assert_eq!(rv, expected);
    }

    #[test]
    fn tag_negative() {
      let sym = Symbol::Tag("hello".into(), false);
      let expected = r#"("meta_tags": "all") NOT "hello""#;
      let rv = String::from(sym);
      assert_eq!(rv, expected);
    }

    #[test]
    fn tag_negative_with_quote() {
      let sym = Symbol::Tag(r#"he" llo"#.into(), false);
      let expected = r#"("meta_tags": "all") NOT "he"" llo""#;
      let rv = String::from(sym);
      assert_eq!(rv, expected);
    }
  }
}
