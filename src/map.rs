use crate::value::Value;

const EMPTY: i32 = -1;

/// Insertion-ordered hash map — the "compact dict" layout (CPython 3.6+,
/// V8's Map): a dense vector of entries in insertion order, plus a sparse
/// index table holding positions into it. Lookups go through the index;
/// iteration walks the dense vector, so insertion order is the spec, not an
/// accident. Deletion always builds a new map (value semantics), so there
/// are no tombstones to manage.
#[derive(Debug, Clone)]
pub struct PlayMap {
    entries: Vec<(Value, Value)>,
    /// Power-of-two sized; each slot is EMPTY or a position in `entries`.
    index: Vec<i32>,
}

impl Default for PlayMap {
    fn default() -> Self {
        PlayMap { entries: Vec::new(), index: vec![EMPTY; 8] }
    }
}

impl PlayMap {
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn get(&self, key: &Value) -> Option<&Value> {
        let mask = self.index.len() - 1;
        let mut i = (hash_key(key) as usize) & mask;

        loop {

            match self.index[i] {
                EMPTY => return None,
                pos => {
                    let (k, v) = &self.entries[pos as usize];

                    if k == key {
                        return Some(v);
                    }
                }
            }

            i = (i + 1) & mask;
        }
    }

    /// Insert or update. Updating never moves an entry: the key keeps its
    /// original insertion position.
    pub fn insert(&mut self, key: Value, value: Value) {

        // Grow first, keeping the load factor under 3/4 so probing always
        // terminates.
        if (self.entries.len() + 1) * 4 >= self.index.len() * 3 {
            self.rebuild_index(self.index.len() * 2);
        }

        let mask = self.index.len() - 1;
        let mut i = (hash_key(&key) as usize) & mask;

        loop {

            match self.index[i] {
                EMPTY => {
                    self.index[i] = self.entries.len() as i32;
                    self.entries.push((key, value));
                    return;
                }
                pos => {

                    if self.entries[pos as usize].0 == key {
                        self.entries[pos as usize].1 = value;
                        return;
                    }
                }
            }

            i = (i + 1) & mask;
        }
    }

    /// A copy without `key`, preserving the insertion order of the others.
    pub fn without(&self, key: &Value) -> PlayMap {
        let mut map = PlayMap::default();

        for (k, v) in &self.entries {

            if k != key {
                map.insert(k.clone(), v.clone());
            }
        }

        map
    }

    /// Entries in insertion order — the contractual iteration order.
    pub fn iter(&self) -> impl Iterator<Item = (&Value, &Value)> {
        self.entries.iter().map(|(k, v)| (k, v))
    }

    fn rebuild_index(&mut self, capacity: usize) {
        let mask = capacity - 1;
        let mut index = vec![EMPTY; capacity];

        for (pos, (key, _)) in self.entries.iter().enumerate() {
            let mut i = (hash_key(key) as usize) & mask;

            while index[i] != EMPTY {
                i = (i + 1) & mask;
            }

            index[i] = pos as i32;
        }

        self.index = index;
    }
}

// Map equality is order-insensitive (a map is a mapping, not a sequence):
// {"a": 1, "b": 2} == {"b": 2, "a": 1}. Insertion order is observable when
// iterating, but it is not part of the value's identity.
impl PartialEq for PlayMap {
    fn eq(&self, other: &Self) -> bool {
        self.entries.len() == other.entries.len()
            && self.entries.iter().all(|(k, v)| other.get(k) == Some(v))
    }
}

/// FNV-1a over a type tag plus the key's payload. Deliberately unseeded:
/// deterministic hashing is safe in a closed world, because a collision
/// flood only burns the attacker's own fuel budget.
fn hash_key(key: &Value) -> u64 {

    match key {
        Value::Number(n) => {
            // -0.0 == 0.0 must hash identically.
            let n = if *n == 0.0 { 0.0 } else { *n };
            fnv(fnv(FNV_BASIS, &[1]), &n.to_bits().to_le_bytes())
        }
        Value::Bool(b) => fnv(FNV_BASIS, &[2, *b as u8]),
        Value::Str(s) => fnv(fnv(FNV_BASIS, &[3]), s.as_bytes()),
        _ => unreachable!("non-primitive keys are rejected before hashing"),
    }
}

const FNV_BASIS: u64 = 0xcbf2_9ce4_8422_2325;

fn fnv(mut hash: u64, bytes: &[u8]) -> u64 {

    for &byte in bytes {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }

    hash
}
