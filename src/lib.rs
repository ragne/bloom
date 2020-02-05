#![feature(test)]
extern crate test;

use bit_vec::BitVec;
use fasthash::{FastHasher, HasherExt, Murmur3HasherExt, SpookyHasherExt};
use std::f64::consts::E;
use std::hash::Hash;


// the highest prime that fits into u64
const HASH_PRIME: u64 = 0xffffffffffffffc5;

pub struct BloomFilter {
    // storage
    array: BitVec,
    // Total size of storage in bytes
    size: usize,
    // Number of passes for hash functions
    k: usize,
    // Maximum number of items that can be stored and retrieved with given fp
    capacity: u64,
    // stored number of items
    stored_items: u64,
    // False probability rate
    fp: f64,
}
///
///  Terms/Parameters:
///  - fp -- false probability
///  - size -- total bits count in filter in _bytes_
///  - capacity -- expected number of elements in filter often used with probability
///  - k -- number of passes for hashing
impl BloomFilter {
    /// Creates new bloomfilter from given size and k
    pub fn new(size: usize, k: usize, fp: f64) -> Self {
        // @TODO: what should be in default constructor?
        BloomFilter::with_parameters(size, k, fp)
    }

    pub fn with_parameters(size: usize, k: usize, fp: f64) -> Self {
        let capacity = BloomFilter::calculate_capacity_from_fp_size(fp, size);
        let nbits = size * 8;
        assert!(
            capacity > 0,
            "Given parameters is too small to create a filter"
        );
        Self {
            array: BitVec::from_elem(nbits, false),
            size,
            k,
            capacity,
            fp,
            stored_items: 0,
        }
    }

    /// Returns current fp rate
    pub fn fp(&self) -> f64 {
        if self.stored_items == 0 {
            self.fp
        } else {
            BloomFilter::calculate_fp_from_capacity_size(self.size, self.stored_items)
        }
    }

    /// Creates a bloomfilter with defined false probability and expected number of elements
    pub fn with_fp_size(fp: f64, expected: u64) -> Self {
        let size = BloomFilter::calculate_size_from_fp_capacity(fp, expected);
        let k = BloomFilter::calculate_k(size, expected);
        BloomFilter::new(size, k as usize, fp)
    }

    /// Returns total capacity
    pub fn capacity(&self) -> u64 {
        self.capacity
    }

    pub fn bits(&self) -> usize {
        self.array.len()
    }

    /// Calculates size in _bytes_ from given false probability and expected capacity
    fn calculate_size_from_fp_capacity(fp: f64, expected: u64) -> usize {
        assert!(fp != 0f64);
        assert!(expected != 0);

        let bits = -(expected as f64 * fp.ln() / (2f64.ln() * 2f64.ln()));
        let bytes = (bits.ceil() / 8.0).ceil();
        bytes as usize
    }

    /// Calculates estimated false probability from given size in _bytes_ and capacity
    fn calculate_fp_from_capacity_size(bytes: usize, capacity: u64) -> f64 {
        let bits = (bytes * 8) as f64;
        assert!(bits != 0f64);
        assert!(capacity != 0);

        let fp = E.powf(-(bits / capacity as f64) * (2f64.ln().powi(2)));
        fp
    }

    /// Calculates number of items for which fp will be held true from given size in _bytes_
    fn calculate_capacity_from_fp_size(fp: f64, bytes: usize) -> u64 {
        let bits = (bytes * 8) as f64;
        assert!(bits != 0f64);
        assert!(fp != 0.0);

        let capacity = -(bits / fp.ln() * (2f64.ln() * 2f64.ln()));
        capacity as u64 // ceil?
    }

    // Calculates optimal k value
    fn calculate_k(bytes: usize, capacity: u64) -> u32 {
        let bits = (bytes * 8) as f64;
        assert!(bits != 0f64);
        assert!(capacity != 0);

        let k = (2f64.ln() * bits / capacity as f64).round();
        k as u32
    }

    #[inline]
    fn _mmr3_hash<T: Hash>(t: &T) -> u128 {
        let mut s = Murmur3HasherExt::with_seed(0);
        t.hash(&mut s);
        s.finish_ext()
    }

    #[inline]
    fn _spooky_hash<T: Hash>(t: &T) -> u128 {
        let mut s = SpookyHasherExt::with_seed((0, 0));
        t.hash(&mut s);
        s.finish_ext()
    }

    // We use the results of
    // 'Less Hashing, Same Performance: Building a Better Bloom Filter'
    // https://www.eecs.harvard.edu/~michaelm/postscripts/tr-02-05.pdf, to use
    // g_i(x) = h1(u) + i * h2(u) mod m'
    //
    fn compute_hashes<I: Hash>(&self, item: &I) -> Vec<u64> {
        let mut result: Vec<u64> = Vec::with_capacity(self.k);

        let h1 = BloomFilter::_spooky_hash(item);
        result.push(h1 as u64);
        result.push((h1 >> 64) as u64);

        let h2 = BloomFilter::_spooky_hash(item);
        result.push(h2 as u64);
        result.push((h2 >> 64) as u64);

        assert!(result.len() >= 4);
        for i in 4..self.k {
            result.insert(
                i,
                result[1].wrapping_add((result[3].wrapping_mul(i as u64)) % HASH_PRIME),
            );
        }

        result
    }

    /// Adds item to filter
    ///
    /// # Example
    ///
    /// ```
    /// let mut f = bloom::BloomFilter::with_fp_size(0.1, 10);
    /// f.add(&42);
    /// assert!(f.get(&42));
    /// ```
    pub fn add<I: Hash>(&mut self, item: I) {
        let hashes = self.compute_hashes(&item);
        for idx in 0..self.k {
            let idx = hashes[idx] % self.bits() as u64;
            self.array.set(idx as usize, true);
        }
        self.stored_items += 1
    }

    /// Checks that item is in filter
    ///
    /// # Example
    ///
    /// ```
    /// let mut f = bloom::BloomFilter::with_fp_size(0.1, 10);
    /// f.add(&42);
    /// assert!(f.get(&42));
    /// assert!(!f.get(&0));
    /// ```
    pub fn get<I: Hash>(&self, item: I) -> bool {
        let mut result = true;
        let hashes = self.compute_hashes(&item);
        for idx in 0..self.k {
            let idx = hashes[idx] % self.bits() as u64;
            if !self.array[idx as usize] {
                result = false;
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::Rng;

    #[derive(Hash, Eq, PartialEq)]
    struct TestItem {
        a: u32,
    }

    #[test]
    fn item_in_filter() {
        let item = TestItem { a: 42 };
        let mut f = BloomFilter::new(1, 1, 0.1);
        f.add(&item);
        assert!(f.get(&item));
    }

    #[test]
    fn false_negatives() {
        let items = (0..64)
            .map(|i| TestItem { a: i })
            .collect::<Vec<TestItem>>();
        let mut f = BloomFilter::new(8, 2, 0.1);
        let _: () = items.iter().map(|i| f.add(i)).collect();
        let mut rng = rand::thread_rng();
        for _ in 0..128 {
            let idx = rng.gen_range(0, items.len());
            let item = &items[idx];
            assert!(f.get(&item));
        }
    }

    #[test]
    fn false_positive() {
        let items = (0..64)
            .map(|i| TestItem { a: i })
            .collect::<Vec<TestItem>>();
        //let mut f = BloomFilter::new(427, 8);
        let mut f = BloomFilter::with_fp_size(0.01, 64);

        let _: () = items.iter().map(|i| f.add(i)).collect();
        let mut rng = rand::thread_rng();
        let false_items = (64..128)
            .map(|i| TestItem { a: i })
            .collect::<Vec<TestItem>>();

        let mut positives = 0;
        for _ in 0..128 {
            let idx = rng.gen_range(0, false_items.len());
            let item = &false_items[idx];
            if f.get(&item) {
                positives += 1;
                assert!(!items.contains(&item));
            }
        }
        dbg!(f.array.len());
        dbg!(positives);
        dbg!(f.k);
        assert!(positives <= (false_items.len() / 10));
    }

    #[test]
    fn fp_changes_with_inserts() {
        // we ask for a filter that should be able to hold 16 items with 0.1 false negative rate
        // and filter will calc optimal k value
        let mut f = BloomFilter::with_fp_size(0.1, 16);
        println!("Current capacity/fp: {}/{:1.6}", f.capacity, f.fp());
        // now insert 32 items which should drive fp up
        for i in 0..32 {
            f.add(&TestItem { a: i })
        }
        // it should become ~0.3
        assert!(f.fp() >= 0.3f64);
    }

    #[test]
    fn stored_items_changed() {
        let mut f = BloomFilter::with_fp_size(0.1, 16);
        f.add(&TestItem { a: 42 });
        assert!(f.stored_items == 1);
    }

    #[test]
    fn calc_k() {
        let k = BloomFilter::calculate_k(512, 5000);
        assert_eq!(k, 1);
    }

    #[test]
    fn calculate_capacity_from_fp_size() {
        let capacity = BloomFilter::calculate_capacity_from_fp_size(0.01, 512);
        assert_eq!(capacity, 427);
    }

    #[test]
    fn calculate_fp_from_capacity_size() {
        let fp = BloomFilter::calculate_fp_from_capacity_size(512, 5000) * 1_000_000.0;
        assert_eq!(fp.round(), 674633.0);
    }

    #[test]
    fn calculate_size_from_fp_capacity() {
        let size = BloomFilter::calculate_size_from_fp_capacity(0.001, 5000);
        assert_eq!(size, 8986);
    }
}

#[cfg(test)]
mod bench {
    use super::*;
    use test::Bencher;
    use rand::Rng;
    use rand::distributions::Uniform;
    #[bench]
    fn inserts_repeatedly(b: &mut Bencher) {
        let mut f = BloomFilter::with_fp_size(0.05, 50000);
        let mut gen = rand::thread_rng();
        b.iter(|| {
            let item = gen.sample(Uniform::new(0, 6_000_000));
            f.add(&item)
        })
    }
}
