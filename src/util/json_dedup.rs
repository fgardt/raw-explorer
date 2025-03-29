use std::{
    collections::{BTreeMap, HashSet},
    hash::Hash,
    ops::Deref,
    sync::Arc,
};

#[derive(Debug, Default, serde::Serialize)]
pub struct Map<V> {
    #[serde(flatten)]
    map: Arc<BTreeMap<Arc<str>, V>>,
}

impl<V> std::clone::Clone for Map<V> {
    fn clone(&self) -> Self {
        Self {
            map: Arc::clone(&self.map),
        }
    }
}

impl<V> Deref for Map<V> {
    type Target = BTreeMap<Arc<str>, V>;

    fn deref(&self) -> &Self::Target {
        &self.map
    }
}

impl<V: PartialEq> PartialEq for Map<V> {
    fn eq(&self, other: &Self) -> bool {
        self.map.eq(&other.map)
    }
}

impl<V: Eq> Eq for Map<V> {}

impl Hash for Map<DedupValue> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let mut kv = Vec::from_iter(&*self.map);
        kv.sort_unstable_by(|a, b| a.0.cmp(b.0));
        kv.hash(state);
    }
}

impl<V> std::iter::FromIterator<(Arc<str>, V)> for Map<V> {
    fn from_iter<T: IntoIterator<Item = (Arc<str>, V)>>(iter: T) -> Self {
        Self {
            map: Arc::new(iter.into_iter().collect()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize)]
#[serde(untagged)]
pub enum DedupValue {
    Null,
    Bool(bool),
    Number(serde_json::Number),
    String(Arc<str>),
    Array(Arc<[Self]>),
    Object(Map<Self>),
}

impl DedupValue {
    fn dedup_helper(value: serde_json::Value, known: &mut HashSet<Arc<str>>) -> Self {
        match value {
            serde_json::Value::Null => Self::Null,
            serde_json::Value::Bool(b) => Self::Bool(b),
            serde_json::Value::Number(n) => Self::Number(n),
            serde_json::Value::String(s) => {
                let s = Arc::from(s);

                #[allow(clippy::single_match_else, clippy::option_if_let_else)]
                let s = match known.get(&s) {
                    Some(exists) => exists.clone(),
                    None => {
                        known.insert(s.clone());
                        s
                    }
                };

                Self::String(s)
            }
            serde_json::Value::Array(a) => {
                let a = a
                    .into_iter()
                    .map(|v| Self::dedup_helper(v, known))
                    .collect();
                Self::Array(a)
            }
            serde_json::Value::Object(o) => {
                let o = o
                    .into_iter()
                    .map(|(k, v)| {
                        let k = Arc::from(k);

                        #[allow(clippy::single_match_else, clippy::option_if_let_else)]
                        let k = match known.get(&k) {
                            Some(exists) => exists.clone(),
                            None => {
                                known.insert(k.clone());
                                k
                            }
                        };

                        (k, Self::dedup_helper(v, known))
                    })
                    .collect();
                Self::Object(Map { map: Arc::new(o) })
            }
        }
    }
}

impl From<serde_json::Value> for DedupValue {
    fn from(value: serde_json::Value) -> Self {
        Self::dedup_helper(value, &mut HashSet::default())
    }
}

impl<'de> serde::Deserialize<'de> for DedupValue {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        serde_json::Value::deserialize(deserializer).map(Self::from)
    }
}
