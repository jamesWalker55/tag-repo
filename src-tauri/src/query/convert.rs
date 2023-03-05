// TODO: Make this module be able to handle complicated queries like in src/repo.rs:478

use super::parser::{parse, Expr};
use std::borrow::Cow;

#[derive(Debug)]
enum WhereClause<'a> {
    FTS(FTSPart<'a>),
    InPath(Cow<'a, str>),
    And(Vec<WhereClause<'a>>),
    Or(Vec<WhereClause<'a>>),
    Not(Box<WhereClause<'a>>),
}

#[derive(Debug)]
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
                WhereClause::And(sql_clauses)
            } else {
                // combine full text search (if any), then return along with sql statements
                let combined_fts_parts = FTSPart::combine_and(fts_parts);
                let combined_fts_clauses = WhereClause::FTS(combined_fts_parts);
                sql_clauses.push(combined_fts_clauses);
                WhereClause::And(sql_clauses)
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
                WhereClause::Or(sql_clauses)
            } else {
                // combine full text search (if any), then return along with sql statements
                let combined_fts_parts = FTSPart::combine_or(fts_parts);
                let combined_fts_clauses = WhereClause::FTS(combined_fts_parts);
                sql_clauses.push(combined_fts_clauses);
                WhereClause::Or(sql_clauses)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn my_test() {
        dbg!(generate_clause(
            parse("a -b inpath:123 | -c (d | -foo)").unwrap()
        ));
    }
}
