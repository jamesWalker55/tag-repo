pub fn unordered_eq<'a, T, U, V>(a: T, b: U)
where
  T: Iterator<Item=V>,
  U: Iterator<Item=V>,
  V: Ord + std::fmt::Debug,
{
  let mut a: Vec<_> = a.collect();
  let mut b: Vec<_> = b.collect();
  a.sort();
  b.sort();
  assert_eq!(a, b);
}
