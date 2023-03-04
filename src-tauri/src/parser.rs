//! This code is based on nom's arithmetic example:
//! https://github.com/rust-bakery/nom/blob/main/tests/arithmetic.rs

use nom::branch::alt;
use nom::bytes::complete::{is_not, tag as nom_tag};
use nom::character::complete::{char as nom_char, none_of, one_of, space0 as space};
use nom::combinator::{map, opt, recognize, value};
use nom::multi::{fold_many0, many0};
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

fn literal(input: &str) -> IResult<&str, &str> {
    recognize(pair(
        none_of("\"' :|&"),
        // take_till(|x| x == ' ' || x == ':'),
        opt(is_not(" :|&")),
    ))(input)
}

fn string_or_literal<'a>(input: &'a str) -> IResult<&str, Cow<'a, str>> {
    alt((
        map(string, |x| Cow::from(x)),
        map(literal, |x| Cow::from(x)),
    ))(input)
}

#[derive(Debug, PartialEq, Eq)]
enum Expr<'a> {
    And(Vec<Expr<'a>>),
    Or(Vec<Expr<'a>>),
    Not(Box<Expr<'a>>),
    Tag(Cow<'a, str>),
    KeyValue(Cow<'a, str>, Cow<'a, str>),
}

fn tag(input: &str) -> IResult<&str, Expr> {
    map(string_or_literal, Expr::Tag)(input)
}

fn key_val<'a>(input: &'a str) -> IResult<&str, Expr<'a>> {
    map(
        separated_pair(literal, nom_char(':'), string_or_literal),
        |(k, v)| Expr::KeyValue(Cow::from(k), v),
    )(input)
}

/// Parse an expression wrapped with parenthesis "(...)"
fn parens(input: &str) -> IResult<&str, Expr> {
    delimited(
        space,
        delimited(nom_char('('), or_terms, nom_char(')')),
        space,
    )(input)
}

/// Parse a single "factor", which is a singular expression, whether that is a tag, key-value, or a
/// group (parenthesis).
///
/// This function is "unsigned", that is it will only check for positive expressions. Negated
/// expressions are not checked, e.g. `a -b -c` only checks `a`.
fn unsigned_factor(input: &str) -> IResult<&str, Expr> {
    alt((parens, delimited(space, alt((key_val, tag)), space)))(input)
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
        // normal factor
        unsigned_factor,
        // negated factor
        map(
            preceded(space, preceded(nom_char('-'), unsigned_factor)),
            |x| Expr::Not(Box::new(x)),
        ),
    ))(input)
}

/// Process AND operators. This also handles implicit ANDs.
/// (AND has the highest precedence)
fn and_terms(input: &str) -> IResult<&str, Expr> {
    // read an initial factor first
    let (input, init) = factor(input)?;

    let (input, mut terms) = many0(alt((preceded(nom_char('&'), factor), factor)))(input)?;
    terms.splice(0..0, vec![init]);

    match terms.len() {
        1 => Ok((input, terms.pop().unwrap())),
        x if x >= 2 => Ok((input, Expr::And(terms))),
        _ => unreachable!(),
    }
}

/// Main entry point for the parser.
/// Process OR operators.
/// (OR has the lower precedence than AND, so it is the one that calls AND)
fn or_terms(input: &str) -> IResult<&str, Expr> {
    let (input, init) = and_terms(input)?;

    let (input, mut terms) = many0(preceded(nom_char('|'), and_terms))(input)?;
    terms.splice(0..0, vec![init]);

    match terms.len() {
        1 => Ok((input, terms.pop().unwrap())),
        x if x >= 2 => Ok((input, Expr::Or(terms))),
        _ => unreachable!(),
    }
}

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

        assert_parse(r#"inpath:src/"#, ("inpath", "src/"));
        assert_parse(
            r#"inpath:"D:/Audio Samples/""#,
            ("inpath", "D:/Audio Samples/"),
        );
        assert_parse(
            r#"inpath:"quote in path for some reason""""#,
            ("inpath", "quote in path for some reason\""),
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
        assert_parse("a:sd qwe", "a");
        assert_parse_fails(":sd qwe");
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
        // fn assert_parse_fails(text: &str) {
        //     assert!(parse_tag(&text).is_err());
        // }
        assert_parse("a", "a");
        assert_parse("abc", "abc");
        assert_parse("mc'donalds", "mc'donalds");
        assert_parse("'tag with spaces'", "tag with spaces");
    }
}

#[cfg(test)]
mod expr_tests {
    use super::*;

    fn and(exprs: Vec<Expr>) -> Expr {
        Expr::And(exprs)
    }

    fn or(exprs: Vec<Expr>) -> Expr {
        Expr::Or(exprs)
    }

    fn not(expr: Expr) -> Expr {
        Expr::Not(Box::new(expr))
    }

    fn tag(name: &str) -> Expr {
        Expr::Tag(name.into())
    }

    fn kv<'a, 'b>(key: &'a str, val: &'a str) -> Expr<'b> {
        Expr::KeyValue(key.to_string().into(), val.to_string().into())
    }

    fn assert_expr(input: &str, expected: Expr) {
        let expr = or_terms(input).unwrap();
        assert_eq!(expr.1, expected);
        // the entire input must be consumed
        assert_eq!(expr.0, "");
    }

    #[test]
    fn just_and_1() {
        assert_expr("a b c", and(vec![tag("a"), tag("b"), tag("c")]));
    }
    #[test]
    fn just_and_2() {
        assert_expr("a & b & c", and(vec![tag("a"), tag("b"), tag("c")]));
    }
    #[test]
    fn just_and_3() {
        assert_expr("a &b & c", and(vec![tag("a"), tag("&b"), tag("c")]));
    }
    #[test]
    fn just_and_4() {
        assert_expr("a&b & c", and(vec![tag("a&b"), tag("c")]));
    }

    #[test]
    fn just_or_1() {
        assert_expr("a | b | c", or(vec![tag("a"), tag("b"), tag("c")]));
    }
    #[test]
    fn just_or_2() {
        assert_expr("a | b | c", and(vec![tag("a"), tag("b"), tag("c")]));
    }
    #[test]
    fn just_or_3() {
        assert_expr("a | |b | c", and(vec![tag("a"), tag("|b"), tag("c")]));
    }
    #[test]
    fn just_or_4() {
        assert_expr("a|b | c", and(vec![tag("a|b"), tag("c")]));
    }

    #[test]
    fn complex_1() {
        assert_expr(
            "a & b | c ( inpath:src/ | d &e ) & f",
            or(vec![
                and(vec![tag("a"), tag("b")]),
                and(vec![
                    tag("c"),
                    or(vec![kv("inpath", "src/"), and(vec![tag("d"), tag("&e")])]),
                    tag("f"),
                ]),
            ]),
        );
    }

    // let text = "a & b | c ( inpath:src/ | d &e ) & f";
    // dbg!(kv("a", "b") == kv("a", "b"));
}
