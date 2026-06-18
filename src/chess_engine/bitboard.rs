//! A [`Bitboard`] is a `u64` in which each bit represents one square of the
//! board, giving a compact set-of-squares representation that the rest of the
//! engine is built on (one bitboard per piece type, the set of empty squares,
//! attack masks, and so on).
//!
//! # Square indexing
//!
//! Squares are numbered `0..64` in little-endian rank-file order: bit `0` is
//! `a1`, bit `7` is `h1`, bit `56` is `a8`, and bit `63` is `h8`. The index of
//! a square is `rank * 8 + file`, where `file` and `rank` both range over
//! `0..8` (files `a`–`h` and ranks `1`–`8`). The least-significant bit is
//! therefore `a1` and the most-significant bit is `h8`.
//!
//! Most methods are `const fn`, so masks and lookup tables can be built at
//! compile time.
//!
//! # Examples
//!
//! ```
//! use chess_engine::chess_engine::bitboard::Bitboard;
//!
//! let mut bb = Bitboard::new();
//! bb.set_square(0); // a1
//! bb.set_square(7); // h1
//! assert_eq!(bb.count_bits(), 2);
//! // both squares lie on the first rank
//! assert_eq!(bb.get_rank(0), bb);
//! ```

use std::fmt;
use std::ops::{
    BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Not, Shl, ShlAssign, Shr,
    ShrAssign,
};

use crate::chess_engine::position::Position;

/// A set of board squares packed into a `u64`, one bit per square.
///
/// See the [module documentation](self) for the bit-to-square mapping
/// (`a1` = bit 0 … `h8` = bit 63).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub struct Bitboard(
    /// The raw 64-bit value; bit `n` is set when square `n` is a member of the set.
    pub u64,
);

impl Bitboard {
    // Constructors

    /// Returns an empty bitboard (no squares set).
    ///
    /// ```
    /// use chess_engine::chess_engine::bitboard::Bitboard;
    /// assert!(Bitboard::new().is_empty());
    /// ```
    pub const fn new() -> Self {
        Bitboard(0)
    }

    /// Wraps a raw `u64`, treating each set bit as a member square.
    ///
    /// ```
    /// use chess_engine::chess_engine::bitboard::Bitboard;
    /// assert_eq!(Bitboard::from_u64(0xFF).get_bits(), 0xFF);
    /// ```
    pub const fn from_u64(value: u64) -> Self {
        Bitboard(value)
    }

    /// Returns a bitboard with every square set.
    ///
    /// ```
    /// use chess_engine::chess_engine::bitboard::Bitboard;
    /// assert_eq!(Bitboard::full().count_bits(), 64);
    /// ```
    pub const fn full() -> Self {
        Bitboard(u64::MAX)
    }

    // Basic square operations

    /// Adds `square` (a `0..64` index) to the set. Out-of-range indices are ignored.
    ///
    /// ```
    /// use chess_engine::chess_engine::bitboard::Bitboard;
    /// let mut bb = Bitboard::new();
    /// bb.set_square(5);
    /// assert!(bb.is_square_set(5));
    /// ```
    pub const fn set_square(&mut self, square: usize) {
        if square < 64 {
            self.0 |= 1 << square;
        }
    }

    /// Removes `square` from the set. Out-of-range indices are ignored.
    pub fn clear_square(&mut self, square: usize) {
        if square < 64 {
            self.0 &= !(1 << square);
        }
    }

    /// Flips membership of `square`. Out-of-range indices are ignored.
    pub fn toggle_square(&mut self, square: usize) {
        if square < 64 {
            self.0 ^= 1 << square;
        }
    }

    /// Returns `true` if `square` is a member of the set (always `false` for
    /// out-of-range indices).
    pub fn is_square_set(&self, square: usize) -> bool {
        if square < 64 {
            (self.0 & (1 << square)) != 0
        } else {
            false
        }
    }

    // Getters

    /// Returns the underlying `u64`.
    pub fn get_bits(&self) -> u64 {
        self.0
    }

    /// Returns `true` if no squares are set.
    pub const fn is_empty(&self) -> bool {
        self.0 == 0
    }

    /// Returns `true` if at least one square is set.
    pub const fn is_not_empty(&self) -> bool {
        self.0 != 0
    }

    // Bit manipulation operations

    /// Clears every square (sets the board to empty).
    pub fn clear(&mut self) {
        self.0 = 0;
    }

    /// Sets every square (fills the board).
    pub fn fill(&mut self) {
        self.0 = u64::MAX;
    }

    /// Returns the number of squares set (the population count).
    ///
    /// ```
    /// use chess_engine::chess_engine::bitboard::Bitboard;
    /// assert_eq!(Bitboard::from_u64(0b1111).count_bits(), 4);
    /// ```
    pub const fn count_bits(&self) -> u32 {
        self.0.count_ones()
    }

    /// Returns the number of leading zero bits (counting from the `h8` end).
    pub const fn leading_zeros(&self) -> u32 {
        self.0.leading_zeros()
    }

    /// Returns the number of trailing zero bits, i.e. the index of the
    /// lowest set square (`64` if the board is empty).
    pub const fn trailing_zeros(&self) -> usize {
        self.0.trailing_zeros() as usize
    }

    /// Returns the index of the least-significant set square (closest to `a1`),
    /// or `None` if the board is empty.
    ///
    /// ```
    /// use chess_engine::chess_engine::bitboard::Bitboard;
    /// assert_eq!(Bitboard::from_u64(0b1010).first_set_bit(), Some(1));
    /// assert_eq!(Bitboard::new().first_set_bit(), None);
    /// ```
    pub const fn first_set_bit(&self) -> Option<usize> {
        if self.0 == 0 {
            None
        } else {
            Some(self.0.trailing_zeros() as usize)
        }
    }

    /// Returns the index of the most-significant set square (closest to `h8`),
    /// or `None` if the board is empty.
    ///
    /// ```
    /// use chess_engine::chess_engine::bitboard::Bitboard;
    /// assert_eq!(Bitboard::from_u64(0b1010).last_set_bit(), Some(3));
    /// ```
    pub const fn last_set_bit(&self) -> Option<usize> {
        if self.0 == 0 {
            None
        } else {
            Some(63 - self.0.leading_zeros() as usize)
        }
    }

    /// Removes and returns the least-significant set square, or `None` if the
    /// board is empty. This is the standard way to iterate over set squares.
    ///
    /// ```
    /// use chess_engine::chess_engine::bitboard::Bitboard;
    /// let mut bb = Bitboard::from_u64(0b1010);
    /// assert_eq!(bb.pop_lsb(), Some(1));
    /// assert_eq!(bb.pop_lsb(), Some(3));
    /// assert_eq!(bb.pop_lsb(), None);
    /// ```
    pub fn pop_lsb(&mut self) -> Option<usize> {
        if self.0 == 0 {
            None
        } else {
            let lsb = self.0.trailing_zeros() as usize;
            self.0 &= self.0 - 1; // Clear the least significant bit
            Some(lsb)
        }
    }

    /// Returns a bitboard containing only the least-significant set square
    /// (empty if `self` is empty).
    pub const fn lsb(&self) -> Bitboard {
        if self.0 == 0 {
            Bitboard(0)
        } else {
            Bitboard(self.0 & (0u64.wrapping_sub(self.0)))
        }
    }

    /// Clears the least-significant set square in place.
    pub fn reset_lsb(&mut self) {
        self.0 &= self.0 - 1;
    }

    // File and rank operations

    /// Returns a mask of the eight squares on `file` (`0` = `a` … `7` = `h`);
    /// an empty board for out-of-range files.
    ///
    /// ```
    /// use chess_engine::chess_engine::bitboard::Bitboard;
    /// assert_eq!(Bitboard::file_mask(0).get_bits(), 0x0101_0101_0101_0101);
    /// ```
    pub const fn file_mask(file: usize) -> Bitboard {
        if file < 8 {
            Bitboard(0x0101010101010101 << file)
        } else {
            Bitboard(0)
        }
    }

    /// Returns a mask of the eight squares on `rank` (`0` = rank 1 … `7` = rank
    /// 8); an empty board for out-of-range ranks.
    ///
    /// ```
    /// use chess_engine::chess_engine::bitboard::Bitboard;
    /// assert_eq!(Bitboard::rank_mask(0).get_bits(), 0xFF);
    /// ```
    pub const fn rank_mask(rank: usize) -> Bitboard {
        if rank < 8 {
            Bitboard(0xFF << (rank * 8))
        } else {
            Bitboard(0)
        }
    }

    /// Returns the subset of `self` that lies on `file`.
    pub fn get_file(&self, file: usize) -> Bitboard {
        *self & Self::file_mask(file)
    }

    /// Returns the subset of `self` that lies on `rank`.
    pub fn get_rank(&self, rank: usize) -> Bitboard {
        *self & Self::rank_mask(rank)
    }

    // Square conversion utilities

    /// Converts `(file, rank)` coordinates to a square index, or `None` if
    /// either coordinate is out of `0..8`.
    ///
    /// ```
    /// use chess_engine::chess_engine::bitboard::Bitboard;
    /// assert_eq!(Bitboard::square_from_coords(4, 3), Some(28)); // e4
    /// ```
    pub const fn square_from_coords(file: usize, rank: usize) -> Option<usize> {
        if file < 8 && rank < 8 {
            Some(rank * 8 + file)
        } else {
            None
        }
    }

    /// Converts a square index back to `(file, rank)` coordinates, or `None`
    /// if the index is out of `0..64`.
    ///
    /// ```
    /// use chess_engine::chess_engine::bitboard::Bitboard;
    /// assert_eq!(Bitboard::coords_from_square(28), Some((4, 3))); // e4
    /// ```
    pub const fn coords_from_square(square: usize) -> Option<(usize, usize)> {
        if square < 64 {
            Some((square % 8, square / 8))
        } else {
            None
        }
    }

    /// Returns an iterator that yields each set square's index, lowest first,
    /// consuming a copy of the board.
    ///
    /// ```
    /// use chess_engine::chess_engine::bitboard::Bitboard;
    /// let squares: Vec<usize> = Bitboard::from_u64(0b1010).iter_set_bits().collect();
    /// assert_eq!(squares, vec![1, 3]);
    /// ```
    pub fn iter_set_bits(&self) -> BitboardIterator {
        BitboardIterator { bitboard: *self }
    }

    /// Reverses the bit order, mirroring the board horizontally (file `a` ↔ `h`)
    /// and vertically at the same time. See [`flip_vertical`](Self::flip_vertical)
    /// and [`rotate_180`](Self::rotate_180) for the individual transforms.
    pub const fn reverse(&self) -> Bitboard {
        Bitboard(self.0.reverse_bits())
    }

    /// Flips the board vertically by swapping ranks (`a1` ↔ `a8`); used to view
    /// a position from the other side's perspective.
    pub const fn flip_vertical(&self) -> Bitboard {
        Bitboard(self.0.swap_bytes())
    }

    /// Rotates the board 180° (the composition of a horizontal and a vertical flip).
    pub const fn rotate_180(&self) -> Bitboard {
        Bitboard(self.0.reverse_bits().swap_bytes())
    }

    /// Prints the board to stdout as an 8×8 grid (`X` for set squares), with
    /// rank 8 at the top — handy when debugging move generation.
    pub fn print_bitboard(&self) {
        println!("  +------------------------+");
        for rank in (0..8).rev() {
            print!("{} |", rank + 1);
            for file in 0..8 {
                let pos = Position::from_file_and_rank(file, rank);
                if self.is_square_set(pos.as_usize()) {
                    print!(" X ");
                } else {
                    print!(" . ");
                }
            }
            println!("|");
        }
        println!("  +------------------------+");
        println!("    a  b  c  d  e  f  g  h");
    }
}

/// Iterator over the set squares of a [`Bitboard`], yielding indices lowest first.
///
/// Created by [`Bitboard::iter_set_bits`].
pub struct BitboardIterator {
    bitboard: Bitboard,
}

impl Iterator for BitboardIterator {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        self.bitboard.pop_lsb()
    }
}

// Bitwise operations implementations
impl BitOr for Bitboard {
    type Output = Bitboard;

    fn bitor(self, rhs: Self) -> Self::Output {
        Bitboard(self.0 | rhs.0)
    }
}

impl BitOrAssign for Bitboard {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for Bitboard {
    type Output = Bitboard;

    fn bitand(self, rhs: Self) -> Self::Output {
        Bitboard(self.0 & rhs.0)
    }
}

impl BitAndAssign for Bitboard {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl BitXor for Bitboard {
    type Output = Bitboard;

    fn bitxor(self, rhs: Self) -> Self::Output {
        Bitboard(self.0 ^ rhs.0)
    }
}

impl BitXorAssign for Bitboard {
    fn bitxor_assign(&mut self, rhs: Self) {
        self.0 ^= rhs.0;
    }
}

impl Not for Bitboard {
    type Output = Bitboard;

    fn not(self) -> Self::Output {
        Bitboard(!self.0)
    }
}

// Shift operations implementations
impl Shl<u32> for Bitboard {
    type Output = Bitboard;

    fn shl(self, rhs: u32) -> Self::Output {
        if rhs >= 64 {
            Bitboard(0)
        } else {
            Bitboard(self.0 << rhs)
        }
    }
}

impl ShlAssign<u32> for Bitboard {
    fn shl_assign(&mut self, rhs: u32) {
        if rhs >= 64 {
            self.0 = 0;
        } else {
            self.0 <<= rhs;
        }
    }
}

impl Shr<u32> for Bitboard {
    type Output = Bitboard;

    fn shr(self, rhs: u32) -> Self::Output {
        if rhs >= 64 {
            Bitboard(0)
        } else {
            Bitboard(self.0 >> rhs)
        }
    }
}

impl ShrAssign<u32> for Bitboard {
    fn shr_assign(&mut self, rhs: u32) {
        if rhs >= 64 {
            self.0 = 0;
        } else {
            self.0 >>= rhs;
        }
    }
}

// Additional shift implementations for different integer types
impl Shl<usize> for Bitboard {
    type Output = Bitboard;

    fn shl(self, rhs: usize) -> Self::Output {
        self << (rhs as u32)
    }
}

impl ShlAssign<usize> for Bitboard {
    fn shl_assign(&mut self, rhs: usize) {
        *self <<= rhs as u32;
    }
}

impl Shr<usize> for Bitboard {
    type Output = Bitboard;

    fn shr(self, rhs: usize) -> Self::Output {
        self >> (rhs as u32)
    }
}

impl ShrAssign<usize> for Bitboard {
    fn shr_assign(&mut self, rhs: usize) {
        *self >>= rhs as u32;
    }
}

impl Shl<i8> for Bitboard {
    type Output = Bitboard;

    fn shl(self, rhs: i8) -> Self::Output {
        if rhs >= 0 {
            self << (rhs as u32)
        } else {
            self >> ((-rhs) as u32)
        }
    }
}

impl ShlAssign<i8> for Bitboard {
    fn shl_assign(&mut self, rhs: i8) {
        if rhs >= 0 {
            *self <<= rhs as u32;
        } else {
            *self >>= (-rhs) as u32;
        }
    }
}

impl Shr<i8> for Bitboard {
    type Output = Bitboard;

    fn shr(self, rhs: i8) -> Self::Output {
        if rhs >= 0 {
            self >> (rhs as u32)
        } else {
            self << ((-rhs) as u32)
        }
    }
}

impl ShrAssign<i8> for Bitboard {
    fn shr_assign(&mut self, rhs: i8) {
        if rhs >= 0 {
            *self >>= rhs as u32;
        } else {
            *self <<= (-rhs) as u32;
        }
    }
}

// Display implementation for debugging
impl fmt::Display for Bitboard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "  +---+---+---+---+---+---+---+---+")?;
        for rank in (0..8).rev() {
            write!(f, "{} |", rank + 1)?;
            for file in 0..8 {
                let square = rank * 8 + file;
                if self.is_square_set(square) {
                    write!(f, " X |")?;
                } else {
                    write!(f, "   |")?;
                }
            }
            writeln!(f)?;
            writeln!(f, "  +---+---+---+---+---+---+---+---+")?;
        }
        writeln!(f, "    a   b   c   d   e   f   g   h")?;
        writeln!(f, "Bitboard: 0x{:016X}", self.0)?;
        writeln!(f, "Set bits: {}", self.count_bits())
    }
}

// From trait implementations
impl From<u64> for Bitboard {
    fn from(value: u64) -> Self {
        Bitboard(value)
    }
}

impl From<Bitboard> for u64 {
    fn from(bitboard: Bitboard) -> Self {
        bitboard.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_operations() {
        let mut bb = Bitboard::new();
        assert!(bb.is_empty());

        bb.set_square(0);
        assert!(bb.is_square_set(0));
        assert!(!bb.is_empty());
        assert_eq!(bb.count_bits(), 1);

        bb.clear_square(0);
        assert!(!bb.is_square_set(0));
        assert!(bb.is_empty());
    }

    #[test]
    fn test_bitwise_operations() {
        let bb1 = Bitboard::from(0b1010);
        let bb2 = Bitboard::from(0b1100);

        assert_eq!((bb1 | bb2).get_bits(), 0b1110);
        assert_eq!((bb1 & bb2).get_bits(), 0b1000);
        assert_eq!((bb1 ^ bb2).get_bits(), 0b0110);
    }

    #[test]
    fn test_bit_manipulation() {
        let bb = Bitboard::from(0b1010);
        assert_eq!(bb.count_bits(), 2);
        assert_eq!(bb.first_set_bit(), Some(1));
        assert_eq!(bb.last_set_bit(), Some(3));
    }
}
