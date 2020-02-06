/// This file should contain implementation of http://www.greenorbs.org/people/liu/guodeke/dynamicbloomfilters.pdf
///
use crate::BloomFilter;
use crate::Hash;

pub struct DynamicBloom {
    filters: Vec<BloomFilter>,
    active_idx: usize,
    expected: u64,
    fp: f64,
    inserted: u64,
}

impl DynamicBloom {
    pub fn new(expected: u64, fp: f64) -> Self {
        let f = BloomFilter::with_fp_size(fp, expected);
        let mut filters = Vec::new();
        filters.push(f);
        Self {
            filters,
            active_idx: 0, // we start from first one
            expected,
            fp,
            inserted: 0,
        }
    }

    /// Returns currently active filter
    fn get_active(&mut self) -> &mut BloomFilter {
        self.filters
            .iter_mut()
            .nth(self.active_idx)
            .expect("Index should be always valid")
    }

    /// Should "resize", if the active filter has achieved its maximum capacity,
    /// it will create a new filter and add it to `filters` and set it as `active`
    fn should_resize(&mut self) {
        let active = self.get_active();
        if active.stored() >= active.capacity() {
            // add new filter
            let f = BloomFilter::with_fp_size(self.fp, self.expected);
            self.filters.push(f);
            self.active_idx += 1;
        }
    }

    pub fn add<I: Hash>(&mut self, item: I) {
        if self.inserted >= (self.expected / 10) {
            self.should_resize()
        }
        let active = self.get_active();
        active.add(item);
        self.inserted += 1;
    }

    pub fn get<I: Hash>(&mut self, item: I) -> bool {
        for filter in self.filters.iter() {
            if filter.get(&item) {
                return true;
            }
        }
        return false;
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn insert_and_get() {
        let mut f = DynamicBloom::new(16, 0.05);
        f.add(&42);
        assert!(f.get(&42));
    }

    #[test]
    fn expect_resize() {
        let mut f = DynamicBloom::new(16, 0.05);
        assert_eq!(f.filters.len(), 1);
        for i in 0..17 {
            f.add(i);
        }
        assert_eq!(f.filters.len(), 2);
    }

    #[test]
    fn insert_and_get_after_resize() {
        let mut f = DynamicBloom::new(16, 0.05);
        for i in 0..17 {
            f.add(i);
        }
        assert!(f.get(&16));
        // should also be able to get from second partition
        f.add(77);
        assert!(f.get(77));
        // testing implementation
        assert!(f.filters[1].get(77));
    }
}
