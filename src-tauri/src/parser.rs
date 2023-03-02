use pest::Parser;

#[derive(Parser)]
#[grammar = "parser.pest"]
pub struct QueryParser;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn my_test() {
        let successful_parse = QueryParser::parse(Rule::tag, "-273.15");
        println!("{:?}", successful_parse);

        let unsuccessful_parse = QueryParser::parse(Rule::tag, "this is not a number");
        println!("{:?}", unsuccessful_parse);

        let unsuccessful_parse = QueryParser::parse(Rule::tag, "你好　阿");
        println!("{:?}", unsuccessful_parse);
    }

    #[test]
    fn test_string() {
        fn assert_parse(text: &str, expected: &str) {
            let x = QueryParser::parse(Rule::string, text).unwrap();
            x.
            dbg!(x);
            // let result = parse_string(&text).unwrap().1;
            // assert_eq!(result, expected);
        }
        fn assert_parse_fails(text: &str) {
            // assert!(parse_string(&text).is_err());
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

    // #[test]
    // fn test_key_value() {
    //     fn assert_parse(text: &str, expected: (&str, &str)) {
    //         let result = parse_key_value(&text).unwrap().1;
    //         assert_eq!(expected.0, result.0);
    //         assert_eq!(expected.1, result.1);
    //     }
    //     fn assert_parse_fails(text: &str) {
    //         assert!(parse_key_value(&text).is_err());
    //     }
    //
    //     assert_parse(r#"inpath:src/"#, ("inpath", "src/"));
    //     assert_parse(
    //         r#"inpath:"D:/Audio Samples/""#,
    //         ("inpath", "D:/Audio Samples/"),
    //     );
    //     assert_parse(
    //         r#"inpath:"quote in path for some reason""""#,
    //         ("inpath", "quote in path for some reason\""),
    //     );
    //     assert_parse_fails(r#""spaced key":hello"#);
    // }
    //
    // #[test]
    // fn test_literal() {
    //     fn assert_parse(text: &str, expected: &str) {
    //         let result = parse_literal(&text).unwrap().1;
    //         assert_eq!(result, expected);
    //     }
    //     fn assert_parse_fails(text: &str) {
    //         assert!(parse_literal(&text).is_err());
    //     }
    //     assert_parse("a", "a");
    //     assert_parse("abc", "abc");
    //     assert_parse("a:sd qwe", "a");
    //     assert_parse_fails(":sd qwe");
    //     assert_parse("m'lady", "m'lady");
    //     assert_parse_fails("'mlady");
    // }
    //
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
