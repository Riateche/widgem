#![allow(dead_code)]

use std::{
    collections::BTreeMap,
    fmt::{self, Debug, Formatter},
    hash::{Hash, Hasher},
};

use itertools::Itertools;

#[derive(Clone, Eq)]
pub enum Key<'a> {
    None,
    U64(u64),
    I64(i64),
    Str(Box<str>),
    StrRef(&'a str),
    Slice(Box<[Key<'a>]>),
    SliceRef(&'a [Key<'a>]),
}

impl Debug for Key<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::None => writeln!(f, "None"),
            Self::U64(v) => writeln!(f, "{v:?}"),
            Self::I64(v) => writeln!(f, "{v:?}"),
            Self::Str(v) => writeln!(f, "{v:?}"),
            Self::StrRef(v) => writeln!(f, "{v:?}"),
            Self::Slice(v) => {
                let mut t = f.debug_tuple("");
                for item in v {
                    t.field(item);
                }
                t.finish()
            }
            Self::SliceRef(v) => {
                let mut t = f.debug_tuple("");
                for item in *v {
                    t.field(item);
                }
                t.finish()
            }
        }
    }
}

impl<'a> Key<'a> {
    fn ord_helper(&self) -> KeyOrdHelper<'_> {
        match self {
            Key::None => KeyOrdHelper::None,
            Key::U64(v) => {
                let greater_than_i64 = *v > i64::MAX as u64;
                KeyOrdHelper::Number(greater_than_i64, *v as i64)
            }
            Key::I64(v) => KeyOrdHelper::Number(false, *v),
            Key::Str(v) => KeyOrdHelper::StrRef(v),
            Key::StrRef(v) => KeyOrdHelper::StrRef(v),
            Key::Slice(v) => KeyOrdHelper::SliceRef(v),
            Key::SliceRef(v) => KeyOrdHelper::SliceRef(v),
        }
    }

    pub fn into_owned(self) -> Key<'static> {
        match self {
            Key::None => Key::None,
            Key::U64(v) => Key::U64(v),
            Key::I64(v) => Key::I64(v),
            Key::Str(v) => Key::Str(v),
            Key::StrRef(v) => Key::Str(v.into()),
            // TODO: avoid reallocation if self didn't contain refs
            Key::Slice(v) => Key::Slice(
                Vec::from(v)
                    .into_iter()
                    .map(|i| i.into_owned())
                    .collect_vec()
                    .into(),
            ),
            Key::SliceRef(v) => Key::Slice(
                v.iter()
                    .map(|i| i.clone().into_owned())
                    .collect_vec()
                    .into(),
            ),
        }
    }

    pub fn from_iter<T, I>(value: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Into<Key<'a>>,
    {
        Key::Slice(value.into_iter().map(|i| i.into()).collect_vec().into())
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash)]
enum KeyOrdHelper<'a> {
    None,
    Number(bool, i64),
    StrRef(&'a str),
    SliceRef(&'a [Key<'a>]),
}

impl PartialEq for Key<'_> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::None, Self::None) => true,
            (Self::U64(l0), Self::U64(r0)) => l0 == r0,
            (Self::I64(l0), Self::I64(r0)) => l0 == r0,
            (Self::U64(l0), Self::I64(r0)) => i64::try_from(*l0).map_or(false, |l0| l0 == *r0),
            (Self::I64(l0), Self::U64(r0)) => i64::try_from(*r0).map_or(false, |r0| r0 == *l0),
            (Self::Str(l0), Self::Str(r0)) => l0 == r0,
            (Self::StrRef(l0), Self::StrRef(r0)) => l0 == r0,
            (Self::Str(l0), Self::StrRef(r0)) => **l0 == **r0,
            (Self::StrRef(l0), Self::Str(r0)) => **l0 == **r0,
            (Self::Slice(l0), Self::Slice(r0)) => l0 == r0,
            (Self::SliceRef(l0), Self::SliceRef(r0)) => l0 == r0,
            (Self::Slice(l0), Self::SliceRef(r0)) => **l0 == **r0,
            (Self::SliceRef(l0), Self::Slice(r0)) => **l0 == **r0,
            _ => false,
        }
    }
}

impl Ord for Key<'_> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.ord_helper().cmp(&other.ord_helper())
    }
}

impl PartialOrd for Key<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Hash for Key<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.ord_helper().hash(state);
    }
}

impl From<usize> for Key<'_> {
    fn from(value: usize) -> Self {
        Key::U64(value.try_into().expect("usize type too large"))
    }
}
impl From<isize> for Key<'_> {
    fn from(value: isize) -> Self {
        Key::I64(value.try_into().expect("usize type too large"))
    }
}
impl From<u64> for Key<'_> {
    fn from(value: u64) -> Self {
        Key::U64(value)
    }
}
impl From<char> for Key<'_> {
    fn from(value: char) -> Self {
        Key::U64(value.into())
    }
}
impl From<u32> for Key<'_> {
    fn from(value: u32) -> Self {
        Key::U64(value.into())
    }
}
impl From<u16> for Key<'_> {
    fn from(value: u16) -> Self {
        Key::U64(value.into())
    }
}
impl From<u8> for Key<'_> {
    fn from(value: u8) -> Self {
        Key::I64(value.into())
    }
}
impl From<i64> for Key<'_> {
    fn from(value: i64) -> Self {
        Key::I64(value)
    }
}
impl From<i32> for Key<'_> {
    fn from(value: i32) -> Self {
        Key::I64(value.into())
    }
}
impl From<i16> for Key<'_> {
    fn from(value: i16) -> Self {
        Key::I64(value.into())
    }
}
impl From<i8> for Key<'_> {
    fn from(value: i8) -> Self {
        Key::I64(value.into())
    }
}
impl<'a> From<&'a usize> for Key<'a> {
    fn from(value: &'a usize) -> Self {
        Key::U64((*value).try_into().expect("usize type too large"))
    }
}
impl<'a> From<&'a isize> for Key<'a> {
    fn from(value: &'a isize) -> Self {
        Key::I64((*value).try_into().expect("usize type too large"))
    }
}
impl<'a> From<&'a char> for Key<'a> {
    fn from(value: &'a char) -> Self {
        Key::U64((*value).into())
    }
}

impl<'a> From<&'a u64> for Key<'a> {
    fn from(value: &'a u64) -> Self {
        Key::U64(*value)
    }
}
impl<'a> From<&'a u32> for Key<'a> {
    fn from(value: &'a u32) -> Self {
        Key::U64((*value).into())
    }
}
impl<'a> From<&'a u16> for Key<'a> {
    fn from(value: &'a u16) -> Self {
        Key::U64((*value).into())
    }
}
impl<'a> From<&'a u8> for Key<'a> {
    fn from(value: &'a u8) -> Self {
        Key::I64((*value).into())
    }
}
impl<'a> From<&'a i64> for Key<'a> {
    fn from(value: &'a i64) -> Self {
        Key::I64(*value)
    }
}
impl<'a> From<&'a i32> for Key<'a> {
    fn from(value: &'a i32) -> Self {
        Key::I64((*value).into())
    }
}
impl<'a> From<&'a i16> for Key<'a> {
    fn from(value: &'a i16) -> Self {
        Key::I64((*value).into())
    }
}
impl<'a> From<&'a i8> for Key<'a> {
    fn from(value: &'a i8) -> Self {
        Key::I64((*value).into())
    }
}

impl<'a> From<&'a str> for Key<'a> {
    fn from(value: &'a str) -> Self {
        Key::StrRef(value)
    }
}
impl<'a> From<&'a String> for Key<'a> {
    fn from(value: &'a String) -> Self {
        Key::StrRef(value)
    }
}
impl From<String> for Key<'_> {
    fn from(value: String) -> Self {
        Key::Str(value.into())
    }
}

impl<'a, T1, T2> From<(T1, T2)> for Key<'a>
where
    T1: Into<Key<'a>>,
    T2: Into<Key<'a>>,
{
    fn from(value: (T1, T2)) -> Self {
        Self::Slice([value.0.into(), value.1.into()].into())
    }
}
impl<'a, T1, T2, T3> From<(T1, T2, T3)> for Key<'a>
where
    T1: Into<Key<'a>>,
    T2: Into<Key<'a>>,
    T3: Into<Key<'a>>,
{
    fn from(value: (T1, T2, T3)) -> Self {
        Self::Slice([value.0.into(), value.1.into(), value.2.into()].into())
    }
}
impl<'a, T1, T2, T3, T4> From<(T1, T2, T3, T4)> for Key<'a>
where
    T1: Into<Key<'a>>,
    T2: Into<Key<'a>>,
    T3: Into<Key<'a>>,
    T4: Into<Key<'a>>,
{
    fn from(value: (T1, T2, T3, T4)) -> Self {
        Self::Slice(
            [
                value.0.into(),
                value.1.into(),
                value.2.into(),
                value.3.into(),
            ]
            .into(),
        )
    }
}
impl<'a, T1, T2> From<&'a (T1, T2)> for Key<'a>
where
    &'a T1: Into<Key<'a>>,
    &'a T2: Into<Key<'a>>,
{
    fn from(value: &'a (T1, T2)) -> Self {
        Self::Slice([(&value.0).into(), (&value.1).into()].into())
    }
}
impl<'a, T1, T2, T3> From<&'a (T1, T2, T3)> for Key<'a>
where
    &'a T1: Into<Key<'a>>,
    &'a T2: Into<Key<'a>>,
    &'a T3: Into<Key<'a>>,
{
    fn from(value: &'a (T1, T2, T3)) -> Self {
        Self::Slice([(&value.0).into(), (&value.1).into(), (&value.2).into()].into())
    }
}
impl<'a, T1, T2, T3, T4> From<&'a (T1, T2, T3, T4)> for Key<'a>
where
    &'a T1: Into<Key<'a>>,
    &'a T2: Into<Key<'a>>,
    &'a T3: Into<Key<'a>>,
    &'a T4: Into<Key<'a>>,
{
    fn from(value: &'a (T1, T2, T3, T4)) -> Self {
        Self::Slice(
            [
                (&value.0).into(),
                (&value.1).into(),
                (&value.2).into(),
                (&value.3).into(),
            ]
            .into(),
        )
    }
}
// TODO: from [Key<'a>, N] for Key<'a> without allocations
// TODO: allocation-free conversion for small lists (supply buffer or use smallvec)
impl<'a, T, const N: usize> From<[T; N]> for Key<'a>
where
    T: Into<Key<'a>>,
{
    fn from(value: [T; N]) -> Self {
        Key::Slice(value.into_iter().map(|i| i.into()).collect_vec().into())
    }
}
impl<'a, T, const N: usize> From<&'a [T; N]> for Key<'a>
where
    &'a T: Into<Key<'a>>,
{
    fn from(value: &'a [T; N]) -> Self {
        Key::Slice(value.iter().map(|i| i.into()).collect_vec().into())
    }
}
impl<'a, T> From<&'a [T]> for Key<'a>
where
    &'a T: Into<Key<'a>>,
{
    fn from(value: &'a [T]) -> Self {
        Key::Slice(value.iter().map(|i| i.into()).collect_vec().into())
    }
}
impl<'a, T> From<Option<T>> for Key<'a>
where
    T: Into<Key<'a>>,
{
    fn from(value: Option<T>) -> Self {
        if let Some(value) = value {
            value.into()
        } else {
            Key::None
        }
    }
}
impl<'a, T> From<&'a Option<T>> for Key<'a>
where
    &'a T: Into<Key<'a>>,
{
    fn from(value: &'a Option<T>) -> Self {
        if let Some(value) = value {
            value.into()
        } else {
            Key::None
        }
    }
}
impl From<Box<str>> for Key<'_> {
    fn from(value: Box<str>) -> Self {
        Key::Str(value)
    }
}
impl<'a> From<Box<[Key<'a>]>> for Key<'a> {
    fn from(value: Box<[Key<'a>]>) -> Self {
        Key::Slice(value)
    }
}

#[test]
fn key_size() {
    assert_eq!(std::mem::size_of::<Key<'static>>(), 24);
}

struct X {
    data: BTreeMap<Key<'static>, String>,
}
impl X {
    fn get<'a: 'b, 'b>(&'b self, key: impl Into<Key<'a>>) -> &'b String {
        self.data.get(&key.into()).unwrap()
    }
}

#[test]
#[allow(clippy::needless_borrows_for_generic_args)]
fn types() {
    let x = X {
        data: BTreeMap::new(),
    };
    x.get(12);
    x.get((12, 34));
    x.get(&(12, 34));
    x.get(&12);
    x.get("abc");
    x.get(&("abc".to_string()));
    x.get("abc".to_string());
    x.get([1, 2, 3]);
    x.get(&[1, 2, 3]);
    x.get(&[1, 2, 3][..]);
    x.get(Some(1));
    x.get(&Some(1));
    x.get(None::<String>);
    {
        let tmp = vec![1, 2, 3];
        x.get(&tmp[..]);
    }
}
