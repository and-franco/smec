use super::*;

#[test]
fn insert_get() {
    let mut arena = GenArena::with_capacity(16);
    dbg!(&arena);
    assert_eq!(arena.push(10), Index::new(0, 0));
    assert_eq!(arena.push(9), Index::new(1, 0));
    assert_eq!(arena.push(8), Index::new(2, 0));
    assert_eq!(arena.get(Index::new(1, 0)), Some(&9));
    if let Some(x) = arena.get_mut(Index::new(2, 0)) {
        *x = 15
    }
    assert_eq!(arena.get(Index::new(2, 0)), Some(&15));
    assert_eq!(arena.len(), 3);
}

#[test]
fn insert_get_no_capacity() {
    let mut arena = GenArena::with_capacity(0);
    dbg!(&arena);
    assert_eq!(arena.push(10), Index::new(0, 0));
    assert_eq!(arena.push(9), Index::new(1, 0));
    assert_eq!(arena.push(8), Index::new(2, 0));
    assert_eq!(arena.get(Index::new(1, 0)), Some(&9));
}

#[test]
fn insert_get_full() {
    let mut arena = GenArena::with_capacity(8);
    for _ in 0..8 {
        arena.push(5);
    }
    assert_eq!(arena.capacity(), 8);
    assert_eq!(arena.len(), 8);
    for _ in 0..4 {
        arena.push(5);
    }
    assert!(arena.capacity() >= 12);
    assert_eq!(arena.len(), 12);
}

#[test]
fn iter() {
    let mut arena = GenArena::with_capacity(4);
    for i in 0..4 {
        arena.push(i as u64);
    }
    let mut iter = arena.iter();
    assert_eq!(iter.next(), Some((Index::new(0, 0), &0)));
    assert_eq!(iter.next(), Some((Index::new(1, 0), &1)));
    assert_eq!(iter.next(), Some((Index::new(2, 0), &2)));
    assert_eq!(iter.next(), Some((Index::new(3, 0), &3)));
    assert_eq!(iter.next(), None);
}

#[test]
fn removals() {
    let mut arena = GenArena::with_capacity(0);
    dbg!(&arena);
    let idx1 = arena.push(10);
    let idx2 = arena.push(9);
    let idx3 = arena.push(8);
    arena.remove(idx3);
    arena.remove(idx2);
    // deleting should return stored value
    assert_eq!(arena.remove(idx1), Some(10));
    // new pushes should have a new generation, and should be at the last place removed
    assert_eq!(arena.push(5), Index::new(0, 1));
    assert_eq!(arena.push(6), Index::new(1, 1));
    // getting the new generation should work
    assert_eq!(arena.get(Index::new(0, 1)), Some(&5));
    // getting non existing id should return None
    assert_eq!(arena.get(idx2), None);
    assert_eq!(arena.get(idx3), None);
    // getting an inex that exists but on a different generation should return None
    assert_eq!(arena.get(idx1), None);
    assert_eq!(arena.push(7), Index::new(2, 1));
    assert_eq!(arena.push(8), Index::new(3, 0));
    assert_eq!(arena.push(9), Index::new(4, 0));

}