//! Tests for the handle shapes and the bitset.

use std::collections::HashSet;

use proptest::prelude::*;

use super::{CommitHandle, HandleSet, Parents};

fn h(n: u32) -> CommitHandle {
    CommitHandle(n)
}

#[test]
fn parents_collect_into_the_inline_shape_for_their_count() {
    assert_eq!(std::iter::empty().collect::<Parents>(), Parents::None);
    assert_eq!([h(1)].into_iter().collect::<Parents>(), Parents::One(h(1)));
    assert_eq!(
        [h(1), h(2)].into_iter().collect::<Parents>(),
        Parents::Two(h(1), h(2))
    );
    assert_eq!(
        [h(1), h(2), h(3)].into_iter().collect::<Parents>(),
        Parents::Many(vec![h(1), h(2), h(3)])
    );
}

#[test]
fn parents_read_back_in_order_whatever_the_shape() {
    for count in 0..5 {
        let handles: Vec<_> = (0..count).map(|n| h(n * 7)).collect();
        let parents: Parents = handles.iter().copied().collect();
        assert_eq!(parents.iter().collect::<Vec<_>>(), handles);
        assert_eq!(parents.len(), handles.len());
        assert_eq!(parents.is_empty(), handles.is_empty());
        assert_eq!(parents.first(), handles.first().copied());
    }
}

#[test]
fn an_empty_set_allocates_nothing() {
    let set = HandleSet::new();
    assert!(set.is_empty());
    assert!(!set.contains(h(u32::MAX)));
    assert_eq!(set.approximate_bytes(), 0);
}

#[test]
fn a_set_costs_a_bit_per_handle_up_to_the_largest() {
    let mut set = HandleSet::new();
    set.insert(h(999_999));
    assert!(set.approximate_bytes() <= 1_000_000 / 8 + 8);
}

#[derive(Clone, Debug)]
enum Op {
    Insert(u32),
    Remove(u32),
    Contains(u32),
}

fn op() -> impl Strategy<Value = Op> {
    let handle = 0u32..2_000;
    prop_oneof![
        handle.clone().prop_map(Op::Insert),
        handle.clone().prop_map(Op::Remove),
        handle.prop_map(Op::Contains),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn a_handle_set_behaves_like_a_hash_set(ops in proptest::collection::vec(op(), 0..400)) {
        let mut set = HandleSet::new();
        let mut model = HashSet::new();
        for op in ops {
            match op {
                Op::Insert(n) => prop_assert_eq!(set.insert(h(n)), model.insert(n)),
                Op::Remove(n) => prop_assert_eq!(set.remove(h(n)), model.remove(&n)),
                Op::Contains(n) => prop_assert_eq!(set.contains(h(n)), model.contains(&n)),
            }
            prop_assert_eq!(set.len(), model.len());
            prop_assert_eq!(set.is_empty(), model.is_empty());
        }
    }
}
