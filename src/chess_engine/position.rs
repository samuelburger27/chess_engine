//! A [`Position`] is a single board square stored as a `0..64` index.
//!
//! Squares are numbered the same way as in [`Bitboard`](super::bitboard):
//! little-endian rank-file order with `a1 = 0`, `h1 = 7`, `a8 = 56`,
//! `h8 = 63`. The index of a square is `rank * 8 + file`, where files run `0..8`
//! (`a`–`h`) and ranks run `0..8` (ranks `1`–`8`).
//!
//! # Examples
//!
//! ```
//! use sabertooth::chess_engine::position::Position;
//!
//! let e4 = Position::from_file_and_rank(4, 3);
//! assert_eq!(e4.as_usize(), 28);
//! assert_eq!(e4.algebraic_notation(), "e4");
//! assert_eq!(Position::try_from("e4"), Ok(e4));
//! ```

use crate::chess_engine::{bitboard::Bitboard, constants::EMPTY_BIT_B};

/// A single board square, stored as a `0..64` index.
///
/// See the [module documentation](self) for the square-numbering scheme.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Position(usize);

impl Position {
    /// The number of squares on the board (and the exclusive upper bound for a
    /// valid index).
    pub const MAX_POS: usize = 64;

    /// Every board square, indexed by its own square number; useful for
    /// iterating over the whole board.
    pub const ALL_POS: [Self; Self::MAX_POS] = Self::generate_all_pos();

    /// Builds the [`ALL_POS`](Self::ALL_POS) array at compile time.
    #[must_use]
    pub const fn generate_all_pos() -> [Self; Self::MAX_POS] {
        let mut result = [Self(0); Self::MAX_POS];
        let mut index = 0;
        while index < Self::MAX_POS {
            result[index] = Self(index);
            index += 1;
        }
        result
    }

    /// Creates a position from a raw square index.
    ///
    /// # Panics
    ///
    /// Panics if `index >= 64`.
    ///
    /// ```
    /// use sabertooth::chess_engine::position::Position;
    /// assert_eq!(Position::new(28).as_usize(), 28);
    /// ```
    #[must_use]
    pub const fn new(index: usize) -> Self {
        assert!(index < Self::MAX_POS, "Index must be between 0 and 63");
        Self(index)
    }

    /// Creates a position from `(file, rank)` coordinates (each `0..8`).
    ///
    /// # Panics
    ///
    /// Panics if `file >= 8` or `rank >= 8`.
    ///
    /// ```
    /// use sabertooth::chess_engine::position::Position;
    /// assert_eq!(Position::from_file_and_rank(0, 0).as_usize(), 0); // a1
    /// assert_eq!(Position::from_file_and_rank(7, 7).as_usize(), 63); // h8
    /// ```
    #[must_use]
    pub const fn from_file_and_rank(file: usize, rank: usize) -> Self {
        assert!(
            file < 8 && rank < 8,
            "File and rank must be between 0 and 7"
        );
        Self(rank * 8 + file)
    }

    /// Returns the `(file, rank)` coordinates of this square.
    ///
    /// ```
    /// use sabertooth::chess_engine::position::Position;
    /// assert_eq!(Position::new(28).get_file_and_rank(), (4, 3)); // e4
    /// ```
    #[must_use]
    pub const fn get_file_and_rank(&self) -> (usize, usize) {
        let file = self.0 % 8;
        let rank = self.0 / 8;
        (file, rank)
    }

    /// Returns the square reached by shifting `d_file` files and `d_rank` ranks,
    /// or `None` if that would leave the board. Used to walk piece-movement rays.
    ///
    /// ```
    /// use sabertooth::chess_engine::position::Position;
    /// let e4 = Position::new(28);
    /// assert_eq!(e4.try_rank_file_offset(0, 1), Some(Position::new(36))); // e5
    /// assert_eq!(Position::new(0).try_rank_file_offset(-1, 0), None); // off the a-file
    /// ```
    #[must_use]
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub const fn try_rank_file_offset(&self, d_file: i8, d_rank: i8) -> Option<Self> {
        let file = (self.0 % 8) as i8 + d_file;
        let rank = (self.0 / 8) as i8 + d_rank;
        if file >= 0 && rank >= 0 && file < 8 && rank < 8 {
            return Some(Self::from_file_and_rank(file as usize, rank as usize));
        }
        None
    }

    /// Returns the square `offset` indices away, or `None` if the result falls
    /// outside `0..64`. Note that this does not respect file wrapping — use
    /// [`try_rank_file_offset`](Self::try_rank_file_offset) when edges matter.
    #[must_use]
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn try_offset(&self, offset: i8) -> Option<Self> {
        let index = self.0 as i8 + offset;
        if (0..64).contains(&index) {
            return Some(Self(index as usize));
        }
        None
    }

    /// Returns the raw square index.
    #[must_use]
    pub const fn as_usize(&self) -> usize {
        self.0
    }

    /// Returns a [`Bitboard`] with exactly this square set.
    ///
    /// ```
    /// use sabertooth::chess_engine::position::Position;
    /// assert!(Position::new(28).bitboard().is_square_set(28));
    /// ```
    #[must_use]
    pub const fn bitboard(&self) -> Bitboard {
        let mut board = EMPTY_BIT_B;
        board.set_square(self.as_usize());
        board
    }

    /// Returns the square in algebraic notation, e.g. `"e4"`.
    ///
    /// ```
    /// use sabertooth::chess_engine::position::Position;
    /// assert_eq!(Position::new(28).algebraic_notation(), "e4");
    /// ```
    #[must_use]
    pub fn algebraic_notation(&self) -> String {
        let (file, rank) = self.get_file_and_rank();
        let file_str = match file {
            0 => "a",
            1 => "b",
            2 => "c",
            3 => "d",
            4 => "e",
            5 => "f",
            6 => "g",
            7 => "h",
            // should never happen
            _ => "-",
        }
        .to_string();

        file_str + &(rank + 1).to_string()
    }
}

/// Parses a two-character algebraic square such as `"e4"`.
///
/// Returns `Err(())` if the string is not exactly a file letter (`a`–`h`)
/// followed by a rank digit (`1`–`8`).
///
/// ```
/// use sabertooth::chess_engine::position::Position;
/// assert_eq!(Position::try_from("a1").unwrap().as_usize(), 0);
/// assert!(Position::try_from("z9").is_err());
/// ```
impl TryFrom<&str> for Position {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if value.len() != 2 {
            return Err(());
        }
        let file: usize = match value.chars().nth(0) {
            Some('a') => 0,
            Some('b') => 1,
            Some('c') => 2,
            Some('d') => 3,
            Some('e') => 4,
            Some('f') => 5,
            Some('g') => 6,
            Some('h') => 7,
            _ => return Err(()),
        };

        let Some(ch) = value.chars().nth(1) else {
            return Err(());
        };
        let Some(rank) = ch.to_digit(10) else {
            return Err(());
        };
        if rank > 8 {
            return Err(());
        }
        Ok(Self::from_file_and_rank(file, (rank - 1) as usize))
    }
}

impl From<Position> for usize {
    fn from(pos: Position) -> Self {
        pos.0
    }
}
