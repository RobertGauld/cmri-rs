/// Used for creating a slice with a maximum length - essentially a `Vec` but on the stack.
///
/// common_implementation!(\<struct name\>, \<max length\>)
#[expect(clippy::doc_markdown)]
macro_rules! common_implementation {
    ($name:ident, $size:literal) => {
        impl $name {
            pub(crate) const MAX_LEN: usize = $size;

            /// The number of bytes currently stored.
            #[must_use]
            pub const fn len(&self) -> usize {
                self.len
            }

            /// Check whether the data is empty.
            #[must_use]
            pub const fn is_empty(&self) -> bool {
                self.len == 0
            }

            /// The number of bytes which can be added.
            #[must_use]
            pub const fn available(&self) -> usize {
                Self::MAX_LEN - self.len
            }

            #[doc = concat!("Clear the `", stringify!($name), "`.")]
            pub fn clear(&mut self) { self.len = 0 }

            /// Extracts a slice of the data.
            #[must_use]
            pub fn as_slice(&self) -> &[u8] {
                &self.raw[..(self.len)]
            }

            /// Extracts a mutable slice of the data.
            pub fn as_mut_slice(&mut self) -> &mut [u8] { &mut self.raw[..(self.len)] }

            /// Get a forward iterator.
            pub fn iter(&self) -> core::slice::Iter<u8> {
                self.raw[..(self.len)].iter()
            }
        }

        impl core::ops::Deref for $name {
            type Target = [u8];
            fn deref(&self) -> &Self::Target {
                &self.raw[0..(self.len)]
            }
        }

        impl core::ops::Index<usize> for $name {
            type Output = u8;
            fn index(&self, index: usize) -> &Self::Output {
                if index >= self.len { panic!("index out of bounds: the len is {} but the index is {}", self.len, index) }
                &self.raw[index]
            }
        }

        impl core::ops::IndexMut<usize> for $name {
            fn index_mut(&mut self, index: usize) -> &mut u8 {
                if index >= self.len { panic!("index out of bounds: the len is {} but the index is {}", self.len, index) }
                &mut self.raw[index]
            }
        }

        impl core::ops::Index<core::ops::Range<usize>> for $name {
            type Output = [u8];
            fn index(&self, index: core::ops::Range<usize>) -> &Self::Output {
                if index.start >= self.len { panic!("range start index {} out of range for data of length {}", index.start, self.len) }
                if index.end > self.len { panic!("range end index {} out of range for data of length {}", index.end - 1, self.len) }
                &self.raw[index]
            }
        }

        impl core::ops::Index<core::ops::RangeInclusive<usize>> for $name {
            type Output = [u8];
            fn index(&self, index: core::ops::RangeInclusive<usize>) -> &Self::Output {
                if index.start() >= &self.len { panic!("range start index {} out of range for data of length {}", index.start(), self.len) }
                if index.end() >= &self.len { panic!("range end index {} out of range for data of length {}", index.end(), self.len) }
                &self.raw[index]
            }
        }

        impl core::ops::Index<core::ops::RangeFrom<usize>> for $name {
            type Output = [u8];
            fn index(&self, index: core::ops::RangeFrom<usize>) -> &Self::Output {
                if index.start >= self.len { panic!("range start index {} out of range for data of length {}", index.start, self.len) }
                &self.raw[(index.start)..(self.len)]
            }
        }

        impl<'a> core::iter::IntoIterator for &'a $name {
            type Item = &'a u8;
            type IntoIter = core::slice::Iter<'a, u8>;
            fn into_iter(self) -> Self::IntoIter {
                self.iter()
            }
        }

        impl<const N: usize> core::cmp::PartialEq<[u8; N]> for $name {
            fn eq(&self, other: &[u8; N]) -> bool {
                other.eq(&self.as_slice())
            }
        }

        impl core::cmp::PartialEq for $name {
            fn eq(&self, other: &Self) -> bool {
                self.as_slice().eq(other.as_slice())
            }
        }

        impl core::hash::Hash for $name {
            fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
                core::hash::Hash::hash(&self.raw[0..(self.len)], state)
            }
        }

        #[cfg(feature = "std")]
        #[cfg_attr(any(docsrs, toolchain = "nightly"), doc(cfg(feature = "std")))]
        #[cfg_attr(not(toolchain = "nightly"), doc = "**Available on crate feature std only.**\n\n")]
        impl From<&$name> for Vec<u8> {
            fn from(value: &$name) -> Self {
                value.as_slice().into()
            }
        }

        #[cfg(feature = "std")]
        #[cfg_attr(any(docsrs, toolchain = "nightly"), doc(cfg(feature = "std")))]
        #[cfg_attr(not(toolchain = "nightly"), doc = "**Available on crate feature std only.**\n\n")]
        impl From<$name> for Vec<u8> {
            fn from(value: $name) -> Self {
                value.as_slice().into()
            }
        }
    }
}
pub(crate) use common_implementation;


#[allow(clippy::missing_panics_doc, reason = "tests")]
#[cfg(test)]
mod tests {
    mod common_implementation {
        const SIZE: usize = 4;

        struct RawTest {
            raw: [u8; SIZE],
            len: usize
        }
        common_implementation!(RawTest, 4);
        impl RawTest {
            const fn new() -> Self {
                Self {
                    raw: [0; SIZE],
                    len: 0
                }
            }
            fn from_slice(values: &[u8]) -> Self {
                let len = values.len();
                let mut raw = [0; SIZE];
                raw[0..len].clone_from_slice(values);
                Self { raw, len }
            }
        }
        impl core::fmt::Debug for RawTest {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                f.debug_struct("RawTest")
                 .field("len", &self.len)
                 .field("raw", &self.raw)
                 .finish()
            }
        }

        #[test]
        fn max_lan() {
            assert_eq!(RawTest::MAX_LEN, SIZE);
        }

        #[test]
        fn new() {
            let value = RawTest::new();
            assert_eq!(&value, &[]);
        }

        #[test]
        fn from_slice() {
            let value = RawTest::from_slice(&[4, 3, 2, 1]);
            assert_eq!(&value, &[4, 3, 2, 1]);
        }

        #[test]
        fn len() {
            let value = RawTest::from_slice(&[4, 3, 2]);
            assert_eq!(value.len(), 3);
        }

        #[test]
        fn is_empty() {
            let value = RawTest::new();
            assert!(value.is_empty());

            let value = RawTest::from_slice(&[4, 3, 2]);
            assert!(!value.is_empty());
        }

        #[test]
        fn available() {
            let value = RawTest::from_slice(&[4, 3, 2]);
            assert_eq!(value.available(), 1);
        }

        #[test]
        fn clear() {
            let mut value = RawTest::from_slice(&[4, 3, 2]);
            assert_eq!(value.len(), 3);
            value.clear();
            assert_eq!(value.len(), 0);
        }

        #[test]
        fn as_slice() {
            let value = RawTest::from_slice(&[4, 3, 2, 1]);
            let slice = value.as_slice();
            assert_eq!(slice, [4, 3, 2, 1]);
        }

        #[test]
        fn as_mut_slice() {
            let mut value = RawTest::from_slice(&[4, 3, 2, 1]);
            let slice = value.as_mut_slice();
            slice[3] = 0;
            assert_eq!(&value, &[4, 3, 2, 0]);
        }

        #[cfg(feature = "std")]
        #[test]
        fn iter() {
            let value = RawTest::from_slice(&[2, 3, 4]);
            let result: Vec<u8> = value.iter().copied().collect();
            assert_eq!(value.as_slice(), result.as_slice());
        }

        #[test]
        fn deref() {
            let value = RawTest::from_slice(&[1, 2, 3, 4]);
            assert_eq!(*value, [1, 2, 3, 4]);
        }

        mod index {
            use super::*;

            mod by_usize {
                use super::*;

                #[test]
                fn valid() {
                    let value = RawTest::from_slice(&[1, 2]);
                    assert_eq!(value[0], 1);
                    assert_eq!(value[1], 2);
                }

                #[test]
                #[should_panic(expected = "index out of bounds: the len is 2 but the index is 2")]
                fn invalid() {
                    let value = RawTest::from_slice(&[1, 2]);
                    #[expect(unused_variables, reason="Trigger the panic")]
                    let a = value[2];
                }
            }

            mod by_range_usize {
                use super::*;

                #[test]
                fn valid() {
                    let value = RawTest::from_slice(&[1, 2, 3, 4]);
                    assert_eq!(value[0..2], [1, 2]);
                    assert_eq!(value[2..4], [3, 4]);
                }

                #[test]
                #[should_panic(expected = "range end index 2 out of range for data of length 2")]
                fn ends_after_end() {
                    let value = RawTest::from_slice(&[1, 2]);
                    #[expect(unused_variables, reason="Trigger the panic")]
                    let a = &value[1..3];
                }

                #[test]
                #[should_panic(expected = "range start index 2 out of range for data of length 2")]
                fn starts_after_end() {
                    let value = RawTest::from_slice(&[1, 2]);
                    #[expect(unused_variables, reason="Trigger the panic")]
                    let a = &value[2..3];
                }
            }

            mod by_range_inclusive_usize {
                use super::*;

                #[test]
                fn valid() {
                    let value = RawTest::from_slice(&[1, 2, 3, 4]);
                    assert_eq!(value[0..=2], [1, 2, 3]);
                    assert_eq!(value[2..=3], [3, 4]);
                }

                #[test]
                #[should_panic(expected = "range end index 2 out of range for data of length 2")]
                fn ends_after_end() {
                    let value = RawTest::from_slice(&[1, 2]);
                    #[expect(unused_variables, reason="Trigger the panic")]
                    let a = &value[1..=2];
                }

                #[test]
                #[should_panic(expected = "range start index 2 out of range for data of length 2")]
                fn starts_after_end() {
                    let value = RawTest::from_slice(&[1, 2]);
                    #[expect(unused_variables, reason="Trigger the panic")]
                    let a = &value[2..=3];
                }
            }

            mod by_range_from_usize {
                use super::*;

                #[test]
                fn valid() {
                    let value = RawTest::from_slice(&[1, 2, 3, 4]);
                    assert_eq!(value[0..], [1, 2, 3, 4]);
                    assert_eq!(value[2..], [3, 4]);
                }

                #[test]
                fn when_partially_full() {
                    let value = RawTest::from_slice(&[1, 2, 3]);
                    assert_eq!(value[0..], [1, 2, 3]);
                }

                #[test]
                #[should_panic(expected = "range start index 2 out of range for data of length 2")]
                fn invalid() {
                    let value = RawTest::from_slice(&[1, 2]);
                    #[expect(unused_variables, reason="Trigger the panic")]
                    let a = &value[2..];
                }
            }
        }

        mod index_mut_by_usize {
            use super::*;

            #[test]
            fn valid() {
               let mut value = RawTest::from_slice(&[1, 2]);
                value[0] = 2;
                assert_eq!(value, [2, 2]);
            }

            #[test]
            #[should_panic(expected = "index out of bounds: the len is 2 but the index is 2")]
            fn invalid() {
                let mut value = RawTest::from_slice(&[1, 2]);
                value[2] = 2;
            }
        }

        #[cfg(feature = "std")]
        mod into_iter {
            use super::RawTest;

            #[test]
            fn taking_self() {
                let value = RawTest::from_slice(&[2, 3, 4]);
                let result: Vec<u8> = value.into_iter().copied().collect();
                assert_eq!(value.as_slice(), result.as_slice());
            }

            #[test]
            fn from_ref() {
                let value = RawTest::from_slice(&[2, 3, 4]);
                let result: Vec<u8> = (&value).into_iter().copied().collect();
                assert_eq!(value.as_slice(), result.as_slice());
            }

        }

        #[test]
        fn eq() {
            let a = RawTest::from_slice(&[4, 5]);
            let b = RawTest::from_slice(&[4, 5]);
            let c = RawTest::from_slice(&[5, 4]);

            assert_eq!(a, a);
            assert_eq!(a, b);
            assert_ne!(a, c);
        }

        #[cfg(feature = "std")]
        #[test]
        fn vec_from() {
            let value = RawTest::from_slice(&[1, 2]);
            let result: Vec<u8> = value.into();
            assert_eq!(result, vec![1, 2]);
        }

        #[cfg(feature = "std")]
        #[test]
        fn vec_from_ptr() {
            let value = RawTest::from_slice(&[1, 2]);
            let ptr = &value;
            let result: Vec<u8> = ptr.into();
            assert_eq!(result, vec![1, 2]);
        }
    }
}
