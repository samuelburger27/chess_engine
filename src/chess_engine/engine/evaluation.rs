use crate::chess_engine::{
    board::{BLACK, WHITE},
    piece::Piece,
    position::Position,
};

use super::super::board::Board;

const PAWN_VALUE: i32 = 100;
const KNIGHT_VALUE: i32 = 320;
const BISHOP_VALUE: i32 = 330;
const ROOK_VALUE: i32 = 500;
const QUEEN_VALUE: i32 = 900;
const KING_VALUE: i32 = 20000;

// --- Game Phase Increments ---
// Used to determine if we are in opening/middlegame vs endgame
const KNIGHT_PHASE: i32 = 1;
const BISHOP_PHASE: i32 = 1;
const ROOK_PHASE: i32 = 2;
const QUEEN_PHASE: i32 = 4;
const TOTAL_PHASE: i32 =
    (KNIGHT_PHASE * 4) + (BISHOP_PHASE * 4) + (ROOK_PHASE * 4) + (QUEEN_PHASE * 2);

// --- Piece-Square Tables (PSTs) ---
// All tables are from White's perspective. Black's are flipped vertically.

// Note on PST format: The array is indexed from A1 (0) to H8 (63).
// The commented grid shows the board layout for clarity.

// Example for Pawn PST:
// Rank 8 (Promotion): 0,  0,  0,  0,  0,  0,  0,  0
// Rank 7           : 50, 50, 50, 50, 50, 50, 50, 50
// ...
// Rank 2           : 5,  5, 10, 25, 25, 10,  5,  5
// Rank 1           : 0,  0,  0,  0,  0,  0,  0,  0

const PAWN_PST: [i32; 64] = [
    0, 0, 0, 0, 0, 0, 0, 0, 50, 50, 50, 50, 50, 50, 50, 50, 10, 10, 20, 30, 30, 20, 10, 10, 5, 5,
    10, 25, 25, 10, 5, 5, 0, 0, 0, 20, 20, 0, 0, 0, 5, -5, -10, 0, 0, -10, -5, 5, 5, 10, 10, -20,
    -20, 10, 10, 5, 0, 0, 0, 0, 0, 0, 0, 0,
];

const KNIGHT_PST: [i32; 64] = [
    -50, -40, -30, -30, -30, -30, -40, -50, -40, -20, 0, 0, 0, 0, -20, -40, -30, 0, 10, 15, 15, 10,
    0, -30, -30, 5, 15, 20, 20, 15, 5, -30, -30, 0, 15, 20, 20, 15, 0, -30, -30, 5, 10, 15, 15, 10,
    5, -30, -40, -20, 0, 5, 5, 0, -20, -40, -50, -40, -30, -30, -30, -30, -40, -50,
];

const BISHOP_PST: [i32; 64] = [
    -20, -10, -10, -10, -10, -10, -10, -20, -10, 0, 0, 0, 0, 0, 0, -10, -10, 0, 5, 10, 10, 5, 0,
    -10, -10, 5, 5, 10, 10, 5, 5, -10, -10, 0, 10, 10, 10, 10, 0, -10, -10, 10, 10, 10, 10, 10, 10,
    -10, -10, 5, 0, 0, 0, 0, 5, -10, -20, -10, -10, -10, -10, -10, -10, -20,
];

const ROOK_PST: [i32; 64] = [
    0, 0, 0, 0, 0, 0, 0, 0, 5, 10, 10, 10, 10, 10, 10, 5, -5, 0, 0, 0, 0, 0, 0, -5, -5, 0, 0, 0, 0,
    0, 0, -5, -5, 0, 0, 0, 0, 0, 0, -5, -5, 0, 0, 0, 0, 0, 0, -5, -5, 0, 0, 0, 0, 0, 0, -5, 0, 0,
    0, 5, 5, 0, 0, 0,
];

const QUEEN_PST: [i32; 64] = [
    -20, -10, -10, -5, -5, -10, -10, -20, -10, 0, 0, 0, 0, 0, 0, -10, -10, 0, 5, 5, 5, 5, 0, -10,
    -5, 0, 5, 5, 5, 5, 0, -5, 0, 0, 5, 5, 5, 5, 0, -5, -10, 5, 5, 5, 5, 5, 0, -10, -10, 0, 5, 0, 0,
    0, 0, -10, -20, -10, -10, -5, -5, -10, -10, -20,
];

const KING_PST_MG: [i32; 64] = [
    // Middlegame
    -30, -40, -40, -50, -50, -40, -40, -30, -30, -40, -40, -50, -50, -40, -40, -30, -30, -40, -40,
    -50, -50, -40, -40, -30, -30, -40, -40, -50, -50, -40, -40, -30, -20, -30, -30, -40, -40, -30,
    -30, -20, -10, -20, -20, -20, -20, -20, -20, -10, 20, 20, 0, 0, 0, 0, 20, 20, 20, 30, 10, 0, 0,
    10, 30, 20,
];

const KING_PST_EG: [i32; 64] = [
    // Endgame
    -50, -40, -30, -20, -20, -30, -40, -50, -30, -20, -10, 0, 0, -10, -20, -30, -30, -10, 20, 30,
    30, 20, -10, -30, -30, -10, 30, 40, 40, 30, -10, -30, -30, -10, 30, 40, 40, 30, -10, -30, -30,
    -10, 20, 30, 30, 20, -10, -30, -30, -30, 0, 0, 0, 0, -30, -30, -50, -30, -30, -30, -30, -30,
    -30, -50,
];

const ALL_PSTS: [[i32; 64]; 6] = [
    PAWN_PST,
    KNIGHT_PST,
    BISHOP_PST,
    ROOK_PST,
    QUEEN_PST,
    [[0; 64]; 1][0], // King handled separately
];

/// Main evaluation function.
/// Returns a score in centipawns from the perspective of the current player.
/// Positive score means the current player is winning.
/// Negative score means the opponent is winning.
pub(crate) fn evaluate(board: &Board) -> i32 {
    // --- Calculate final tapered score for KINGS ---

    let mut total_score = 0;
    let mut game_phase = 0;

    // Iterate over piece types (Pawn to Queen), KING IS HANDLED SEPARATELY
    for (index, mut piece_board) in board.piece_boards.into_iter().enumerate() {
        let (piece, color) = Board::get_piece_information_index(index);
        // kings handled separatly
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
            let pos_score = piece_square_table[pos];
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
    let white_king_sq = board
        .get_piece_bitboard(Piece::King, WHITE)
        .trailing_zeros();
    let white_king_mg_score = KING_PST_MG[white_king_sq];
    let white_king_eg_score = KING_PST_EG[white_king_sq];
    total_score +=
        (white_king_mg_score * phase + white_king_eg_score * (TOTAL_PHASE - phase)) / TOTAL_PHASE;

    // Black King
    let black_king_sq = board
        .get_piece_bitboard(Piece::King, BLACK)
        .trailing_zeros();
    let flipped_black_king_sq = black_king_sq ^ 56;
    let black_king_mg_score = KING_PST_MG[flipped_black_king_sq];
    let black_king_eg_score = KING_PST_EG[flipped_black_king_sq];
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
