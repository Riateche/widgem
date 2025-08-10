#![allow(dead_code)]

use std::{
    collections::BTreeMap,
    fmt::{self, Debug, Formatter},
    hash::Hash,
    io::Write,
    rc::Rc,
};

// TODO: smallvec optimization?
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ChildKey {
    sort: Rc<[u8]>,
    debug: Rc<str>,
}

pub trait ChildKeyData: Debug {
    fn sort_key(&self, out: impl Write);
}

impl<T: ChildKeyData + ?Sized> ChildKeyData for &T {
    fn sort_key(&self, out: impl Write) {
        <T as ChildKeyData>::sort_key(self, out)
    }
}

impl<T: ChildKeyData> From<T> for ChildKey {
    fn from(value: T) -> Self {
        let debug = format!("{value:?}");
        let mut sort = Vec::new();
        value.sort_key(&mut sort);
        Self {
            sort: sort.into(),
            debug: debug.into(),
        }
    }
}

impl PartialOrd for ChildKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ChildKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.sort.cmp(&other.sort)
    }
}

impl Debug for ChildKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.debug)
    }
}

impl From<&ChildKey> for ChildKey {
    fn from(value: &ChildKey) -> Self {
        value.clone()
    }
}

macro_rules! impl_int {
    ($t:ty) => {
        impl ChildKeyData for $t {
            fn sort_key(&self, mut out: impl Write) {
                out.write_all(&self.to_be_bytes()).unwrap();
            }
        }
    };
}

impl_int!(u128);
impl_int!(u64);
impl_int!(u32);
impl_int!(u16);
impl_int!(u8);
impl_int!(i128);
impl_int!(i64);
impl_int!(i32);
impl_int!(i16);
impl_int!(i8);
impl_int!(usize);
impl_int!(isize);

impl ChildKeyData for char {
    fn sort_key(&self, mut out: impl Write) {
        out.write_all(&u32::from(*self).to_be_bytes()).unwrap();
    }
}

impl From<Rc<str>> for ChildKey {
    fn from(value: Rc<str>) -> ChildKey {
        Self {
            debug: format!("{value:?}").into(),
            sort: value.into(),
        }
    }
}

impl From<&Rc<str>> for ChildKey {
    fn from(value: &Rc<str>) -> ChildKey {
        Self {
            debug: format!("{value:?}").into(),
            sort: value.clone().into(),
        }
    }
}

impl ChildKeyData for String {
    fn sort_key(&self, mut out: impl Write) {
        out.write_all(self.as_bytes()).unwrap();
    }
}

impl ChildKeyData for str {
    fn sort_key(&self, mut out: impl Write) {
        out.write_all(self.as_bytes()).unwrap();
    }
}

macro_rules! impl_tuple {
    (($($ty:ident,)*), ($($fi:tt,)*)) => {
        impl<$($ty,)*> ChildKeyData for ($($ty,)*)
        where $($ty: ChildKeyData,)*
        {
            fn sort_key(&self, mut out: impl Write) {
                $({
                    self.$fi.sort_key(&mut out);
                })*
            }
        }
    };
}

impl_tuple!((T0, T1,), (0, 1,));
impl_tuple!((T0, T1, T2,), (0, 1, 2,));
impl_tuple!((T0, T1, T2, T3,), (0, 1, 2, 3,));

impl<T, const N: usize> ChildKeyData for [T; N]
where
    T: ChildKeyData,
{
    fn sort_key(&self, out: impl Write) {
        <[T] as ChildKeyData>::sort_key(self, out)
    }
}

impl<T> ChildKeyData for [T]
where
    T: ChildKeyData,
{
    fn sort_key(&self, mut out: impl Write) {
        for item in self {
            item.sort_key(&mut out);
        }
    }
}

impl<T> ChildKeyData for Option<T>
where
    T: ChildKeyData,
{
    fn sort_key(&self, mut out: impl Write) {
        match self {
            None => {
                out.write_all(&[0]).unwrap();
            }
            Some(value) => {
                out.write_all(&[1]).unwrap();
                value.sort_key(&mut out);
            }
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
