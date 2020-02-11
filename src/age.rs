use crate::BloomFilter;
use crate::Hash;
use arraydeque::{Array, ArrayDeque, Wrapping};
use crate::queue::BoundedVecDeque;
use std::time::Instant;

pub struct AgeBloom {
    // should be a ringbuffer
    filters: Vec<BloomFilter>,
    // index of currently active filter
    active_idx: usize,
    // number of inserted items
    inserted: u64,
    // expected number of elemets:
    expected: u64,
    // target false positive rate
    fp: f64,
    // number of hash functions
    k: u8,
    // additional number of slices
    l: u8,
}

impl AgeBloom {
    pub fn new(expected: u64, fp: f64) -> Self {
        // @TODO: calculate `l` and `k` properly
        let l = 4;
        let k = 3;
        let size = BloomFilter::calculate_size_from_fp_capacity(fp, expected);
        let filters = (0..k + l)
            .map(|i| BloomFilter::with_parameters(size, k, fp, i as u32))
            .collect::<Vec<BloomFilter>>();

        Self {
            filters,
            inserted: 0,
            active_idx: 0,
            expected,
            fp,
            k: k as u8,
            l: l as u8,
        }
    }

    #[inline]
    fn get_active_idx(&mut self) -> Vec<usize> {
        let len = self.filters.len();
        let to = self.active_idx + self.k as usize - 1;
        let overflow_to = to % len;
        dbg!(len, to, overflow_to);
        if to <= len {
            (self.active_idx..to + 1).collect()
        } else {
            let mut v = Vec::with_capacity(to);
            v.extend(self.active_idx..len + 1);
            v.extend(0..overflow_to);
            v
        }
    }

    pub fn add<I: Hash>(&mut self, item: I) {
        for index in self.get_active_idx() {
            self.filters[index].add(&item)
        }
        self.inserted += 1;
    }

    pub fn get<I: Hash>(&mut self, item: I) -> bool {
        let mut i: isize = self.l as isize;
        let mut p = 0;
        let mut c = 0;
        while i >= 0 {
            if self.filters[i as usize].get(&item) {
                c += 1;
                i += 1;

                if p + c == self.k {
                    return true;
                }
            } else {
                i = i - self.k as isize;
                p = c;
                c = 0;
            }
        }
        return false;
    }
}

#[derive(PartialEq, Debug)]
struct Slice {
    size: usize,
    count: u64,
    hash_index: usize,
    timestamp: std::time::Instant,
    data: bit_vec::BitVec,
}

#[derive(Debug)]
struct AgeFilter {
    num_hash: usize,
    batches: usize,
    optimal_slices: u32,

    error_rate: f64,
    capacity: u64,
    inserts: u64,

    num_slices: u32,
    assess_freq: u32,

    slices: BoundedVecDeque<Slice>,
}

impl Slice {
    pub fn new(size: usize, hash_index: usize, timestamp: std::time::Instant) -> Self {
        Self {
            size,
            count: 0,
            hash_index,
            timestamp,
            data: bit_vec::BitVec::from_elem(size * 8, false),
        }
    }
}


impl AgeFilter {
    //APBF_shiftSlice
    fn shift(&mut self) {
        let hash_index = (self.slices[1].hash_index - 1 + self.num_hash) % self.num_hash;
        self.slices.push_back(Slice::new(self.slices[1].size, hash_index, std::time::Instant::now()));
    }

    fn retire_slices(&mut self) {
        let ts = Instant::now();
        let mut removed = 0;
        dbg!(self.num_slices);
        for i in (self.optimal_slices - 1)..(self.num_slices - 1) {
            if self.slices[(i - removed) as usize].timestamp < ts {
                self.slices.remove(i as usize);
                
                removed += 1;
                continue;
            }
            break
        }
        self.num_slices = self.slices.len() as u32;
    }

    fn add_slice(&mut self, size: usize) {
        let hash_index = (self.slices[1].hash_index - 1 + self.num_hash) % self.num_hash;
        self.slices.extend_with(Slice::new(self.slices[1].size, hash_index, std::time::Instant::now()));
        self.num_slices += 1;
    }

    pub fn new(num_hash: usize, batches: usize, slice_size: u64) -> Self {
        let optimal_size = batches + num_hash;
        let slices = (0..optimal_size).map(|i| {
            Slice::new(slice_size as usize, i % num_hash, Instant::now())
        }).collect::<BoundedVecDeque<Slice>>();
        Self {
            num_hash,
            batches,
            num_slices: optimal_size as u32,
            optimal_slices: optimal_size as u32,
            slices,
            error_rate: 0.0,
            inserts: 0,
            capacity: 0,
            assess_freq: 0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_active_idx() {
        let mut f = AgeBloom::new(16, 0.05);
        f.active_idx = f.filters.len();
        dbg!(f.get_active_idx());
        assert!(f.get_active_idx() == [f.filters.len(), 0, 1].to_vec());
    }

    #[test]
    fn insert_and_get() {
        let mut f = AgeBloom::new(16, 0.05);
        f.add(&42);
        assert!(f.get(&42));
    }

    #[test]
    fn insert_and_get_after_resize() {
        let mut f = AgeBloom::new(16, 0.05);
        for i in 0..17 {
            f.add(i);
        }
        assert!(f.get(&16));
        f.add(77);
        assert!(f.get(77));
    }

    #[test]
    fn test_arraydeq() {
        use arraydeque::{ArrayDeque, Wrapping};
        let mut deque: ArrayDeque<[_; 3], Wrapping> = ArrayDeque::new();
        let s1 = Slice::new(1, 1, std::time::Instant::now());
        let s2 = Slice::new(2, 2, std::time::Instant::now());
        let s3 = Slice::new(3, 3, std::time::Instant::now());
        deque.push_back(s1);
        deque.push_back(s2);
        deque.push_back(s3);

        assert!(deque.get(0).unwrap().size == 1);

    }

    #[test]
    fn test_new() {
        let mut f = AgeFilter::new(2, 4, 16);
        println!("{:?}", f.slices.len());
        f.retire_slices();
        println!("{:?}", f.slices.len());
    }

    #[test]
    fn test_add_slice() {
        let mut f = AgeFilter::new(2, 4, 16);
        f.add_slice(16);
        assert_eq!(f.slices.len(), 7);
        assert_eq!(f.num_slices, 7);
    }

    #[test]
    fn test_retire_slices() {
        let mut f = AgeFilter::new(2, 4, 16);
        f.add_slice(16);
        assert_eq!(f.slices.len(), 7);
        assert_eq!(f.num_slices, 7);
        f.add_slice(16);
        f.add_slice(16);
        f.add_slice(16);
        println!("before {:?}", f.slices.len());
        assert_eq!(f.slices.len(), 10);
        assert_eq!(f.num_slices, 10);
        f.retire_slices();
        println!("after {:?}", f.slices.len());
        assert_eq!(f.slices.len(), 7);
        assert_eq!(f.num_slices, 7);
    }
}
