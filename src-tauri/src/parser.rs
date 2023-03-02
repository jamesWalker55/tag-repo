//! This example shows an example of how to parse an escaped string. The
//! rules for the string are similar to JSON and rust. A string is:
//!
//! - Enclosed by double quotes
//! - Can contain any raw unescaped code point besides \ and "
//! - Matches the following escape sequences: \b, \f, \n, \r, \t, \", \\, \/
//! - Matches code points like Rust: \u{XXXX}, where XXXX can be up to 6
//!   hex characters
//! - an escape followed by whitespace consumes all whitespace between the
//!   escape and the next non-whitespace character

// #![cfg(feature = "alloc")]

use nom::branch::alt;
use nom::bytes::complete::{is_not, tag, take_till};
use nom::character::complete::{none_of, one_of};
use nom::combinator::{map, recognize, value};
use nom::multi::fold_many0;
use nom::sequence::{delimited, pair, separated_pair};
use nom::IResult;
use std::borrow::Cow;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StringFragment<'a> {
    Literal(&'a str),
    EscapedQuote(char),
}

fn parse_double_quoted_string_fragment(input: &str) -> IResult<&str, StringFragment> {
    alt((
        map(is_not("\""), StringFragment::Literal),
        map(value('"', tag("\"\"")), StringFragment::EscapedQuote),
    ))(input)
}

fn parse_single_quoted_string_fragment(input: &str) -> IResult<&str, StringFragment> {
    alt((
        map(is_not("'"), StringFragment::Literal),
        map(value('\'', tag("''")), StringFragment::EscapedQuote),
    ))(input)
}

fn parse_string(input: &str) -> IResult<&str, String> {
    use nom::character::streaming::char;

    let (_, quote_char): (_, char) = one_of("'\"")(input)?;

    let parse_string_fragment = if quote_char == '"' {
        parse_double_quoted_string_fragment
    } else {
        parse_single_quoted_string_fragment
    };

    let build_string = fold_many0(
        parse_string_fragment,
        String::new,
        |mut string, fragment| {
            match fragment {
                StringFragment::Literal(s) => string.push_str(s),
                StringFragment::EscapedQuote(c) => string.push(c),
            }
            string
        },
    );

    delimited(char(quote_char), build_string, char(quote_char))(input)
}

fn parse_literal(input: &str) -> IResult<&str, &str> {
    recognize(pair(none_of("\"': "), take_till(|x| x == ' ' || x == ':')))(input)
}

fn parse_string_or_literal<'a>(input: &'a str) -> IResult<&str, Cow<'a, str>> {
    alt((
        map(parse_string, |x| Cow::Owned(x)),
        map(parse_literal, |x| x.into()),
    ))(input)
}

fn parse_key_value<'a>(input: &'a str) -> IResult<&str, (&str, Cow<'a, str>)> {
    use nom::character::streaming::char;

    separated_pair(parse_literal, char(':'), parse_string_or_literal)(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string() {
        fn assert_parse(text: &str, expected: &str) {
            let result = parse_string(&text).unwrap().1;
            assert_eq!(result, expected);
        }
        fn assert_parse_fails(text: &str) {
            assert!(parse_string(&text).is_err());
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
            let result = parse_key_value(&text).unwrap().1;
            assert_eq!(expected.0, result.0);
            assert_eq!(expected.1, result.1);
        }
        fn assert_parse_fails(text: &str) {
            assert!(parse_key_value(&text).is_err());
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
            let result = parse_literal(&text).unwrap().1;
            assert_eq!(result, expected);
        }
        fn assert_parse_fails(text: &str) {
            assert!(parse_literal(&text).is_err());
        }
        assert_parse("a", "a");
        assert_parse("abc", "abc");
        assert_parse("a:sd qwe", "a");
        assert_parse_fails(":sd qwe");
        assert_parse("m'lady", "m'lady");
        assert_parse_fails("'mlady");
    }
}
