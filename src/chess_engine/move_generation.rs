//! Move generation: turning a [`Board`] into the list of moves available to a
//! side.
//!
//! Generation is two-phase. First, *pseudo-legal* moves are produced cheaply —
//! pawns by bit-shifting masks, knights and the king by table lookup, and
//! sliding pieces via the [magic bitboard](super::magic_tables) attack tables —
//! without regard for whether they leave the mover's king in check. Then
//! [`Board::generate_moves`] filters those down to *legal* moves with
//! [`Board::would_check`].
//!
//! Castle generation lives in its own path so that check detection (which asks
//! whether the king passes through an attacked square) does not recurse back
//! into castle generation; that is why
//! [`generate_pseudo_non_castle_moves`] exists separately from
//! [`generate_pseudo_legal_moves`].

use crate::chess_engine::computed_boards::{
    BISHOP_ATTACKS, BISHOP_BLOCKERS, BISHOP_MAGICS, ROOK_ATTACKS, ROOK_BLOCKERS, ROOK_MAGICS,
};
use crate::chess_engine::constants::{
    B_KING_SIDE_BISHOP_START, B_KING_START, B_QUEEN_START, NORTH, NORTH_EAST, NORTH_WEST, SOUTH,
    SOUTH_EAST, SOUTH_WEST, W_KING_SIDE_BISHOP_START, W_KING_START, W_QUEEN_START,
};

use super::bitboard::Bitboard;
use super::board::{Board, Turn, WHITE};
use super::computed_boards::{BETWEEN, BISHOP_RAYS, LINE, PAWN_ATTACKS, ROOK_RAYS};
use super::computed_boards::{KING_RING_MOVES, KNIGHT_MOVES};
use super::constants::EMPTY_BIT_B;
use super::masks::{
    B_KING_CASTLE_EMPTY, B_QUEEN_CASTLE_EMPTY, NOT_A_FILE, NOT_H_FILE, RANK_1, RANK_2, RANK_7,
    RANK_8, W_KING_CASTLE_EMPTY, W_QUEEN_CASTLE_EMPTY,
};
use super::moves::{EN_PASSANT, Move, SpecialMove};
use super::piece::Piece;
use super::position::Position;

impl Board {
    /// Returns every fully legal move for `turn` in this position.
    ///
    /// Pseudo-legal moves are generated and then filtered to remove any that
    /// would leave `turn`'s own king in check.
    ///
    /// ```
    /// use chess_engine::chess_engine::board::{Board, WHITE};
    /// use chess_engine::chess_engine::utils::init_tables;
    ///
    /// init_tables();
    /// let mut board = Board::new_start_pos().unwrap();
    /// // twenty legal first moves: sixteen pawn pushes and four knight moves
    /// assert_eq!(board.generate_moves(WHITE).len(), 20);
    /// ```
    pub fn generate_moves(&mut self, turn: Turn) -> Vec<Move> {
        // Generate pseudo-legal moves, then keep the legal ones. Instead of a
        // make/unmake check per move, a check mask and a pinned-piece set are
        // computed once for the position (see [`check_and_pin_masks`]); most
        // moves are then verified with O(1) bitboard tests. Only the irregular
        // king, castle, and en-passant moves need a per-move attack test.
        let mut moves = generate_pseudo_legal_moves(self, turn);
        let king_sq = self.get_piece_bitboard(Piece::King, turn).trailing_zeros();
        let masks = self.check_and_pin_masks(turn, king_sq);
        moves.retain(|m| self.is_move_legal(*m, turn, king_sq, &masks));
        moves
    }

    /// Decides whether a single pseudo-legal move is legal, given the
    /// position's [`CheckPinMasks`] and the mover's `king_sq`.
    fn is_move_legal(
        &mut self,
        move_: Move,
        turn: Turn,
        king_sq: usize,
        masks: &CheckPinMasks,
    ) -> bool {
        // Castling and en passant have irregular geometry (a passed-over square,
        // a captured pawn off the destination, the en-passant discovered-check
        // edge case); fall back to the exact make/unmake test for those. Both
        // are rare, so the cost is negligible.
        let special = move_.get_special_move();
        if special == SpecialMove::Castle || special == SpecialMove::EnPassant {
            return !self.would_check(move_);
        }

        let (origin, dest) = move_.get_org_and_dest();
        let (from, to) = (origin.as_usize(), dest.as_usize());

        // King moves: legal iff the destination is not attacked once the king
        // has vacated its square (so it cannot slide along a checking ray).
        if from == king_sq {
            let occ_without_king = !self.empty_tiles ^ origin.bitboard();
            return !self.is_square_attacked_occ(to, !turn, occ_without_king);
        }

        // In double check only the king may move.
        if masks.num_checkers >= 2 {
            return false;
        }
        // The move must resolve any check (check_mask is the full board when not
        // in check), capturing the checker or blocking the checking ray.
        if !masks.check_mask.is_square_set(to) {
            return false;
        }
        // A pinned piece may only move along the line through the king and
        // itself (toward the king or toward/onto the pinning piece).
        if masks.pinned.is_square_set(from) && !LINE[king_sq][from].is_square_set(to) {
            return false;
        }
        true
    }

    /// Computes, for `turn`'s king on `king_sq`, the squares a non-king piece may
    /// move to in order to deal with check (`check_mask`), how many pieces give
    /// check (`num_checkers`), and which of `turn`'s pieces are pinned to the
    /// king (`pinned`). When the king is not in check, `check_mask` is the full
    /// board so it constrains nothing.
    fn check_and_pin_masks(&self, turn: Turn, king_sq: usize) -> CheckPinMasks {
        let enemy = !turn;
        let occupied = !self.empty_tiles;
        let own_pieces = self.player_boards[usize::from(turn)];

        let their_rooks_queens = self.get_piece_bitboard(Piece::Rook, enemy)
            | self.get_piece_bitboard(Piece::Queen, enemy);
        let their_bishops_queens = self.get_piece_bitboard(Piece::Bishop, enemy)
            | self.get_piece_bitboard(Piece::Queen, enemy);

        // Contact and jumping checkers (knights and pawns) cannot be blocked,
        // only captured, so the mask is just their square.
        let mut checkers = KNIGHT_MOVES[king_sq] & self.get_piece_bitboard(Piece::Knight, enemy);
        checkers |=
            PAWN_ATTACKS[usize::from(turn)][king_sq] & self.get_piece_bitboard(Piece::Pawn, enemy);
        let mut check_mask = checkers;
        let mut pinned = Bitboard::new();

        // Enemy sliders that lie on a ray from the king either give a (blockable)
        // check or, with exactly one friendly piece in the way, create a pin.
        let mut snipers = (ROOK_RAYS[king_sq] & their_rooks_queens)
            | (BISHOP_RAYS[king_sq] & their_bishops_queens);
        while snipers.is_not_empty() {
            let sniper = snipers.trailing_zeros();
            let between = BETWEEN[king_sq][sniper];
            let blockers = between & occupied;
            if blockers.is_empty() {
                // direct check: the ray and the sniper itself are the targets
                checkers.set_square(sniper);
                check_mask |= between;
                check_mask.set_square(sniper);
            } else if blockers.count_bits() == 1 && (blockers & own_pieces).is_not_empty() {
                // exactly one of our pieces shields the king: it is pinned
                pinned |= blockers;
            }
            snipers.reset_lsb();
        }

        let num_checkers = checkers.count_bits();
        if num_checkers == 0 {
            check_mask = Bitboard::full();
        }
        CheckPinMasks {
            check_mask,
            pinned,
            num_checkers,
        }
    }
}

/// Per-position masks that turn legality into O(1) bitboard tests during move
/// filtering. Produced by [`Board::check_and_pin_masks`].
struct CheckPinMasks {
    /// Squares a non-king piece may move to in order to end the check; the full
    /// board when the king is not in check.
    check_mask: Bitboard,
    /// `turn`'s pieces that are pinned against their king.
    pinned: Bitboard,
    /// Number of enemy pieces giving check (0, 1, or 2).
    num_checkers: u32,
}

/// Generates all pseudo-legal moves for `turn`, including castling. "Pseudo-legal"
/// means the mover's king may be left in check — callers must filter those out.
pub fn generate_pseudo_legal_moves(board: &mut Board, turn: Turn) -> Vec<Move> {
    let mut moves = generate_pseudo_non_castle_moves(board, turn);
    castle_moves(board, turn, &mut moves);
    moves
}

/// Generates pseudo-legal moves *excluding* castling. Used by attack/check
/// detection, where including castling would recurse infinitely.
pub fn generate_pseudo_non_castle_moves(board: &Board, turn: Turn) -> Vec<Move> {
    // skip castle moves, used when checking for checks to avoid inf loop
    // capacity tuned to comfortably hold a typical position's move count so the
    // sub-generators never reallocate mid-fill
    let mut moves = Vec::with_capacity(48);
    sliding_pieces_moves(board, turn, &mut moves);
    knight_moves(board, turn, &mut moves);
    king_ring_moves(board, turn, &mut moves);
    pawn_moves(board, turn, &mut moves);
    moves
}

/// Walks the four `deltas` outward from `pos`, stopping each ray at the first
/// `blockers` square (inclusive), and returns the union of reachable squares.
///
/// This is the slow, blocker-aware reference used when *building* the magic
/// attack tables; runtime move generation reads those tables directly.
#[allow(clippy::trivially_copy_pass_by_ref)]
pub fn get_sliding_moves(deltas: &[(i8, i8); 4], pos: Position, blockers: Bitboard) -> Bitboard {
    let mut moves = EMPTY_BIT_B;
    for &(df, dr) in deltas {
        let mut ray = pos;
        while !blockers.is_square_set(ray.as_usize()) {
            if let Some(shifted) = ray.try_rank_file_offset(df, dr) {
                ray = shifted;
                moves |= ray.bitboard();
            } else {
                break;
            }
        }
    }
    moves
}

/// Pushes one normal [`Move`] from `origin` to each set square in `bit_board`.
fn extract_moves(mut bit_board: Bitboard, origin: usize, moves: &mut Vec<Move>) {
    while bit_board.is_not_empty() {
        let dest = bit_board.trailing_zeros();

        moves.push(Move::new_default(
            Position::new(origin),
            Position::new(dest),
        ));

        bit_board.reset_lsb();
    }
}

/// Pushes one pawn [`Move`] per set destination square in `bit_board`,
/// recovering each origin by subtracting `shift` from the destination index.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn extract_pawn_moves(mut bit_board: Bitboard, shift: i8, moves: &mut Vec<Move>) {
    // Extract moves from bitboard representation
    // and push them to the moves vector
    while bit_board.is_not_empty() {
        let dest = bit_board.trailing_zeros();
        let origin = (dest as i8 - shift) as usize;

        moves.push(Move::new_default(
            Position::new(origin),
            Position::new(dest),
        ));

        bit_board.reset_lsb();
    }
}

/// Like [`extract_pawn_moves`], but emits all four promotion moves per
/// destination square (used for pawns reaching the last rank).
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn extract_promotions(mut bit_board: Bitboard, shift: i8, moves: &mut Vec<Move>) {
    while bit_board.is_not_empty() {
        let dest = bit_board.trailing_zeros();
        let origin = (dest as i8 - shift) as usize;
        moves.extend(Move::new_promote(
            Position::new(origin),
            Position::new(dest),
        ));
        bit_board.reset_lsb();
    }
}

/// Generates all pawn moves for `turn`: single and double pushes, diagonal
/// captures, en-passant captures, and promotions, computed with file-masked
/// bit shifts.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn pawn_moves(board: &Board, turn: Turn, moves: &mut Vec<Move>) {
    let direction = if turn == WHITE { NORTH } else { SOUTH };
    let start_rank = if turn == WHITE { RANK_2 } else { RANK_7 };
    let promotion_rank = if turn == WHITE { RANK_8 } else { RANK_1 };
    let enemy_pieces = board.player_boards[usize::from(!turn)];

    let pawns = board.get_piece_bitboard(Piece::Pawn, turn);
    let forward_moves = (pawns << direction) & board.empty_tiles;

    // double move
    let start_pawns = pawns & start_rank;
    let double_moves =
        start_pawns << (2 * direction) & board.empty_tiles & (board.empty_tiles << direction);

    //capture moves
    let r_dir = if turn == WHITE {
        NORTH_EAST
    } else {
        SOUTH_EAST
    };
    let l_dir = if turn == WHITE {
        NORTH_WEST
    } else {
        SOUTH_WEST
    };
    let right_captures = ((pawns & NOT_H_FILE) << r_dir) & enemy_pieces;
    let left_captures = ((pawns & NOT_A_FILE) << l_dir) & enemy_pieces;

    // en passant
    let right_en_p = ((pawns & NOT_H_FILE) << r_dir) & board.en_passant;
    let left_en_p = ((pawns & NOT_A_FILE) << l_dir) & board.en_passant;
    // TODO maybe rewrite to separate function
    if left_en_p.is_not_empty() {
        let dest = left_en_p.trailing_zeros();
        let origin = (dest as i8 - l_dir) as usize;

        moves.push(Move::new_special(
            Position::new(origin),
            Position::new(dest),
            EN_PASSANT,
        ));
    }
    if right_en_p.is_not_empty() {
        let dest = right_en_p.trailing_zeros();
        let origin = (dest as i8 - r_dir) as usize;

        moves.push(Move::new_special(
            Position::new(origin),
            Position::new(dest),
            EN_PASSANT,
        ));
    }

    // extract moves
    extract_pawn_moves(double_moves, 2 * direction, moves);
    extract_pawn_moves(forward_moves & !promotion_rank, direction, moves);
    extract_pawn_moves(right_captures & !promotion_rank, r_dir, moves);
    extract_pawn_moves(left_captures & !promotion_rank, l_dir, moves);

    // promotion handle separately
    extract_promotions(forward_moves & promotion_rank, direction, moves);
    extract_promotions(right_captures & promotion_rank, r_dir, moves);
    extract_promotions(left_captures & promotion_rank, l_dir, moves);
}

/// Generates rook, bishop, and queen moves for `turn` (the queen contributes to
/// both the rook-like and bishop-like rays) using the magic attack tables.
fn sliding_pieces_moves(board: &Board, turn: Turn, moves: &mut Vec<Move>) {
    let queen_board = board.get_piece_bitboard(Piece::Queen, turn);
    // add queen board
    let mut rook_board = board.get_piece_bitboard(Piece::Rook, turn) | queen_board;
    // rook moves
    while rook_board.is_not_empty() {
        let origin = rook_board.trailing_zeros();
        let relevant_blockers = !board.empty_tiles & ROOK_BLOCKERS[origin];
        let magic_entry = ROOK_MAGICS[origin];
        let moves_bb =
            ROOK_ATTACKS[magic_entry.magic_index(relevant_blockers) + magic_entry.offset];
        let legal_bb = moves_bb & !board.player_boards[usize::from(turn)];
        extract_moves(legal_bb, origin, moves);
        rook_board.reset_lsb();
    }
    // bishop moves
    let mut bishop_board = board.get_piece_bitboard(Piece::Bishop, turn) | queen_board;
    while bishop_board.is_not_empty() {
        let origin = bishop_board.trailing_zeros();
        let relevant_blockers = !board.empty_tiles & BISHOP_BLOCKERS[origin];
        let magic_entry = BISHOP_MAGICS[origin];
        let moves_bb =
            BISHOP_ATTACKS[magic_entry.magic_index(relevant_blockers) + magic_entry.offset];
        let legal_bb = moves_bb & !board.player_boards[usize::from(turn)];
        extract_moves(legal_bb, origin, moves);
        bishop_board.reset_lsb();
    }
}

/// Generates knight moves for `turn` from the precomputed [`KNIGHT_MOVES`] table.
fn knight_moves(board: &Board, turn: Turn, moves: &mut Vec<Move>) {
    let mut knight_board = board.get_piece_bitboard(Piece::Knight, turn);
    let not_my_pieces = !board.player_boards[usize::from(turn)];
    while knight_board.is_not_empty() {
        let origin = knight_board.trailing_zeros();
        let moves_bb = KNIGHT_MOVES[origin] & not_my_pieces;
        extract_moves(moves_bb, origin, moves);
        knight_board.reset_lsb();
    }
}

/// Generates the king's one-square moves for `turn` from the precomputed
/// [`KING_RING_MOVES`] table (castling is handled by [`castle_moves`]).
fn king_ring_moves(board: &Board, turn: Turn, moves: &mut Vec<Move>) {
    let king_board = board.get_piece_bitboard(Piece::King, turn);
    let not_my_pieces = !board.player_boards[usize::from(turn)];
    let origin = king_board.trailing_zeros();

    let ring_moves = KING_RING_MOVES[origin] & not_my_pieces;
    extract_moves(ring_moves, origin, moves);
}

/// Adds castling moves for `turn` when legal: the right is still held, the
/// squares between king and rook are empty, the king is not in check, and it
/// does not pass through an attacked square.
fn castle_moves(board: &mut Board, turn: Turn, moves: &mut Vec<Move>) {
    let castle_data = [
        (
            false,
            [W_QUEEN_START, B_QUEEN_START],
            [W_QUEEN_CASTLE_EMPTY, B_QUEEN_CASTLE_EMPTY],
        ),
        (
            true,
            [W_KING_SIDE_BISHOP_START, B_KING_SIDE_BISHOP_START],
            [W_KING_CASTLE_EMPTY, B_KING_CASTLE_EMPTY],
        ),
    ];
    if !board.in_check(turn) {
        for (king_side, move_over_tile, empty_tiles) in castle_data {
            if !board.castle_rights.can_castle(turn, king_side) {
                continue;
            }
            let empty_space = empty_tiles[usize::from(turn)];
            // check that space between rook and king is empty
            if (empty_space & board.empty_tiles) == empty_space {
                // tile between king and destination square is not under attack
                let king_pos = if turn == WHITE {
                    W_KING_START
                } else {
                    B_KING_START
                };
                let slide_tile = move_over_tile[usize::from(turn)];
                let slide_move = Move::new_default(king_pos, slide_tile);
                if !board.would_check(slide_move) {
                    moves.push(Move::new_castle(king_side, turn));
                }
            }
        }
    }
}
