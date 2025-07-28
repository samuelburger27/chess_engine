use std::fmt;
use std::ops::{
    BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Not, Shl, ShlAssign, Shr,
    ShrAssign,
};

use crate::board_representation::position::Position;

// 64 bits representing the squares on the chessboard
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub struct Bitboard(pub u64);

impl Bitboard {
    // Constructors
    pub const fn new() -> Self {
        Bitboard(0)
    }

    pub const fn from_u64(value: u64) -> Self {
        Bitboard(value)
    }

    pub const fn full() -> Self {
        Bitboard(u64::MAX)
    }

    // Basic square operations
    pub const fn set_square(&mut self, square: usize) {
        if square < 64 {
            self.0 |= 1 << square;
        }
    }

    pub fn clear_square(&mut self, square: usize) {
        if square < 64 {
            self.0 &= !(1 << square);
        }
    }

    pub fn toggle_square(&mut self, square: usize) {
        if square < 64 {
            self.0 ^= 1 << square;
        }
    }

    pub fn is_square_set(&self, square: usize) -> bool {
        if square < 64 {
            (self.0 & (1 << square)) != 0
        } else {
            false
        }
    }

    // Getters
    pub fn get_bits(&self) -> u64 {
        self.0
    }

    pub const fn is_empty(&self) -> bool {
        self.0 == 0
    }

    pub const fn is_not_empty(&self) -> bool {
        self.0 != 0
    }

    // Bit manipulation operations
    pub fn clear(&mut self) {
        self.0 = 0;
    }

    pub fn fill(&mut self) {
        self.0 = u64::MAX;
    }

    pub const fn count_bits(&self) -> u32 {
        self.0.count_ones()
    }

    pub const fn leading_zeros(&self) -> u32 {
        self.0.leading_zeros()
    }

    pub const fn trailing_zeros(&self) -> usize {
        self.0.trailing_zeros() as usize
    }

    // Find first set bit (least significant bit)
    pub const fn first_set_bit(&self) -> Option<usize> {
        if self.0 == 0 {
            None
        } else {
            Some(self.0.trailing_zeros() as usize)
        }
    }

    // Find last set bit (most significant bit)
    pub const fn last_set_bit(&self) -> Option<usize> {
        if self.0 == 0 {
            None
        } else {
            Some(63 - self.0.leading_zeros() as usize)
        }
    }

    // Pop least significant bit and return its position
    pub fn pop_lsb(&mut self) -> Option<usize> {
        if self.0 == 0 {
            None
        } else {
            let lsb = self.0.trailing_zeros() as usize;
            self.0 &= self.0 - 1; // Clear the least significant bit
            Some(lsb)
        }
    }

    // Get least significant bit as a new bitboard
    pub const fn lsb(&self) -> Bitboard {
        if self.0 == 0 {
            Bitboard(0)
        } else {
            Bitboard(self.0 & (0u64.wrapping_sub(self.0)))
        }
    }

    // Reset least significant bit
    pub fn reset_lsb(&mut self) {
        self.0 &= self.0 - 1;
    }

    // File and rank operations
    pub const fn file_mask(file: usize) -> Bitboard {
        if file < 8 {
            Bitboard(0x0101010101010101 << file)
        } else {
            Bitboard(0)
        }
    }

    pub const fn rank_mask(rank: usize) -> Bitboard {
        if rank < 8 {
            Bitboard(0xFF << (rank * 8))
        } else {
            Bitboard(0)
        }
    }

    pub fn get_file(&self, file: usize) -> Bitboard {
        *self & Self::file_mask(file)
    }

    pub fn get_rank(&self, rank: usize) -> Bitboard {
        *self & Self::rank_mask(rank)
    }

    // Square conversion utilities
    pub const fn square_from_coords(file: usize, rank: usize) -> Option<usize> {
        if file < 8 && rank < 8 {
            Some(rank * 8 + file)
        } else {
            None
        }
    }

    pub const fn coords_from_square(square: usize) -> Option<(usize, usize)> {
        if square < 64 {
            Some((square % 8, square / 8))
        } else {
            None
        }
    }

    // Iterator over set bits
    pub fn iter_set_bits(&self) -> BitboardIterator {
        BitboardIterator { bitboard: *self }
    }

    // Reverse bits (mirror horizontally)
    pub const fn reverse(&self) -> Bitboard {
        Bitboard(self.0.reverse_bits())
    }

    // Flip vertically (swap ranks)
    pub const fn flip_vertical(&self) -> Bitboard {
        Bitboard(self.0.swap_bytes())
    }

    // Rotate 180 degrees
    pub const fn rotate_180(&self) -> Bitboard {
        Bitboard(self.0.reverse_bits().swap_bytes())
    }

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

// Iterator for set bits in a bitboard
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
