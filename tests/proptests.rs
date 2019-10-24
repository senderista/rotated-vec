// adapted from https://github.com/ssomers/rust_bench_btreeset_intersection/blob/master/src/tests/set.rs
extern crate proptest;
use self::proptest::prelude::*;
use rotated_vec::RotatedVec;
use std::cmp::min;

prop_compose! {
    fn arbitrary_instance()
                    (vec: Vec<u8>)
                    -> RotatedVec<u8>
    {
        vec.iter().cloned().collect()
    }
}

// note that we can return an index up to len() inclusive.
// this is necessary to provide a valid range to the RNG for empty instances.
prop_compose! {
    fn arbitrary_instance_with_index()
                    (vec in any::<Vec<u8>>())
                    (index in 0..=vec.len(), vec in Just(vec))
                    -> (RotatedVec<u8>, usize)
    {
        (vec.iter().cloned().collect(), index)
    }
}

proptest! {
    #[test]
    fn push_pop(mut v in arbitrary_instance(), x: u8) {
        v.push(x);
        prop_assert_eq!(v.pop().unwrap(), x);
    }

    #[test]
    fn insert_remove((mut v, i) in arbitrary_instance_with_index(), x: u8) {
        v.insert(i, x);
        prop_assert_eq!(v.remove(i), x);
    }

    #[test]
    fn compare_iter(v in arbitrary_instance()) {
        let iter = v.iter();
        for (i, &x) in iter.enumerate() {
            prop_assert_eq!(*v.get(i).unwrap(), x);
        }
    }

    #[test]
    fn compare_into_iter(v in arbitrary_instance()) {
        let mut iter = v.clone().into_iter();
        for i in 0..v.len() {
            prop_assert_eq!(*v.get(i).unwrap(), iter.next().unwrap());
        }
    }

    #[test]
    fn test_iter_overrides((v, i) in arbitrary_instance_with_index()) {
        let len = v.len();
        let index = if len > 0 { min(i, len - 1) } else { 0 };
        let last_index = if len > 0 { len - 1 } else { 0 };
        let iter = v.iter();
        prop_assert_eq!(iter.last(), v.get(last_index));
        prop_assert_eq!(iter.count(), len);
        let mut iter_nth = iter;
        prop_assert_eq!(iter_nth.nth(index), v.get(index));
        let mut iter_nth_back = iter;
        prop_assert_eq!(iter_nth_back.nth_back(index), v.get(last_index - index)
        );
        let mut iter_mut = v.iter();
        for j in 0..(len / 2) {
            prop_assert_eq!(iter_mut.next(), v.get(j));
            prop_assert_eq!(iter_mut.next_back(), v.get(last_index - j));
        }
        iter_mut.next();
        iter_mut.next_back();
        prop_assert!(iter_mut.next().is_none());
    }
}
