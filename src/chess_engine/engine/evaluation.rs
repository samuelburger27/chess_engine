//! Static evaluation: scoring a position without searching.
//!
//! The evaluation is *tapered*: every piece has a **middlegame** and an
//! **endgame** value and piece-square table (PST), and the two scores are
//! blended by a game phase computed from the remaining non-pawn material. With
//! all pieces on the board the middlegame score dominates; as material comes
//! off, the endgame score takes over. Material values and PSTs are taken from
//! `PeSTO` (Rofchade's tuned tables, see
//! <https://www.chessprogramming.org/PeSTO%27s_Evaluation_Function>).
//!
//! On top of material + PSTs the evaluation knows a handful of classic terms:
//! passed, isolated, and doubled pawns; the bishop pair; rooks on open and
//! semi-open files; a pawn shield in front of a castled king; and a tempo
//! bonus for the side to move. Passed pawns are further scaled by blockade,
//! the king race to the stop square, and a rook behind the passer. Two
//! board-wide terms round it out: *mobility* (safe attack squares per piece,
//! against a per-piece baseline) and *king danger* (attack units on the king
//! zone through a non-linear table, plus open files next to the king).
//!
//! PSTs are written rank-8-first (array index `0` = `a8`), while board squares
//! are indexed from `a1` = `0`, so white squares are flipped with `sq ^ 56`
//! before indexing and black squares index directly (see `pst_index`).
//!
//! `evaluate` returns centipawns from the side-to-move's perspective:
//! positive is good for whoever is to move.

use crate::chess_engine::{
    bitboard::Bitboard,
    board::{BLACK, Turn, WHITE},
    computed_boards::{
        KING_RING_MOVES, KNIGHT_MOVES, PASSED_PAWN_MASKS, PAWN_ATTACKS, bishop_attacks,
        rook_attacks,
    },
    masks::{ADJACENT_FILE_MASKS, FILE_MASKS, RANK_MASKS},
    piece::Piece,
};

use super::super::board::Board;

// --- Material values (middlegame / endgame), in centipawns ---

/// Middlegame material value of a pawn.
const PAWN_MG: i32 = 82;
/// Endgame material value of a pawn.
const PAWN_EG: i32 = 94;
/// Middlegame material value of a knight.
const KNIGHT_MG: i32 = 337;
/// Endgame material value of a knight.
const KNIGHT_EG: i32 = 281;
/// Middlegame material value of a bishop.
const BISHOP_MG: i32 = 365;
/// Endgame material value of a bishop.
const BISHOP_EG: i32 = 297;
/// Middlegame material value of a rook.
const ROOK_MG: i32 = 477;
/// Endgame material value of a rook.
const ROOK_EG: i32 = 512;
/// Middlegame material value of a queen.
const QUEEN_MG: i32 = 1025;
/// Endgame material value of a queen.
const QUEEN_EG: i32 = 936;

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
/// Total phase with all pieces on the board; the evaluation interpolates
/// between middlegame (`phase == TOTAL_PHASE`) and endgame (`phase == 0`).
const TOTAL_PHASE: i32 =
    (KNIGHT_PHASE * 4) + (BISHOP_PHASE * 4) + (ROOK_PHASE * 4) + (QUEEN_PHASE * 2);

/// Middlegame tempo bonus for the side to move.
const TEMPO_MG: i32 = 15;

// --- Pawn structure and piece feature terms ---
// Starting values from common engine practice; tuning waits for self-play.

/// Passed-pawn middlegame bonus, indexed by the pawn's *relative* rank
/// (0 = own back rank; a pawn only ever occupies ranks 1–6).
const PASSED_PAWN_MG: [i32; 8] = [0, 5, 10, 20, 35, 50, 70, 0];
/// Passed-pawn endgame bonus by relative rank.
const PASSED_PAWN_EG: [i32; 8] = [0, 10, 20, 35, 60, 90, 120, 0];
/// Middlegame penalty for a pawn with no friendly pawn on an adjacent file.
const ISOLATED_PAWN_MG: i32 = -12;
/// Endgame penalty for an isolated pawn.
const ISOLATED_PAWN_EG: i32 = -8;
/// Middlegame penalty per *extra* friendly pawn on a file.
const DOUBLED_PAWN_MG: i32 = -10;
/// Endgame penalty per extra friendly pawn on a file.
const DOUBLED_PAWN_EG: i32 = -20;
/// Middlegame bonus for owning both bishops.
const BISHOP_PAIR_MG: i32 = 25;
/// Endgame bonus for owning both bishops.
const BISHOP_PAIR_EG: i32 = 45;
/// Middlegame bonus for a rook on a file with no pawns of either colour.
const ROOK_OPEN_FILE_MG: i32 = 25;
/// Middlegame bonus for a rook on a file with enemy but no friendly pawns.
const ROOK_SEMI_OPEN_FILE_MG: i32 = 12;
/// Middlegame penalty per missing pawn in the shield directly in front of a
/// king standing on its back two ranks.
const KING_SHIELD_MISSING_MG: i32 = -15;

// --- Mobility ---
// Per-piece (mg, eg) weight for each safe attack square, and the typical
// square count subtracted so the term is roughly zero-mean (a piece of
// average activity neither gains nor loses).

/// Knight mobility: (mg, eg) centipawns per square beyond the baseline.
const KNIGHT_MOBILITY: (i32, i32) = (4, 4);
/// Baseline safe-square count for a knight.
const KNIGHT_MOBILITY_BASE: i32 = 4;
/// Bishop mobility weight.
const BISHOP_MOBILITY: (i32, i32) = (3, 3);
/// Baseline safe-square count for a bishop.
const BISHOP_MOBILITY_BASE: i32 = 6;
/// Rook mobility weight.
const ROOK_MOBILITY: (i32, i32) = (2, 4);
/// Baseline safe-square count for a rook.
const ROOK_MOBILITY_BASE: i32 = 7;
/// Queen mobility weight.
const QUEEN_MOBILITY: (i32, i32) = (1, 2);
/// Baseline safe-square count for a queen.
const QUEEN_MOBILITY_BASE: i32 = 13;

// --- King danger (attack units) ---

/// Attack units contributed by a knight or bishop attacking the king zone.
const KING_ATTACK_MINOR: usize = 2;
/// Attack units contributed by a rook attacking the king zone.
const KING_ATTACK_ROOK: usize = 3;
/// Attack units contributed by a queen attacking the king zone.
const KING_ATTACK_QUEEN: usize = 5;
/// Middlegame penalty by total attack units on a king's zone: roughly
/// quadratic, so a single attacker is noise but a coordinated attack by
/// several pieces snowballs. One attacking piece alone (≤ 5 units) is ignored.
const KING_DANGER_MG: [i32; 20] = [
    0, 0, 0, 0, 0, 0, 12, 18, 25, 34, 44, 56, 69, 84, 100, 118, 137, 158, 180, 204,
];
/// Middlegame penalty for a file next to the king with no friendly pawns.
const KING_FILE_SEMI_OPEN_MG: i32 = -10;
/// Additional middlegame penalty when that file has no pawns of either colour.
const KING_FILE_OPEN_MG: i32 = -8;

// --- Passed-pawn refinements ---

/// Endgame weight of the king-distance race to a passer's stop square: the
/// Chebyshev-distance difference (enemy king minus own king) times the pawn's
/// relative rank, times this.
const PASSED_KING_DIST_EG: i32 = 2;
/// (mg, eg) bonus for a friendly rook on the passer's file behind it.
const ROOK_BEHIND_PASSER: (i32, i32) = (5, 15);

// --- Piece-Square Tables (PSTs) ---
//
// PeSTO's tables. All tables are from White's perspective and written
// rank-8-first (index 0 = a8, index 63 = h1). Board squares are indexed from
// a1 (0), so white squares must be flipped vertically (sq ^ 56) before
// indexing; black squares index the table directly.

/// Pawn middlegame PST (rank-8-first; see the [module docs](self)).
const PAWN_PST_MG: [i32; 64] = [
    0, 0, 0, 0, 0, 0, 0, 0, //
    98, 134, 61, 95, 68, 126, 34, -11, //
    -6, 7, 26, 31, 65, 56, 25, -20, //
    -14, 13, 6, 21, 23, 12, 17, -23, //
    -27, -2, -5, 12, 17, 6, 10, -25, //
    -26, -4, -4, -10, 3, 3, 33, -12, //
    -35, -1, -20, -23, -15, 24, 38, -22, //
    0, 0, 0, 0, 0, 0, 0, 0,
];

/// Pawn endgame PST (rank-8-first).
const PAWN_PST_EG: [i32; 64] = [
    0, 0, 0, 0, 0, 0, 0, 0, //
    178, 173, 158, 134, 147, 132, 165, 187, //
    94, 100, 85, 67, 56, 53, 82, 84, //
    32, 24, 13, 5, -2, 4, 17, 17, //
    13, 9, -3, -7, -7, -8, 3, -1, //
    4, 7, -6, 1, 0, -5, -1, -8, //
    13, 8, 8, 10, 13, 0, 2, -7, //
    0, 0, 0, 0, 0, 0, 0, 0,
];

/// Knight middlegame PST (rank-8-first).
const KNIGHT_PST_MG: [i32; 64] = [
    -167, -89, -34, -49, 61, -97, -15, -107, //
    -73, -41, 72, 36, 23, 62, 7, -17, //
    -47, 60, 37, 65, 84, 129, 73, 44, //
    -9, 17, 19, 53, 37, 69, 18, 22, //
    -13, 4, 16, 13, 28, 19, 21, -8, //
    -23, -9, 12, 10, 19, 17, 25, -16, //
    -29, -53, -12, -3, -1, 18, -14, -19, //
    -105, -21, -58, -33, -17, -28, -19, -23,
];

/// Knight endgame PST (rank-8-first).
const KNIGHT_PST_EG: [i32; 64] = [
    -58, -38, -13, -28, -31, -27, -63, -99, //
    -25, -8, -25, -2, -9, -25, -24, -52, //
    -24, -20, 10, 9, -1, -9, -19, -41, //
    -17, 3, 22, 22, 22, 11, 8, -18, //
    -18, -6, 16, 25, 16, 17, 4, -18, //
    -23, -3, -1, 15, 10, -3, -20, -22, //
    -42, -20, -10, -5, -2, -20, -23, -44, //
    -29, -51, -23, -15, -22, -18, -50, -64,
];

/// Bishop middlegame PST (rank-8-first).
const BISHOP_PST_MG: [i32; 64] = [
    -29, 4, -82, -37, -25, -42, 7, -8, //
    -26, 16, -18, -13, 30, 59, 18, -47, //
    -16, 37, 43, 40, 35, 50, 37, -2, //
    -4, 5, 19, 50, 37, 37, 7, -2, //
    -6, 13, 13, 26, 34, 12, 10, 4, //
    0, 15, 15, 15, 14, 27, 18, 10, //
    4, 15, 16, 0, 7, 21, 33, 1, //
    -33, -3, -14, -21, -13, -12, -39, -21,
];

/// Bishop endgame PST (rank-8-first).
const BISHOP_PST_EG: [i32; 64] = [
    -14, -21, -11, -8, -7, -9, -17, -24, //
    -8, -4, 7, -12, -3, -13, -4, -14, //
    2, -8, 0, -1, -2, 6, 0, 4, //
    -3, 9, 12, 9, 14, 10, 3, 2, //
    -6, 3, 13, 19, 7, 10, -3, -9, //
    -12, -3, 8, 10, 13, 3, -7, -15, //
    -14, -18, -7, -1, 4, -9, -15, -27, //
    -23, -9, -23, -5, -9, -16, -5, -17,
];

/// Rook middlegame PST (rank-8-first).
const ROOK_PST_MG: [i32; 64] = [
    32, 42, 32, 51, 63, 9, 31, 43, //
    27, 32, 58, 62, 80, 67, 26, 44, //
    -5, 19, 26, 36, 17, 45, 61, 16, //
    -24, -11, 7, 26, 24, 35, -8, -20, //
    -36, -26, -12, -1, 9, -7, 6, -23, //
    -45, -25, -16, -17, 3, 0, -5, -33, //
    -44, -16, -20, -9, -1, 11, -6, -71, //
    -19, -13, 1, 17, 16, 7, -37, -26,
];

/// Rook endgame PST (rank-8-first).
const ROOK_PST_EG: [i32; 64] = [
    13, 10, 18, 15, 12, 12, 8, 5, //
    11, 13, 13, 11, -3, 3, 8, 3, //
    7, 7, 7, 5, 4, -3, -5, -3, //
    4, 3, 13, 1, 2, 1, -1, 2, //
    3, 5, 8, 4, -5, -6, -8, -11, //
    -4, 0, -5, -1, -7, -12, -8, -16, //
    -6, -6, 0, 2, -9, -9, -11, -3, //
    -9, 2, 3, -1, -5, -13, 4, -20,
];

/// Queen middlegame PST (rank-8-first).
const QUEEN_PST_MG: [i32; 64] = [
    -28, 0, 29, 12, 59, 44, 43, 45, //
    -24, -39, -5, 1, -16, 57, 28, 54, //
    -13, -17, 7, 8, 29, 56, 47, 57, //
    -27, -27, -16, -16, -1, 17, -2, 1, //
    -9, -26, -9, -10, -2, -4, 3, -3, //
    -14, 2, -11, -2, -5, 2, 14, 5, //
    -35, -8, 11, 2, 8, 15, -3, 1, //
    -1, -18, -9, 10, -15, -25, -31, -50,
];

/// Queen endgame PST (rank-8-first).
const QUEEN_PST_EG: [i32; 64] = [
    -9, 22, 22, 27, 27, 19, 10, 20, //
    -17, 20, 32, 41, 58, 25, 30, 0, //
    -20, 6, 9, 49, 47, 35, 19, 9, //
    3, 22, 24, 45, 57, 40, 57, 36, //
    -18, 28, 19, 47, 31, 34, 39, 23, //
    -16, -27, 15, 6, 9, 17, 10, 5, //
    -22, -23, -30, -16, -16, -23, -36, -32, //
    -33, -28, -22, -43, -5, -32, -20, -41,
];

/// King middlegame PST (rank-8-first).
const KING_PST_MG: [i32; 64] = [
    -65, 23, 16, -15, -56, -34, 2, 13, //
    29, -1, -20, -7, -8, -4, -38, -29, //
    -9, 24, 2, -16, -20, 6, 22, -22, //
    -17, -20, -12, -27, -30, -25, -14, -36, //
    -49, -1, -27, -39, -46, -44, -33, -51, //
    -14, -14, -22, -46, -44, -30, -15, -27, //
    1, 7, -8, -64, -43, -16, 9, 8, //
    -15, 36, 12, -54, 8, -28, 24, 14,
];

/// King endgame PST (rank-8-first).
const KING_PST_EG: [i32; 64] = [
    -74, -35, -18, -18, -11, 15, 4, -17, //
    -12, 17, 14, 17, 17, 38, 23, 11, //
    10, 17, 23, 15, 20, 45, 44, 13, //
    -8, 22, 24, 27, 26, 33, 26, 3, //
    -18, -4, 21, 24, 27, 23, 9, -11, //
    -19, -3, 11, 21, 23, 16, 7, -9, //
    -27, -11, 4, 13, 14, 4, -5, -17, //
    -53, -34, -21, -11, -28, -14, -24, -43,
];

/// Per-piece evaluation data: mg/eg material values, mg/eg PSTs, and the
/// piece's game-phase weight.
struct PieceEval {
    mg_value: i32,
    eg_value: i32,
    mg_pst: &'static [i32; 64],
    eg_pst: &'static [i32; 64],
    phase: i32,
}

/// Looks up the [`PieceEval`] for a piece type. The king's material value is
/// zero (king vs king is implicit).
const fn piece_eval(piece: Piece) -> PieceEval {
    match piece {
        Piece::Pawn => PieceEval {
            mg_value: PAWN_MG,
            eg_value: PAWN_EG,
            mg_pst: &PAWN_PST_MG,
            eg_pst: &PAWN_PST_EG,
            phase: 0,
        },
        Piece::Knight => PieceEval {
            mg_value: KNIGHT_MG,
            eg_value: KNIGHT_EG,
            mg_pst: &KNIGHT_PST_MG,
            eg_pst: &KNIGHT_PST_EG,
            phase: KNIGHT_PHASE,
        },
        Piece::Bishop => PieceEval {
            mg_value: BISHOP_MG,
            eg_value: BISHOP_EG,
            mg_pst: &BISHOP_PST_MG,
            eg_pst: &BISHOP_PST_EG,
            phase: BISHOP_PHASE,
        },
        Piece::Rook => PieceEval {
            mg_value: ROOK_MG,
            eg_value: ROOK_EG,
            mg_pst: &ROOK_PST_MG,
            eg_pst: &ROOK_PST_EG,
            phase: ROOK_PHASE,
        },
        Piece::Queen => PieceEval {
            mg_value: QUEEN_MG,
            eg_value: QUEEN_EG,
            mg_pst: &QUEEN_PST_MG,
            eg_pst: &QUEEN_PST_EG,
            phase: QUEEN_PHASE,
        },
        // King (Piece::None never occurs in a piece board).
        _ => PieceEval {
            mg_value: 0,
            eg_value: 0,
            mg_pst: &KING_PST_MG,
            eg_pst: &KING_PST_EG,
            phase: 0,
        },
    }
}

/// Computes the pawn-structure and piece-feature terms for one side, returning
/// `(mg, eg)` bonuses from that side's perspective: passed, isolated, and
/// doubled pawns, the bishop pair, rooks on open/semi-open files, and the
/// king's pawn shield.
#[allow(
    clippy::cast_possible_wrap,
    clippy::cast_possible_truncation,
    clippy::similar_names
)]
fn side_features(board: &Board, color: Turn) -> (i32, i32) {
    let own_pawns = board.get_piece_bitboard(Piece::Pawn, color);
    let enemy_pawns = board.get_piece_bitboard(Piece::Pawn, !color);
    let all_pawns = own_pawns | enemy_pawns;
    let mut mg = 0;
    let mut eg = 0;

    // Passed and isolated pawns.
    let own_king_sq = board
        .get_piece_bitboard(Piece::King, color)
        .trailing_zeros();
    let enemy_king_sq = board
        .get_piece_bitboard(Piece::King, !color)
        .trailing_zeros();
    let mut pawns = own_pawns;
    while pawns.is_not_empty() {
        let sq = pawns.trailing_zeros();
        let file = sq % 8;
        let relative_rank = if color == WHITE { sq / 8 } else { 7 - sq / 8 };
        if (PASSED_PAWN_MASKS[usize::from(color)][sq] & enemy_pawns).is_empty() {
            let mut pass_mg = PASSED_PAWN_MG[relative_rank];
            let mut pass_eg = PASSED_PAWN_EG[relative_rank];

            // a blockaded passer (stop square occupied) is worth much less
            let stop_sq = if color == WHITE { sq + 8 } else { sq - 8 };
            if !board.empty_tiles.is_square_set(stop_sq) {
                pass_mg /= 2;
                pass_eg /= 2;
            }

            // the endgame king race: shepherding king close to the stop
            // square, defending king far — scaled by how advanced the pawn is
            pass_eg += PASSED_KING_DIST_EG
                * (chebyshev(enemy_king_sq, stop_sq) - chebyshev(own_king_sq, stop_sq))
                * relative_rank as i32;

            // a rook behind the passer pushes it and defends it as it runs
            let behind = FILE_MASKS[file] & board.get_piece_bitboard(Piece::Rook, color);
            let rook_is_behind = if color == WHITE {
                (behind & Bitboard::from_u64((1_u64 << sq) - 1)).is_not_empty()
            } else {
                (behind & !Bitboard::from_u64((1_u64 << (sq + 1)) - 1)).is_not_empty()
            };
            if rook_is_behind {
                pass_mg += ROOK_BEHIND_PASSER.0;
                pass_eg += ROOK_BEHIND_PASSER.1;
            }

            mg += pass_mg;
            eg += pass_eg;
        }
        if (ADJACENT_FILE_MASKS[file] & own_pawns).is_empty() {
            mg += ISOLATED_PAWN_MG;
            eg += ISOLATED_PAWN_EG;
        }
        pawns.reset_lsb();
    }

    // Doubled pawns: penalise each pawn beyond the first on a file.
    for file_mask in FILE_MASKS {
        let pawns_on_file = (own_pawns & file_mask).count_bits() as i32;
        if pawns_on_file > 1 {
            mg += DOUBLED_PAWN_MG * (pawns_on_file - 1);
            eg += DOUBLED_PAWN_EG * (pawns_on_file - 1);
        }
    }

    // Bishop pair.
    if board.get_piece_bitboard(Piece::Bishop, color).count_bits() >= 2 {
        mg += BISHOP_PAIR_MG;
        eg += BISHOP_PAIR_EG;
    }

    // Rooks on open and semi-open files.
    let mut rooks = board.get_piece_bitboard(Piece::Rook, color);
    while rooks.is_not_empty() {
        let file = rooks.trailing_zeros() % 8;
        if (FILE_MASKS[file] & all_pawns).is_empty() {
            mg += ROOK_OPEN_FILE_MG;
        } else if (FILE_MASKS[file] & own_pawns).is_empty() {
            mg += ROOK_SEMI_OPEN_FILE_MG;
        }
        rooks.reset_lsb();
    }

    // Pawn shield: for a king on its back two ranks, count the missing pawns on
    // the up-to-three squares of the rank directly in front of it (the
    // passed-pawn mask ANDed with that rank handles board edges naturally).
    let king_sq = board
        .get_piece_bitboard(Piece::King, color)
        .trailing_zeros();
    let king_rank = king_sq / 8;
    let front_rank = if color == WHITE {
        (king_rank <= 1).then(|| king_rank + 1)
    } else {
        (king_rank >= 6).then(|| king_rank - 1)
    };
    if let Some(front_rank) = front_rank {
        let shield = PASSED_PAWN_MASKS[usize::from(color)][king_sq] & RANK_MASKS[front_rank];
        let missing = shield.count_bits() - (shield & own_pawns).count_bits();
        mg += KING_SHIELD_MISSING_MG * missing as i32;
    }

    (mg, eg)
}

/// Chebyshev (king-move) distance between two square indices.
#[allow(clippy::cast_possible_wrap, clippy::cast_possible_truncation)]
const fn chebyshev(a: usize, b: usize) -> i32 {
    let file_dist = ((a % 8) as i32 - (b % 8) as i32).abs();
    let rank_dist = ((a / 8) as i32 - (b / 8) as i32).abs();
    if file_dist > rank_dist {
        file_dist
    } else {
        rank_dist
    }
}

/// Squares attacked by `color`'s pawns.
fn pawn_attack_map(board: &Board, color: Turn) -> Bitboard {
    let mut attacks = Bitboard::new();
    let mut pawns = board.get_piece_bitboard(Piece::Pawn, color);
    while pawns.is_not_empty() {
        attacks |= PAWN_ATTACKS[usize::from(color)][pawns.trailing_zeros()];
        pawns.reset_lsb();
    }
    attacks
}

/// Mobility for one side, `(mg, eg)`: each knight, bishop, rook, and queen
/// scores its count of *safe* attack squares (not occupied by a friendly
/// piece, not covered by an enemy pawn) against a per-piece baseline, so an
/// averagely active piece contributes nothing and trapped pieces are punished.
#[allow(clippy::cast_possible_wrap)]
fn side_mobility(board: &Board, color: Turn) -> (i32, i32) {
    let occupancy = !board.empty_tiles;
    // safe: not a friendly piece's square, not attacked by an enemy pawn
    let safe = !(board.player_boards[usize::from(color)] | pawn_attack_map(board, !color));
    let mut mg = 0;
    let mut eg = 0;

    let mut score_piece = |count: i32, base: i32, weight: (i32, i32)| {
        mg += weight.0 * (count - base);
        eg += weight.1 * (count - base);
    };

    let mut knights = board.get_piece_bitboard(Piece::Knight, color);
    while knights.is_not_empty() {
        let sq = knights.trailing_zeros();
        let moves = (KNIGHT_MOVES[sq] & safe).count_bits() as i32;
        score_piece(moves, KNIGHT_MOBILITY_BASE, KNIGHT_MOBILITY);
        knights.reset_lsb();
    }
    let mut bishops = board.get_piece_bitboard(Piece::Bishop, color);
    while bishops.is_not_empty() {
        let sq = bishops.trailing_zeros();
        let moves = (bishop_attacks(sq, occupancy) & safe).count_bits() as i32;
        score_piece(moves, BISHOP_MOBILITY_BASE, BISHOP_MOBILITY);
        bishops.reset_lsb();
    }
    let mut rooks = board.get_piece_bitboard(Piece::Rook, color);
    while rooks.is_not_empty() {
        let sq = rooks.trailing_zeros();
        let moves = (rook_attacks(sq, occupancy) & safe).count_bits() as i32;
        score_piece(moves, ROOK_MOBILITY_BASE, ROOK_MOBILITY);
        rooks.reset_lsb();
    }
    let mut queens = board.get_piece_bitboard(Piece::Queen, color);
    while queens.is_not_empty() {
        let sq = queens.trailing_zeros();
        let moves = ((bishop_attacks(sq, occupancy) | rook_attacks(sq, occupancy)) & safe)
            .count_bits() as i32;
        score_piece(moves, QUEEN_MOBILITY_BASE, QUEEN_MOBILITY);
        queens.reset_lsb();
    }

    (mg, eg)
}

/// Middlegame king-danger penalty for `color`'s king (returned ≤ 0): sums
/// attack units over the enemy knights, bishops, rooks, and queens whose
/// attacks touch the king zone (the king and its ring) and maps the total
/// through [`KING_DANGER_MG`], plus a penalty for semi-open/open files next to
/// the king.
fn king_danger(board: &Board, color: Turn) -> i32 {
    let king_sq = board
        .get_piece_bitboard(Piece::King, color)
        .trailing_zeros();
    let mut zone = KING_RING_MOVES[king_sq];
    zone.set_square(king_sq);
    let occupancy = !board.empty_tiles;

    let mut units = 0_usize;
    let mut count_attackers =
        |mut pieces: Bitboard, weight: usize, diagonal: bool, straight: bool| {
            while pieces.is_not_empty() {
                let sq = pieces.trailing_zeros();
                let mut attacks = Bitboard::new();
                if diagonal {
                    attacks |= bishop_attacks(sq, occupancy);
                }
                if straight {
                    attacks |= rook_attacks(sq, occupancy);
                }
                if !diagonal && !straight {
                    attacks = KNIGHT_MOVES[sq];
                }
                if (attacks & zone).is_not_empty() {
                    units += weight;
                }
                pieces.reset_lsb();
            }
        };
    count_attackers(
        board.get_piece_bitboard(Piece::Knight, !color),
        KING_ATTACK_MINOR,
        false,
        false,
    );
    count_attackers(
        board.get_piece_bitboard(Piece::Bishop, !color),
        KING_ATTACK_MINOR,
        true,
        false,
    );
    count_attackers(
        board.get_piece_bitboard(Piece::Rook, !color),
        KING_ATTACK_ROOK,
        false,
        true,
    );
    count_attackers(
        board.get_piece_bitboard(Piece::Queen, !color),
        KING_ATTACK_QUEEN,
        true,
        true,
    );
    let mut danger = -KING_DANGER_MG[units.min(KING_DANGER_MG.len() - 1)];

    // files on and next to the king with no pawn cover invite heavy pieces
    let own_pawns = board.get_piece_bitboard(Piece::Pawn, color);
    let all_pawns = own_pawns | board.get_piece_bitboard(Piece::Pawn, !color);
    let king_file = king_sq % 8;
    for file_mask in &FILE_MASKS[king_file.saturating_sub(1)..=(king_file + 1).min(7)] {
        if (*file_mask & own_pawns).is_empty() {
            danger += KING_FILE_SEMI_OPEN_MG;
            if (*file_mask & all_pawns).is_empty() {
                danger += KING_FILE_OPEN_MG;
            }
        }
    }

    danger
}

/// Maps a board square (`a1` = 0) to its index into a rank-8-first PST array.
///
/// White squares are flipped vertically (`sq ^ 56`) so they read the table from
/// White's perspective; black squares index it directly.
const fn pst_index(square: usize, is_white: bool) -> usize {
    if is_white { square ^ 0x38 } else { square }
}

/// Statically evaluates `board`, returning a centipawn score from the
/// perspective of the side to move (positive = better for the mover).
///
/// Accumulates separate middlegame and endgame scores (material + PST for
/// every piece, from White's perspective) plus a tempo bonus for the side to
/// move, then blends the two by game phase.
#[allow(clippy::similar_names)]
pub(crate) fn evaluate(board: &Board) -> i32 {
    let mut mg = 0;
    let mut eg = 0;
    let mut game_phase = 0;

    for (index, mut piece_board) in board.piece_boards.into_iter().enumerate() {
        let (piece, color) = Board::get_piece_information_index(index);
        let eval = piece_eval(piece);
        // extract all pieces from bitboard
        while piece_board.is_not_empty() {
            let pos = piece_board.trailing_zeros();
            let pst_idx = pst_index(pos, color == WHITE);
            let mg_score = eval.mg_value + eval.mg_pst[pst_idx];
            let eg_score = eval.eg_value + eval.eg_pst[pst_idx];
            if color == WHITE {
                mg += mg_score;
                eg += eg_score;
            } else {
                mg -= mg_score;
                eg -= eg_score;
            }
            game_phase += eval.phase;
            piece_board.reset_lsb();
        }
    }

    // Pawn structure and piece features, white minus black.
    let (white_mg, white_eg) = side_features(board, WHITE);
    let (black_mg, black_eg) = side_features(board, BLACK);
    mg += white_mg - black_mg;
    eg += white_eg - black_eg;

    // Mobility, white minus black.
    let (white_mob_mg, white_mob_eg) = side_mobility(board, WHITE);
    let (black_mob_mg, black_mob_eg) = side_mobility(board, BLACK);
    mg += white_mob_mg - black_mob_mg;
    eg += white_mob_eg - black_mob_eg;

    // King danger (each side's own penalty, ≤ 0), middlegame only: with the
    // heavy pieces off the board an "attacked" king zone means little.
    mg += king_danger(board, WHITE) - king_danger(board, BLACK);

    // Tempo: the side to move has the initiative in the middlegame.
    if board.turn == WHITE {
        mg += TEMPO_MG;
    } else {
        mg -= TEMPO_MG;
    }

    // Blend by game phase (clamped: early promotions can exceed TOTAL_PHASE).
    let phase = std::cmp::min(game_phase, TOTAL_PHASE);
    let total_score = (mg * phase + eg * (TOTAL_PHASE - phase)) / TOTAL_PHASE;

    // Return score from the perspective of the current player
    if board.turn == WHITE {
        total_score
    } else {
        -total_score
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BISHOP_PAIR_EG, BISHOP_PAIR_MG, DOUBLED_PAWN_EG, DOUBLED_PAWN_MG, ISOLATED_PAWN_EG,
        ISOLATED_PAWN_MG, KING_SHIELD_MISSING_MG, KNIGHT_MOBILITY, KNIGHT_MOBILITY_BASE,
        PASSED_PAWN_EG, PASSED_PAWN_MG, ROOK_BEHIND_PASSER, ROOK_OPEN_FILE_MG,
        ROOK_SEMI_OPEN_FILE_MG, TEMPO_MG, evaluate, king_danger, side_features, side_mobility,
    };
    use crate::chess_engine::board::{Board, WHITE};
    use crate::chess_engine::utils::init_tables;

    #[test]
    fn start_position_gives_only_tempo() {
        // the opening position is mirror-symmetric, so the only edge is the
        // side to move's tempo bonus — identical for either mover
        let white =
            Board::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();
        let black =
            Board::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR b KQkq - 0 1").unwrap();
        assert_eq!(evaluate(&white), TEMPO_MG);
        assert_eq!(evaluate(&white), evaluate(&black));
    }

    #[test]
    fn colour_flipped_positions_mirror() {
        // a position and its vertical mirror (colours swapped) must evaluate
        // identically from the mover's perspective
        let original =
            Board::from_fen("r1bqkbnr/pppp1ppp/2n5/4p3/4P3/5N2/PPPP1PPP/RNBQKB1R w KQkq - 0 1")
                .unwrap();
        let mirrored =
            Board::from_fen("rnbqkb1r/pppp1ppp/5n2/4p3/4P3/2N5/PPPP1PPP/R1BQKBNR b KQkq - 0 1")
                .unwrap();
        assert_eq!(evaluate(&original), evaluate(&mirrored));
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

    /// `side_features(WHITE)` difference between two positions, `(mg, eg)`.
    /// Kings are kept off the back ranks in these fixtures so the pawn-shield
    /// term stays silent and single features can be isolated.
    #[allow(clippy::similar_names)]
    fn white_features_diff(fen_a: &str, fen_b: &str) -> (i32, i32) {
        let a = Board::from_fen(fen_a).unwrap();
        let b = Board::from_fen(fen_b).unwrap();
        let (a_mg, a_eg) = side_features(&a, WHITE);
        let (b_mg, b_eg) = side_features(&b, WHITE);
        (a_mg - b_mg, a_eg - b_eg)
    }

    #[test]
    fn isolated_pawns_are_penalised() {
        // a2+c2 (both isolated) vs a2+b2 (connected); both pairs are blocked
        // by the black pawns so no passed-pawn bonus interferes
        let diff = white_features_diff(
            "8/p1p5/8/7k/K7/8/P1P5/8 w - - 0 1",
            "8/pp6/8/7k/K7/8/PP6/8 w - - 0 1",
        );
        assert_eq!(diff, (2 * ISOLATED_PAWN_MG, 2 * ISOLATED_PAWN_EG));
    }

    #[test]
    fn doubled_pawns_are_penalised() {
        // e2+e3 (doubled) vs c3+e2 (split); isolation and blocking identical
        let diff = white_features_diff(
            "8/2p1p3/8/7k/K7/4P3/4P3/8 w - - 0 1",
            "8/2p1p3/8/7k/K7/2P5/4P3/8 w - - 0 1",
        );
        assert_eq!(diff, (DOUBLED_PAWN_MG, DOUBLED_PAWN_EG));
    }

    #[test]
    fn passed_pawn_beats_blocked_pawn() {
        // White pawn e5 is passed when Black's pawn sits on a7, and not passed
        // (and Black's pawn more relevant) when it sits on d7. Same material;
        // the passed-pawn bonus should decide the comparison.
        let passed = Board::from_fen("4k3/p7/8/4P3/8/8/8/4K3 w - - 0 1").unwrap();
        let blocked = Board::from_fen("4k3/3p4/8/4P3/8/8/8/4K3 w - - 0 1").unwrap();
        assert!(evaluate(&passed) > evaluate(&blocked));
    }

    #[test]
    fn bishop_pair_is_rewarded() {
        let diff = white_features_diff(
            "8/8/8/7k/K7/8/8/1B3B2 w - - 0 1",
            "8/8/8/7k/K7/8/8/1N3B2 w - - 0 1",
        );
        assert_eq!(diff, (BISHOP_PAIR_MG, BISHOP_PAIR_EG));
    }

    #[test]
    fn rook_prefers_open_over_semi_open_file() {
        // rook a1 with no pawns on the a-file (open) vs a black pawn on a7
        // (semi-open)
        let diff = white_features_diff(
            "8/1p6/8/7k/K7/8/8/R7 w - - 0 1",
            "8/p7/8/7k/K7/8/8/R7 w - - 0 1",
        );
        assert_eq!(diff, (ROOK_OPEN_FILE_MG - ROOK_SEMI_OPEN_FILE_MG, 0));
    }

    #[test]
    fn bare_king_pays_full_shield_penalty() {
        // king on e1 with no pawns at all: all three shield squares are empty
        let board = Board::from_fen("4k3/8/8/8/8/8/8/4K3 w - - 0 1").unwrap();
        assert_eq!(
            side_features(&board, WHITE),
            (3 * KING_SHIELD_MISSING_MG, 0)
        );
        // a king that has left the back ranks takes no shield penalty
        let out = Board::from_fen("4k3/8/8/8/4K3/8/8/8 w - - 0 1").unwrap();
        assert_eq!(side_features(&out, WHITE), (0, 0));
    }

    #[test]
    fn central_knight_outmoves_cornered_knight() {
        init_tables();
        // a knight on d5 reaches all 8 squares (no friendly pieces or enemy
        // pawn cover interfere)
        let center = Board::from_fen("8/8/8/3N4/8/8/8/K6k w - - 0 1").unwrap();
        let expected = KNIGHT_MOBILITY.0 * (8 - KNIGHT_MOBILITY_BASE);
        assert_eq!(side_mobility(&center, WHITE).0, expected);
        // a cornered knight has two squares and scores below the baseline
        let corner = Board::from_fen("N7/8/8/8/8/8/8/K6k w - - 0 1").unwrap();
        assert!(side_mobility(&corner, WHITE).0 < 0);
    }

    #[test]
    fn attacked_king_zone_raises_danger() {
        init_tables();
        // queen g4 + rook a1 both bear on the white king's zone…
        let attacked = Board::from_fen("k7/8/8/8/6q1/8/8/r5K1 w - - 0 1").unwrap();
        // …while a lone distant queen touches nothing near g1
        let safe = Board::from_fen("k7/8/8/q7/8/8/8/6K1 w - - 0 1").unwrap();
        assert!(king_danger(&attacked, WHITE) < king_danger(&safe, WHITE));
    }

    #[test]
    fn rook_behind_passer_is_rewarded() {
        init_tables();
        // same passer and kings; only the rook moves from behind (c1) to in
        // front (c8) of the pawn's path
        let diff = white_features_diff(
            "4k3/8/8/2P5/8/8/8/2R1K3 w - - 0 1",
            "2R1k3/8/8/2P5/8/8/8/4K3 w - - 0 1",
        );
        assert_eq!(diff, ROOK_BEHIND_PASSER);
    }

    #[test]
    fn blockaded_passer_is_worth_half() {
        init_tables();
        // black knight sits on the c5-passer's stop square vs. loiters on f6
        let diff = white_features_diff(
            "4k3/8/5n2/2P5/8/8/8/4K3 w - - 0 1",
            "4k3/8/2n5/2P5/8/8/8/4K3 w - - 0 1",
        );
        let rank = 4; // c5 from White's point of view
        assert_eq!(
            diff,
            (
                PASSED_PAWN_MG[rank] - PASSED_PAWN_MG[rank] / 2,
                PASSED_PAWN_EG[rank] - PASSED_PAWN_EG[rank] / 2
            )
        );
    }

    #[test]
    fn pawn_advancement_matters_more_in_the_endgame() {
        // A pawn on the 7th rank with few pieces left should dwarf the same
        // pawn's value with full material on the board.
        let endgame = Board::from_fen("4k3/2P5/8/8/8/8/8/4K3 w - - 0 1").unwrap();
        let base_endgame = Board::from_fen("4k3/8/8/8/8/8/2P5/4K3 w - - 0 1").unwrap();
        assert!(evaluate(&endgame) - evaluate(&base_endgame) > 100);
    }
}
