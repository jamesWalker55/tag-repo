//! This code is based on nom's arithmetic example:
//! https://github.com/rust-bakery/nom/blob/main/tests/arithmetic.rs
//!
//! TODO: Simplify expressions using
//! [Disjunctive normal form](https://en.wikipedia.org/wiki/Disjunctive_normal_form). This
//! simplifies expressions into many AND groups, joined by a single OR group.
//!
//! You can test DNF with Sympy:
//!
//! ```python
//! from sympy.logic import simplify_logic
//! a,b,c,d,e,f = symbols("a,b,c,d,e,f")
//!
//! eq = a & b | c & ( Symbol("in:src/") | d & e ) & f
//!
//! simplify_logic(eq, form='dnf', force=True)
//! // => (a ∧ b) ∨ (c ∧ f ∧ in:src/) ∨ (c ∧ d ∧ e ∧ f)
//! ```

use nom::branch::alt;
use nom::bytes::complete::{is_not, tag as nom_tag};
use nom::character::complete::{char as nom_char, none_of, one_of, space0, space1};
use nom::combinator::{map, opt, recognize, value};
use nom::multi::fold_many0;
use nom::sequence::{delimited, pair, preceded, separated_pair};
use nom::IResult;
use std::borrow::Cow;

fn double_quoted_string_fragment(input: &str) -> IResult<&str, Cow<str>> {
    alt((
        map(is_not("\""), Cow::from),
        map(value("\"", nom_tag("\"\"")), Cow::from),
    ))(input)
}

fn single_quoted_string_fragment(input: &str) -> IResult<&str, Cow<str>> {
    alt((
        map(is_not("'"), Cow::from),
        map(value("'", nom_tag("''")), Cow::from),
    ))(input)
}

/// string = '...' | "..."
fn string(input: &str) -> IResult<&str, String> {
    let (_, quote_char): (_, char) = one_of("'\"")(input)?;

    let parse_string_fragment = if quote_char == '"' {
        double_quoted_string_fragment
    } else {
        single_quoted_string_fragment
    };

    let build_string = fold_many0(
        parse_string_fragment,
        String::new,
        |mut string, fragment| {
            string.push_str(&fragment);
            string
        },
    );

    delimited(nom_char(quote_char), build_string, nom_char(quote_char))(input)
}

/// literal = [^"' -()] [^ ()]*
///
/// Literals cannot start with a quote (', "), a minus (-), or parentheses ("(", ")").
///
/// Literals cannot contain spaces (" "), or parentheses ("(", ")").
fn literal(input: &str) -> IResult<&str, &str> {
    let (new_input, name) = recognize(pair(none_of("\"' -()"), opt(is_not(" ()"))))(input)?;

    // disallow operators as tags
    if ["|", "(", ")"].contains(&name) {
        Err(nom::Err::Error(nom::error::Error {
            input,
            code: nom::error::ErrorKind::IsNot,
        }))
    } else {
        Ok((new_input, name))
    }
}

fn string_or_literal<'a>(input: &'a str) -> IResult<&str, Cow<'a, str>> {
    alt((
        map(string, |x| Cow::from(x)),
        map(literal, |x| Cow::from(x)),
    ))(input)
}

#[derive(Debug, PartialEq, Eq)]
pub enum Expr<'a> {
    And(Vec<Expr<'a>>),
    Or(Vec<Expr<'a>>),
    Not(Box<Expr<'a>>),
    Tag(Cow<'a, str>),
    KeyValue(Cow<'a, str>, Cow<'a, str>),
}

/// tag = string | literal
fn tag(input: &str) -> IResult<&str, Expr> {
    map(string_or_literal, Expr::Tag)(input)
}

/// allowed_key = "in"
fn allowed_key(input: &str) -> IResult<&str, &str> {
    nom_tag("in")(input)
}

/// key_val = allowed_key ":" (string | literal)
fn key_val<'a>(input: &'a str) -> IResult<&str, Expr<'a>> {
    map(
        separated_pair(allowed_key, nom_char(':'), string_or_literal),
        |(k, v)| Expr::KeyValue(Cow::from(k), v),
    )(input)
}

/// Parse an expression wrapped with parenthesis "(...)"
///
/// parens = "(" or_terms ")"
fn parens(input: &str) -> IResult<&str, Expr> {
    delimited(
        pair(nom_char('('), space0),
        or_terms,
        pair(space0, nom_char(')')),
    )(input)
}

/// Parse a single "factor", which is a singular expression, whether that is a tag, key-value, or a
/// group (parenthesis).
///
/// This function is "unsigned", that is it will only check for positive expressions. Negated
/// expressions are not checked, e.g. `a -b -c` only checks `a`.
fn unsigned_factor(input: &str) -> IResult<&str, Expr> {
    alt((parens, key_val, tag))(input)
}

/// Parse a single "factor", which is a singular expression, whether that is a tag, key-value, or a
/// group (parenthesis).
///
/// (This is as opposed to multi-term expressions like `And` and `Or`)
///
/// This function is "signed", that is it will check for normal and negated expressions.
/// E.g. `a -b -c` will check `a`, `-b` and `-c`
fn factor(input: &str) -> IResult<&str, Expr> {
    alt((
        // negated factor
        map(
            preceded(pair(nom_char('-'), space0), unsigned_factor),
            |x| Expr::Not(Box::new(x)),
        ),
        // normal factor
        unsigned_factor,
    ))(input)
}

/// Process AND operators. This also handles implicit ANDs.
/// (AND has the highest precedence)
fn and_terms(input: &str) -> IResult<&str, Expr> {
    // read an initial term first
    let (input, first_term) = factor(input)?;

    let mut all_terms = vec![first_term];

    // read remaining terms (if any)
    let (input, _) = fold_many0(
        preceded(space0, factor),
        || (),
        |_, item| all_terms.push(item),
    )(input)?;

    if all_terms.len() == 1 {
        // just return the first term
        return Ok((input, all_terms.pop().unwrap()));
    } else if all_terms.len() > 1 {
        // wrap all terms with an AND group, flattening any inner AND groups
        let mut flattened_terms = vec![];

        for term in all_terms {
            if let Expr::And(mut x) = term {
                flattened_terms.append(&mut x);
            } else {
                flattened_terms.push(term);
            }
        }

        Ok((input, Expr::And(flattened_terms)))
    } else {
        unreachable!();
    }
}

/// Process OR operators.
/// (OR has the lower precedence than AND, so it is the one that calls AND)
fn or_terms(input: &str) -> IResult<&str, Expr> {
    let (input, first_term) = and_terms(input)?;

    let mut all_terms = vec![first_term];

    // read remaining terms (if any)
    let (input, _) = fold_many0(
        preceded(delimited(space1, nom_tag("|"), space1), and_terms),
        || (),
        |_, item| all_terms.push(item),
    )(input)?;

    if all_terms.len() == 1 {
        // just return the first term
        return Ok((input, all_terms.pop().unwrap()));
    } else if all_terms.len() > 1 {
        // wrap all terms with an OR group, flattening any inner OR groups
        let mut flattened_terms = vec![];

        for term in all_terms {
            if let Expr::Or(mut x) = term {
                flattened_terms.append(&mut x);
            } else {
                flattened_terms.push(term);
            }
        }

        Ok((input, Expr::Or(flattened_terms)))
    } else {
        unreachable!();
    }
}

#[derive(Debug)]
pub(crate) enum ParseError<'a> {
    NomError(nom::Err<nom::error::Error<&'a str>>),
    InputNotFullyConsumed(&'a str, Expr<'a>),
}

impl<'a> From<nom::Err<nom::error::Error<&'a str>>> for ParseError<'a> {
    fn from(value: nom::Err<nom::error::Error<&'a str>>) -> Self {
        ParseError::NomError(value)
    }
}

/// Main entry point for the parser.
/// Calls `or_terms` and skips padded spaces in the beginning and end of input.
pub(crate) fn parse(input: &str) -> Result<Expr, ParseError> {
    let (unparsed_input, expr) = delimited(space0, or_terms, space0)(input)?;
    if unparsed_input.len() > 0 {
        Err(ParseError::InputNotFullyConsumed(unparsed_input, expr))
    } else {
        Ok(expr)
    }
}

#[rustfmt::skip]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string() {
        fn assert_parse(text: &str, expected: &str) {
            let result = string(&text).unwrap().1;
            assert_eq!(result, expected);
        }
        fn assert_parse_fails(text: &str) {
            assert!(string(&text).is_err());
        }

        // double quoted
        assert_parse(r#""aaa""sss""#, r#"aaa"sss"#);
        assert_parse(r#""aaaa' ' ''bbbb""cccc""#, r#"aaaa' ' ''bbbb"cccc"#);
        assert_parse(r#""aaa""""#, r#"aaa""#);
        assert_parse(r#""""""#, r#"""#);

        // single quoted
        assert_parse(r#"'aaa''sss'"#, r#"aaa'sss"#);
        assert_parse(r#"'aaaa' ' ''bbbb""cccc""#, r#"aaaa"#);
        assert_parse(r#"'aaa""''"'"#, r#"aaa""'""#);

        // fail cases
        assert_parse_fails(r#""aaa'"#);
    }

    #[test]
    fn test_key_value() {
        fn assert_parse(text: &str, expected: (&str, &str)) {
            let result = key_val(&text).unwrap().1;
            if let Expr::KeyValue(key, val) = result {
                assert_eq!(expected.0, key);
                assert_eq!(expected.1, val);
            } else {
                unreachable!();
            }
        }
        fn assert_parse_fails(text: &str) {
            assert!(key_val(&text).is_err());
        }

        assert_parse(
            r#"in:src/"#,
            ("in", "src/"),
        );
        assert_parse(
            r#"in:"D:/Audio Samples/""#,
            ("in", "D:/Audio Samples/"),
        );
        assert_parse(
            r#"in:"quote in path for some reason""""#,
            ("in", "quote in path for some reason\""),
        );
        assert_parse_fails(r#""spaced key":hello"#);
    }

    #[test]
    fn test_literal() {
        fn assert_parse(text: &str, expected: &str) {
            let result = literal(&text).unwrap().1;
            assert_eq!(result, expected);
        }
        fn assert_parse_fails(text: &str) {
            assert!(literal(&text).is_err());
        }
        assert_parse("a", "a");
        assert_parse("abc", "abc");
        assert_parse("a:sd qwe", "a:sd");
        assert_parse(":sd qwe", ":sd");
        assert_parse("m'lady", "m'lady");
        assert_parse_fails("'mlady");
    }

    #[test]
    fn test_tag() {
        fn assert_parse(text: &str, expected: &str) {
            let result = tag(&text).unwrap().1;
            if let Expr::Tag(text) = result {
                assert_eq!(text, expected);
            } else {
                unreachable!();
            }
        }
        fn assert_parse_fails(text: &str) {
            assert!(tag(&text).is_err());
        }
        assert_parse("a", "a");
        assert_parse("abc", "abc");
        assert_parse("mc'donalds", "mc'donalds");
        assert_parse("'tag with spaces'", "tag with spaces");
        assert_parse_fails("'mlady");
    }
}

#[rustfmt::skip]
#[cfg(test)]
mod expr_tests {
    use super::*;

    fn and(exprs: Vec<Expr>) -> Expr { Expr::And(exprs) }

    fn or(exprs: Vec<Expr>) -> Expr { Expr::Or(exprs) }

    fn not(expr: Expr) -> Expr { Expr::Not(Box::new(expr)) }

    fn t(name: &str) -> Expr { Expr::Tag(name.into()) }

    fn kv<'a, 'b>(key: &'a str, val: &'a str) -> Expr<'b> {
        Expr::KeyValue(key.to_string().into(), val.to_string().into())
    }

    fn assert_expr(input: &str, expected: Expr) {
        let expr = parse(input).unwrap();
        assert_eq!(expr, expected);
    }

    #[test] fn just_and_1() { assert_expr("a b c", and(vec![t("a"), t("b"), t("c")])); }
    #[test] fn just_and_2() { assert_expr("a & b c", and(vec![t("a"), t("&"), t("b"), t("c")])); }
    #[test] fn just_and_3() { assert_expr("a &b & c", and(vec![t("a"), t("&b"), t("&"), t("c")]), ); }
    #[test] fn just_and_4() { assert_expr("a&b&c", t("a&b&c")); }

    #[test] fn just_or_1() { assert_expr("a | b | c", or(vec![t("a"), t("b"), t("c")])); }
    #[test] fn just_or_2() { assert_expr("a | b | c", or(vec![t("a"), t("b"), t("c")])); }
    #[test] fn just_or_3() { assert_expr("a | |b | c", or(vec![t("a"), t("|b"), t("c")])); }
    #[test] fn just_or_4() { assert_expr("a|b | c", or(vec![t("a|b"), t("c")])); }

    #[test] fn and_or_1() { assert_expr("a| b | c", or(vec![and(vec![t("a|"), t("b")]), t("c")])); }
    #[test] fn and_or_2() { assert_expr("a b | c | d e f", or(vec![and(vec![t("a"), t("b")]), t("c"), and(vec![t("d"), t("e"), t("f")])])); }

    #[test] fn parens_1() { assert_expr("(a b) | c", or(vec![and(vec![t("a"), t("b")]), t("c")])); }
    #[test] fn parens_2() { assert_expr("(a b) c", and(vec![t("a"), t("b"), t("c")])); }
    #[test] fn parens_3() { assert_expr("( a b ) c", and(vec![t("a"), t("b"), t("c")])); }
    #[test] fn parens_4() { assert_expr("c ( a b )", and(vec![t("c"), t("a"), t("b")])); }

    #[test] fn string_tags_1() { assert_expr(r#""c" ( 'a' "b" )"#, and(vec![t("c"), t("a"), t("b")])); }
    #[test] fn string_tags_2() { assert_expr(r#""c ( 'a' b )""#, t("c ( 'a' b )")); }
    #[test] fn string_tags_3() { assert_expr(r#" as "#, t("as")); }

    #[test] fn not_1() { assert_expr("a b -e in:1 | d e in:0",
        or(vec![
            and(vec![t("a"), t("b"), not(t("e")), kv("in", "1")]),
            and(vec![t("d"), t("e"), kv("in", "0")]),
        ]),
    ); }
    #[test] fn not_2() { assert_expr("a -(b e in:1) | -d e in:0",
        or(vec![
            and(vec![t("a"), not(and(vec![t("b"), t("e"), kv("in", "1")]))]),
            and(vec![not(t("d")), t("e"), kv("in", "0")]),
        ]),
    ); }

    #[test]
    fn common_1() {
        assert_expr(
            "kick drum (black_octopus_sounds | in:'black octopus' | in:'black-octopus')",
            and(vec![
                t("kick"),
                t("drum"),
                or(vec![
                    t("black_octopus_sounds"),
                    kv("in", "black octopus"),
                    kv("in", "black-octopus"),
                ]),
            ]),
        )
    }

    #[test]
    fn complex_1() {
        assert_expr(
            "a & b | c ( in:src/ | d &e ) & f",
            or(vec![
                and(vec![
                    t("a"),
                    t("&"),
                    t("b")
                ]),
                and(vec![
                    t("c"),
                    or(vec![
                        kv("in", "src/"),
                        and(vec![
                            t("d"),
                            t("&e"),
                        ]),
                    ]),
                    t("&"),
                    t("f"),
                ]),
            ]),
        );
    }

    // TODO: Detect unicode spaces
    // #[test] fn cjk01() { assert_expr("你好　亞視啲",
    //     and(vec![t("你好"), t("亞視啲")]),
    // ); }
}
