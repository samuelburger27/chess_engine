//! [`CastleRights`] tracks which of the four castling moves are still available,
//! packed into the low four bits of a `u8`.
//!
//! The bit layout is `bit 0` = white king-side, `bit 1` = white queen-side,
//! `bit 2` = black king-side, `bit 3` = black queen-side. The index of a given
//! `(colour, side)` pair is `2 * colour + (0 for king-side, 1 for queen-side)`,
//! where `colour` is the [`Turn`] (`0` for white, `1` for black); see
//! [`castle_index`](CastleRights::castle_index).

use crate::chess_engine::board::Turn;
use std::fmt::Debug;

/// The set of still-available castling rights for both players.
///
/// See the [module documentation](self) for the bit layout.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct CastleRights {
    // white_king_side, white_queen_side, black_king_side, black_queen_side
    flags: u8,
}

const WHITE_KING_SIDE: u8 = 0b0001;
const WHITE_QUEEN_SIDE: u8 = 0b0010;
const BLACK_KING_SIDE: u8 = 0b0100;
const BLACK_QUEEN_SIDE: u8 = 0b1000;

impl CastleRights {
    /// Returns the starting rights, with all four castles available.
    ///
    /// ```
    /// use chess_engine::chess_engine::castle_rights::CastleRights;
    /// use chess_engine::chess_engine::board::{WHITE, BLACK};
    /// let cr = CastleRights::make_default();
    /// assert!(cr.can_castle(WHITE, true));
    /// assert!(cr.can_castle(BLACK, false));
    /// ```
    #[must_use]
    pub const fn make_default() -> Self {
        Self { flags: 0b1111 }
    }

    /// Builds rights from four booleans, one per `(colour, side)` combination.
    ///
    /// ```
    /// use chess_engine::chess_engine::castle_rights::CastleRights;
    /// use chess_engine::chess_engine::board::WHITE;
    /// // White may only castle king-side.
    /// let cr = CastleRights::make(true, false, false, false);
    /// assert!(cr.can_castle(WHITE, true));
    /// assert!(!cr.can_castle(WHITE, false));
    /// ```
    #[must_use]
    #[allow(clippy::fn_params_excessive_bools)]
    pub const fn make(
        white_king_side: bool,
        white_queen_side: bool,
        black_king_side: bool,
        black_queen_side: bool,
    ) -> Self {
        let mut flags = 0;
        if white_king_side {
            flags |= WHITE_KING_SIDE;
        }
        if white_queen_side {
            flags |= WHITE_QUEEN_SIDE;
        }
        if black_king_side {
            flags |= BLACK_KING_SIDE;
        }
        if black_queen_side {
            flags |= BLACK_QUEEN_SIDE;
        }
        Self { flags }
    }

    /// Returns `true` if the given player may still castle on the given side
    /// (`king_side = true` for the short castle, `false` for the long castle).
    #[must_use]
    pub fn can_castle(&self, turn: Turn, king_side: bool) -> bool {
        self.flags & (1 << self.castle_index(turn, king_side)) != 0
    }

    /// Clears the right to castle for the given `(player, side)`, e.g. after the
    /// king or the relevant rook moves.
    pub fn remove_castle_right(&mut self, turn: Turn, king_side: bool) {
        self.flags &= !(1 << self.castle_index(turn, king_side));
    }

    /// Returns the `0..4` bit index for a `(player, side)` pair
    /// (`2 * colour + 0/1`); see the [module documentation](self).
    #[must_use]
    pub fn castle_index(&self, turn: Turn, king_side: bool) -> usize {
        2 * usize::from(turn) + usize::from(!king_side)
    }

    /// Returns `true` if the right at the raw bit `index` (`0..4`) is set.
    #[must_use]
    pub const fn castle_at_index(&self, index: usize) -> bool {
        self.flags & (1 << index) != 0
    }
}

/// Formats the four rights as named booleans.
impl Debug for CastleRights {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let wk = self.flags & WHITE_KING_SIDE != 0;
        let wq = self.flags & WHITE_QUEEN_SIDE != 0;
        let bk = self.flags & BLACK_KING_SIDE != 0;
        let bq = self.flags & BLACK_QUEEN_SIDE != 0;

        write!(
            f,
            "CastleRights {{ white_king_side: {wk}, white_queen_side: {wq}, black_king_side: {bk}, black_queen_side: {bq} }}"
        )
    }
}
