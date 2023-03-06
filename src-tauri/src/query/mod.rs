mod convert;
mod parser;

use parser::ParseError;

pub(crate) fn to_sql(query: &str) -> Result<String, ParseError> {
    let expr = parser::parse(query)?;
    let clause = convert::generate_clause(&expr);
    Ok(clause.to_sql_clause())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn common_1() {
        assert_eq!(
            to_sql("a b c").unwrap(),
            r#"tq.tag_query = '(tags:"a" AND tags:"b" AND tags:"c")'"#,
        )
    }

    #[test]
    fn common_2() {
        assert_eq!(
            to_sql("a -b in:samples/").unwrap(),
            r#"(i.id IN (SELECT id FROM tag_query('(tags:"a" NOT tags:"b")')) AND i.path LIKE 'samples/%' ESCAPE '\')"#,
        )
    }
}
