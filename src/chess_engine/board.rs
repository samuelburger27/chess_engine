use super::bitboard::Bitboard;
use super::castle_rights::CastleRights;
use super::game_state::StateDelta;
use super::move_generation::generate_pseudo_non_castle_moves;
use super::position::Position;
use super::r#const::EMPTY_BIT_B;
use super::r#move::Move;
use crate::chess_engine::{
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
pub enum GameState {
    Playing,
    Check,
    Draw,
    CheckMate,
}

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
    pub(crate) zobrist_key: ZobristHash,
    pub(crate) history: Vec<StateDelta>,
    pub(crate) game_state: GameState,
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
            game_state: GameState::Playing,
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

    /// Neither side can possibly deliver mate:
    /// K vs K, K+B vs K, K+N vs K, or K+B vs K+B with both bishops on the
    /// same square colour.
    pub fn is_insufficient_material(&self) -> bool {
        let pawns = self.get_piece_bitboard(Piece::Pawn, WHITE)
            | self.get_piece_bitboard(Piece::Pawn, BLACK);
        let majors = self.get_piece_bitboard(Piece::Rook, WHITE)
            | self.get_piece_bitboard(Piece::Rook, BLACK)
            | self.get_piece_bitboard(Piece::Queen, WHITE)
            | self.get_piece_bitboard(Piece::Queen, BLACK);
        if (pawns | majors).is_not_empty() {
            return false;
        }

        let knights = self.get_piece_bitboard(Piece::Knight, WHITE)
            | self.get_piece_bitboard(Piece::Knight, BLACK);
        let bishops = self.get_piece_bitboard(Piece::Bishop, WHITE)
            | self.get_piece_bitboard(Piece::Bishop, BLACK);
        let minor_count = knights.count_bits() + bishops.count_bits();

        match minor_count {
            0 | 1 => true,
            2 => {
                // only K+B vs K+B with same-coloured bishops is a dead draw
                let dark_squares = Bitboard::from_u64(0xAA55_AA55_AA55_AA55);
                self.get_piece_bitboard(Piece::Bishop, WHITE).count_bits() == 1
                    && self.get_piece_bitboard(Piece::Bishop, BLACK).count_bits() == 1
                    && ((bishops & dark_squares) == bishops || (bishops & dark_squares).is_empty())
            }
            _ => false,
        }
    }

    /// Number of times the current position already occurred earlier in the
    /// game/search path (history stores the pre-move hash of every position).
    pub(crate) fn get_count_of_current_position_reached(&self) -> usize {
        self.history
            .iter()
            .filter(|s| s.zobrist_hash == self.zobrist_key)
            .count()
    }

    pub(crate) fn update_game_result(&mut self) {
        // this method should be called when creating board and when committing a move
        // TODO finish
        // TODO maybe think about
        // let moves = self.generate_moves(self.turn);
        // if self.in_check(self.turn) {
        //     if moves.is_empty() {
        //         self.game_state = GameState::CheckMate;
        //     } else {
        //         self.game_state = GameState::Check;
        //     }
        // } else if self.halfmove_count >= 50 || moves.is_empty() || self.is_insufficient_material() ||
        // self.get_count_of_current_position_reached() >= 3 {
        //     self.game_state = GameState::Draw
        // }
        // // TODO check for dead position
        // // add check for positions
        // self.game_state = GameState::Playing
    }

    pub(crate) fn remove_piece(&mut self, turn: Turn, piece: Piece, pos: Position) {
        // remove piece from bitboard and zobrist key
        let bb_index = Board::get_bb_index(piece, turn);
        self.piece_boards[bb_index].clear_square(pos.as_usize());
        self.xor_piece_from_zobrist(turn, piece, pos);
    }

    pub(crate) fn add_piece(&mut self, turn: Turn, piece: Piece, pos: Position) {
        // add piece to bitboard and zobrist key
        let bb_index = Board::get_bb_index(piece, turn);
        self.piece_boards[bb_index].set_square(pos.as_usize());
        self.xor_piece_from_zobrist(turn, piece, pos);
    }

    pub(crate) fn xor_piece_from_zobrist(&mut self, turn: Turn, piece: Piece, pos: Position) {
        self.zobrist_key ^=
            ZOBRIST_TABLE.piece_square[turn as usize][piece as usize][pos.as_usize()];
    }

    pub(crate) fn xor_en_pass_from_zobrist(&mut self, en_passant: Bitboard) {
        if en_passant.is_not_empty() {
            let pos = Position::new(en_passant.trailing_zeros());
            let (file, _) = pos.get_file_and_rank();
            self.zobrist_key ^= ZOBRIST_TABLE.en_passant_file[file];
        }
    }

    pub(crate) fn remove_castle(&mut self, turn: Turn, king_side: bool) {
        // remove castle rights
        // also updates zobrist key
        if self.castle_rights.can_castle(turn, king_side) {
            self.zobrist_key ^=
                ZOBRIST_TABLE.castle_rights[self.castle_rights.castle_index(turn, king_side)];
            self.castle_rights.remove_castle_right(turn, king_side);
        }
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

    pub(crate) fn get_piece_information_index(index: usize) -> (Piece, Turn) {
        (Piece::from(index % PIECE_COUNT), index >= PIECE_COUNT)
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
