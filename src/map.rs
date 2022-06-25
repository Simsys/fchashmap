#![allow(dead_code)]
use arrayvec::ArrayVec;
use core::{
    borrow::Borrow,
    hash::{BuildHasher, Hash},
    mem, slice,
};
use hash32::{BuildHasherDefault, FnvHasher, Hasher};

cfg_if::cfg_if! {
    if #[cfg(feature = "hugesize")] {
        use core::convert::TryFrom;

        #[derive(Clone, Copy, PartialEq)]
        struct HashValue(u32);

        // There are up to 0x7fffffff (2^31 - 1) elements allowed. The first bit of u32 is used to mark
        // a empty element
        const HASH_VALUE_IS_EMPTY: HashValue = HashValue(0x80000000);

        impl HashValue {
            // Drop the negative sign
            fn new(hash: u32) -> Self {
                HashValue((hash & 0x7fffffff) as u32)
            }

            // Calulate expected index from hash value
            fn desired_h_idx(&self, mask: usize) -> usize {
                usize::try_from(self.0).unwrap() & mask
            }

            // Calculate distance from expected index from current index
            fn h_idx_distance(&self, mask: usize, current_h_idx: usize) -> usize {
                current_h_idx.wrapping_sub(self.desired_h_idx(mask) as usize) & mask
            }
        }

        // A Combination of hash value and index into the bucket list
        #[derive(Clone, Copy)]
        struct HashIndex {
            hash: HashValue,
            b_idx: u32,
        }

        impl HashIndex {
            // Create a nuew hash index from given parameters
            fn new(hash: HashValue, b_idx: usize) -> Self {
                Self {
                    hash,
                    b_idx: b_idx as u32,
                }
            }

            // Clear actual hash index an mark it as empty
            fn clear(&mut self) {
                self.hash = HASH_VALUE_IS_EMPTY;
            }

            // Check if hash index is empty
            fn is_empty(&self) -> bool {
                self.hash == HASH_VALUE_IS_EMPTY
            }
        }
    } else {
        #[derive(Clone, Copy, PartialEq)]
        struct HashValue(u16);

        // There are up to 0x7fff (32767) elements allowed. The first bit of u16 is used to mark
        // a empty element
        const HASH_VALUE_IS_EMPTY: HashValue = HashValue(0x8000);

        impl HashValue {
            // Create 15 bit hash value from u32 hash
            fn new(hash: u32) -> Self {
                HashValue((hash & 0x7fff) as u16)
            }

            // Calulate expected index from hash value
            fn desired_h_idx(&self, mask: usize) -> usize {
                usize::from(self.0) & mask
            }

            // Calculate distance from expected index from current index
            fn h_idx_distance(&self, mask: usize, current_h_idx: usize) -> usize {
                current_h_idx.wrapping_sub(self.desired_h_idx(mask) as usize) & mask
            }
        }

        // A Combination of hash value and index into the bucket list
        #[derive(Clone, Copy)]
        struct HashIndex {
            hash: HashValue,
            b_idx: u16,
        }

        impl HashIndex {
            // Create a nuew hash index from given parameters
            fn new(hash: HashValue, b_idx: usize) -> Self {
                Self {
                    hash,
                    b_idx: b_idx as u16,
                }
            }

            // Clear actual hash index an mark it as empty
            fn clear(&mut self) {
                self.hash = HASH_VALUE_IS_EMPTY;
            }

            // Check if hash index is empty
            fn is_empty(&self) -> bool {
                self.hash == HASH_VALUE_IS_EMPTY
            }
        }
    }
}

#[derive(Clone, Copy)]
pub struct Bucket<K, V> {
    pub key: K,
    pub value: V,
    hash: HashValue,
}

pub struct Map<K, V, const CAP: usize> {
    pub buckets: ArrayVec<Bucket<K, V>, CAP>,
    hash_table: [HashIndex; CAP],
    build_hasher: BuildHasherDefault<FnvHasher>,
}

impl<K, V, const CAP: usize> Map<K, V, CAP> {
    // Create a new map
    pub fn new() -> Self {
        debug_assert!((Self::capacity() as u32) < u32::MAX);
        debug_assert!(Self::capacity().count_ones() == 1);
        Map {
            buckets: ArrayVec::new(),
            hash_table: [HashIndex {
                hash: HASH_VALUE_IS_EMPTY,
                b_idx: 0,
            }; CAP],
            build_hasher: BuildHasherDefault::new(),
        }
    }

    // Return (fixed) capacity of the map
    fn capacity() -> usize {
        CAP
    }

    // Returns a bit mask that can be used to limit the index to the hash_table matching the
    // capacity
    fn mask() -> usize {
        Self::capacity() - 1
    }

    // Calculate a hash for a key
    fn hash_with<Q>(&self, key: &Q) -> HashValue
    where
        Q: ?Sized + Hash,
    {
        let mut h = self.build_hasher.build_hasher();
        key.hash(&mut h);
        HashValue::new(h.finish32())
    }

    // Inserts a key-value pair into the map.
    pub fn insert(&mut self, key: K, value: V) -> Result<Option<V>, (K, V)>
    where
        K: Hash + PartialEq,
    {
        if self.buckets.is_full() {
            return Err((key, value));
        }

        let hash = self.hash_with(&key);
        let mut h_idx = hash.desired_h_idx(Self::mask());
        let mut h_idx_dist = 0;

        // Search for a suitable place to put the HashIndex and the bucket. There are 3 cases to respect
        loop {
            let hash_index = &mut self.hash_table[h_idx];

            if hash_index.is_empty() {
                // Case 1: empty hash index found, insert data and return None
                *hash_index = HashIndex::new(hash, self.buckets.len());
                // unsafe is ok, we already checked that we aren't exceeding the capacity
                unsafe { self.buckets.push_unchecked(Bucket { key, value, hash }) }
                return Ok(None);
            } else {
                let b_idx = hash_index.b_idx as usize;
                debug_assert!(b_idx < self.buckets.len());
                let their_h_idx_dist = hash_index.hash.h_idx_distance(Self::mask(), h_idx);
                if their_h_idx_dist < h_idx_dist {
                    // Case 2: a place in the hash_table has been found that is suitable. There
                    // is already a HashIndex there, but it has more favorable conditions than we
                    // have. We steal from the rich and give it to thee poor, as Robin Hood once
                    // did, and move the remainig HashIndices to the back.
                    let b_idx = self.buckets.len();
                    let mut hash_index = HashIndex::new(hash, b_idx);
                    loop {
                        // unsafe ist ok, because we checked that h_idx is inside the array size
                        let next_hash_index = unsafe { self.hash_table.get_unchecked_mut(h_idx) };

                        if next_hash_index.is_empty() {
                            // We found the right place: store and return
                            *next_hash_index = hash_index;
                            unsafe { self.buckets.push_unchecked(Bucket { key, value, hash }) }
                            return Ok(None);
                        } else {
                            // Replace HashIndexs and continue shifting and searching for a vacancy
                            hash_index = mem::replace(next_hash_index, hash_index);
                        }
                        h_idx += 1;
                        h_idx &= Self::mask();
                    }
                } else if hash_index.hash == hash
                    && unsafe { self.buckets.get_unchecked(b_idx).key == key }
                {
                    // Case 3: There was already an entry for this key. We leave the place in the
                    // hash table untouched and only exchange the value and return the old one.
                    // Unsafe is ok here, because we checked b_idx inside the loop
                    return Ok(Some(mem::replace(
                        unsafe { &mut self.buckets.get_unchecked_mut(b_idx).value },
                        value,
                    )));
                }
            };
            h_idx_dist += 1;
            h_idx += 1;
            h_idx &= Self::mask();
        }
    }

    // Find a key in the map and return indices for hash_table and bucket list
    pub fn find<Q>(&self, key: &Q) -> Option<(usize, usize)>
    where
        K: Borrow<Q>,
        Q: ?Sized + Hash + Eq,
    {
        if self.buckets.len() == 0 {
            return None;
        }

        let hash = self.hash_with(key);
        let mut h_idx = hash.desired_h_idx(Self::mask());
        let mut h_idx_dist: usize = 0;

        loop {
            let hash_index = &self.hash_table[h_idx];
            if hash_index.is_empty() {
                return None;
            } else {
                let b_idx = hash_index.b_idx as usize;
                debug_assert!(b_idx < self.buckets.len());

                if h_idx_dist > hash.h_idx_distance(Self::mask(), h_idx) {
                    // give up after full table scan (wrap arround)
                    return None;
                } else if hash == hash_index.hash && // unsafe is ok, because we checked the idx
                    unsafe { self.buckets.get_unchecked(b_idx).key.borrow() == key }
                {
                    return Some((h_idx, b_idx));
                }
            }
            h_idx_dist += 1;
            h_idx += 1;
            h_idx &= Self::mask();
        }
    }

    // Delete a found key value pair
    fn remove_found(&mut self, found_h_idx: usize, found_b_idx: usize) -> (K, V) {
        // The HashIndex at location h_idx and the bucket at location b_idx are deleted.
        self.hash_table[found_h_idx].clear();
        let deleted_bucket = self.buckets.swap_pop(found_b_idx).unwrap(); // ArrayVec;
                                                                          //let deleted_bucket = unsafe { self.buckets.swap_remove_unchecked(found_b_idx) }; // heapless::Vec;

        // Correct index that points to the entry that had to swap places.
        // This has only to be done, if wass not the last element in self.buckets
        if found_b_idx < self.buckets.len() {
            let bucket = self.buckets.get(found_b_idx).unwrap();
            let mut h_idx = bucket.hash.desired_h_idx(Self::mask());
            loop {
                if self.hash_table[h_idx].b_idx as usize >= self.buckets.len() {
                    self.hash_table[h_idx] = HashIndex::new(bucket.hash, found_b_idx);
                    break;
                }
                h_idx += 1;
                h_idx &= Self::mask();
            }
        }

        // Now a backward shift deletion is performed to close the gap in the hash_table created
        // by the removal.
        let mut h_idx = found_h_idx;
        loop {
            let last_h_idx = h_idx;
            h_idx += 1;
            h_idx &= Self::mask();

            let hash_index = self.hash_table[h_idx];
            if hash_index.is_empty() {
                break;
            } else {
                if hash_index.hash.h_idx_distance(Self::mask(), h_idx) > 0 {
                    // Shift HashIndex one step
                    // unsafe is ok here, because last_h_idx is known within the limits
                    unsafe { *self.hash_table.get_unchecked_mut(last_h_idx) = hash_index }
                    // clear the moved hash_index entry
                    self.hash_table[h_idx].clear();
                } else {
                    break;
                }
            }
        }
        (deleted_bucket.key, deleted_bucket.value)
    }

    // Delete all keys and values of the map
    pub fn clear(&mut self) {
        self.buckets.clear();
        for hash_index in self.hash_table.iter_mut() {
            hash_index.clear();
        }
    }

    // Returns a reference to the value corresponding to the key.
    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: ?Sized + Hash + Eq,
    {
        self.find(key)
            // unsafe is ok here, because find() checks already the index
            .map(|(_, b_idx)| unsafe { &self.buckets.get_unchecked(b_idx).value })
    }

    // Returns a mutable reference to the value corresponding to the key.
    pub fn get_mut<'v, Q>(&'v mut self, key: &Q) -> Option<&'v mut V>
    where
        K: Borrow<Q>,
        Q: ?Sized + Hash + Eq,
    {
        if let Some((_, b_idx)) = self.find(key) {
            Some(unsafe { &mut self.buckets.get_unchecked_mut(b_idx).value })
        } else {
            None
        }
    }

    // Remove key and coresponding value from the map
    pub fn remove<Q>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: ?Sized + Hash + Eq,
    {
        self.find(key)
            .map(|(h_idx, b_idx)| self.remove_found(h_idx, b_idx).1)
    }
}

// Implement Clone trait
impl<K, V, const CAP: usize> Clone for Map<K, V, CAP>
where
    K: Eq + Hash + Clone,
    V: Clone,
{
    fn clone(&self) -> Self {
        Self {
            buckets: self.buckets.clone(),
            hash_table: self.hash_table.clone(),
            build_hasher: self.build_hasher.clone(),
        }
    }
}

pub struct Iter<'a, K, V> {
    pub iter: slice::Iter<'a, Bucket<K, V>>,
}

impl<'a, K, V> Iterator for Iter<'a, K, V> {
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|bucket| (&bucket.key, &bucket.value))
    }
}

pub struct IterMut<'a, K, V> {
    pub iter: slice::IterMut<'a, Bucket<K, V>>,
}

impl<'a, K, V> Iterator for IterMut<'a, K, V> {
    type Item = (&'a K, &'a mut V);

    fn next(&mut self) -> Option<Self::Item> {
        self.iter
            .next()
            .map(|bucket| (&bucket.key, &mut bucket.value))
    }
}
