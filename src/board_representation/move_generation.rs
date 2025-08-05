use crate::board_representation::computed_boards::{
    BISHOP_ATTACKS, BISHOP_BLOCKERS, BISHOP_MAGICS, ROOK_ATTACKS, ROOK_BLOCKERS, ROOK_MAGICS,
};
use crate::board_representation::r#const::{
    B_KING_SIDE_BISHOP_START, B_KING_START, B_QUEEN_START, NORTH, NORTH_EAST,
    NORTH_WEST, SOUTH, SOUTH_EAST, SOUTH_WEST, W_KING_SIDE_BISHOP_START, W_KING_START,
    W_QUEEN_START,
};

use super::bitboard::Bitboard;
use super::board::{Board, Turn, WHITE};
use super::computed_boards::{KING_RING_MOVES, KNIGHT_MOVES};
use super::masks::*;
use super::piece::Piece;
use super::position::Position;
use super::r#const::EMPTY_BIT_B;
use super::r#move::{Move, EN_PASSANT};

impl Board {
    pub fn generate_moves(&mut self, turn: Turn) -> Vec<Move> {
        generate_pseudo_legal_moves(self, turn)
            .iter()
            .filter(|m| !self.would_check(**m))
            .cloned()
            .collect()
    }
}

pub fn generate_pseudo_legal_moves(board: &mut Board, turn: Turn) -> Vec<Move> {
    let mut moves = generate_pseudo_non_castle_moves(board, turn);
    castle_moves(board, turn, &mut moves);
    moves
}

pub fn generate_pseudo_non_castle_moves(board: &Board, turn: Turn) -> Vec<Move> {
    // skip castle moves, used when checking for checks to avoid inf loop
    let mut moves = Vec::new();
    sliding_pieces_moves(board, turn, &mut moves);
    knight_moves(board, turn, &mut moves);
    king_ring_moves(board, turn, &mut moves);
    pawn_moves(board, turn, &mut moves);
    moves
}

pub(crate) fn get_sliding_moves(
    deltas: &[(i8, i8); 4],
    pos: Position,
    blockers: Bitboard,
) -> Bitboard {
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

fn pawn_moves(board: &Board, turn: Turn, moves: &mut Vec<Move>) {
    let direction = if turn == WHITE { NORTH } else { SOUTH };
    let start_rank = if turn == WHITE { RANK_2 } else { RANK_7 };
    let promotion_rank = if turn == WHITE { RANK_8 } else { RANK_1 };
    let enemy_pieces = board.player_boards[!turn as usize];

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
        let legal_bb = moves_bb & !board.player_boards[turn as usize];
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
        let legal_bb = moves_bb & !board.player_boards[turn as usize];
        extract_moves(legal_bb, origin, moves);
        bishop_board.reset_lsb();
    }
}

fn knight_moves(board: &Board, turn: Turn, moves: &mut Vec<Move>) {
    let mut knight_board = board.get_piece_bitboard(Piece::Knight, turn);
    let not_my_pieces = !board.player_boards[turn as usize];
    while knight_board.is_not_empty() {
        let origin = knight_board.trailing_zeros();
        let moves_bb = KNIGHT_MOVES[origin] & not_my_pieces;
        extract_moves(moves_bb, origin, moves);
        knight_board.reset_lsb();
    }
}

fn king_ring_moves(board: &Board, turn: Turn, moves: &mut Vec<Move>) {
    let king_board = board.get_piece_bitboard(Piece::King, turn);
    let not_my_pieces = !board.player_boards[turn as usize];
    let origin = king_board.trailing_zeros();

    let ring_moves = KING_RING_MOVES[origin] & not_my_pieces;
    extract_moves(ring_moves, origin, moves);
}

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
            let empty_space = empty_tiles[turn as usize];
            // check that space between rook and king is empty
            if (empty_space & board.empty_tiles) == empty_space {
                // tile between king and destination square is not under attack
                let king_pos = if turn == WHITE {
                    W_KING_START
                } else {
                    B_KING_START
                };
                let slide_tile = move_over_tile[turn as usize];
                let slide_move = Move::new_default(king_pos, slide_tile);
                if !board.would_check(slide_move) {
                    moves.push(Move::new_castle(king_side, turn));
                }
            }
        }
    }
}
