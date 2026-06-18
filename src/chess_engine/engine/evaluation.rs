//! Static evaluation: scoring a position without searching.
//!
//! The score combines **material** (fixed value per piece) with **piece-square
//! tables** (PSTs) that reward each piece for standing on good squares. The
//! king's table is *tapered*: a game phase is computed from the remaining
//! non-pawn material and used to interpolate between a middlegame table (king
//! tucked away) and an endgame table (king active), so the king's ideal square
//! shifts smoothly as pieces come off.
//!
//! PSTs are written rank-8-first (array index `0` = `a8`), while board squares
//! are indexed from `a1` = `0`, so white squares are flipped with `sq ^ 56`
//! before indexing and black squares index directly (see `pst_index`).
//!
//! `evaluate` returns centipawns from the side-to-move's perspective:
//! positive is good for whoever is to move.

use crate::chess_engine::{
    board::{BLACK, WHITE},
    piece::Piece,
};

use super::super::board::Board;

/// Material value of a pawn, in centipawns.
const PAWN_VALUE: i32 = 100;
/// Material value of a knight, in centipawns.
const KNIGHT_VALUE: i32 = 320;
/// Material value of a bishop, in centipawns.
const BISHOP_VALUE: i32 = 330;
/// Material value of a rook, in centipawns.
const ROOK_VALUE: i32 = 500;
/// Material value of a queen, in centipawns.
const QUEEN_VALUE: i32 = 900;

// --- Game Phase Increments ---
// Used to determine if we are in opening/middlegame vs endgame
/// Phase weight contributed by each knight.
const KNIGHT_PHASE: i32 = 1;
/// Phase weight contributed by each bishop.
const BISHOP_PHASE: i32 = 1;
/// Phase weight contributed by each rook.
const ROOK_PHASE: i32 = 2;
/// Phase weight contributed by each queen.
const QUEEN_PHASE: i32 = 4;
/// Total phase with all pieces on the board; the king PST interpolates between
/// middlegame (`phase == TOTAL_PHASE`) and endgame (`phase == 0`).
const TOTAL_PHASE: i32 =
    (KNIGHT_PHASE * 4) + (BISHOP_PHASE * 4) + (ROOK_PHASE * 4) + (QUEEN_PHASE * 2);

// --- Piece-Square Tables (PSTs) ---
// All tables are from White's perspective. Black's are flipped vertically.

// All tables are from White's perspective.
//
// Note on PST format: the tables below are written rank-8-first (index 0 = a8,
// index 63 = h1). Board squares are indexed from a1 (0), so white squares must
// be flipped vertically (sq ^ 56) before indexing; black squares index the
// table directly.

// Example for Pawn PST:
// Rank 8 (Promotion): 0,  0,  0,  0,  0,  0,  0,  0
// Rank 7           : 50, 50, 50, 50, 50, 50, 50, 50
// ...
// Rank 2           : 5,  5, 10, 25, 25, 10,  5,  5
// Rank 1           : 0,  0,  0,  0,  0,  0,  0,  0

/// Pawn piece-square table (rank-8-first; see the [module docs](self)).
const PAWN_PST: [i32; 64] = [
    0, 0, 0, 0, 0, 0, 0, 0, 50, 50, 50, 50, 50, 50, 50, 50, 10, 10, 20, 30, 30, 20, 10, 10, 5, 5,
    10, 25, 25, 10, 5, 5, 0, 0, 0, 20, 20, 0, 0, 0, 5, -5, -10, 0, 0, -10, -5, 5, 5, 10, 10, -20,
    -20, 10, 10, 5, 0, 0, 0, 0, 0, 0, 0, 0,
];

/// Knight piece-square table (rank-8-first).
const KNIGHT_PST: [i32; 64] = [
    -50, -40, -30, -30, -30, -30, -40, -50, -40, -20, 0, 0, 0, 0, -20, -40, -30, 0, 10, 15, 15, 10,
    0, -30, -30, 5, 15, 20, 20, 15, 5, -30, -30, 0, 15, 20, 20, 15, 0, -30, -30, 5, 10, 15, 15, 10,
    5, -30, -40, -20, 0, 5, 5, 0, -20, -40, -50, -40, -30, -30, -30, -30, -40, -50,
];

/// Bishop piece-square table (rank-8-first).
const BISHOP_PST: [i32; 64] = [
    -20, -10, -10, -10, -10, -10, -10, -20, -10, 0, 0, 0, 0, 0, 0, -10, -10, 0, 5, 10, 10, 5, 0,
    -10, -10, 5, 5, 10, 10, 5, 5, -10, -10, 0, 10, 10, 10, 10, 0, -10, -10, 10, 10, 10, 10, 10, 10,
    -10, -10, 5, 0, 0, 0, 0, 5, -10, -20, -10, -10, -10, -10, -10, -10, -20,
];

/// Rook piece-square table (rank-8-first).
const ROOK_PST: [i32; 64] = [
    0, 0, 0, 0, 0, 0, 0, 0, 5, 10, 10, 10, 10, 10, 10, 5, -5, 0, 0, 0, 0, 0, 0, -5, -5, 0, 0, 0, 0,
    0, 0, -5, -5, 0, 0, 0, 0, 0, 0, -5, -5, 0, 0, 0, 0, 0, 0, -5, -5, 0, 0, 0, 0, 0, 0, -5, 0, 0,
    0, 5, 5, 0, 0, 0,
];

/// Queen piece-square table (rank-8-first).
const QUEEN_PST: [i32; 64] = [
    -20, -10, -10, -5, -5, -10, -10, -20, -10, 0, 0, 0, 0, 0, 0, -10, -10, 0, 5, 5, 5, 5, 0, -10,
    -5, 0, 5, 5, 5, 5, 0, -5, 0, 0, 5, 5, 5, 5, 0, -5, -10, 5, 5, 5, 5, 5, 0, -10, -10, 0, 5, 0, 0,
    0, 0, -10, -20, -10, -10, -5, -5, -10, -10, -20,
];

/// King piece-square table for the middlegame (rank-8-first); blended with
/// [`KING_PST_EG`] by game phase.
const KING_PST_MG: [i32; 64] = [
    // Middlegame
    -30, -40, -40, -50, -50, -40, -40, -30, -30, -40, -40, -50, -50, -40, -40, -30, -30, -40, -40,
    -50, -50, -40, -40, -30, -30, -40, -40, -50, -50, -40, -40, -30, -20, -30, -30, -40, -40, -30,
    -30, -20, -10, -20, -20, -20, -20, -20, -20, -10, 20, 20, 0, 0, 0, 0, 20, 20, 20, 30, 10, 0, 0,
    10, 30, 20,
];

/// King piece-square table for the endgame (rank-8-first); blended with
/// [`KING_PST_MG`] by game phase.
const KING_PST_EG: [i32; 64] = [
    // Endgame
    -50, -40, -30, -20, -20, -30, -40, -50, -30, -20, -10, 0, 0, -10, -20, -30, -30, -10, 20, 30,
    30, 20, -10, -30, -30, -10, 30, 40, 40, 30, -10, -30, -30, -10, 30, 40, 40, 30, -10, -30, -30,
    -10, 20, 30, 30, 20, -10, -30, -30, -30, 0, 0, 0, 0, -30, -30, -50, -30, -30, -30, -30, -30,
    -30, -50,
];

/// Maps a board square (`a1` = 0) to its index into a rank-8-first PST array.
///
/// White squares are flipped vertically (`sq ^ 56`) so they read the table from
/// White's perspective; black squares index it directly.
fn pst_index(square: usize, is_white: bool) -> usize {
    if is_white {
        square ^ 56
    } else {
        square
    }
}

/// Statically evaluates `board`, returning a centipawn score from the
/// perspective of the side to move (positive = better for the mover).
///
/// Sums material and piece-square bonuses for every non-king piece, then adds
/// the tapered king score (a phase-weighted blend of [`KING_PST_MG`] and
/// [`KING_PST_EG`]). The result is negated for Black so it is always reported
/// from the mover's point of view. The starting position is symmetric, so it
/// evaluates to `0`.
pub(crate) fn evaluate(board: &Board) -> i32 {
    // --- Calculate final tapered score for KINGS ---

    let mut total_score = 0;
    let mut game_phase = 0;

    // Iterate over piece types (Pawn to Queen), KING IS HANDLED SEPARATELY
    for (index, mut piece_board) in board.piece_boards.into_iter().enumerate() {
        let (piece, color) = Board::get_piece_information_index(index);
        // kings handled separately
        if piece == Piece::King {
            continue;
        }
        let piece_value;
        let piece_phase;
        let piece_square_table;
        match piece {
            Piece::Pawn => {
                piece_value = PAWN_VALUE;
                piece_phase = 0;
                piece_square_table = &PAWN_PST;
            }
            Piece::Rook => {
                piece_value = ROOK_VALUE;
                piece_phase = ROOK_PHASE;
                piece_square_table = &ROOK_PST;
            }
            Piece::Knight => {
                piece_value = KNIGHT_VALUE;
                piece_phase = KNIGHT_PHASE;
                piece_square_table = &KNIGHT_PST;
            }
            Piece::Bishop => {
                piece_value = BISHOP_VALUE;
                piece_phase = BISHOP_PHASE;
                piece_square_table = &BISHOP_PST;
            }
            // only queen left
            _ => {
                piece_value = QUEEN_VALUE;
                piece_phase = QUEEN_PHASE;
                piece_square_table = &QUEEN_PST;
            }
        }
        // extract all pieces from bitboard
        while piece_board.is_not_empty() {
            let pos = piece_board.trailing_zeros();
            let pos_score = piece_square_table[pst_index(pos, color == WHITE)];
            total_score += if color == WHITE {
                pos_score + piece_value
            } else {
                -(pos_score + piece_value)
            };
            game_phase += piece_phase;
            piece_board.reset_lsb();
        }
    }
    let phase = std::cmp::min(game_phase, TOTAL_PHASE);
    // White King
    let white_king_sq = pst_index(
        board
            .get_piece_bitboard(Piece::King, WHITE)
            .trailing_zeros(),
        true,
    );
    let white_king_mg_score = KING_PST_MG[white_king_sq];
    let white_king_eg_score = KING_PST_EG[white_king_sq];
    total_score +=
        (white_king_mg_score * phase + white_king_eg_score * (TOTAL_PHASE - phase)) / TOTAL_PHASE;

    // Black King
    let black_king_sq = pst_index(
        board
            .get_piece_bitboard(Piece::King, BLACK)
            .trailing_zeros(),
        false,
    );
    let black_king_mg_score = KING_PST_MG[black_king_sq];
    let black_king_eg_score = KING_PST_EG[black_king_sq];
    total_score -=
        (black_king_mg_score * phase + black_king_eg_score * (TOTAL_PHASE - phase)) / TOTAL_PHASE;

    // Material score for kings is implicitly neutral (king vs king), so we don't add it.
    // If you evaluate checkmate, you would use KING_VALUE here.

    // Return score from the perspective of the current player
    if board.turn == WHITE {
        total_score
    } else {
        -total_score
    }
}

#[cfg(test)]
mod tests {
    use super::evaluate;
    use crate::chess_engine::board::Board;

    #[test]
    fn start_position_is_balanced() {
        // the opening position is mirror-symmetric, so neither side is ahead
        let board = Board::new_start_pos().unwrap();
        assert_eq!(evaluate(&board), 0);
    }

    #[test]
    fn extra_material_favours_the_mover() {
        // White has an extra queen and it is White to move.
        let white = Board::from_fen("4k3/8/8/8/8/8/8/3QK3 w - - 0 1").unwrap();
        assert!(evaluate(&white) > 0);
        // Same position, Black to move: the score is reported from Black's
        // (losing) perspective, so it flips sign.
        let black = Board::from_fen("4k3/8/8/8/8/8/8/3QK3 b - - 0 1").unwrap();
        assert!(evaluate(&black) < 0);
    }
}
