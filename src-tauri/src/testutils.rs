pub fn assert_unordered_eq<'a, T, U, V>(a: T, b: U)
where
    T: IntoIterator<Item = V>,
    U: IntoIterator<Item = V>,
    V: Ord + std::fmt::Debug,
{
    let mut a: Vec<_> = a.into_iter().collect();
    let mut b: Vec<_> = b.into_iter().collect();
    a.sort();
    b.sort();
    assert_eq!(a, b);
}
