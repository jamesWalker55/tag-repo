// TODO: Make this module be able to handle complicated queries like in src/repo.rs:478

use super::parser::{parse, Expr};
use crate::helpers::sql::escape_fts5_string;
use itertools::Itertools;
use std::borrow::{Borrow, Cow};
use std::cmp::Ordering;

#[derive(Debug, PartialEq, Eq)]
enum WhereClause<'a> {
    FTS(FTSPart<'a>),
    InPath(Cow<'a, str>),
    And(Vec<WhereClause<'a>>),
    Or(Vec<WhereClause<'a>>),
    Not(Box<WhereClause<'a>>),
}

#[derive(Debug, PartialEq, Eq)]
enum FTSPart<'a> {
    Phrase(Cow<'a, str>),
    And(Vec<FTSPart<'a>>),
    Or(Vec<FTSPart<'a>>),
    Not(Box<FTSPart<'a>>),
}

impl<'a> FTSPart<'a> {
    fn combine_and(mut parts: Vec<FTSPart>) -> FTSPart {
        if parts.len() == 1 {
            parts.pop().unwrap()
        } else if parts.len() == 0 {
            panic!();
        } else {
            let mut group = vec![];
            for part in parts {
                if let FTSPart::And(subparts) = part {
                    group.extend(subparts);
                } else {
                    group.push(part);
                }
            }
            FTSPart::And(group)
        }
    }

    fn combine_or(mut parts: Vec<FTSPart>) -> FTSPart {
        if parts.len() == 1 {
            parts.pop().unwrap()
        } else if parts.len() == 0 {
            panic!();
        } else {
            let mut group = vec![];
            for part in parts {
                if let FTSPart::Or(subparts) = part {
                    group.extend(subparts);
                } else {
                    group.push(part);
                }
            }
            FTSPart::Or(group)
        }
    }

    fn to_fts_query(&self) -> String {
        use FTSPart::*;

        match self {
            Phrase(name) => {
                format!("tags:\"{}\"", escape_fts5_string(name.as_ref()))
            }
            And(parts) => {
                let mut parts_contain_pos = false;
                let mut parts_contain_neg = false;
                for p in parts {
                    if parts_contain_pos && parts_contain_neg {
                        break;
                    }

                    if matches!(p, Not(_)) {
                        parts_contain_neg = true;
                    } else {
                        parts_contain_pos = true;
                    }
                }

                if parts_contain_pos && parts_contain_neg {
                    // both positive and negative terms
                    let mut parts: Vec<&FTSPart> = parts.into_iter().collect();
                    parts.sort_by(|x, y| {
                        let x_is_neg = matches!(x, Not(_));
                        let y_is_neg = matches!(y, Not(_));
                        if x_is_neg == y_is_neg {
                            Ordering::Equal
                        } else if x_is_neg {
                            Ordering::Greater
                        } else {
                            Ordering::Less
                        }
                    });
                    let mut result = String::from("(");
                    for (i, part) in parts.iter().enumerate() {
                        if let Not(inner) = part {
                            // is negative
                            result.push_str(" NOT ");
                            result.push_str(inner.to_fts_query().as_str());
                        } else {
                            // is positive
                            if i == 0 {
                                // first element
                                result.push_str(part.to_fts_query().as_str());
                            } else {
                                // other elements
                                result.push_str(" AND ");
                                result.push_str(part.to_fts_query().as_str());
                            }
                        }
                    }
                    result.push(')');
                    result
                } else if parts_contain_pos {
                    // only positive terms
                    format!("({})", parts.iter().map(|x| x.to_fts_query()).join(" AND "))
                } else {
                    // only negative terms
                    format!(
                        r#"(meta_tags:"all" NOT {})"#,
                        parts
                            .iter()
                            .map(|x| {
                                if let Not(inner) = x {
                                    inner.to_fts_query()
                                } else {
                                    unreachable!("There should only be negative terms here")
                                }
                            })
                            .join(" NOT ")
                    )
                }
            }
            Or(parts) => {
                format!("({})", parts.iter().map(|x| x.to_fts_query()).join(" OR "))
            }
            Not(part) => {
                format!(r#"(meta_tags:"all" NOT {})"#, part.to_fts_query())
            }
        }
    }
}

/// The main endpoint of this module.
/// This receives the root of an expression tree and generates SQL where clauses.
///
/// NOTE: This assumes all AND and OR groups don't have nested groups of the same type. i.e. An
/// AND group doesn't directly contain another AND group, but may contain an OR group (which can
/// contain an AND group).
fn generate_clause<'a>(root: &'a Expr<'a>) -> WhereClause<'a> {
    match root {
        Expr::And(exprs) => {
            // this vector must be non-empty
            assert!(exprs.len() > 0, "Somehow got 0 items in this expr");

            // vector for storing all FTS clauses, for combining later
            let mut fts_parts = vec![];
            // vector for normal SQL clauses, like `path = "..."`
            let mut sql_clauses = vec![];
            for expr in exprs {
                match generate_clause(expr) {
                    WhereClause::FTS(query) => fts_parts.push(query),
                    subclause => sql_clauses.push(subclause),
                }
            }

            if fts_parts.len() == 0 {
                // no full text search, just return the sql statements in an OR group
                if sql_clauses.len() == 1 {
                    sql_clauses.pop().unwrap()
                } else {
                    WhereClause::And(sql_clauses)
                }
            } else {
                // combine full text search (if any), then return along with sql statements
                let combined_fts_parts = FTSPart::combine_and(fts_parts);
                let combined_fts_clause = WhereClause::FTS(combined_fts_parts);
                sql_clauses.insert(0, combined_fts_clause);
                if sql_clauses.len() == 1 {
                    sql_clauses.pop().unwrap()
                } else {
                    WhereClause::And(sql_clauses)
                }
            }
        }
        Expr::Or(exprs) => {
            // this vector must be non-empty
            assert!(exprs.len() > 0, "Somehow got 0 items in this expr");

            // vector for storing all FTS clauses, for combining later
            let mut fts_parts = vec![];
            // vector for normal SQL clauses, like `path = "..."`
            let mut sql_clauses = vec![];
            for expr in exprs {
                match generate_clause(expr) {
                    WhereClause::FTS(query) => fts_parts.push(query),
                    subclause => sql_clauses.push(subclause),
                }
            }

            if fts_parts.len() == 0 {
                // no full text search, just return the sql statements in an OR group
                if sql_clauses.len() == 0 {
                    sql_clauses.pop().unwrap()
                } else {
                    WhereClause::Or(sql_clauses)
                }
            } else {
                // combine full text search (if any), then return along with sql statements
                let combined_fts_parts = FTSPart::combine_or(fts_parts);
                let combined_fts_clause = WhereClause::FTS(combined_fts_parts);
                sql_clauses.insert(0, combined_fts_clause);
                if sql_clauses.len() == 1 {
                    sql_clauses.pop().unwrap()
                } else {
                    WhereClause::Or(sql_clauses)
                }
            }
        }
        Expr::Not(expr) => {
            let clause = generate_clause(expr);
            if let WhereClause::FTS(ftspart) = clause {
                WhereClause::FTS(FTSPart::Not(Box::new(ftspart)))
            } else {
                WhereClause::Not(Box::new(clause))
            }
        }
        Expr::Tag(name) => {
            let name: &str = name.borrow();
            WhereClause::FTS(FTSPart::Phrase(Cow::from(name)))
        }
        Expr::KeyValue(key, val) => match key.as_ref() {
            "inpath" => {
                let val: &str = val.borrow();
                WhereClause::InPath(Cow::from(val))
            }
            _ => panic!(
                "Unrecognised key-value pair received: {:?} = {:?}",
                key, val
            ),
        },
    }
}

#[rustfmt::skip]
#[cfg(test)]
mod test_clauses {
    use super::*;

    fn fts(part: FTSPart) -> WhereClause { WhereClause::FTS(part) }
    fn inpath(path: &str) -> WhereClause { WhereClause::InPath(Cow::from(path)) }
    fn and(clauses: Vec<WhereClause>) -> WhereClause { WhereClause::And(clauses) }
    fn or(clauses: Vec<WhereClause>) -> WhereClause { WhereClause::Or(clauses) }
    fn not(clause: WhereClause) -> WhereClause { WhereClause::Not(Box::new(clause)) }

    fn ftsphrase(name: &str) -> FTSPart { FTSPart::Phrase(Cow::from(name)) }
    fn ftsand(clauses: Vec<FTSPart>) -> FTSPart { FTSPart::And(clauses) }
    fn ftsor(clauses: Vec<FTSPart>) -> FTSPart { FTSPart::Or(clauses) }
    fn ftsnot(clause: FTSPart) -> FTSPart { FTSPart::Not(Box::new(clause)) }

    fn assert_clause(query: &str, expected: WhereClause) {
        let expr = parse(query).unwrap();
        let clause = generate_clause(&expr);
        assert_eq!(clause, expected);
    }

    #[test]
    fn fts_1() {
        assert_clause(
            "a b c",
            fts(ftsand(vec![ftsphrase("a"), ftsphrase("b"), ftsphrase("c")])),
        );
    }

    #[test]
    fn fts_2() {
        assert_clause(
            "a | b -c",
            fts(
                ftsor(vec![
                    ftsphrase("a"),
                    ftsand(vec![
                        ftsphrase("b"),
                        ftsnot(ftsphrase("c")),
                    ]),
                ])
            ),
        );
    }

    #[test]
    fn fts_3() {
        assert_clause(
            "(a | b) c",
            fts(
                ftsand(vec![
                    ftsor(vec![
                        ftsphrase("a"),
                        ftsphrase("b"),
                    ]),
                ftsphrase("c"),
            ])),
        );
    }

    #[test]
    fn fts_4() {
        assert_clause(
            "-(a | b c) d | e",
            fts(
                ftsor(vec![
                    ftsand(vec![
                        ftsnot(
                            ftsor(vec![
                                ftsphrase("a"),
                                ftsand(vec![
                                    ftsphrase("b"),
                                    ftsphrase("c"),
                                ])
                            ])
                        ),
                        ftsphrase("d"),
                    ]),
                    ftsphrase("e"),
                ]),
            ),
        );
    }

    #[test]
    fn fts_5() {
        assert_clause(
            "a",
            fts(ftsphrase("a")),
        );
    }

    #[test]
    fn inpath_1() {
        assert_clause(
            "inpath:a",
            inpath("a"),
        );
    }

    #[test]
    fn inpath_2() {
        assert_clause(
            "inpath:a inpath:b inpath:c",
            and(vec![inpath("a"), inpath("b"), inpath("c")]),
        );
    }

    #[test]
    fn inpath_3() {
        assert_clause(
            "inpath:a | inpath:b inpath:c",
            or(vec![inpath("a"), and(vec![inpath("b"), inpath("c")])]),
        );
    }

    #[test]
    fn inpath_4() {
        assert_clause(
            "(inpath:a | inpath:b) inpath:c",
            and(vec![or(vec![inpath("a"), inpath("b")]), inpath("c")]),
        );
    }

    #[test]
    fn inpath_5() {
        assert_clause(
            "-(inpath:a | -inpath:b) inpath:c",
            and(vec![not(or(vec![inpath("a"), not(inpath("b"))])), inpath("c")]),
        );
    }

    #[test]
    fn inpath_6() {
        assert_clause(
            "(((inpath:a inpath:b))) inpath:c",
            and(vec![inpath("a"), inpath("b"), inpath("c")]),
        );
    }

    #[test]
    fn common_1() {
        assert_clause(
            "a b inpath:c",
            and(vec![
                fts(ftsand(vec![ftsphrase("a"), ftsphrase("b")])),
                inpath("c"),
            ]),
        );
    }

    #[test]
    fn common_2() {
        assert_clause(
            "a | b -inpath:c",
            or(vec![
                fts(ftsphrase("a")),
                and(vec![
                    fts(ftsphrase("b")),
                    not(inpath("c")),
                ]),
            ]),
        );
    }

    #[test]
    fn common_3() {
        assert_clause(
            "a -(b e inpath:1) | -d e inpath:0",
            or(vec![
                and(vec![
                    fts(ftsphrase("a")),
                    not(and(vec![
                        fts(ftsand(vec![ftsphrase("b"), ftsphrase("e")])),
                        inpath("1"),
                    ])),
                ]),
                and(vec![
                    fts(ftsand(vec![
                        ftsnot(ftsphrase("d")),
                        ftsphrase("e"),
                    ])),
                    inpath("0"),
                ]),
            ]),
        );
    }
}

#[rustfmt::skip]
#[cfg(test)]
mod test_fts_query {
    use super::*;

    fn assert_fts_statement(query: &str, expected: &str) {
        let expr = parse(query).unwrap();
        let clause = generate_clause(&expr);
        if let WhereClause::FTS(ftspart) = clause {
            let fts_query = ftspart.to_fts_query();
            println!("{}", fts_query);
            assert_eq!(fts_query, expected);
        } else {
            panic!("Query isn't a pure FTS query: {}", query);
        }
    }

    #[test]
    fn and_1() { assert_fts_statement(
        "a b c",
        r#"(tags:"a" AND tags:"b" AND tags:"c")"#); }

    #[test]
    fn and_2() { assert_fts_statement(
        "a q ((b c))",
        r#"(tags:"a" AND tags:"q" AND tags:"b" AND tags:"c")"#); }

    #[test]
    fn or_1() { assert_fts_statement(
        "a | b c",
        r#"(tags:"a" OR (tags:"b" AND tags:"c"))"#); }

    #[test]
    fn or_2() { assert_fts_statement(
        "(a | b) c",
        r#"((tags:"a" OR tags:"b") AND tags:"c")"#); }

    #[test]
    fn neg_1() { assert_fts_statement(
        "a -b",
        r#"(tags:"a" NOT tags:"b")"#); }

    #[test]
    fn neg_2() { assert_fts_statement(
        "a -b -c d",
        r#"(tags:"a" AND tags:"d" NOT tags:"b" NOT tags:"c")"#); }

    #[test]
    fn neg_3() { assert_fts_statement(
        "-b -c d",
        r#"(tags:"d" NOT tags:"b" NOT tags:"c")"#); }

    #[test]
    fn neg_4() { assert_fts_statement(
        "-b -c",
        r#"(meta_tags:"all" NOT tags:"b" NOT tags:"c")"#); }

    #[test]
    fn neg_5() { assert_fts_statement(
        "(-a b) -(c d) | -e",
        r#"((tags:"b" NOT tags:"a" NOT (tags:"c" AND tags:"d")) OR (meta_tags:"all" NOT tags:"e"))"#); }

    #[test]
    fn complex_1() { assert_fts_statement(
        "-(a | b a -c -d) d | e",
        r#"((tags:"d" NOT (tags:"a" OR (tags:"b" AND tags:"a" NOT tags:"c" NOT tags:"d"))) OR tags:"e")"#); }
}
