use std::{borrow::Cow, ops::Deref};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct CowStr<'a>(#[serde(borrow)] pub Cow<'a, str>);

impl From<String> for CowStr<'_> {
    fn from(string: String) -> Self {
        Self(string.into())
    }
}

impl<'a> From<&'a str> for CowStr<'a> {
    fn from(str: &'a str) -> Self {
        Self(str.into())
    }
}

impl AsRef<str> for CowStr<'_> {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl Deref for CowStr<'_> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}