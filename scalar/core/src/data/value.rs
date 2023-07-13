use serde::ser::{SerializeMap, SerializeSeq, Serializer};
use serde::Serialize;
use std::collections::BTreeMap;
use std::convert::TryFrom;
use std::iter::FromIterator;
/// An immutable string that is more memory-efficient since it only has an
/// overhead of 16 bytes for storing a string vs the 24 bytes that `String`
/// requires
#[derive(Clone, Default, Debug, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct Word(Box<str>);

impl Word {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Word {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::ops::Deref for Word {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<&str> for Word {
    fn from(s: &str) -> Self {
        Word(s.into())
    }
}

impl From<String> for Word {
    fn from(s: String) -> Self {
        Word(s.into_boxed_str())
    }
}

impl From<Word> for String {
    fn from(w: Word) -> Self {
        w.0.into()
    }
}

impl Serialize for Word {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for Word {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        String::deserialize(deserializer).map(Into::into)
    }
}

impl AsRef<str> for Word {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl PartialEq<&str> for Word {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl PartialEq<Word> for &str {
    fn eq(&self, other: &Word) -> bool {
        self == &other.as_str()
    }
}
