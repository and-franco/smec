use super::{GenArena, Index, Entry};

impl<'a, T> IntoIterator for &'a GenArena<T> {
    type Item = (Index, &'a T);
    type IntoIter = Iter<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut GenArena<T> {
    type Item = (Index, &'a mut T);
    type IntoIter = IterMut<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

#[derive(Debug, Clone)]
pub struct Iter<'a, T> {
    pub (super) entries: &'a [Entry<T>],
    /// The total length, including Free + Occupied. Used for ExactSizeIterator
    pub (super) tot_length: usize,
    pub (super) seen: usize,
    pub (super) curr: usize,
}

impl <'a, T> Iterator for Iter<'a, T> {
    type Item = (Index, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        for i in self.curr..self.entries.len() {
            self.curr += 1;
            if let Entry::Occupied { generation, value } = &self.entries[i] {
                self.seen += 1;
                return Some((Index::new(i, *generation), value));
            } else {
                continue;
            }
        }
        None
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.tot_length.saturating_sub(self.seen);
        (remaining, Some(remaining))
    }
}

impl<'a, T> ExactSizeIterator for Iter<'a, T> {
    fn len(&self) -> usize {
        self.tot_length
    }
}

#[derive(Debug)]
pub struct IterMut<'a, T> {
    pub (super) entries: &'a mut [Entry<T>],
    /// The total length, including Free + Occupied. Used for ExactSizeIterator
    pub (super) tot_length: usize,
    pub (super) curr: usize,
    pub (super) seen: usize,
}

impl <'a, T> Iterator for IterMut<'a, T> {
    type Item = (Index, &'a mut T);

    fn next(&mut self) -> Option<Self::Item> {
        for i in self.curr..self.entries.len() {
            self.curr += 1;
            if let Entry::Occupied { generation, value } = &mut self.entries[i] {
                self.seen += 1;

                // this unsafe code is necessary (as it is in general to have IterMut iterators)
                // because otherwise we get borrow errors.
                // here we can say that 2 .next() will never call the 2 same value because self.curr
                // increments every loop
                #[allow(unsafe_code)]
                let value = unsafe { &mut *(value as *mut _) };
                return Some((Index::new(i, *generation), value));
            } else {
                continue;
            }
        }
        None
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.tot_length.saturating_sub(self.seen);
        (remaining, Some(remaining))
    }
}

impl<'a, T> ExactSizeIterator for IterMut<'a, T> {
    fn len(&self) -> usize {
        self.tot_length
    }
}