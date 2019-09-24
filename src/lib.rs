#![doc(html_root_url = "https://senderista.github.io/sorted-vec/")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/senderista/sorted-vec/master/cells.png")]
#![feature(const_int_conversion)]

use std::mem;
use std::cmp::{min, Ordering};
use std::fmt::Debug;
use std::hash::{Hash, Hasher};
use std::iter::{DoubleEndedIterator, ExactSizeIterator, FromIterator, FusedIterator};
use std::ops::{Index, IndexMut};

/// A dynamic array based on a 2-level rotated array.
///
/// See <a href="https://github.com/senderista/sorted-vec/blob/master/README.md">the repository README</a> for a detailed discussion of this collection's performance
/// benefits and drawbacks.
///
/// # Examples
///
/// ```
/// use rotated_vec::RotatedVec;
///
/// // Type inference lets us omit an explicit type signature (which
/// // would be `RotatedVec<i32>` in this example).
/// let mut vec = RotatedVec::new();
///
/// // Add some integers.
/// vec.push(-1);
/// vec.push(6);
/// vec.push(1729);
/// vec.push(24);
///
/// // Check for a specific one.
/// if !vec.contains(&42) {
///     println!("We don't have the answer to Life, the Universe, and Everything :-(");
/// }
///
/// // Remove an integer at a given index.
/// vec.remove(1);
///
/// // Iterate over everything.
/// for int in &vec {
///     println!("{}", int);
/// }
/// ```
#[derive(Debug, Clone)]
pub struct RotatedVec<T> {
    data: Vec<T>,
    start_indexes: Vec<usize>,
}

/// An iterator over the items of a `RotatedVec`.
///
/// This `struct` is created by the [`iter`] method on [`RotatedVec`][`RotatedVec`].
/// See its documentation for more.
#[derive(Debug, Copy, Clone)]
pub struct Iter<'a, T: 'a> {
    container: &'a RotatedVec<T>,
    next_index: usize,
    next_rev_index: usize,
}

/// An iterator over the items of a `RotatedVec`.
///
/// This `struct` is created by the [`iter_mut`] method on [`RotatedVec`][`RotatedVec`].
/// See its documentation for more.
#[derive(Debug)]
pub struct IterMut<'a, T: 'a> {
    container: &'a mut RotatedVec<T>,
    next_index: usize,
    next_rev_index: usize,
}

/// An owning iterator over the items of a `RotatedVec`.
///
/// This `struct` is created by the [`into_iter`] method on [`RotatedVec`][`RotatedVec`]
/// (provided by the `IntoIterator` trait). See its documentation for more.
///
/// [`RotatedVec`]: struct.RotatedVec.html
/// [`into_iter`]: struct.RotatedVec.html#method.into_iter
#[derive(Debug, Clone)]
pub struct IntoIter<T> {
    vec: Vec<T>,
    next_index: usize,
}

impl<T> RotatedVec<T>
where
    T: Copy + Default + Debug,
{
    /// Makes a new `RotatedVec` without any heap allocations.
    ///
    /// This is a constant-time operation.
    ///
    /// # Examples
    ///
    /// ```
    /// #![allow(unused_mut)]
    /// use rotated_vec::RotatedVec;
    ///
    /// let mut vec: RotatedVec<i32> = RotatedVec::new();
    /// ```
    pub fn new() -> Self {
        RotatedVec {
            data: Vec::new(),
            start_indexes: Vec::new(),
        }
    }

    /// Constructs a new, empty `RotatedVec<T>` with the specified capacity.
    ///
    /// The vector will be able to hold exactly `capacity` elements without
    /// reallocating. If `capacity` is 0, the vector will not allocate.
    ///
    /// It is important to note that although the returned vector has the
    /// *capacity* specified, the vector will have a zero *length*.
    ///
    /// # Examples
    ///
    /// ```
    /// use rotated_vec::RotatedVec;
    ///
    /// let mut vec = RotatedVec::with_capacity(10);
    ///
    /// // The vector contains no items, even though it has capacity for more
    /// assert_eq!(vec.len(), 0);
    ///
    /// // These are all done without reallocating...
    /// for i in 0..10 {
    ///     vec.push(i);
    /// }
    ///
    /// // ...but this may make the vector reallocate
    /// vec.push(11);
    /// ```
    pub fn with_capacity(capacity: usize) -> RotatedVec<T> {
        let start_indexes_capacity = if capacity > 0 {
            Self::get_subarray_idx_from_array_idx(capacity - 1) + 1
        } else {
            0
        };
        RotatedVec {
            data: Vec::with_capacity(capacity),
            start_indexes: Vec::with_capacity(start_indexes_capacity),
        }
    }


    /// Returns a reference to the value in the array, if any, at the given index.
    ///
    /// This is a constant-time operation.
    ///
    /// # Examples
    ///
    /// ```
    /// use rotated_vec::RotatedVec;
    ///
    /// let vec: RotatedVec<_> = vec![1, 2, 3].into();
    /// assert_eq!(vec.get(0), Some(&1));
    /// assert_eq!(vec.get(3), None);
    /// ```
    pub fn get(&self, index: usize) -> Option<&T> {
        if index >= self.data.len() {
            return None;
        }
        let real_idx = self.get_real_index(index);
        Some(&self.data[real_idx])
    }

    /// Returns a mutable reference to the value in the array, if any, at the given index.
    ///
    /// This is a constant-time operation.
    ///
    /// # Examples
    ///
    /// ```
    /// use rotated_vec::RotatedVec;
    ///
    /// let mut vec: RotatedVec<_> = vec![1, 2, 3].into();
    /// assert_eq!(vec.get_mut(0), Some(&mut 1));
    /// assert_eq!(vec.get_mut(3), None);
    /// ```
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        if index >= self.data.len() {
            return None;
        }
        let real_idx = self.get_real_index(index);
        Some(&mut self.data[real_idx])
    }

    /// Swaps two elements in the vector.
    ///
    /// This is a constant-time operation.
    ///
    /// # Arguments
    ///
    /// * a - The index of the first element
    /// * b - The index of the second element
    ///
    /// # Panics
    ///
    /// Panics if `a` or `b` are out of bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// use rotated_vec::RotatedVec;
    ///
    /// let mut vec: RotatedVec<_> = vec!["a", "b", "c", "d"].into();
    /// vec.swap(1, 3);
    /// assert_eq!(vec, vec!["a", "d", "c", "b"].into());
    /// ```
    pub fn swap(&mut self, a: usize, b: usize) {
        self.data.swap(a, b);
    }

    /// Returns the number of elements the vector can hold without
    /// reallocating.
    ///
    /// # Examples
    ///
    /// ```
    /// use rotated_vec::RotatedVec;
    ///
    /// let vec: RotatedVec<i32> = RotatedVec::with_capacity(10);
    /// assert_eq!(vec.capacity(), 10);
    ///
    pub fn capacity(&self) -> usize {
        self.data.capacity()
    }

    /// Reserves the minimum capacity for exactly `additional` more elements to
    /// be inserted in the given `RotatedVec<T>`. After calling `reserve_exact`,
    /// capacity will be greater than or equal to `self.len() + additional`.
    /// Does nothing if the capacity is already sufficient.
    ///
    /// Note that the allocator may give the collection more space than it
    /// requests. Therefore, capacity can not be relied upon to be precisely
    /// minimal. Prefer `reserve` if future insertions are expected.
    ///
    /// # Panics
    ///
    /// Panics if the new capacity overflows `usize`.
    ///
    /// # Examples
    ///
    /// ```
    /// use rotated_vec::RotatedVec;
    ///
    /// let mut vec: RotatedVec<_> = vec![1].into();
    /// vec.reserve_exact(10);
    /// assert!(vec.capacity() >= 11);
    /// ```
    pub fn reserve_exact(&mut self, additional: usize) {
        self.data.reserve_exact(additional);
    }

    /// Reserves capacity for at least `additional` more elements to be inserted
    /// in the given `RotatedVec<T>`. The collection may reserve more space to avoid
    /// frequent reallocations. After calling `reserve`, capacity will be
    /// greater than or equal to `self.len() + additional`. Does nothing if
    /// capacity is already sufficient.
    ///
    /// # Panics
    ///
    /// Panics if the new capacity overflows `usize`.
    ///
    /// # Examples
    ///
    /// ```
    /// use rotated_vec::RotatedVec;
    ///
    /// let mut vec: RotatedVec<_> = vec![1].into();
    /// vec.reserve(10);
    /// assert!(vec.capacity() >= 11);
    /// ```
    pub fn reserve(&mut self, additional: usize) {
        self.data.reserve(additional);
    }

    /// Shrinks the capacity of the vector as much as possible.
    ///
    /// It will drop down as close as possible to the length but the allocator
    /// may still inform the vector that there is space for a few more elements.
    ///
    /// # Examples
    ///
    /// ```
    /// use rotated_vec::RotatedVec;
    ///
    /// let mut vec = RotatedVec::with_capacity(10);
    /// vec.extend([1, 2, 3].iter().cloned());
    /// assert_eq!(vec.capacity(), 10);
    /// vec.shrink_to_fit();
    /// assert!(vec.capacity() >= 3);
    /// ```
    pub fn shrink_to_fit(&mut self) {
        self.data.shrink_to_fit();
    }

   /// Shortens the vector, keeping the first `len` elements and dropping
    /// the rest.
    ///
    /// This is an O(√n) operation.
    ///
    /// If `len` is greater than the vector's current length, this has no
    /// effect.
    ///
    /// Note that this method has no effect on the allocated capacity
    /// of the vector.
    ///
    /// # Examples
    ///
    /// Truncating a five element vector to two elements:
    ///
    /// ```
    /// use rotated_vec::RotatedVec;
    ///
    /// let mut vec: RotatedVec<_> = vec![1, 2, 3, 4, 5].into();
    /// vec.truncate(2);
    /// assert_eq!(vec, vec![1, 2].into());
    /// ```
    ///
    /// No truncation occurs when `len` is greater than the vector's current
    /// length:
    ///
    /// ```
    /// use rotated_vec::RotatedVec;
    ///
    /// let mut vec: RotatedVec<_> = vec![1, 2, 3].into();
    /// vec.truncate(8);
    /// assert_eq!(vec, vec![1, 2, 3].into());
    /// ```
    ///
    /// Truncating when `len == 0` is equivalent to calling the [`clear`]
    /// method.
    ///
    /// ```
    /// use rotated_vec::RotatedVec;
    ///
    /// let mut vec: RotatedVec<_> = vec![1, 2, 3].into();
    /// vec.truncate(0);
    /// assert_eq!(vec, vec![].into());
    /// ```
    ///
    pub fn truncate(&mut self, len: usize) {
        if len >= self.len() {
            return
        }
        // conceptually, we drop all subarrays after the truncated length,
        // then un-rotate the new last subarray, then drop any remaining elements.
        self.unrotate_last_subarray();
         // drop subarrays after truncated length
        let last_subarray_idx = Self::get_subarray_idx_from_array_idx(self.len() - 1);
        self.start_indexes.truncate(last_subarray_idx + 1);
        // truncate data array
        self.data.truncate(len);
    }

    /// Gets an iterator that visits the values in the `RotatedVec` in order.
    ///
    /// # Examples
    ///
    /// ```
    /// use rotated_vec::RotatedVec;
    ///
    /// let vec: RotatedVec<usize> = vec![1, 2, 3].into();
    /// let mut iter = vec.iter();
    /// assert_eq!(iter.next(), Some(&1));
    /// assert_eq!(iter.next(), Some(&2));
    /// assert_eq!(iter.next(), Some(&3));
    /// assert_eq!(iter.next(), None);
    /// ```
    pub fn iter(&self) -> Iter<T> {
        Iter {
            container: self,
            next_index: 0,
            next_rev_index: self.len() - 1,
        }
    }

    /// Gets a mutable iterator that visits the values in the `RotatedVec` in order.
    ///
    /// # Examples
    ///
    /// ```
    /// use rotated_vec::RotatedVec;
    ///
    /// let mut vec: RotatedVec<usize> = vec![1, 2, 3].into();
    /// let mut iter = vec.iter_mut();
    /// let mut current_elem = None;
    /// current_elem = iter.next();
    /// assert_eq!(current_elem, Some(&mut 1));
    /// *current_elem.unwrap() = 2;
    /// current_elem = iter.next();
    /// assert_eq!(current_elem, Some(&mut 2));
    /// *current_elem.unwrap() = 3;
    /// current_elem = iter.next();
    /// assert_eq!(current_elem, Some(&mut 3));
    /// *current_elem.unwrap() = 4;
    /// assert_eq!(iter.next(), None);
    /// assert_eq!(vec, vec![2, 3, 4].into());
    /// ```
    // pub fn iter_mut(&mut self) -> IterMut<'_, T> {
    pub fn iter_mut(&mut self) -> IterMut<T> {
        let len = self.len();
        IterMut {
            container: self,
            next_index: 0,
            next_rev_index: len - 1,
        }
    }

    /// Returns the number of elements in the set.
    ///
    /// This is a constant-time operation.
    ///
    /// # Examples
    ///
    /// ```
    /// use rotated_vec::RotatedVec;
    ///
    /// let mut vec = RotatedVec::new();
    /// assert_eq!(vec.len(), 0);
    /// vec.push(1);
    /// assert_eq!(vec.len(), 1);
    /// ```
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Returns `true` if the set contains no elements.
    ///
    /// This is a constant-time operation.
    ///
    /// # Examples
    ///
    /// ```
    /// use rotated_vec::RotatedVec;
    ///
    /// let mut vec = RotatedVec::new();
    /// assert!(vec.is_empty());
    /// vec.push(1);
    /// assert!(!vec.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Clears the vector, removing all values.
    ///
    /// This is a constant-time operation.
    ///
    /// # Examples
    ///
    /// ```
    /// use rotated_vec::RotatedVec;
    ///
    /// let mut vec = RotatedVec::new();
    /// vec.push(1);
    /// vec.clear();
    /// assert!(vec.is_empty());
    /// ```
    pub fn clear(&mut self) {
        self.data.clear();
        self.start_indexes.clear();
    }

    /// Returns `true` if the `RotatedVec` contains an element equal to the
    /// given value.
    ///
    /// # Examples
    ///
    /// ```
    /// use rotated_vec::RotatedVec;
    ///
    /// let mut vec = RotatedVec::new();
    ///
    /// vec.push(0);
    /// vec.push(1);
    ///
    /// assert_eq!(vec.contains(&1), true);
    /// assert_eq!(vec.contains(&10), false);
    /// ```
    pub fn contains(&self, x: &T) -> bool
        where T: PartialEq<T>
    {
        self.data.contains(x)
    }

    /// Appends an element to the back of a collection.
    ///
    /// This is a constant-time operation.
    ///
    /// # Panics
    ///
    /// Panics if the number of elements in the vector overflows a `usize`.
    ///
    /// # Examples
    ///
    /// ```
    /// use rotated_vec::RotatedVec;
    ///
    /// let mut vec: RotatedVec<_> = vec![1, 2].into();
    /// vec.push(3);
    /// assert_eq!(vec, vec![1, 2, 3].into());
    /// ```
    pub fn push(&mut self, value: T) {
        self.insert(self.len(), value);
    }

    /// Removes the last element from a vector and returns it, or [`None`] if it
    /// is empty.
    ///
    /// This is a constant-time operation.
    ///
    /// # Examples
    ///
    /// ```
    /// use rotated_vec::RotatedVec;
    ///
    /// let mut vec: RotatedVec<_> = vec![1, 2, 3].into();
    /// assert_eq!(vec.pop(), Some(3));
    /// assert_eq!(vec, vec![1, 2].into());
    /// ```
    pub fn pop(&mut self) -> Option<T> {
        if self.is_empty() {
            None
        } else {
            Some(self.remove(self.len() - 1))
        }
    }

    /// Inserts an element at position `index` within the vector.
    ///
    /// This is an O(√n) operation.
    ///
    /// # Panics
    ///
    /// Panics if `index > len`.
    ///
    /// # Examples
    ///
    ///
    /// ```
    /// use rotated_vec::RotatedVec;
    ///
    /// let mut vec: RotatedVec<_> = vec![1, 2, 3].into();
    /// vec.insert(1, 4);
    /// assert_eq!(vec, vec![1, 4, 2, 3].into());
    /// vec.insert(4, 5);
    /// assert_eq!(vec, vec![1, 4, 2, 3, 5].into());
    /// ```
    pub fn insert(&mut self, index: usize, element: T) {
        assert!(index <= self.len());
        let insert_idx = if index < self.len() {
            self.get_real_index(index)
        } else {
            self.len()
        };
        // find subarray containing this insertion point
        let subarray_idx = Self::get_subarray_idx_from_array_idx(insert_idx);
        // inserted element could be in a new subarray
        debug_assert!(subarray_idx <= self.start_indexes.len());
        // create a new subarray if necessary
        if subarray_idx == self.start_indexes.len() {
            self.start_indexes.push(0);
        }
        let subarray_offset = Self::get_array_idx_from_subarray_idx(subarray_idx);
        // if insertion point is in last subarray and last subarray isn't full, just insert the new element
        if subarray_idx == self.start_indexes.len() - 1 && !self.is_last_subarray_full() {
            // Since we always insert into a partially full subarray in order,
            // there is no need to update the pivot location.
            debug_assert!(self.start_indexes[subarray_idx] == 0);
            self.data.insert(insert_idx, element);
            debug_assert!(self.assert_invariants());
            return;
        }
        // From now on, we can assume that the subarray we're inserting into is always full.
        let next_subarray_offset = Self::get_array_idx_from_subarray_idx(subarray_idx + 1);
        let subarray = &mut self.data[subarray_offset..next_subarray_offset];
        let pivot_offset = self.start_indexes[subarray_idx];
        let insert_offset = insert_idx - subarray_offset;
        let end_offset = if pivot_offset == 0 {
            subarray.len() - 1
        } else {
            pivot_offset - 1
        };
        let mut prev_end_elem = subarray[end_offset];
        // this logic is best understood with a diagram of a rotated array, e.g.:
        //
        // ------------------------------------------------------------------------
        // | 12 | 13 | 14 | 15 | 16 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | 11 |
        // ------------------------------------------------------------------------
        //
        if end_offset < pivot_offset && insert_offset >= pivot_offset {
            subarray.copy_within(pivot_offset..insert_offset, end_offset);
            subarray[insert_offset - 1] = element;
            self.start_indexes[subarray_idx] = end_offset;
        } else {
            subarray.copy_within(insert_offset..end_offset, insert_offset + 1);
            subarray[insert_offset] = element;
        }
        debug_assert!(self.assert_invariants());
        let max_subarray_idx = self.start_indexes.len() - 1;
        let next_subarray_idx = subarray_idx + 1;
        let last_subarray_full = self.is_last_subarray_full();
        // now loop over all remaining subarrays, setting the first (pivot) of each to the last of its predecessor
        for (i, pivot_offset_ref) in self.start_indexes[next_subarray_idx..].iter_mut().enumerate() {
            let cur_subarray_idx = next_subarray_idx + i;
            // if the last subarray isn't full, skip it
            if cur_subarray_idx == max_subarray_idx && !last_subarray_full {
                break;
            }
            let end_offset = if *pivot_offset_ref == 0 {
                cur_subarray_idx
            } else {
                *pivot_offset_ref - 1
            };
            let end_idx = end_offset + Self::get_array_idx_from_subarray_idx(cur_subarray_idx);
            let next_end_elem = self.data[end_idx];
            self.data[end_idx] = prev_end_elem;
            *pivot_offset_ref = end_offset;
            prev_end_elem = next_end_elem;
        }
        // if the last subarray was full, append current last element to a new subarray, otherwise insert last element in rotated order
        if last_subarray_full {
            self.data.push(prev_end_elem);
            self.start_indexes.push(0);
        } else {
            let max_subarray_offset = Self::get_array_idx_from_subarray_idx(max_subarray_idx);
            // since `prev_end_elem` is guaranteed to be <= the pivot value, we always insert it at the pivot location
            self.data.insert(max_subarray_offset, prev_end_elem);
        }
        // debug_assert!(self.data[self.get_real_index(index)] == element);
        debug_assert!(self.assert_invariants());
    }

    /// Removes and returns the element at position `index` within the vector.
    ///
    /// This is an O(√n) operation.
    ///
    /// # Panics
    ///
    /// Panics if `index` is out of bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// use rotated_vec::RotatedVec;
    ///
    /// let mut vec: RotatedVec<_> = vec![1, 2, 3].into();
    /// assert_eq!(vec.remove(1), 2);
    /// assert_eq!(vec, vec![1, 3].into());
    /// ```
    pub fn remove(&mut self, index: usize) -> T {
        assert!(index < self.len());
        let old_len = self.len();
        let mut remove_idx = self.get_real_index(index);
        let max_subarray_idx = self.start_indexes.len() - 1;
        let max_subarray_offset = Self::get_array_idx_from_subarray_idx(max_subarray_idx);
        // find subarray containing the element to remove
        let subarray_idx = Self::get_subarray_idx_from_array_idx(remove_idx);
        debug_assert!(subarray_idx <= max_subarray_idx);
        let subarray_offset = Self::get_array_idx_from_subarray_idx(subarray_idx);
        // if we're not removing an element in the last subarray, then we end up deleting its first element,
        // which is always at the first offset since it's in order
        let mut max_subarray_remove_idx = if subarray_idx == max_subarray_idx {
            remove_idx
        } else {
            max_subarray_offset
        };
        // if the last subarray was rotated, un-rotate it to maintain insert invariant
        if self.is_last_subarray_full() {
            let last_start_offset = self.start_indexes[max_subarray_idx];
            // rotate left by the start offset
            self.data[max_subarray_offset..].rotate_left(last_start_offset);
            self.start_indexes[max_subarray_idx] = 0;
            // the remove index might change after un-rotating the last subarray
            if subarray_idx == max_subarray_idx {
                remove_idx = self.get_real_index(index);
                max_subarray_remove_idx = remove_idx;
            }
        }
        // if insertion point is not in last subarray, perform a "hard exchange"
        if subarray_idx < max_subarray_idx {
            // From now on, we can assume that the subarray we're removing from is full.
            let next_subarray_offset = Self::get_array_idx_from_subarray_idx(subarray_idx + 1);
            let subarray = &mut self.data[subarray_offset..next_subarray_offset];
            let pivot_offset = self.start_indexes[subarray_idx];
            let remove_offset = remove_idx - subarray_offset;
            let end_offset = if pivot_offset == 0 {
                subarray.len() - 1
            } else {
                pivot_offset - 1
            };
            // this logic is best understood with a diagram of a rotated array, e.g.:
            //
            // ------------------------------------------------------------------------
            // | 12 | 13 | 14 | 15 | 16 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | 11 |
            // ------------------------------------------------------------------------
            //
            let mut prev_end_offset = if end_offset < pivot_offset && remove_offset >= pivot_offset
            {
                subarray.copy_within(pivot_offset..remove_offset, pivot_offset + 1);
                let new_pivot_offset = if pivot_offset == subarray.len() - 1 {
                    0
                } else {
                    pivot_offset + 1
                };
                self.start_indexes[subarray_idx] = new_pivot_offset;
                pivot_offset
            } else {
                subarray.copy_within(remove_offset + 1..=end_offset, remove_offset);
                end_offset
            };
            let next_subarray_idx = min(max_subarray_idx, subarray_idx + 1);
            // now perform an "easy exchange" in all remaining subarrays except the last,
            // setting the last element of each to the first element of its successor.
            for (i, pivot_offset_ref) in self.start_indexes[next_subarray_idx..max_subarray_idx]
                .iter_mut()
                .enumerate()
            {
                let cur_subarray_idx = next_subarray_idx + i;
                let cur_subarray_offset = Self::get_array_idx_from_subarray_idx(cur_subarray_idx);
                let prev_end_idx =
                    prev_end_offset + Self::get_array_idx_from_subarray_idx(cur_subarray_idx - 1);
                self.data[prev_end_idx] = self.data[cur_subarray_offset + *pivot_offset_ref];
                prev_end_offset = *pivot_offset_ref;
                let new_start_offset = if *pivot_offset_ref == cur_subarray_idx {
                    0
                } else {
                    *pivot_offset_ref + 1
                };
                *pivot_offset_ref = new_start_offset;
            }
            // now we fix up the last subarray. if it was initially full, we need to un-rotate it to maintain the insert invariant.
            // if the removed element is in the last subarray, we just un-rotate and remove() on the vec, updating auxiliary arrays.
            // otherwise, we copy the first element to the last position of the previous subarray, then remove it and fix up
            // auxiliary arrays.
            let prev_end_idx =
                prev_end_offset + Self::get_array_idx_from_subarray_idx(max_subarray_idx - 1);
            // since the last subarray is always in order, its first element is always on the first offset
            self.data[prev_end_idx] = self.data[max_subarray_offset];
        }
        let element = self.data.remove(max_subarray_remove_idx);
        // if last subarray is now empty, trim start_indexes
        if max_subarray_offset == self.data.len() {
            self.start_indexes.pop();
        }
        debug_assert!(self.len() == old_len - 1);
        debug_assert!(self.assert_invariants());
        element
    }

    /// Moves all the elements of `other` into `self`, leaving `other` empty.
    ///
    /// # Panics
    ///
    /// Panics if the number of elements in the array overflows a `usize`.
    ///
    /// # Examples
    ///
    /// ```
    /// use rotated_vec::RotatedVec;
    ///
    /// let mut vec: RotatedVec<_> = vec![1, 2, 3].into();
    /// let mut vec2: RotatedVec<_> = vec![4, 5, 6].into();
    /// vec.append(&mut vec2);
    /// assert_eq!(vec, vec![1, 2, 3, 4, 5, 6].into());
    /// assert_eq!(vec2, vec![].into());
    /// ```
    pub fn append(&mut self, other: &mut Self) {
        // if the last subarray is partially full, un-rotate it so we can append directly
        if !self.is_last_subarray_full() {
            self.unrotate_last_subarray();
        }
        // append data directly to backing array
        self.data.append(&mut other.data);
        // fix up start indexes
        let last_subarray_idx = Self::get_subarray_idx_from_array_idx(self.data.len() - 1);
        self.start_indexes.resize(last_subarray_idx + 1, 0);
        // clear all data in `other`
        other.clear();
    }

    /// Sorts the vector.
    ///
    /// This sort is stable (i.e., does not reorder equal elements) and `O(n log n)` worst-case.
    ///
    /// When applicable, unstable sorting is preferred because it is generally faster than stable
    /// sorting and it doesn't allocate auxiliary memory.
    /// See [`sort_unstable`](#method.sort_unstable).
    ///
    /// # Current implementation
    ///
    /// The current algorithm is an adaptive, iterative merge sort inspired by
    /// [timsort](https://en.wikipedia.org/wiki/Timsort).
    /// It is designed to be very fast in cases where the vector is nearly sorted, or consists of
    /// two or more sorted sequences concatenated one after another.
    ///
    /// Also, it allocates temporary storage half the size of `self`, but for short vectors a
    /// non-allocating insertion sort is used instead.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(is_sorted)]
    /// use rotated_vec::RotatedVec;
    ///
    /// let mut vec: RotatedVec<_> = vec![-5, 4, 1, -3, 2].into();
    ///
    /// vec.sort();
    /// assert!(vec.iter().is_sorted());
    /// ```
    pub fn sort(&mut self)
        where T: Ord
    {
        self.data.sort();
        // TODO: we really want slice.fill() here when it becomes available
        for idx in self.start_indexes.as_mut_slice() {
            *idx = 0;
        }
    }

    /// Sorts the vector, but may not preserve the order of equal elements.
    ///
    /// This sort is unstable (i.e., may reorder equal elements), in-place
    /// (i.e., does not allocate), and `O(n log n)` worst-case.
    ///
    /// # Current implementation
    ///
    /// The current algorithm is based on [pattern-defeating quicksort][pdqsort] by Orson Peters,
    /// which combines the fast average case of randomized quicksort with the fast worst case of
    /// heapsort, while achieving linear time on vectors with certain patterns. It uses some
    /// randomization to avoid degenerate cases, but with a fixed seed to always provide
    /// deterministic behavior.
    ///
    /// It is typically faster than stable sorting, except in a few special cases, e.g., when the
    /// vector consists of several concatenated sorted sequences.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(is_sorted)]
    /// use rotated_vec::RotatedVec;
    ///
    /// let mut vec: RotatedVec<_> = vec![-5, 4, 1, -3, 2].into();
    ///
    /// vec.sort_unstable();
    /// assert!(vec.iter().is_sorted());
    /// ```
    ///
    /// [pdqsort]: https://github.com/orlp/pdqsort
    pub fn sort_unstable(&mut self)
        where T: Ord
    {
        self.data.sort_unstable();
        // TODO: we really want slice.fill() here when it becomes available
        for idx in self.start_indexes.as_mut_slice() {
            *idx = 0;
        }
    }

    // this returns the index in the backing array of the given logical index
    fn get_real_index(&self, index: usize) -> usize {
        debug_assert!(index < self.data.len());
        let subarray_idx = Self::get_subarray_idx_from_array_idx(index);
        let subarray_start_idx = Self::get_array_idx_from_subarray_idx(subarray_idx);
        let subarray_len = if subarray_idx == self.start_indexes.len() - 1 {
            self.data.len() - subarray_start_idx
        } else {
            subarray_idx + 1
        };
        debug_assert!(index >= subarray_start_idx);
        let idx_offset = index - subarray_start_idx;
        let pivot_offset = self.start_indexes[subarray_idx];
        let rotated_offset = (pivot_offset + idx_offset) % subarray_len;
        debug_assert!(rotated_offset < subarray_len);
        let real_idx = subarray_start_idx + rotated_offset;
        real_idx
    }

    fn integer_sum(n: usize)    -> usize {
        // I learned this from a 10-year-old named Gauss
        (n * (n + 1)) / 2
    }

    fn integer_sum_inverse(n: usize) -> usize {
        // y = (x * (x + 1)) / 2
        // x = (sqrt(8 * y + 1) - 1) / 2
        ((f64::from((n * 8 + 1) as u32).sqrt() as usize) - 1) / 2
    }

    fn get_subarray_idx_from_array_idx(idx: usize) -> usize {
        if idx == 0 {
            0
        } else {
            Self::integer_sum_inverse(idx)
        }
    }

    fn get_array_idx_from_subarray_idx(idx: usize) -> usize {
        if idx == 0 {
            0
        } else {
            Self::integer_sum(idx)
        }
    }

    fn is_last_subarray_full(&self) -> bool {
        self.data.len() == Self::get_array_idx_from_subarray_idx(self.start_indexes.len())
    }

    fn unrotate_last_subarray(&mut self) {
        let last_subarray_idx = Self::get_subarray_idx_from_array_idx(self.len() - 1);
        let last_subarray_start_idx = Self::get_array_idx_from_subarray_idx(last_subarray_idx);
        let last_subarray_len = if last_subarray_idx == self.start_indexes.len() - 1 {
            self.len() - last_subarray_start_idx
        } else {
            last_subarray_idx + 1
        };
        let last_subarray_end_idx = last_subarray_start_idx + last_subarray_len;
        let last_subarray = &mut self.data[last_subarray_start_idx..last_subarray_end_idx];
        // un-rotate subarray in-place
        let pivot_offset = self.start_indexes[last_subarray_idx];
        last_subarray.rotate_left(pivot_offset);
        self.start_indexes[last_subarray_idx] = 0;
    }

    #[inline(always)]
    fn assert_invariants(&self) -> bool {
        // assert offset array has proper length
        let expected_start_indexes_len = if self.is_empty() {
            0
        } else {
            Self::get_subarray_idx_from_array_idx(self.len() - 1) + 1
        };
        assert_eq!(self.start_indexes.len(), expected_start_indexes_len);
        // assert index of each subarray's first element lies within the subarray
        assert!(self
            .start_indexes
            .iter()
            .enumerate()
            .all(|(idx, &offset)| offset <= idx));
        true
    }

    // given data array, initialize offset array
    fn init(&mut self) {
        if self.data.is_empty() {
            debug_assert!(self.start_indexes.is_empty());
        } else {
            let last_subarray_idx = Self::get_subarray_idx_from_array_idx(self.data.len() - 1);
            self.start_indexes = vec![0; last_subarray_idx + 1];
        }
    }
}

impl<T> PartialEq for RotatedVec<T>
where
    T: Copy + Default + Debug + PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        if self.len() != other.len() {
            return false;
        }
        for i in 0..self.len() {
            if self.get(i).unwrap() != other.get(i).unwrap() {
                return false;
            }
        }
        true
    }
}

impl<T> Eq for RotatedVec<T>
where
    T: Copy + Default + Debug + PartialEq
{}

impl<T> PartialOrd for RotatedVec<T>
where
    T: Copy + Default + Debug + PartialOrd
{
    fn partial_cmp(&self, other: &RotatedVec<T>) -> Option<Ordering> {
        self.iter().partial_cmp(other.iter())
    }
}

impl<T> Ord for RotatedVec<T>
where
    T: Copy + Default + Debug + Ord
{
    fn cmp(&self, other: &RotatedVec<T>) -> Ordering {
        self.iter().cmp(other.iter())
    }
}

impl<T> Hash for RotatedVec<T>
where
    T: Copy + Default + Debug + PartialEq + Hash
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        for i in 0..self.len() {
            self.get(i).hash(state);
        }
    }
}

impl<T> Index<usize> for RotatedVec<T>
where
    T: Copy + Default + Debug,
{
    type Output = T;

    #[inline]
    fn index(&self, index: usize) -> &T {
        self.get(index).expect("Out of bounds access")
    }
}

impl<T> IndexMut<usize> for RotatedVec<T>
where
    T: Copy + Default + Debug,
{
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut T {
        self.get_mut(index).expect("Out of bounds access")
    }
}

impl<T> Extend<T> for RotatedVec<T>
where
    T: Copy + Default + Debug,
{
    fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = T>,
    {
        // if the last subarray is partially full, un-rotate it so we can append directly
        if !self.is_last_subarray_full() {
            self.unrotate_last_subarray();
        }
        // append data directly to backing array
        self.data.extend(iter);
        // fix up start indexes
        let last_subarray_idx = Self::get_subarray_idx_from_array_idx(self.data.len() - 1);
        self.start_indexes.resize(last_subarray_idx + 1, 0);
    }
}

impl<'a, T> Iterator for Iter<'a, T>
where
    T: Copy + Default + Debug,
{
    type Item = &'a T;

    fn next(&mut self) -> Option<&'a T> {
        if self.next_index > self.next_rev_index {
            None
        } else {
            let current = self.container.get(self.next_index);
            self.next_index += 1;
            current
        }
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.next_index += n;
        if self.next_index > self.next_rev_index {
            None
        } else {
            let nth = self.container.get(self.next_index);
            self.next_index += 1;
            nth
        }
    }

    fn count(self) -> usize {
        self.container.data.len() - self.next_index
    }

    fn last(self) -> Option<Self::Item> {
        self.container.get(self.container.data.len() - 1)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining_count = self.container.data.len() - self.next_index;
        (remaining_count, Some(remaining_count))
    }
}

impl<'a, T> DoubleEndedIterator for Iter<'a, T>
where
    T: Copy + Default + Debug,
{
    fn next_back(&mut self) -> Option<&'a T> {
        if self.next_rev_index < self.next_index {
            None
        } else {
            let current = self.container.get(self.next_rev_index);
            self.next_rev_index -= 1;
            current
        }
    }

    fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
        self.next_rev_index -= n;
        if self.next_rev_index < self.next_index {
            None
        } else {
            let nth = self.container.get(self.next_rev_index);
            self.next_rev_index -= 1;
            nth
        }
    }
}

impl<T> ExactSizeIterator for Iter<'_, T>
where
    T: Copy + Default + Debug,
{
    fn len(&self) -> usize {
        self.container.len()
    }
}

impl<T> FusedIterator for Iter<'_, T> where T: Copy + Default + Debug {}

impl<'a, T> Iterator for IterMut<'a, T>
where
    T: Copy + Default + Debug,
{
    type Item = &'a mut T;

    // unsafe code required, see:
    // https://www.reddit.com/r/rust/comments/6ffrbs/implementing_a_safe_mutable_iterator/
    // https://stackoverflow.com/questions/25730586/how-can-i-create-my-own-data-structure-with-an-iterator-that-returns-mutable-ref
    // https://stackoverflow.com/questions/27118398/simple-as-possible-example-of-returning-a-mutable-reference-from-your-own-iterat
    fn next(&mut self) -> Option<Self::Item> {
        if self.next_index > self.next_rev_index {
            None
        } else {
            let current = self.container.get_mut(self.next_index);
            self.next_index += 1;
            // see MutItems example at https://docs.rs/strided/0.2.9/src/strided/base.rs.html
            // per above links, rustc cannot understand that we never return two mutable references to the same object,
            // so we have to use unsafe code to coerce the return value to the desired lifetime
            unsafe { mem::transmute(current) }
        }
    }
}

impl<'a, T> IntoIterator for &'a RotatedVec<T>
where
    T: Copy + Default + Debug,
{
    type Item = &'a T;
    type IntoIter = Iter<'a, T>;

    fn into_iter(self) -> Iter<'a, T> {
        self.iter()
    }
}

impl<T> IntoIterator for RotatedVec<T>
where
    T: Copy + Default + Debug,
{
    type Item = T;
    type IntoIter = IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter {
            vec: self.into(),
            next_index: 0,
        }
    }
}

impl<'a, T> Iterator for IntoIter<T>
where
    T: Copy + Default + Debug,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next_index >= self.vec.len() {
            None
        } else {
            let current = self.vec[self.next_index];
            self.next_index += 1;
            Some(current)
        }
    }
}

impl<'a, T> From<&'a [T]> for RotatedVec<T>
where
    T: Copy + Default + Debug,
{
    fn from(slice: &'a [T]) -> Self {
        let mut this = RotatedVec {
            data: slice.to_vec(),
            start_indexes: Vec::new(),
        };
        this.init();
        this
    }
}

impl<T> From<Vec<T>> for RotatedVec<T>
where
    T: Copy + Default + Debug,
{
    fn from(vec: Vec<T>) -> Self {
        let mut this = RotatedVec {
            data: vec,
            start_indexes: Vec::new(),
        };
        this.init();
        this
    }
}

impl<T> Into<Vec<T>> for RotatedVec<T>
where
    T: Copy + Default + Debug,
{
    fn into(mut self) -> Vec<T> {
        // un-rotate the data array in-place and steal it from self
        for (i, &pivot_offset) in self.start_indexes.iter().enumerate() {
            let subarray_start_idx = Self::get_array_idx_from_subarray_idx(i);
            let subarray_len = if i == self.start_indexes.len() - 1 {
                self.data.len() - subarray_start_idx
            } else {
                i + 1
            };
            let subarray_end_idx = subarray_start_idx + subarray_len;
            let subarray = &mut self.data[subarray_start_idx..subarray_end_idx];
            // un-rotate subarray in-place
            subarray.rotate_left(pivot_offset);
        }
        self.data
    }
}

impl<T> FromIterator<T> for RotatedVec<T>
where
    T: Copy + Default + Debug,
{
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut this = RotatedVec {
            data: Vec::from_iter(iter.into_iter()),
            start_indexes: Vec::new(),
        };
        this.init();
        this
    }
}

impl<T> Default for RotatedVec<T>
where
    T: Copy + Default + Debug,
{
    #[inline]
    fn default() -> RotatedVec<T> {
        RotatedVec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::distributions::Standard;
    use rand::prelude::*;
    use rand::rngs::SmallRng;

    const NUM_ELEMS: usize = 1 << 10;
    const SEED: u64 = u64::from_be_bytes(*b"cafebabe");

    #[test]
    fn push_pop() {
        let mut rng: SmallRng = SeedableRng::seed_from_u64(SEED);
        let iter = rng.sample_iter(&Standard).take(NUM_ELEMS);
        let mut rotated_vec: RotatedVec<usize> = RotatedVec::new();
        for v in iter {
            rotated_vec.push(v);
        }
        let mut rng: SmallRng = SeedableRng::seed_from_u64(SEED);
        let iter = rng.sample_iter(&Standard).take(NUM_ELEMS).collect::<Vec<usize>>().into_iter().rev();
        for v in iter {
            assert_eq!(rotated_vec.pop().unwrap(), v);
        }
        assert!(rotated_vec.is_empty());
    }

    #[test]
    fn compare_iter() {
        let mut rng: SmallRng = SeedableRng::seed_from_u64(SEED);
        let iter = rng.sample_iter(&Standard).take(NUM_ELEMS);
        let mut rotated_vec: RotatedVec<usize> = RotatedVec::new();
        for v in iter {
            rotated_vec.push(v);
        }
        let iter = rotated_vec.iter();
        for (i, &v) in iter.enumerate() {
            assert!(*rotated_vec.get(i).unwrap() == v);
        }
    }

    #[test]
    fn compare_into_iter() {
        let mut rng: SmallRng = SeedableRng::seed_from_u64(SEED);
        let iter = rng.sample_iter(&Standard).take(NUM_ELEMS as usize);
        let mut rotated_vec: RotatedVec<usize> = RotatedVec::new();
        for v in iter {
            rotated_vec.push(v);
        }
        let mut iter = rotated_vec.clone().into_iter();
        for i in 0..NUM_ELEMS {
            assert!(*rotated_vec.get(i).unwrap() == iter.next().unwrap());
        }
    }

    #[test]
    fn test_iter_overrides() {
        let rotated_vec: RotatedVec<_> = (0usize..NUM_ELEMS).collect();
        let iter = rotated_vec.iter();
        assert!(*iter.min().unwrap() == *rotated_vec.get(0).unwrap());
        assert!(*iter.max().unwrap() == *rotated_vec.get(NUM_ELEMS - 1).unwrap());
        assert!(*iter.last().unwrap() == *rotated_vec.get(NUM_ELEMS - 1).unwrap());
        assert!(iter.count() == rotated_vec.len());
        assert!(*iter.last().unwrap() == *rotated_vec.get(NUM_ELEMS - 1).unwrap());
        let step = NUM_ELEMS / 10;
        let mut iter_nth = iter;
        assert!(*iter_nth.nth(step - 1).unwrap() == *rotated_vec.get(step - 1).unwrap());
        assert!(*iter_nth.nth(step - 1).unwrap() == *rotated_vec.get((2 * step) - 1).unwrap());
        let mut iter_nth_back = iter;
        let last_index = rotated_vec.len() - 1;
        assert!(*iter_nth_back.nth_back(step - 1).unwrap() == *rotated_vec.get(last_index - step + 1).unwrap());
        assert!(*iter_nth_back.nth_back(step - 1).unwrap() == *rotated_vec.get(last_index - (2 * step) + 1).unwrap());
        let mut iter_mut = rotated_vec.iter();
        for i in 0..(NUM_ELEMS / 2) {
            assert!(*iter_mut.next().unwrap() == *rotated_vec.get(i).unwrap());
            assert!(*iter_mut.next_back().unwrap() == *rotated_vec.get(last_index - i).unwrap());
        }
        assert!(iter_mut.next().is_none());
    }
}
