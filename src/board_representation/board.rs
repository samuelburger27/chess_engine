use super::bitboard::Bitboard;
use super::castle_rights::CastleRights;
use super::game_state::StateDelta;
use super::move_generation::*;
use super::position::Position;
use super::r#const::EMPTY_BIT_B;
use super::r#move::Move;
use crate::board_representation::{
    fen_parser::START_POS_FEN,
    piece::{Piece, PIECE_COUNT},
    r#const::{
        B_KING_ROOK_START, B_KING_SIDE_BISHOP_START, B_QUEEN_ROOK_START, B_QUEEN_START, NORTH,
        SOUTH, W_KING_ROOK_START, W_KING_SIDE_BISHOP_START, W_QUEEN_ROOK_START, W_QUEEN_START,
    },
    r#move::SpecialMove,
};

pub type Turn = bool;
pub const PLAYER_COUNT: usize = 2;
pub const WHITE: Turn = false;
pub const BLACK: Turn = true;

type BitboardMutIter<'a> = std::iter::Take<std::iter::Skip<std::slice::IterMut<'a, Bitboard>>>;

#[derive(Clone, PartialEq)]
pub struct Board {
    piece_boards: [Bitboard; PLAYER_COUNT * PIECE_COUNT],
    pub player_boards: [Bitboard; PLAYER_COUNT],
    pub empty_tiles: Bitboard,
    pub turn: Turn,
    // if any pawn can be captured by en passant(just made double move)
    // position that enemy should attack at will be recorded here
    pub en_passant: Bitboard,

    pub halfmove_count: u8,
    pub fullmove_count: u16,

    pub castle_rights: CastleRights,

    history: Vec<StateDelta>,
}

impl Board {
    pub fn new_empty() -> Self {
        Board {
            player_boards: [EMPTY_BIT_B, EMPTY_BIT_B],
            piece_boards: [
                EMPTY_BIT_B,
                EMPTY_BIT_B,
                EMPTY_BIT_B,
                EMPTY_BIT_B,
                EMPTY_BIT_B,
                EMPTY_BIT_B,
                EMPTY_BIT_B,
                EMPTY_BIT_B,
                EMPTY_BIT_B,
                EMPTY_BIT_B,
                EMPTY_BIT_B,
                EMPTY_BIT_B,
            ],
            empty_tiles: EMPTY_BIT_B,
            turn: WHITE,
            en_passant: EMPTY_BIT_B,
            halfmove_count: 0,
            fullmove_count: 0,
            castle_rights: CastleRights::make_default(),
            history: Vec::new(),
        }
    }

    pub fn new_from_bitboards(
        piece_boards: [Bitboard; PLAYER_COUNT * PIECE_COUNT],
        turn: Turn,
        en_passant: Bitboard,
        halfmove: u8,
        fullmove: u16,
        castle_rights: CastleRights,
    ) -> Self {
        let mut board = Board::new_empty();
        board.piece_boards = piece_boards;
        board.turn = turn;
        board.en_passant = en_passant;
        board.halfmove_count = halfmove;
        board.fullmove_count = fullmove;
        board.castle_rights = castle_rights;
        board.compute_bitboards();
        board
    }
    pub fn new_start_pos() -> Result<Board, String> {
        Board::from_fen(START_POS_FEN)
    }

    fn compute_bitboards(&mut self) {
        // recompute empty tiles, and player boards
        let mut white_pieces = Bitboard::new();
        let mut black_pieces = Bitboard::new();
        for index in 0..PIECE_COUNT {
            white_pieces |= self.piece_boards[index];
            black_pieces |= self.piece_boards[index + PIECE_COUNT];
        }
        self.player_boards = [white_pieces, black_pieces];
        self.empty_tiles = !(white_pieces | black_pieces);
    }

    pub fn get_piece_bitboard(&self, piece: Piece, turn: Turn) -> Bitboard {
        self.piece_boards[Board::get_bb_index(piece, turn)]
    }

    fn get_piece_bitboard_ref(&mut self, piece: Piece, turn: Turn) -> &mut Bitboard {
        &mut self.piece_boards[Board::get_bb_index(piece, turn)]
    }

    pub(crate) fn get_bb_index(piece: Piece, turn: Turn) -> usize {
        piece as usize + (turn as usize * PIECE_COUNT)
    }

    pub fn tile_under_attack(&self, tile: Position, attacking_player: Turn) -> bool {
        let moves = generate_pseudo_non_castle_moves(self, attacking_player);
        moves.iter().any(|m| m.get_dest() == tile)
    }

    pub fn in_check(&self, turn: Turn) -> bool {
        let king_board = self.get_piece_bitboard(Piece::King, turn);
        let king_pos = Position::new(king_board.trailing_zeros());
        self.tile_under_attack(king_pos, !turn)
    }

    fn update_game_state(&mut self) {
        // this method should be called when creating board and when committing a move
        // TODO, draw, stalemate
    }

    pub fn would_check(&mut self, move_: Move) -> bool {
        self.commit_verified_move(move_);
        let is_check = self.in_check(!self.turn);
        self.unmake_move();
        is_check
    }

    fn get_piece_type_containing_position(&self, pos: Position) -> Piece {
        for (index, board) in self.piece_boards.iter().enumerate() {
            if board.is_square_set(pos.into()) {
                return Piece::from(index % PIECE_COUNT);
            }
        }
        Piece::None
    }

    fn get_player_bit_boards_iter(&mut self, turn: Turn) -> BitboardMutIter {
        self.piece_boards
            .iter_mut()
            .skip(turn as usize * PIECE_COUNT)
            .take(PIECE_COUNT)
    }

    pub fn commit_verified_move(&mut self, move_: Move) {
        // commit move
        // move should be verified before
        let (origin, destination) = move_.get_org_and_dest();
        let mut captured_piece = if !self.empty_tiles.is_square_set(destination.into()) {
            Some(self.get_piece_type_containing_position(destination))
        } else {
            None
        };

        let moving_piece = self.get_piece_type_containing_position(origin);

        let b_b_index = Board::get_bb_index(moving_piece, self.turn);

        self.piece_boards[b_b_index].clear_square(origin.as_usize());
        self.piece_boards[b_b_index].set_square(destination.as_usize());

        for enemy_board in self.get_player_bit_boards_iter(!self.turn) {
            enemy_board.clear_square(destination.into());
        }

        match move_.get_special_move() {
            SpecialMove::Promotion => {
                self.piece_boards[b_b_index].clear_square(destination.as_usize());
                let promote_to = move_.get_promotion();
                let promote_bb_index = Board::get_bb_index(promote_to, self.turn);
                self.piece_boards[promote_bb_index].set_square(destination.into());
            }

            SpecialMove::Castle => {
                let (dest_file, _) = destination.get_file_and_rank();
                let rook_bb_i = Board::get_bb_index(Piece::Rook, self.turn);
                // queen side
                if dest_file == 2 {
                    if self.turn == WHITE {
                        self.piece_boards[rook_bb_i].clear_square(W_QUEEN_ROOK_START.into());
                        self.piece_boards[rook_bb_i].set_square(W_QUEEN_START.into());
                    } else {
                        self.piece_boards[rook_bb_i].clear_square(B_QUEEN_ROOK_START.into());
                        self.piece_boards[rook_bb_i].set_square(B_QUEEN_START.into());
                    }
                }
                // king side
                else {
                    if self.turn == WHITE {
                        self.piece_boards[rook_bb_i].clear_square(W_KING_ROOK_START.into());
                        self.piece_boards[rook_bb_i].set_square(W_KING_SIDE_BISHOP_START.into());
                    } else {
                        self.piece_boards[rook_bb_i].clear_square(B_KING_ROOK_START.into());
                        self.piece_boards[rook_bb_i].set_square(B_KING_SIDE_BISHOP_START.into());
                    }
                }
            }

            SpecialMove::EnPassant => {
                let backward = if self.turn == WHITE { SOUTH } else { NORTH };
                let enemy_pawns = self.get_piece_bitboard_ref(Piece::Pawn, !self.turn);
                enemy_pawns.clear_square((destination.as_usize() as i8 + backward) as usize);
                captured_piece = Some(Piece::Pawn);
            }

            SpecialMove::NormalMove => (),
        }

        self.history.push(StateDelta::new(
            move_,
            captured_piece,
            self.en_passant,
            self.castle_rights,
            self.halfmove_count,
        ));

        let pawn_moved = moving_piece == Piece::Pawn;

        // update board state
        self.en_passant = Bitboard::new();
        match moving_piece {
            Piece::Pawn => {
                // en passant
                if (origin.as_usize() as i8 - destination.as_usize() as i8).abs() == 2 * NORTH {
                    // middle between des and origin
                    let en_passant_square = (origin.as_usize() + destination.as_usize()) / 2;

                    self.en_passant.set_square(en_passant_square);
                }
            }
            // castle rights
            Piece::King => {
                self.castle_rights.remove_castle_right(self.turn, true);
                self.castle_rights.remove_castle_right(self.turn, false);
            }

            Piece::Rook => {
                if (origin == W_KING_ROOK_START && self.turn == WHITE)
                    || (origin == B_KING_ROOK_START && self.turn == BLACK)
                {
                    self.castle_rights.remove_castle_right(self.turn, true);
                }
                if (origin == W_QUEEN_ROOK_START && self.turn == WHITE)
                    || (origin == B_QUEEN_ROOK_START && self.turn == BLACK)
                {
                    self.castle_rights.remove_castle_right(self.turn, false);
                }
            }

            _ => (),
        }

        self.captured_rook_remove_castle_rights();

        self.fullmove_count += 1;
        if let None = captured_piece {
            if !pawn_moved {
                self.halfmove_count += 1;
            }
        }
        self.turn = !self.turn;

        self.compute_bitboards();

        self.update_game_state();
    }

    fn captured_rook_remove_castle_rights(&mut self) {
        // capture rook, remove castle right
        let enemy_rooks = self.get_piece_bitboard(Piece::Rook, !self.turn);
        let enemy_queen_rook = if self.turn == WHITE {
            B_QUEEN_ROOK_START
        } else {
            W_QUEEN_ROOK_START
        };
        let enemy_king_rook = if self.turn == WHITE {
            B_KING_ROOK_START
        } else {
            W_KING_ROOK_START
        };
        if !enemy_rooks.is_square_set(enemy_king_rook.as_usize()) {
            self.castle_rights.remove_castle_right(!self.turn, true);
        }
        if !enemy_rooks.is_square_set(enemy_queen_rook.as_usize()) {
            self.castle_rights.remove_castle_right(!self.turn, false);
        }
    }

    fn unmake_move(&mut self) {
        let Some(move_delta) = self.history.pop() else {
            return;
        };
        self.en_passant = move_delta.en_pass;
        self.castle_rights = move_delta.castle_rights;
        self.halfmove_count = move_delta.halfmove;
        self.fullmove_count -= 1;
        self.turn = !self.turn;

        let last_move = move_delta.move_;
        let (origin, dest) = last_move.get_org_and_dest();

        let pawn_bb_index = Board::get_bb_index(Piece::Pawn, self.turn);

        let moving_piece = self.get_piece_type_containing_position(dest);
        let b_b_index = Board::get_bb_index(moving_piece, self.turn);

        self.piece_boards[b_b_index].set_square(origin.as_usize());
        self.piece_boards[b_b_index].clear_square(dest.as_usize());

        match last_move.get_special_move() {
            SpecialMove::NormalMove => {
                if let Some(captured_piece) = move_delta.captured_piece {
                    let cap_bb_index = Board::get_bb_index(captured_piece, !self.turn);
                    self.piece_boards[cap_bb_index].set_square(dest.as_usize());
                }
            }

            SpecialMove::Promotion => {
                if let Some(captured_piece) = move_delta.captured_piece {
                    let cap_bb_index = Board::get_bb_index(captured_piece, !self.turn);
                    self.piece_boards[cap_bb_index].set_square(dest.as_usize());
                }
                self.piece_boards[b_b_index].clear_square(origin.as_usize());
                self.piece_boards[pawn_bb_index].set_square(origin.as_usize());
            }

            SpecialMove::EnPassant => {
                let backward = if self.turn == WHITE { SOUTH } else { NORTH };
                if let Some(pawn_pos) = dest.try_offset(backward) {
                    let enemy_pawn_index = Board::get_bb_index(Piece::Pawn, !self.turn);
                    self.piece_boards[enemy_pawn_index].set_square(pawn_pos.as_usize());
                }
            }

            SpecialMove::Castle => {
                let (dest_file, _) = dest.get_file_and_rank();
                let rook_bb_i = Board::get_bb_index(Piece::Rook, self.turn);
                // queen side
                if dest_file == 2 {
                    if self.turn == WHITE {
                        self.piece_boards[rook_bb_i].set_square(W_QUEEN_ROOK_START.into());
                        self.piece_boards[rook_bb_i].clear_square(W_QUEEN_START.into());
                    } else {
                        self.piece_boards[rook_bb_i].set_square(B_QUEEN_ROOK_START.into());
                        self.piece_boards[rook_bb_i].clear_square(B_QUEEN_START.into());
                    }
                }
                // king side
                else {
                    if self.turn == WHITE {
                        self.piece_boards[rook_bb_i].set_square(W_KING_ROOK_START.into());
                        self.piece_boards[rook_bb_i].clear_square(W_KING_SIDE_BISHOP_START.into());
                    } else {
                        self.piece_boards[rook_bb_i].set_square(B_KING_ROOK_START.into());
                        self.piece_boards[rook_bb_i].clear_square(B_KING_SIDE_BISHOP_START.into());
                    }
                }
            }
        }
        self.compute_bitboards();
        self.update_game_state();
    }

    fn make_input_move(&mut self, origin: Position, dest: Position, promote: Piece) -> bool {
        let moves = self.generate_moves(self.turn);
        for move_ in moves {
            if move_.get_origin() == origin
                && move_.get_dest() == dest
                && move_.get_promotion() == promote
            {
                self.commit_verified_move(move_);
                return true;
            }
        }
        return false;
    }

    pub fn play_string_move(&mut self, s_move: &str) -> bool {
        if s_move.len() != 4 && s_move.len() != 5 {
            return false;
        }
        let promote = if s_move.len() == 5 {
            if let Ok(piece) = Piece::try_from(&s_move[4..5]) {
                piece
            } else {
                return false;
            }
        } else {
            Piece::Queen
        };

        let (from_s, to_s) = (&s_move[..2], &s_move[2..4]);

        let (Ok(origin), Ok(dest)) = (Position::try_from(from_s), Position::try_from(to_s)) else {
            return false;
        };
        return self.make_input_move(origin, dest, promote);
    }

    pub fn print_board(&self) {
        let piece_chars = [
            'P', 'R', 'N', 'B', 'K', 'Q', // White pieces
            'p', 'r', 'n', 'b', 'k', 'q', // Black pieces
        ];

        println!("  +------------------------+");
        for rank in (0..8).rev() {
            print!("{} |", rank + 1);
            for file in 0..8 {
                let pos = Position::from_file_and_rank(file, rank);
                let mut found = false;

                for (i, &bb) in self.piece_boards.iter().enumerate() {
                    if bb.is_square_set(pos.as_usize()) {
                        print!(" {} ", piece_chars[i]);
                        found = true;
                        break;
                    }
                }

                if !found {
                    print!(" . ");
                }
            }
            println!("|");
        }
        println!("  +------------------------+");
        println!("    a  b  c  d  e  f  g  h");
    }
}
