//! [`StateDelta`], the per-move record used to undo moves.
//!
//! Before a move is applied, [`Board`](super::board::Board) pushes a
//! `StateDelta` capturing the pieces of state that a move cannot reconstruct on
//! its own — the captured piece, the previous en-passant target, castling
//! rights, and the half-move clock — plus the Zobrist hash of the position
//! *before* the move (used for repetition detection). `unmake_move` pops the
//! stack and restores these fields.

use crate::chess_engine::{
    bitboard::Bitboard, castle_rights::CastleRights, piece::Piece, r#move::Move,
    zobrist::ZobristHash,
};

/// Snapshot of the irreversible board state captured before a move, so the move
/// can later be undone. See the [module documentation](self).
#[derive(Clone, Debug, PartialEq)]
pub struct StateDelta {
    /// The move that was applied.
    pub move_: Move,
    /// The piece captured by the move, if any.
    pub captured_piece: Option<Piece>,
    /// The en-passant target square that was in effect before the move.
    pub en_pass: Bitboard,
    /// The castling rights that were in effect before the move.
    pub castle_rights: CastleRights,
    /// The half-move (fifty-move-rule) clock before the move.
    pub halfmove: u8,
    /// The Zobrist hash of the position before the move; used for threefold
    /// repetition detection without recomputing the hash from scratch.
    pub zobrist_hash: ZobristHash,
}

impl StateDelta {
    /// Bundles the pre-move state into a [`StateDelta`].
    pub fn new(
        move_: Move,
        captured_piece: Option<Piece>,
        en_pass: Bitboard,
        castle_rights: CastleRights,
        halfmove: u8,
        zobrist_hash: ZobristHash,
    ) -> StateDelta {
        StateDelta {
            move_,
            captured_piece,
            en_pass,
            castle_rights,
            halfmove,
            zobrist_hash,
        }
    }
}
