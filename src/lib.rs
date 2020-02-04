use bit_vec::BitVec;
use fasthash::{murmur, murmur3, spooky};

use std::convert::TryInto;
use std::f64::consts::E;

const HASH_PRIME: u64 = 18446744073709551557;

pub struct BloomFilter {
    array: BitVec,
    size: usize,
    hash_count: usize,
}

impl BloomFilter {
    pub fn new(size: usize, hash_count: usize) -> Self {
        BloomFilter::with_bitscount(size * 8, hash_count)
    }

    pub fn with_bitscount(nbits: usize, hash_count: usize) -> Self {
        let array = BitVec::from_elem(nbits, false);

        Self {
            array,
            size: nbits,
            hash_count,
        }
    }

    pub fn with_fp_size(fp: f64, expected: u64) -> Self {
        let size = BloomFilter::calculate_size_from_fp_capacity(fp, expected);
        let k = BloomFilter::calculate_k(size, expected);
        BloomFilter::new(size, k as usize)
    }

    fn calculate_size_from_fp_capacity(fp: f64, capacity: u64) -> usize {
        assert!(fp != 0f64);
        assert!(capacity != 0);

        let bits = -(capacity as f64 * fp.ln() / (2f64.ln() * 2f64.ln()));
        let bytes = (bits.ceil() / 8.0).ceil();
        bytes as usize
    }

    fn calculate_fp_from_capacity_size(bytes: usize, capacity: u64) -> f64 {
        let bits = (bytes * 8) as f64;
        assert!(bits != 0f64);
        assert!(capacity != 0);

        let fp = E.powf(-(bits / capacity as f64) * (2f64.ln().powi(2)));
        fp
    }

    fn calculate_capacity_from_fp_size(fp: f64, bytes: usize) -> u64 {
        let bits = (bytes * 8) as f64;
        assert!(bits != 0f64);
        assert!(fp != 0.0);

        let capacity = -(bits / fp.ln() * (2f64.ln() * 2f64.ln()));
        capacity as u64 // ceil?
    }

    fn calculate_k(bytes: usize, capacity: u64) -> u32 {
        let bits = (bytes * 8) as f64;
        assert!(bits != 0f64);
        assert!(capacity != 0);

        let k = (2f64.ln() * bits / capacity as f64).round();
        k as u32
    }

    // We use the results of
    // 'Less Hashing, Same Performance: Building a Better Bloom Filter'
    // https://www.eecs.harvard.edu/~michaelm/postscripts/tr-02-05.pdf, to use
    // g_i(x) = h1(u) + i * h2(u) mod m'
    //
    fn compute_hashes<I: AsRef<[u8]>>(&self, item: &I) -> Vec<u64> {
        let mut result: Vec<u64> = Vec::with_capacity(self.hash_count);
        let h1 = murmur3::hash128_with_seed(item, 0);
        result.push(h1 as u64);
        result.push((h1 >> 64) as u64);

        let h2 = spooky::hash128_with_seed(item, 0);
        result.push(h2 as u64);
        result.push((h2 >> 64) as u64);

        assert!(result.len() >= 4);
        for i in 4..self.hash_count {
            println!("result[3], i: {:#x}, {}", result[3], i);
            println!(
                "result[3].wrapping_mul: {:#x}",
                (result[3].wrapping_mul(i as u64)) % HASH_PRIME
            );
            println!(
                "{:#x}",
                result[1].wrapping_add((result[3].wrapping_mul(i as u64)) % HASH_PRIME)
            );
            result.insert(
                i,
                result[1].wrapping_add((result[3].wrapping_mul(i as u64)) % HASH_PRIME),
            );
        }

        result
    }

    fn get_idx<'a, I: AsRef<[u8]>>(&self, item: &'a I) -> impl Iterator<Item = usize> + 'a {
        let size = self.size;
        (0..self.hash_count)
            .map(move |cnt| (murmur::hash32_with_seed(item, cnt as u32) % size as u32) as usize)
    }

    pub fn add<I: AsRef<[u8]>>(&mut self, item: I) {
        let hashes = self.compute_hashes(&item);
        for idx in 0..self.hash_count {
            let idx = hashes[idx] % self.size as u64;
            self.array.set(idx as usize, true);
        }
    }

    pub fn get<I: AsRef<[u8]>>(&self, item: I) -> bool {
        let mut result = true;
        let hashes = self.compute_hashes(&item);
        for idx in 0..self.hash_count {
            let idx = hashes[idx] % self.size as u64;
            println!("get idx {}, in array: {}", idx, self.array[idx as usize] == true);
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

    pub(crate) fn any_as_u8_slice<T: Sized>(p: &T) -> &[u8] {
        unsafe {
            ::std::slice::from_raw_parts((p as *const T) as *const u8, ::std::mem::size_of::<T>())
        }
    }

    pub(crate) fn any_as_u8_vec<T: Sized>(p: T) -> Vec<u8> {
        unsafe {
            ::std::slice::from_raw_parts((&p as *const T) as *const u8, ::std::mem::size_of::<T>())
                .to_vec()
        }
    }

    #[repr(C)]
    struct TestItem {
        a: u32,
    }

    #[test]
    fn item_in_filter() {
        let item = any_as_u8_slice(&TestItem { a: 42 });
        let mut f = BloomFilter::with_bitscount(1, 1);
        f.add(&item);
        assert!(f.get(&item));
    }

    #[test]
    fn false_negatives() {
        let items = (0..64)
            .map(|i| any_as_u8_vec(TestItem { a: i }))
            .collect::<Vec<Vec<u8>>>();
        let mut f = BloomFilter::new(8, 8);
        let _: () = items.iter().map(|i| f.add(i)).collect();
        let mut rng = rand::thread_rng();
        for _ in 0..128 {
            let idx = rng.gen_range(0, items.len());
            let item = &items[idx];
            assert!(f.get(&item));
        }
    }

    #[test]
    fn compute_hashes_correctly() {
        let mut result: Vec<u64> = Vec::with_capacity(4);
        let h1 = murmur3::hash128_with_seed(any_as_u8_vec(TestItem { a: 42 }), 0);
        result.push(h1 as u64); // lower
        result.push((h1 >> 64) as u64); // upper

        assert!(h1 == unsafe { std::mem::transmute((result[0], result[1])) });
    }

    #[test]
    fn false_positive() {
        let items = (0..64)
            .map(|i| any_as_u8_vec(TestItem { a: i }))
            .collect::<Vec<Vec<u8>>>();
        //let mut f = BloomFilter::new(427, 8);
        let mut f = BloomFilter::with_fp_size(0.01, 64);

        let _: () = items.iter().map(|i| f.add(i)).collect();
        let mut rng = rand::thread_rng();
        let false_items = (64..128)
            .map(|i| any_as_u8_vec(TestItem { a: i }))
            .collect::<Vec<Vec<u8>>>();

        let mut positives = 0;
        for _ in 0..128 {
            let idx = rng.gen_range(0, false_items.len());
            let item = &false_items[idx];
            if f.get(&item) {
                positives += 1;
                dbg!(item);
            }
        }
        dbg!(f.array.len());
        dbg!(positives);
        dbg!(f.hash_count);
        assert!(positives <= (false_items.len() / 10));
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
