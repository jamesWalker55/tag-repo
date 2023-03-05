// TODO: Make this module be able to handle complicated queries like in src/repo.rs:478

use super::parser::{parse, Expr};
use std::borrow::Cow;

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
}

/// The main endpoint of this module.
/// This receives the root of an expression tree and generates SQL where clauses.
fn generate_clause(root: Expr) -> WhereClause {
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
            let clause = generate_clause(*expr);
            if let WhereClause::FTS(ftspart) = clause {
                WhereClause::FTS(FTSPart::Not(Box::new(ftspart)))
            } else {
                WhereClause::Not(Box::new(clause))
            }
        }
        Expr::Tag(name) => WhereClause::FTS(FTSPart::Phrase(name)),
        Expr::KeyValue(key, val) => match key.as_ref() {
            "inpath" => WhereClause::InPath(val.into()),
            _ => panic!(
                "Unrecognised key-value pair received: {:?} = {:?}",
                key, val
            ),
        },
    }
}

#[rustfmt::skip]
#[cfg(test)]
mod tests {
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
        let clause = generate_clause(expr);
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
