#![allow(dead_code)]

use std::{
    collections::BTreeMap,
    fmt::{self, Debug, Display, Formatter},
    hash::Hash,
};

// TODO: smallvec optimization?
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ChildKey(Box<str>);

impl Debug for ChildKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub trait FormatChildKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result;
}

struct KeyFormatter<T>(T);

impl<T> Display for KeyFormatter<&T>
where
    T: FormatChildKey,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        <T as FormatChildKey>::fmt(self.0, f)
    }
}

impl<T> From<T> for ChildKey
where
    T: FormatChildKey,
{
    fn from(value: T) -> ChildKey {
        Self(KeyFormatter(&value).to_string().into())
    }
}
impl From<&ChildKey> for ChildKey {
    fn from(value: &ChildKey) -> Self {
        value.clone()
    }
}

impl From<Box<str>> for ChildKey {
    fn from(value: Box<str>) -> ChildKey {
        Self(value)
    }
}

macro_rules! impl_from_debug {
    ($t:ty) => {
        impl FormatChildKey for $t {
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                write!(f, "{:?}", self)
            }
        }
        impl FormatChildKey for &$t {
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                write!(f, "{:?}", self)
            }
        }
    };
}

// macro_rules! impl_from_into {
//     ($t:ty) => {
//         impl From<$t> for Key {
//             fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
//                 Key(value.into())
//             }
//         }
//     };
// }
impl_from_debug!(char);
impl_from_debug!(usize);
impl_from_debug!(isize);
impl_from_debug!(u128);
impl_from_debug!(u64);
impl_from_debug!(u32);
impl_from_debug!(u16);
impl_from_debug!(u8);
impl_from_debug!(i128);
impl_from_debug!(i64);
impl_from_debug!(i32);
impl_from_debug!(i16);
impl_from_debug!(i8);
impl_from_debug!(String);
impl_from_debug!(&str);

// impl_from_into!(String);
// impl_from_into!(&str);

// impl From<Box<str>> for Key {
//     fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
//         Key(value)
//     }
// }
// impl From<&String> for Key {
//     fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
//         Key(value.as_str().into())
//     }
// }
impl<T0, T1> FormatChildKey for (T0, T1)
where
    T0: FormatChildKey,
    T1: FormatChildKey,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "({},{})", KeyFormatter(&self.0), KeyFormatter(&self.1))
    }
}
impl<T0, T1, T2> FormatChildKey for (T0, T1, T2)
where
    T0: FormatChildKey,
    T1: FormatChildKey,
    T2: FormatChildKey,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "({},{},{})",
            KeyFormatter(&self.0),
            KeyFormatter(&self.1),
            KeyFormatter(&self.2)
        )
    }
}
impl<T0, T1, T2, T3> FormatChildKey for (T0, T1, T2, T3)
where
    T0: FormatChildKey,
    T1: FormatChildKey,
    T2: FormatChildKey,
    T3: FormatChildKey,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "({},{},{},{})",
            KeyFormatter(&self.0),
            KeyFormatter(&self.1),
            KeyFormatter(&self.2),
            KeyFormatter(&self.3)
        )
    }
}
impl<T0, T1> FormatChildKey for &(T0, T1)
where
    T0: FormatChildKey,
    T1: FormatChildKey,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "({},{})", KeyFormatter(&self.0), KeyFormatter(&self.1))
    }
}
impl<T0, T1, T2> FormatChildKey for &(T0, T1, T2)
where
    T0: FormatChildKey,
    T1: FormatChildKey,
    T2: FormatChildKey,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "({},{},{})",
            KeyFormatter(&self.0),
            KeyFormatter(&self.1),
            KeyFormatter(&self.2)
        )
    }
}
impl<T0, T1, T2, T3> FormatChildKey for &(T0, T1, T2, T3)
where
    T0: FormatChildKey,
    T1: FormatChildKey,
    T2: FormatChildKey,
    T3: FormatChildKey,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "({},{},{},{})",
            KeyFormatter(&self.0),
            KeyFormatter(&self.1),
            KeyFormatter(&self.2),
            KeyFormatter(&self.3)
        )
    }
}

impl<T, const N: usize> FormatChildKey for [T; N]
where
    T: FormatChildKey,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "[")?;
        let mut is_first = true;
        for item in self {
            if is_first {
                write!(f, ",")?;
            }
            write!(f, "{}", KeyFormatter(item))?;
            is_first = false;
        }
        write!(f, "]")
    }
}
impl<T, const N: usize> FormatChildKey for &[T; N]
where
    T: FormatChildKey,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "[")?;
        let mut is_first = true;
        for item in *self {
            if is_first {
                write!(f, ",")?;
            }
            write!(f, "{}", KeyFormatter(item))?;
            is_first = false;
        }
        write!(f, "]")
    }
}
impl<T> FormatChildKey for [T]
where
    T: FormatChildKey,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "[")?;
        let mut is_first = true;
        for item in self {
            if is_first {
                write!(f, ",")?;
            }
            write!(f, "{}", KeyFormatter(item))?;
            is_first = false;
        }
        write!(f, "]")
    }
}
impl<T> FormatChildKey for &[T]
where
    T: FormatChildKey,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "[")?;
        let mut is_first = true;
        for item in *self {
            if is_first {
                write!(f, ",")?;
            }
            write!(f, "{}", KeyFormatter(item))?;
            is_first = false;
        }
        write!(f, "]")
    }
}

impl<T> FormatChildKey for Option<T>
where
    T: FormatChildKey,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if let Some(value) = self {
            write!(f, "Some({})", KeyFormatter(value))
        } else {
            write!(f, "None")
        }
    }
}
impl<T> FormatChildKey for &Option<T>
where
    T: FormatChildKey,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if let Some(value) = self {
            write!(f, "Some({})", KeyFormatter(value))
        } else {
            write!(f, "None")
        }
    }
}

struct X {
    data: BTreeMap<ChildKey, String>,
}
impl X {
    fn get(&self, key: impl Into<ChildKey>) {
        let _ = self.data.get(&key.into());
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
        let s = "1".to_string();
        let tmp = [s.clone(), s];
        x.get(&tmp[..]);
    }
}
