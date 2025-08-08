use super::bitboard::Bitboard;
use super::castle_rights::CastleRights;
use super::game_state::StateDelta;
use super::move_generation::generate_pseudo_non_castle_moves;
use super::position::Position;
use super::r#const::EMPTY_BIT_B;
use super::r#move::Move;
use crate::board_representation::{
    computed_boards::ZOBRIST_TABLE,
    fen_parser::START_POS_FEN,
    piece::{Piece, PIECE_COUNT},
    zobrist::ZobristHash,
};

pub type Turn = bool;
pub const PLAYER_COUNT: usize = 2;
pub const WHITE: Turn = false;
pub const BLACK: Turn = true;

#[derive(Clone, PartialEq)]
pub struct Board {
    pub(crate) piece_boards: [Bitboard; PLAYER_COUNT * PIECE_COUNT],
    pub player_boards: [Bitboard; PLAYER_COUNT],
    pub empty_tiles: Bitboard,
    pub turn: Turn,
    // if any pawn can be captured by en passant(just made double move)
    // position that enemy should attack at will be recorded here
    pub en_passant: Bitboard,

    pub halfmove_count: u8,
    pub fullmove_count: u16,

    pub castle_rights: CastleRights,
    pub zobrist_key: ZobristHash,

    pub(crate) history: Vec<StateDelta>,
}

impl Board {
    fn new_empty() -> Self {
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
            zobrist_key: 0,
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
        board.compute_initial_zobrist();
        board
    }
    pub fn new_start_pos() -> Result<Board, String> {
        Board::from_fen(START_POS_FEN)
    }

    fn compute_initial_zobrist(&mut self) {
        self.zobrist_key = ZOBRIST_TABLE.hash_position(&self);
    }

    pub(crate) fn compute_bitboards(&mut self) {
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

    pub(crate) fn update_game_state(&mut self) {
        // this method should be called when creating board and when committing a move
        // TODO, draw, stalemate
    }

    pub fn would_check(&mut self, move_: Move) -> bool {
        self.commit_verified_move(move_);
        let is_check = self.in_check(!self.turn);
        self.unmake_move();
        is_check
    }

    pub(crate) fn get_piece_type_containing_position(&self, pos: Position) -> Piece {
        for (index, board) in self.piece_boards.iter().enumerate() {
            if board.is_square_set(pos.into()) {
                return Piece::from(index % PIECE_COUNT);
            }
        }
        Piece::None
    }

    pub fn get_piece_at(&self, pos: Position) -> Option<(Piece, Turn)> {
        for (index, board) in self.piece_boards.iter().enumerate() {
            if board.is_square_set(pos.into()) {
                return Some((
                    Piece::from(index % PIECE_COUNT),
                    if index < PIECE_COUNT { WHITE } else { BLACK },
                ));
            }
        }
        None
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
