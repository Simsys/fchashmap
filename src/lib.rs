//! A fixed capacity no_std hashmap.
//!
//! A Hashmap is a data structure that implements an associative array, a structure that can map
//! keys to values. Inserting, deleting and searching of entries is fast. This size limited
//! hashmap is intended for small systems and does not require a dynamic heap allocator and can
//! be used on the stack. The basis of this implementation is the so-called Robin Hood hashing,
//! which was originally developed by
//! [Pedro Celis](https://cs.uwaterloo.ca/research/tr/1986/CS-86-14.pdf).
//! In these two publications from Emmanuel Goossaert
//! ([1](https://codecapsule.com/2013/11/11/robin-hood-hashing/),
//! [2](https://codecapsule.com/2013/11/17/robin-hood-hashing-backward-shift-deletion/))
//! the functionality is explained very nicely.
#![cfg_attr(not(test), no_std)]
mod map;
use map::{Iter, IterMut, Map};
//use std::{fmt::Display};
use core::{borrow::Borrow, fmt, iter::FromIterator, ops};
use hash32::Hash;

/// A fixed capacity no_std hashmap.
///
/// The realization of the hashmap is based on the Robin Hood hashing algorithm. This method  
/// is simple and robust with reasonable performance. However, the fixed capacity implementation
/// has some limitations:
///
/// - The size of the hashmap must be fixed at compile time
/// - 8 bytes ram are consumed per entry without keys and values
/// - The maximum capacity is limited to 32768 entries
/// - The capacity must be chosen as a power of 2
/// - The hashmap should not be used to its full capacity, otherwise it will become slow.
///   10 to 20 percent of the capacity should always be kept free.
///
/// ## Example
///
/// ```
/// use fchashmap::FcHashMap;
/// use hash32_derive::Hash32;
/// use hash32::Hash;
///
/// #[derive(Debug)]
/// struct Reading {
///     temperature: f32,
///     humidy: f32,
/// }
///
/// #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash32)]
/// struct DeviceId([u8; 8]);
///
/// impl DeviceId {
///     fn new(input: &[u8; 8]) -> Self {
///         let mut id = [0_u8; 8];
///         id.copy_from_slice(input);
///         Self(id)
///     }
/// }
///
/// let mut fc_hash_map = FcHashMap::<DeviceId, Reading, 128>::new();
///
/// let dev1 = DeviceId::new(b"12345678");
/// let dev2 = DeviceId::new(b"12345679");
/// let dev3 = DeviceId::new(b"12345680");
///
/// fc_hash_map.insert(dev1, Reading { temperature: 23.1, humidy: 76.3 }).unwrap();
/// fc_hash_map.insert(dev2, Reading { temperature: 22.7, humidy: 55.5 }).unwrap();
///
/// let reading = fc_hash_map.get(&dev1).unwrap();
/// assert_eq!(reading.temperature, 23.1);
/// assert_eq!(reading.humidy, 76.3);
///
/// let reading = fc_hash_map.get(&dev2).unwrap();
/// assert_eq!(reading.temperature, 22.7);
/// assert_eq!(reading.humidy, 55.5);
///
/// assert!(fc_hash_map.get(&dev3).is_none());
/// ```
/// 
/// ## Performance
///
/// The following diagram shows the timing behavior on a Cortex M4f system (STM32F3) at 72 MHz. It
/// can be seen that the performance of the hashmap decreases significantly from a fill margin of 
/// about 80%.
///
/// ![Image](https://raw.githubusercontent.com/Simsys/fchashmap/master/benches/cm4_performance/fchashmap.png)
pub struct FcHashMap<K, V, const CAP: usize> {
    map: Map<K, V, CAP>,
}

impl<K, V, const CAP: usize> FcHashMap<K, V, CAP>
{
    //    pub fn show(&self) { self.map.show() }

    /// Creates an empty HashMap.
    ///
    /// The hash map is initially created with no elements inside. The maximum capacity must be set
    /// at complile time.
    ///
    /// ## Example
    ///
    /// ```
    /// use fchashmap::FcHashMap;
    /// let mut map: FcHashMap<u32, i32, 16> = FcHashMap::new();
    /// ```
    pub fn new() -> Self {
        FcHashMap { map: Map::new() }
    }

    /// Returns the number of elements the map can hold.
    pub fn capacity(&self) -> usize {
        CAP
    }

    /// Remove all key-value pairs in the map.
    ///
    /// ## Example
    ///
    /// ```
    /// use fchashmap::FcHashMap;
    ///
    /// let mut map = FcHashMap::<_, _, 16>::new();
    /// map.insert(1, "a");
    ///
    /// map.clear();
    /// assert!(map.is_empty());
    /// ```
    pub fn clear(&mut self) {
        self.map.clear();
    }

    /// Returns true if the map contains a value for the specified key.
    ///
    /// The key may be any borrowed form of the map's key type, but 'Hash` and `Eq` on the borrowed
    /// form must match those for the key type.
    ///
    /// ## Example
    ///
    /// ```
    /// use fchashmap::FcHashMap;
    ///
    /// let mut map = FcHashMap::<_, _, 8>::new();
    /// map.insert(1, "a").unwrap();
    ///
    /// assert_eq!(map.contains_key(&1), true);
    /// assert_eq!(map.contains_key(&2), false);
    /// ```
    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: ?Sized + Eq + Hash,
    {
        self.map.find(key).is_some()
    }

    /// Returns a reference to the value corresponding to the key.
    ///
    /// The key may be any borrowed form of the map's key type, but `Hash` and `Eq` on the borrowed
    /// form must match those for the key type.
    ///
    /// ## Example
    ///
    /// ```
    /// use fchashmap::FcHashMap;
    ///
    /// let mut map = FcHashMap::<_, _, 16>::new();
    /// map.insert(1, "a").unwrap();
    ///
    /// assert_eq!(map.get(&1), Some(&"a"));
    /// assert_eq!(map.get(&2), None);
    /// ```
    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: ?Sized + Hash + Eq,
    {
        self.map.get(key)
    }

    /// Returns a mutable reference to the value corresponding to the key.
    ///
    /// The key may be any borrowed form of the map's key type, but `Hash` and `Eq` on the borrowed
    /// form *must* match those for the key type.
    ///
    /// ## Example
    ///
    /// ```
    /// use fchashmap::FcHashMap;
    ///
    /// let mut map = FcHashMap::<_, _, 8>::new();
    /// map.insert(1, "a").unwrap();
    /// if let Some(x) = map.get_mut(&1) {
    ///     *x = "b";
    /// }
    /// assert_eq!(map.get(&1), Some(&"b"));
    /// ```
    pub fn get_mut<'v, Q>(&'v mut self, key: &Q) -> Option<&'v mut V>
    where
        K: Borrow<Q>,
        Q: ?Sized + Hash + Eq,
    {
        self.map.get_mut(key)
    }

    /// Inserts a key-value pair into the map.
    ///
    /// If an equivalent key already exists in the map: the key remains and retains in its place in
    /// the order, its corresponding value is updated with `value` and the older value is returned
    /// inside `Some(_)`.
    ///
    /// If no equivalent key existed in the map: the new key-value pair is inserted, and `None`
    /// is returned.
    ///
    /// ## Example
    ///
    /// ```
    /// use fchashmap::FcHashMap;
    ///
    /// let mut map = FcHashMap::<_, _, 8>::new();
    /// assert_eq!(map.insert(37, "a"), Ok(None));
    ///
    /// map.insert(37, "b");
    /// assert_eq!(map.insert(37, "c"), Ok(Some("b")));
    /// assert_eq!(map.get(&37), Some(&"c"));
    /// ```
    pub fn insert(&mut self, key: K, value: V) -> Result<Option<V>, (K, V)>
    where
        K: Hash + PartialEq,
    {
        self.map.insert(key, value)
    }

    /// Returns true if the map contains no elements.
    ///
    /// ## Example
    ///
    /// ```
    /// use fchashmap::FcHashMap;
    ///
    /// let mut map = FcHashMap::<_, _, 16>::new();
    /// assert_eq!(map.is_empty(), true);
    ///
    /// map.insert(1, "a");
    /// assert_eq!(map.is_empty(), false);
    /// ```
    pub fn is_empty(&self) -> bool {
        self.map.buckets.len() == 0
    }

    /// Return an iterator over the key-value pairs of the map, in their order.
    ///
    /// ## Example
    ///
    /// ```
    /// use fchashmap::FcHashMap;
    ///
    /// let mut map = FcHashMap::<_, _, 16>::new();
    /// map.insert("a", 1).unwrap();
    /// map.insert("b", 2).unwrap();
    /// map.insert("c", 3).unwrap();
    ///
    /// let v: Vec<_> = map.iter().collect();
    /// assert_eq!(v, vec![(&"a", &1), (&"b", &2), (&"c", &3)]);
    /// ```
    pub fn iter(&self) -> Iter<'_, K, V> {
        Iter {
            iter: self.map.buckets.iter(),
        }
    }

    /// Return an iterator over the key-value pairs of the map, in their order.
    ///
    /// ## Example
    ///
    /// ```
    /// use fchashmap::FcHashMap;
    ///
    /// let mut map = FcHashMap::<_, _, 16>::new();
    /// map.insert("a", 1).unwrap();
    /// map.insert("b", 2).unwrap();
    /// map.insert("c", 3).unwrap();
    ///
    /// for (_, val) in map.iter_mut() {
    ///     *val = 23;
    /// }
    ///
    /// let v: Vec<_> = map.iter().collect();
    /// assert_eq!(v, vec![(&"a", &23), (&"b", &23), (&"c", &23)]);
    /// ```
    pub fn iter_mut(&mut self) -> IterMut<'_, K, V> {
        IterMut {
            iter: self.map.buckets.iter_mut(),
        }
    }

    /// Return an iterator over the keys of the map, in their order.
    ///
    /// ## Example
    ///
    /// ```
    /// use fchashmap::FcHashMap;
    ///
    /// let mut map = FcHashMap::<_, _, 16>::new();
    /// map.insert("a", 1).unwrap();
    /// map.insert("b", 2).unwrap();
    /// map.insert("c", 3).unwrap();
    ///
    /// let v: Vec<_> = map.keys().collect();
    /// assert_eq!(v, vec![&"a", &"b", &"c"]);
    /// ```
    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.map.buckets.iter().map(|bucket| &bucket.key)
    }

    /// Return the number of key-value pairs in the map.
    ///
    /// ## Example
    ///
    /// ```
    /// use fchashmap::FcHashMap;
    ///
    /// let mut map = FcHashMap::<_, _, 16>::new();
    /// assert_eq!(map.len(), 0);
    ///
    /// map.insert(1, "a").unwrap();
    /// assert_eq!(map.len(), 1);
    /// ```
    pub fn len(&self) -> usize {
        self.map.buckets.len()
    }

    /// Removes a key from the map, returning the value at the key if the key was previously
    /// in the map.
    ///
    /// The key may be any borrowed form of the map’s key type, but Hash and Eq on the borrowed
    /// form must match those for the key type.
    ///
    /// ## Example
    ///
    /// ```
    /// use fchashmap::FcHashMap;
    ///
    /// let mut map = FcHashMap::<_, _, 16>::new();
    /// map.insert(1, "a");
    /// assert_eq!(map.remove(&1), Some("a"));
    /// assert_eq!(map.remove(&1), None);
    /// ```
    pub fn remove<Q>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: ?Sized + Hash + Eq,
    {
        self.map.remove(key)
    }

    /// Return an iterator over the values of the map, in their order.
    ///
    /// ## Example
    ///
    /// ```
    /// use fchashmap::FcHashMap;
    ///
    /// let mut map = FcHashMap::<_, _, 16>::new();
    /// map.insert("a", 1).unwrap();
    /// map.insert("b", 2).unwrap();
    /// map.insert("c", 3).unwrap();
    ///
    /// let v: Vec<_> = map.values().collect();
    /// assert_eq!(v, vec![&1, &2, &3]);
    /// ```
    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.map.buckets.iter().map(|bucket| &bucket.value)
    }

    /// Return an iterator over mutable references to the the values of the map, in their order.
    ///
    /// ## Example
    ///
    /// ```
    /// use fchashmap::FcHashMap;
    ///
    /// let mut map = FcHashMap::<_, _, 16>::new();
    /// map.insert("a", 1).unwrap();
    /// map.insert("b", 2).unwrap();
    /// map.insert("c", 3).unwrap();
    ///
    /// for val in map.values_mut() {
    ///     *val += 10;
    /// }
    ///
    /// let v: Vec<_> = map.values().collect();
    /// assert_eq!(v, vec![&11, &12, &13]);
    /// ```
    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut V> {
        self.map.buckets.iter_mut().map(|bucket| &mut bucket.value)
    }
}

// Implement Clone trait
impl<K, V, const CAP: usize> Clone for FcHashMap<K, V, CAP>
where
    K: Eq + Hash + Clone,
    V: Clone,
{
    fn clone(&self) -> Self {
        Self {
            map: self.map.clone(),
        }
    }
}

// Enable possibility to extract debug informations
impl<K, V, const CAP: usize> fmt::Debug for FcHashMap<K, V, CAP>
where
    K: Eq + Hash + fmt::Debug,
    V: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

// Extend map with data of another map, consuming input
impl<K, V, const CAP: usize> Extend<(K, V)> for FcHashMap<K, V, CAP>
where
    K: Eq + Hash,
{
    fn extend<I>(&mut self, iterable: I)
    where
        I: IntoIterator<Item = (K, V)>,
    {
        for (k, v) in iterable {
            self.insert(k, v).ok().unwrap();
        }
    }
}

// Extend map with data of another map
impl<'a, K, V, const CAP: usize> Extend<(&'a K, &'a V)> for FcHashMap<K, V, CAP>
where
    K: Eq + Hash + Copy,
    V: Copy,
{
    fn extend<I>(&mut self, iterable: I)
    where
        I: IntoIterator<Item = (&'a K, &'a V)>,
    {
        self.extend(iterable.into_iter().map(|(&key, &value)| (key, value)))
    }
}

// Enable possibility to use the "collection.collect()" method
impl<K, V, const CAP: usize> FromIterator<(K, V)> for FcHashMap<K, V, CAP>
where
    K: Eq + Hash,
{
    fn from_iter<I>(fc_hash_map: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
    {
        let mut map = FcHashMap::new();
        map.extend(fc_hash_map);
        map
    }
}

// Indexing operation (container[index]) in immutable contexts
impl<'a, K, Q, V, const CAP: usize> ops::Index<&'a Q> for FcHashMap<K, V, CAP>
where
    K: Eq + Hash + Borrow<Q>,
    Q: ?Sized + Eq + Hash,
{
    type Output = V;

    fn index(&self, key: &Q) -> &V {
        self.get(key).expect("key not found")
    }
}

// Indexing operations (container[index]) in mutable contexts
impl<'a, K, Q, V, const N: usize> ops::IndexMut<&'a Q> for FcHashMap<K, V, N>
where
    K: Eq + Hash + Borrow<Q>,
    Q: ?Sized + Eq + Hash,
{
    fn index_mut(&mut self, key: &Q) -> &mut V {
        self.get_mut(key).expect("key not found")
    }
}

// Enables possibilito to use a "for .. in map" iterator
impl<'a, K, V, const CAP: usize> IntoIterator for &'a FcHashMap<K, V, CAP>
where
    K: Eq + Hash,
{
    type Item = (&'a K, &'a V);
    type IntoIter = Iter<'a, K, V>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
