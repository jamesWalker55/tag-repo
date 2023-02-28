// use savage_core::expression::Expression;
// use savage_core::expression::Expression::*;

use std::collections::HashMap;

// fn var(name: &str) -> Box<Expression> {
//   Box::new(Variable(String::from(name)))
// }

// drum & ((inpath:kick fat) | (inpath:snare thin fat))
// FTS("drum") AND ((PATH="kick%" AND FTS("fat")) OR (PATH="snare%" AND FTS("thin fat")))

// a & b & p:1 & ~(p:2 & c)

fn main() {
  // let expr = And(
  //   var("drum"),
  //   Box::new(Or(
  //     Box::new(And(
  //       var("inpath:kick"),
  //       var("fat"))
  //     ),
  //     Box::new(And(
  //       var("inpath:snare"),
  //       Box::new(And(
  //         var("thin"),
  //         var("fat")
  //       ))
  //     ))
  //   ))
  // );
  // let expr = expr.evaluate(HashMap::new()).unwrap();
  // println!("{}", expr.to_string());
}
