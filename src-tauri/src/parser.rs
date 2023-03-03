use nom::branch::alt;
use nom::bytes::complete::{is_not, tag as nom_tag, take_till, take_while};
use nom::character::complete::{none_of, one_of};
use nom::character::is_space;
use nom::character::streaming::char as nom_char;
use nom::combinator::{map, recognize, value};
use nom::multi::fold_many0;
use nom::sequence::{delimited, pair, separated_pair};
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
    recognize(pair(none_of("\"': "), take_till(|x| x == ' ' || x == ':')))(input)
}

fn string_or_literal<'a>(input: &'a str) -> IResult<&str, Cow<'a, str>> {
    alt((
        map(string, |x| Cow::from(x)),
        map(literal, |x| Cow::from(x)),
    ))(input)
}

enum Expr<'a> {
    And(Vec<Expr<'a>>),
    Or(Vec<Expr<'a>>),
    Not(Box<Expr<'a>>),
    Tag(Cow<'a, str>),
    KeyValue(Cow<'a, str>, Cow<'a, str>),
}

fn parse_tag(input: &str) -> IResult<&str, Expr> {
    map(string_or_literal, Expr::Tag)(input)
}

fn key_val<'a>(input: &'a str) -> IResult<&str, Expr<'a>> {
    map(
        separated_pair(literal, nom_char(':'), string_or_literal),
        |(k, v)| Expr::KeyValue(Cow::from(k), v),
    )(input)
}

fn parens(input: &str) -> IResult<&str, Expr> {
    delimited(nom_char('('), parse_expr, nom_char(')'))(input)
}

// fn parse_implicit_and_group(input: &str) -> IResult<&str, Expr> {
//     alt((
//         delimited(take_while(is_space), parse_tag, take_while(is_space)),
//         delimited(take_while(is_space), parse_key_value, take_while(is_space)),
//     ))(input)
// }

fn parse_expr(input: &str) -> IResult<&str, Expr> {
    alt((parens, key_val, parse_tag))(input)
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
            let result = parse_tag(&text).unwrap().1;
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

    // #[test]
    // fn test_tag() {
    //     fn assert_parse(text: &str, expected: &str) {
    //         let result = parse_tag(&text).unwrap().1;
    //         assert_eq!(result, expected);
    //     }
    //     fn assert_parse_fails(text: &str) {
    //         assert!(parse_tag(&text).is_err());
    //     }
    //     assert_parse("a", "a");
    //     assert_parse("abc", "abc");
    //     assert_parse("mc'donalds", "mc'donalds");
    //     assert_parse("'tag with spaces'", "tag with spaces");
    // }
}
