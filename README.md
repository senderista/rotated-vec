# rotated-vec
_A dynamic array with O(1) access and O(√n) inserts and deletes_

This is roughly a drop-in replacement for `Vec`, except that there is no deref to a slice, so underlying slice methods are unavailable. Many of the most useful slice methods have been ported.

Complete documentation is available at https://docs.rs/rotated-vec/.
