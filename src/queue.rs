use std::iter::FromIterator;
use std::ops::{Index, IndexMut};

#[derive(Debug)]
pub struct BoundedVecDeque<T> {
    inner: std::collections::VecDeque<T>,
    size: usize,
}
impl<T> BoundedVecDeque<T> {
    pub fn new(size: usize) -> Self {
        Self {
            inner: std::collections::VecDeque::with_capacity(size),
            size,
        }
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn remove(&mut self, index: usize) -> Option<T> {
        self.inner.remove(index)
    }

    /// Grows the underpinning VecDeque by adding an item and adjusting size accordingly
    pub fn extend_with(&mut self, item: T) {
        self.inner.push_back(item);
        self.size += 1;
    }

    pub fn is_full(&self) -> bool {
        self.inner.len() == self.size
    }

    pub fn capacity(&self) -> usize {
        self.inner.capacity()
    }

    pub fn pop_front(&mut self) -> Option<T> {
        self.inner.pop_front()
    }

    pub fn pop_back(&mut self) -> Option<T> {
        self.inner.pop_back()
    }

    pub fn push_back(&mut self, item: T) -> Option<T> {
        let existing = if self.is_full() {
            if self.capacity() == 0 {
                return Some(item);
            } else {
                self.pop_front()
            }
        } else {
            None
        };
        self.inner.push_back(item);
        existing
    }

    pub fn push_front(&mut self, item: T) -> Option<T> {
        let existing = if self.is_full() {
            if self.capacity() == 0 {
                return Some(item);
            } else {
                self.pop_back()
            }
        } else {
            None
        };
        self.inner.push_front(item);
        existing
    }

    pub fn iter(&self) -> std::collections::vec_deque::Iter<T> {
        self.inner.iter()
    }
}

impl<T> FromIterator<T> for BoundedVecDeque<T> {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        let mut queue = std::collections::VecDeque::new();
        queue.extend(iter);
        let size = queue.len();
        Self { inner: queue, size }
    }
}

impl<T> IntoIterator for BoundedVecDeque<T> {
    type Item = T;
    type IntoIter = std::collections::vec_deque::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

impl<T: Clone> Clone for BoundedVecDeque<T> {
    fn clone(&self) -> Self {
        self.iter().cloned().collect()
    }
}

impl<T> Index<usize> for BoundedVecDeque<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        &self.inner[index]
    }
}

impl<T> IndexMut<usize> for BoundedVecDeque<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.inner[index]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bounded_vec_deque() {
        let mut q = BoundedVecDeque::new(3);
        q.push_back(1);
        q.push_back(2);
        q.push_back(3);

        assert!(q[0] == 1);

        q.push_back(4);
        assert!(q[0] == 2);
        assert!(q[2] == 4);

        println!("{:?}", q);
    }

}