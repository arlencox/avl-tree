use std::borrow::Borrow;

use bumpalo::Bump;

use crate::{
    iter::Iter,
    node::{Link, get, insert, iter, remove},
};

#[derive(Clone, Copy)]
pub struct AvlMap<'a, K, V> {
    root: Link<'a, K, V>,
}

impl<'a, K, V> AvlMap<'a, K, V> {
    pub fn new() -> Self {
        Self { root: None }
    }

    pub fn is_empty(&self) -> bool {
        self.root.is_none()
    }

    pub fn len(&self) -> usize {
        let mut length = 0usize;
        iter(self.root, |_, _| length += 1);
        length
    }

    pub fn iter(&self) -> Iter<'a, K, V> {
        Iter::new(self.root)
    }
}

impl<'a, K: Ord + Clone, V: Clone + PartialEq> AvlMap<'a, K, V> {
    pub fn insert(&mut self, bump: &'a Bump, key: K, val: V) -> bool {
        let new_root = Some(insert(bump, K::cmp, V::eq, key, val, self.root));
        let result = !node::link_eq(self.root, new_root);
        self.root = new_root;
        result
    }
}

impl<'a, K: Ord + Clone, V: Clone> AvlMap<'a, K, V> {
    pub fn remove<Q>(&mut self, bump: &'a Bump, key: &Q)
    where
        K: Borrow<Q>,
        Q: Ord,
    {
        self.root = remove(bump, |k| Q::cmp(key, k.borrow()), self.root);
    }
}

impl<'a, K: Ord, V> AvlMap<'a, K, V> {
    pub fn get<Q>(&self, key: &Q) -> Option<&'a V>
    where
        K: Borrow<Q>,
        Q: Ord,
    {
        get(self.root, |k| Q::cmp(key, k.borrow()))
    }
}

pub mod iter {
    use crate::node::{Link, Ref, height};

    pub struct Iter<'a, K, V> {
        stack: Vec<Ref<'a, K, V>>,
    }

    impl<'a, K, V> Iter<'a, K, V> {
        #[inline(always)]
        pub fn new(mut t: Link<'a, K, V>) -> Iter<'a, K, V> {
            let mut stack = Vec::with_capacity(height(t));
            while let Some(node) = t {
                stack.push(node);
                t = node.left;
            }
            Self { stack }
        }
    }

    impl<'a, K, V> Iterator for Iter<'a, K, V> {
        type Item = (&'a K, &'a V);
        fn next(&mut self) -> Option<Self::Item> {
            let Some(top) = self.stack.pop() else {
                return None;
            };
            let result = Some((&top.key, &top.val));

            let mut curr = top.right;
            while let Some(node) = curr {
                self.stack.push(node);
                curr = node.left;
            }

            result
        }
    }
}

pub mod node {
    use bumpalo::Bump;

    pub struct Node<'a, K, V> {
        pub key: K,
        pub val: V,
        pub height: usize,
        pub left: Link<'a, K, V>,
        pub right: Link<'a, K, V>,
    }

    pub type Ref<'a, K, V> = &'a Node<'a, K, V>;

    pub type Link<'a, K, V> = Option<Ref<'a, K, V>>;

    #[inline(always)]
    pub fn link_eq<'a, K, V>(a: Link<'a, K, V>, b: Link<'a, K, V>) -> bool {
        match (a, b) {
            (None, None) => true,
            (Some(a_ref), Some(b_ref)) => a_ref as *const _ == b_ref as *const _,
            (_, _) => false,
        }
    }

    #[inline(always)]
    pub fn height<'a, K, V>(l: Link<'a, K, V>) -> usize {
        match l {
            Some(node) => node.height,
            None => 0,
        }
    }

    #[inline(always)]
    fn create<'a, K, V>(
        bump: &'a Bump,
        l: Link<'a, K, V>,
        k: K,
        v: V,
        r: Link<'a, K, V>,
    ) -> Ref<'a, K, V> {
        let hl = height(l);
        let hr = height(r);
        bump.alloc(Node {
            key: k,
            val: v,
            height: hl.max(hr) + 1,
            left: l,
            right: r,
        })
    }

    #[inline(always)]
    fn balance<'a, K: Clone, V: Clone>(
        bump: &'a Bump,
        l: Link<'a, K, V>,
        x: K,
        d: V,
        r: Link<'a, K, V>,
    ) -> Ref<'a, K, V> {
        let hl = height(l);
        let hr = height(r);
        if hl > hr + 2 {
            match l {
                None => panic!("invalid_arg"),
                Some(Node {
                    key: lv,
                    val: ld,
                    left: ll,
                    right: lr,
                    ..
                }) => {
                    if height(*ll) >= height(*lr) {
                        create(
                            bump,
                            *ll,
                            lv.clone(),
                            ld.clone(),
                            Some(create(bump, *lr, x, d, r)),
                        )
                    } else {
                        match *lr {
                            None => panic!("invalid_arg"),
                            Some(Node {
                                key: lrv,
                                val: lrd,
                                left: lrl,
                                right: lrr,
                                ..
                            }) => create(
                                bump,
                                Some(create(bump, *ll, lv.clone(), ld.clone(), *lrl)),
                                lrv.clone(),
                                lrd.clone(),
                                Some(create(bump, *lrr, x, d, r)),
                            ),
                        }
                    }
                }
            }
        } else if hr > hl + 2 {
            match r {
                None => panic!("invalid_arg"),
                Some(Node {
                    left: rl,
                    key: rv,
                    val: rd,
                    right: rr,
                    ..
                }) => {
                    if height(*rr) >= height(*rl) {
                        create(
                            bump,
                            Some(create(bump, l, x, d, *rl)),
                            rv.clone(),
                            rd.clone(),
                            *rr,
                        )
                    } else {
                        match *rl {
                            None => panic!("invalid_arg"),
                            Some(Node {
                                left: rll,
                                key: rlv,
                                val: rld,
                                right: rlr,
                                ..
                            }) => create(
                                bump,
                                Some(create(bump, l, x, d, *rll)),
                                rlv.clone(),
                                rld.clone(),
                                Some(create(bump, *rlr, rv.clone(), rd.clone(), *rr)),
                            ),
                        }
                    }
                }
            }
        } else {
            create(bump, l, x, d, r)
        }
    }

    pub fn insert<'a, K: Clone, V: Clone>(
        bump: &'a Bump,
        key_compare: impl Fn(&K, &K) -> std::cmp::Ordering,
        val_equals: impl Fn(&V, &V) -> bool,
        ins_key: K,
        ins_val: V,
        tree: Link<'a, K, V>,
    ) -> Ref<'a, K, V> {
        match tree {
            None => create(bump, None, ins_key, ins_val, None),
            Some(
                node @ Node {
                    left,
                    key,
                    val,
                    right,
                    ..
                },
            ) => {
                let c = key_compare(&ins_key, key);
                match c {
                    std::cmp::Ordering::Equal => {
                        if val_equals(&ins_val, val) {
                            node
                        } else {
                            create(bump, *left, ins_key, ins_val, *right)
                        }
                    }
                    std::cmp::Ordering::Less => {
                        let ll = insert(bump, key_compare, val_equals, ins_key, ins_val, *left);
                        if link_eq(Some(ll), *left) {
                            node
                        } else {
                            balance(bump, Some(ll), key.clone(), val.clone(), *right)
                        }
                    }
                    std::cmp::Ordering::Greater => {
                        let rr = insert(bump, key_compare, val_equals, ins_key, ins_val, *right);
                        if link_eq(Some(rr), *right) {
                            node
                        } else {
                            balance(bump, *left, key.clone(), val.clone(), Some(rr))
                        }
                    }
                }
            }
        }
    }

    pub fn min_binding<'a, K, V>(t: Link<'a, K, V>) -> Option<(&'a K, &'a V)> {
        let Some(mut t) = t else {
            return None;
        };
        loop {
            match t.left {
                None => return Some((&t.key, &t.val)),
                Some(left) => t = left,
            }
        }
    }

    fn remove_min_binding<'a, K: Clone, V: Clone>(
        bump: &'a Bump,
        t: Link<'a, K, V>,
    ) -> Link<'a, K, V> {
        match t {
            None => panic!("invalid_arg"),
            Some(Node {
                left: None, right, ..
            }) => *right,
            Some(Node {
                left,
                key,
                val,
                right,
                ..
            }) => Some(balance(
                bump,
                remove_min_binding(bump, *left),
                key.clone(),
                val.clone(),
                *right,
            )),
        }
    }

    fn merge<'a, K: Clone, V: Clone>(
        bump: &'a Bump,
        t1: Link<'a, K, V>,
        t2: Link<'a, K, V>,
    ) -> Link<'a, K, V> {
        match (t1, t2) {
            (None, t2) => t2,
            (t1, None) => t1,
            (Some(t1), Some(t2)) => {
                let (key, val) = min_binding(Some(t2)).unwrap();
                Some(balance(
                    bump,
                    Some(t1),
                    key.clone(),
                    val.clone(),
                    remove_min_binding(bump, Some(t2)),
                ))
            }
        }
    }

    pub fn remove<'a, K: Clone, V: Clone>(
        bump: &'a Bump,
        compare: impl Fn(&K) -> std::cmp::Ordering,
        t: Link<'a, K, V>,
    ) -> Link<'a, K, V> {
        let Some(Node {
            left,
            key,
            val,
            right,
            ..
        }) = t
        else {
            return None;
        };

        match compare(key) {
            std::cmp::Ordering::Equal => merge(bump, *left, *right),
            std::cmp::Ordering::Less => {
                let new_left = remove(bump, compare, *left);
                if link_eq(new_left, *left) {
                    t
                } else {
                    Some(balance(bump, new_left, key.clone(), val.clone(), *right))
                }
            }
            std::cmp::Ordering::Greater => {
                let new_right = remove(bump, compare, *right);
                if link_eq(new_right, *right) {
                    t
                } else {
                    Some(balance(bump, *left, key.clone(), val.clone(), new_right))
                }
            }
        }
    }

    pub fn get<'a, K, V>(
        mut t: Link<'a, K, V>,
        compare: impl Fn(&K) -> std::cmp::Ordering,
    ) -> Option<&'a V> {
        while let Some(node) = t {
            match compare(&node.key) {
                std::cmp::Ordering::Equal => return Some(&node.val),
                std::cmp::Ordering::Less => t = node.left,
                std::cmp::Ordering::Greater => t = node.right,
            }
        }
        None
    }

    pub fn iter<'a, K, V>(t: Link<'a, K, V>, mut f: impl FnMut(&K, &V)) {
        pub fn iter_rec<'a, K, V>(t: Link<'a, K, V>, f: &mut impl FnMut(&'a K, &'a V)) {
            match t {
                None => (),
                Some(Node {
                    key,
                    val,
                    left,
                    right,
                    ..
                }) => {
                    iter_rec(*left, f);
                    f(key, val);
                    iter_rec(*right, f);
                }
            }
        }
        iter_rec(t, &mut f);
    }
}

pub fn insert_i32<'a>(m: &mut AvlMap<'a, i32, i32>, bump: &'a Bump, k: i32, v: i32) {
    m.insert(bump, k, v);
}

#[cfg(test)]
mod tests {
    use crate::*;
    use bumpalo::Bump;

    #[test]
    fn empty_len() {
        let map: AvlMap<i32, i32> = AvlMap::new();
        assert_eq!(map.len(), 0);
    }

    #[test]
    fn insert_get() {
        let bump = Bump::new();
        let mut map: AvlMap<i32, i32> = AvlMap::new();
        for i in 0..50 {
            map.insert(&bump, i, i);
        }

        for i in 0..50 {
            assert_eq!(map.get(&i), Some(&i));
        }
    }

    #[test]
    fn insert_iter() {
        let bump = Bump::new();
        let mut map: AvlMap<usize, usize> = AvlMap::new();
        for i in 0..50 {
            map.insert(&bump, i, i);
        }

        for (i, (&j, &k)) in map.iter().enumerate() {
            assert_eq!(i, j);
            assert_eq!(i, k);
        }
    }

    #[test]
    fn remove_missing_lesser() {
        let bump = Bump::new();
        let mut map: AvlMap<i32, i32> = AvlMap::new();
        map.insert(&bump, 1, 1);
        assert_eq!(map.len(), 1);
        map.remove(&bump, &0);
        assert_eq!(map.len(), 1);
    }

    #[test]
    fn remove_missing_greater() {
        let bump = Bump::new();
        let mut map: AvlMap<i32, i32> = AvlMap::new();
        map.insert(&bump, 1, 1);
        assert_eq!(map.len(), 1);
        map.remove(&bump, &2);
        assert_eq!(map.len(), 1);
    }

    #[test]
    fn insert_remove() {
        let bump = Bump::new();
        let mut map: AvlMap<i32, i32> = AvlMap::new();
        for i in 0..50 {
            map.insert(&bump, i, i);
        }

        assert_eq!(map.len(), 50);

        //map.remove(&bump, &25);

        //assert_eq!(map.len(), 49);

        for i in 0..50 {
            map.remove(&bump, &(i * 2 + 1));
        }

        assert_eq!(map.len(), 25);

        for i in 0..50 {
            if i % 2 != 0 {
                continue;
            }
            assert_eq!(map.get(&i), Some(&i));
        }
    }
}
