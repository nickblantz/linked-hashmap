use std::{
    borrow::Borrow,
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    mem,
    ops::Index,
};

const INITIAL_BUCKETS: usize = 1;
const BUCKET_SCALE_FACTOR: usize = 2;
const RESIZE_NUM: usize = 3;
const RESIZE_DEN: usize = 4;
const fn resize_pred(items: usize, buckets: usize) -> bool {
    items >= RESIZE_NUM * buckets / RESIZE_DEN
}

#[derive(Debug)]
pub struct HashMap<K, V> {
    buckets: Vec<Vec<(K, V)>>,
    items: usize,
}

impl<K, V> HashMap<K, V> {
    pub fn new() -> Self {
        HashMap {
            buckets: Vec::new(),
            items: 0,
        }
    }
}

impl<K, V> HashMap<K, V>
where
    K: Hash + PartialEq + Eq,
{
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        if resize_pred(self.items, self.buckets.len()) {
            self.resize();
        }
        let bucket = self.bucket(&key);
        let bucket = &mut self.buckets[bucket];
        match bucket.iter_mut().find(|(e_key, _)| *e_key == key) {
            Some((_, e_value)) => Some(mem::replace(e_value, value)),
            None => {
                bucket.push((key, value));
                self.items += 1;
                None
            }
        }
    }

    pub fn entry(&mut self, key: K) -> Entry<K, V> {
        if resize_pred(self.items, self.buckets.len()) {
            self.resize();
        }
        let bucket = self.bucket(&key);
        if let Some(entry) = self.buckets[bucket]
            .iter_mut()
            .find(|&&mut (ref e_k, _)| e_k == &key)
        {
            Entry::Occupied(OccupiedEntry {
                entry: unsafe { &mut *(entry as *mut _) },
            })
        } else {
            Entry::Vacant(VacantEntry {
                key: key,
                map: self,
                bucket,
            })
        }
    }

    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.buckets[self.bucket(key.borrow())]
            .iter()
            .find(|(e_k, _)| e_k.borrow() == key)
            .map(|&(_, ref v)| v)
    }

    pub fn remove<Q>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let bucket = self.bucket(&key);
        let bucket = &mut self.buckets[bucket];
        let position = bucket.iter().position(|(k, _)| k.borrow() == key)?;
        self.items -= 1;
        Some(bucket.swap_remove(position).1)
    }

    pub fn len(&self) -> usize {
        self.items
    }

    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.get(&key).is_some()
    }

    fn resize(&mut self) {
        let target_size = match self.buckets.len() {
            0 => INITIAL_BUCKETS,
            x => x * BUCKET_SCALE_FACTOR,
        };
        let mut new_buckets = Vec::with_capacity(target_size);
        new_buckets.extend((0..target_size).map(|_| Vec::new()));
        self.buckets
            .iter_mut()
            .flat_map(|b| b.drain(..))
            .for_each(|(key, value)| {
                let mut hasher = DefaultHasher::new();
                key.hash(&mut hasher);
                new_buckets[(hasher.finish() % target_size as u64) as usize].push((key, value));
            });

        mem::replace(&mut self.buckets, new_buckets);
    }

    fn bucket<Q>(&self, key: &Q) -> usize
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        (hasher.finish() % self.buckets.len() as u64) as usize
    }
}

impl<'a, K, Q, V> Index<&'a Q> for HashMap<K, V>
where
    K: Eq + Hash + Borrow<Q>,
    Q: Eq + Hash + ?Sized,
{
    type Output = V;

    fn index(&self, index: &Q) -> &Self::Output {
        self.get(index).expect("no entry found for key")
    }
}

pub struct HMIter<'a, K: 'a, V: 'a> {
    map: &'a HashMap<K, V>,
    bucket: usize,
    item: usize,
}

impl<'a, K, V> Iterator for HMIter<'a, K, V> {
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.map.buckets.get(self.bucket) {
                Some(bucket) => match bucket.get(self.item) {
                    Some(&(ref k, ref v)) => {
                        self.item += 1;
                        break Some((k, v));
                    }
                    None => {
                        self.bucket += 1;
                        self.item = 0;
                        continue;
                    }
                },
                None => break None,
            }
        }
    }
}

impl<'a, K, V> IntoIterator for &'a HashMap<K, V> {
    type Item = (&'a K, &'a V);
    type IntoIter = HMIter<'a, K, V>;

    fn into_iter(self) -> <Self as IntoIterator>::IntoIter {
        HMIter {
            map: self,
            bucket: 0,
            item: 0,
        }
    }
}

pub struct OccupiedEntry<'a, K: 'a, V: 'a> {
    entry: &'a mut (K, V),
}

pub struct VacantEntry<'a, K: 'a, V: 'a> {
    key: K,
    map: &'a mut HashMap<K, V>,
    bucket: usize,
}

impl<'a, K, V> VacantEntry<'a, K, V>
where
    K: Eq + Hash,
{
    fn insert(self, value: V) -> &'a mut V {
        self.map.buckets[self.bucket].push((self.key, value));
        self.map.items += 1;
        &mut self.map.buckets[self.bucket].last_mut().unwrap().1
    }
}

pub enum Entry<'a, K: 'a, V: 'a> {
    Occupied(OccupiedEntry<'a, K, V>),
    Vacant(VacantEntry<'a, K, V>),
}

impl<'a, K, V> Entry<'a, K, V>
where
    K: Eq + Hash,
{
    pub fn or_insert(self, value: V) -> &'a mut V {
        match self {
            Entry::Occupied(e) => &mut e.entry.1,
            Entry::Vacant(e) => e.insert(value),
        }
    }

    pub fn or_insert_with<F>(self, default: F) -> &'a mut V
    where
        F: FnOnce() -> V,
    {
        match self {
            Entry::Occupied(e) => &mut e.entry.1,
            Entry::Vacant(e) => e.insert(default()),
        }
    }

    pub fn or_default(self) -> &'a mut V
    where
        V: Default,
    {
        match self {
            Entry::Occupied(e) => &mut e.entry.1,
            Entry::Vacant(e) => e.insert(V::default()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hashmap_insert() {
        // Setup
        let mut map = HashMap::new();

        // Scenario: Insert an item
        let expected = map.insert(42, "a");
        assert!(expected.is_none());

        // Scenario: Insert a duplicate
        let expected = map.insert(42, "b");
        assert!(expected.is_some() && expected.unwrap() == "a");
    }

    #[test]
    fn hashmap_get() {
        // Setup
        let mut map = HashMap::new();
        map.insert(0, "a");
        map.insert(1, "b");
        map.insert(2, "c");
        map.insert(3, "d");
        map.insert(4, "e");

        println!("{:?}", &map);

        // Scenario: The key exists
        let expected = map.get(&2);
        assert!(expected.is_some() && *expected.unwrap() == "c");

        // Scenario: The key doesn't exist
        let expected = map.get(&5);
        assert!(expected.is_none());
    }

    #[test]
    fn hashmap_remove() {
        // Setup
        let mut map = HashMap::new();
        map.insert(0, "a");
        map.insert(1, "b");
        map.insert(2, "c");

        // Scenario: Delete an item that exists
        let expected = map.remove(&2);
        assert!(expected.is_some() && expected.unwrap() == "c");

        // Scenario: Delete an item that does not exist
        let expected = map.remove(&3);
        assert!(expected.is_none());
    }

    #[test]
    fn hashmap_len() {
        // Setup
        let mut map = HashMap::new();

        // Scenario: The map is empty
        let expected = map.len();
        assert!(expected == 0);

        // Scenario: The map has items in it
        map.insert(0, "a");
        map.insert(1, "b");
        map.insert(2, "c");
        let expected = map.len();
        assert!(expected == 3);

        // Scenario: The map is empty again
        map.remove(&0);
        map.remove(&1);
        map.remove(&2);
        let expected = map.len();
        assert!(expected == 0);
    }

    #[test]
    fn hashmap_contains_key() {
        // Setup
        let mut map = HashMap::new();
        map.insert(0, "a");
        map.insert(1, "b");
        map.insert(2, "c");

        // Scenario: The key exists
        let expected = map.contains_key(&2);
        assert!(expected);

        // Scenario: The key doesn't exist
        let expected = map.contains_key(&3);
        assert!(!expected);
    }

    #[test]
    fn hashmap_into_iter() {
        // Setup
        let mut map = HashMap::new();
        map.insert(0, "a");
        map.insert(1, "b");
        map.insert(2, "c");

        // Scenario: I can index into my hashmap
        for (&k, &v) in &map {
            match k {
                0 => assert_eq!(v, "a"),
                1 => assert_eq!(v, "b"),
                2 => assert_eq!(v, "c"),
                _ => unreachable!(),
            }
        }
    }

    #[test]
    fn entry_or_insert() {
        // Setup
        let mut map: HashMap<&str, u32> = HashMap::new();

        // Scenario: Insert a new entry
        map.entry("poneyland").or_insert(3);
        assert_eq!(map["poneyland"], 3);

        // Scenario: Modify an existing entry
        *map.entry("poneyland").or_insert(10) *= 2;
        assert_eq!(map["poneyland"], 6);
    }

    #[test]
    fn entry_or_insert_with() {
        // Setup
        let mut map: HashMap<&str, String> = HashMap::new();
        let s = "hoho".to_string();

        // Scenario: Insert a new entry with a closure
        map.entry("poneyland").or_insert_with(|| s);
        assert_eq!(map["poneyland"], "hoho".to_string());
    }

    #[test]
    fn entry_or_default() {
        let mut map: HashMap<&str, Option<u32>> = HashMap::new();

        // Scenario: Insert a new entry with a default value
        map.entry("poneyland").or_default();
        assert_eq!(map["poneyland"], None);
    }
}
