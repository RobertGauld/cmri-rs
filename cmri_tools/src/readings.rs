//! A ring buffer allowing any number of items to be added, but only the last N retrieved.

/// A collection of upto N readings of type T.
///
/// When it's full the oldest reading is dropped when more space is required.
///
/// # Example:
/// ```
/// let mut readings = cmri_tools::readings::Readings::<u8, 4>::new();
/// assert_eq!(readings.as_slice(), &[] as &[u8]);
/// assert_eq!(readings.len(), 0);
/// readings.push(1);
/// readings.push(2);
/// readings.push(3);
/// assert_eq!(readings.len(), 3);
/// assert_eq!(readings.as_slice(), &[1, 2, 3]);
/// assert_eq!(readings.as_vec(), vec![1, 2, 3]);
/// readings.push(4);
/// assert_eq!(readings.as_slice(), &[1, 2, 3, 4]);
/// assert_eq!(readings.as_vec(), vec![1, 2, 3, 4]);
///
/// // Now readings is full, old values will start being replaced with newer ones.
/// readings.push(5);
/// assert_eq!(readings.as_slice(), &[5, 2, 3, 4]);
/// assert_eq!(readings.as_vec(), vec![2, 3, 4, 5]);
/// // Notice how as_slice makes no order guarantees but is useful for summing etc.
/// // Whilst as_vec keeps values in oldest -> newest order making it useful for displaying values.
/// readings.push(6);
/// assert_eq!(readings.as_slice(), &[5, 6, 3, 4]);
/// assert_eq!(readings.as_vec(), vec![3, 4, 5, 6]);
/// ```
#[derive(Debug, Eq, PartialEq, Clone)]
pub struct Readings<T, const N: usize> {
    values: [T; N],
    full: bool,
    next: usize
}

impl<T, const N: usize> Readings<T, N> {
    /// Push a new reading into the collection.
    pub fn push(&mut self, value: T) {
        self.values[self.next] = value;
        if self.next == N - 1 { self.full = true }
        self.next = (self.next + 1) % N;
    }

    /// Get all the added values, in no guaranteed order.
    pub fn as_slice(&self) -> &[T] {
        if self.full {
            &self.values
        } else {
            &self.values[0..(self.next)]
        }
    }

    /// Get the number of values in the collection.
    pub const fn len(&self) -> usize {
        if self.full { N } else { self.next }
    }

    /// Get the last value added to the collection.
    pub const fn last(&self) -> Option<&T> {
        if self.is_empty() { return None }
        Some(&self.values[if self.next == 0 { N - 1 } else { self.next - 1 }])
    }

    /// Whether the collection is empty.
    pub const fn is_empty(&self) -> bool {
        !self.full && self.next == 0
    }
}

impl <T, const N: usize> Readings<T, N> where T: Clone {
    /// Get all the values, from oldest to newest.
    pub fn as_vec(&self) -> Vec<T> {
        let cutoff = self.next;
        if self.full {
            [&self.values[cutoff..], &self.values[..cutoff]].concat()
        } else {
            self.values[..cutoff].into()
        }
    }
}

impl<T, const N: usize> Readings<T, N> where T: Default {
    /// Create a new collection.
    #[must_use]
    pub fn new() -> Self {
        Self {
            values: std::array::from_fn(|_| T::default()),
            full: false,
            next: 0
        }
    }
}

impl<T, const N: usize> Default for Readings<T, N> where T: Default {
    fn default() -> Self {
        Self::new()
    }
}

impl<T, const N: usize> From<Readings<T, N>> for Vec<T> where T: Clone {
    fn from(value: Readings<T, N>) -> Self {
        value.as_vec()
    }
}

#[allow(clippy::missing_panics_doc, reason="tests")]
#[cfg(test)]
mod tests {
    const LEN: usize = 4;
    type Readings = super::Readings<u8, LEN>;

    #[test]
    fn new() {
        assert_eq!(
            Readings::new(),
            Readings { values: [0; LEN], full: false, next: 0 }
        );
    }

    mod push {
        use super::*;

        #[test]
        fn when_full() {
            let mut readings = Readings { values: [1, 2, 3, 4], full: true, next: 0 };
            readings.push(5);
            assert_eq!(readings.as_slice(), &[5, 2, 3, 4]);
        }

        #[test]
        fn when_not_full() {
            let mut readings = Readings { values: [1, 2, 0, 0], full: false, next: 2 };
            readings.push(3);
            assert_eq!(readings.as_slice(), &[1, 2, 3]);
        }

        #[test]
        fn when_filled() {
            let mut readings = Readings { values: [1, 2, 3, 0], full: false, next: LEN - 1 };

            readings.push(4);
            assert_eq!(
                readings,
                Readings { values: [1, 2, 3, 4], full: true, next: 0 }
            );
        }
    }

    mod as_slice {
        use super::*;

        #[test]
        fn when_full() {
            let readings = Readings { values: [1, 2, 3, 4], full: true, next: 2 };
            assert_eq!(readings.as_slice(), &[1, 2, 3, 4]);
        }

        #[test]
        fn when_not_full() {
            let readings = Readings { values: [1, 2, 0, 0], full: false, next: 2 };
            assert_eq!(readings.as_slice(), &[1, 2]);
        }
    }

    mod as_vec {
        use super::*;

        #[test]
        fn when_full() {
            let readings = Readings { values: [1, 2, 3, 4], full: true, next: 2 };
            assert_eq!(readings.as_vec(), &[3, 4, 1, 2]);
        }

        #[test]
        fn when_not_full() {
            let readings = Readings { values: [1, 2, 0, 0], full: false, next: 2 };
            assert_eq!(readings.as_vec(), &[1, 2]);
        }
    }

    mod len {
        use super::*;

        #[test]
        fn when_full() {
            let mut readings = Readings { values: [0; LEN], full: true, next: LEN - 1 };
            assert_eq!(readings.len(), LEN);

            // Test wrapping around
            for _ in 0..2 {
                readings.push(1);
                assert_eq!(readings.len(), LEN);
            }
        }

        #[test]
        fn when_empty() {
            let mut readings = Readings { values: [0; LEN], full: false, next: 0 };
            assert_eq!(readings.len(), 0);

            readings.push(1);
            assert_eq!(readings.len(), 1);
        }
    }

    #[test]
    fn is_empty() {
        let mut readings = Readings::new();
        assert!(readings.is_empty());

        for _ in 0..(LEN + 2) {
            readings.push(0);
            assert!(!readings.is_empty());
        }
    }

    mod last {
        use super::*;

        #[test]
        fn when_full() {
            let mut readings = Readings { values: [10, 11, 12, 13], full: true, next: LEN - 1 };
            assert_eq!(readings.last(), Some(12).as_ref());

            // Test wrapping around
            for n in 0..2 {
                readings.push(n);
                assert_eq!(readings.last(), Some(n).as_ref());
            }
        }

        #[test]
        fn when_empty() {
            let mut readings = Readings { values: [0; LEN], full: false, next: 0 };
            assert_eq!(readings.last(), None);

            readings.push(5);
            assert_eq!(readings.last(), Some(5).as_ref());
        }
    }

    #[test]
    fn from_for_vec() {
        let readings = Readings { values: [10, 20, 0, 0], full: false, next: 2 };
        assert_eq!(readings.as_vec(), &[10, 20]);
    }

    #[test]
    fn default() {
        assert_eq!(
            Readings::default(),
            Readings { values: [0; LEN], full: false, next: 0 }
        );
    }
}
