use std::borrow::Borrow;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::mem;

const INITIAL_BUCKETS: usize = 1;
const BUCKET_SCALE_FACTOR: usize = 2;

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
        if self.items >= 3 * self.buckets.len() / 4 {
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

    // pub fn get(&self, key: &K) -> Option<&V> {
    //     self.buckets[self.bucket(&key)]
    //         .iter()
    //         .find(|(e_k, _)| e_k == key)
    //         .map(|&(_, ref v)| v)
    // }

    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized
    {
        self.buckets[self.bucket(key.borrow())]
            .iter()
            .find(|(e_k, _)| e_k.borrow() == key)
            .map(|&(_, ref v)| v)
    }

    pub fn remove<Q>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized
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
        Q: Hash + Eq + ?Sized
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
        Q: Hash + Eq + ?Sized
    {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        (hasher.finish() % self.buckets.len() as u64) as usize
    }
}

pub struct HMIter<'a, K: 'a, V: 'a> {
    map: &'a HashMap<K, V>,
    bucket: usize,
    item: usize,
}

// impl <'a, K, V> for HMIter<'a, K: 'a, V: 'a> {
//     pub fn new() -> Self {
//         HMIter {
//         }
//     }
// }

impl<'a, K, V> Iterator for HMIter<'a, K, V> {
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.map.buckets.get(self.bucket) {
                Some(bucket) => {
                    match bucket.get(self.item) {
                        Some(&(ref k, ref v)) => {
                            self.item += 1;
                            break Some((k, v));
                        },
                        None => {
                            self.bucket += 1;
                            self.item = 0;
                            continue;
                        },
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
        HMIter { map: self, bucket: 0, item: 0 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert() {
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
    fn get() {
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
    fn remove() {
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
    fn len() {
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
    fn contains_key() {
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
    fn into_iter() {
        // Setup
        let mut map = HashMap::new();
        map.insert(0, "a");
        map.insert(1, "b");
        map.insert(2, "c");

        for (&k, &v) in &map {
            match k {
                0 => assert_eq!(v, "a"),
                1 => assert_eq!(v, "b"),
                2 => assert_eq!(v, "c"),
                _ => unreachable!(),
            }
        }
    }
}
