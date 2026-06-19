//! [`Board`], the central position type, and the colour/result helpers that go
//! with it.
//!
//! A position is stored as twelve [`Bitboard`]s — one per `(piece, colour)`
//! pair — indexed by `piece as usize + colour * 6` (see
//! `get_bb_index`), so white pieces occupy slots `0..6`
//! and black pieces `6..12`. The per-colour unions ([`player_boards`](Board::player_boards))
//! and the [`empty_tiles`](Board::empty_tiles) set are derived aggregates,
//! recomputed after edits by `compute_bitboards`. A Zobrist hash (`u64`) is
//! maintained incrementally as pieces move, and a history stack of `StateDelta`
//! records lets moves be undone.
//!
//! Colours are modelled by the [`Turn`] type alias (`bool`): [`WHITE`] is
//! `false` and [`BLACK`] is `true`, so the opponent of `turn` is always `!turn`.

use super::bitboard::Bitboard;
use super::castle_rights::CastleRights;
use super::game_state::StateDelta;
use super::position::Position;
use super::r#const::EMPTY_BIT_B;
use super::r#move::Move;
use crate::chess_engine::{
    computed_boards::{
        BISHOP_ATTACKS, BISHOP_BLOCKERS, BISHOP_MAGICS, KING_RING_MOVES, KNIGHT_MOVES,
        PAWN_ATTACKS, ROOK_ATTACKS, ROOK_BLOCKERS, ROOK_MAGICS, ZOBRIST_TABLE,
    },
    fen_parser::START_POS_FEN,
    piece::{Piece, PIECE_COUNT},
    zobrist::ZobristHash,
};

/// The side to move, modelled as a `bool`: [`WHITE`] (`false`) or [`BLACK`]
/// (`true`). The opponent of a `Turn` is its logical negation.
pub type Turn = bool;
/// The number of players (always two).
pub const PLAYER_COUNT: usize = 2;
/// The white side ([`Turn`] = `false`).
pub const WHITE: Turn = false;
/// The black side ([`Turn`] = `true`).
pub const BLACK: Turn = true;

/// High-level status of a position. Currently informational; the search derives
/// terminal results (checkmate/draw) directly while exploring.
#[derive(Clone, PartialEq, Eq)]
pub enum GameState {
    /// Normal play, no special condition.
    Playing,
    /// The side to move is in check.
    Check,
    /// The game is drawn.
    Draw,
    /// The side to move is checkmated.
    CheckMate,
}

/// A full chess position: piece placement, side to move, and the auxiliary
/// state (castling rights, en passant, clocks, Zobrist hash, undo history).
///
/// See the [module documentation](self) for the bitboard layout and indexing.
#[derive(Clone, PartialEq, Eq)]
pub struct Board {
    /// One bitboard per `(piece, colour)` pair, indexed by
    /// `get_bb_index` (white `0..6`, black `6..12`).
    pub(crate) piece_boards: [Bitboard; PLAYER_COUNT * PIECE_COUNT],
    /// Union of all pieces for each colour (`[white, black]`); a derived cache.
    pub player_boards: [Bitboard; PLAYER_COUNT],
    /// The set of unoccupied squares; a derived cache.
    pub empty_tiles: Bitboard,
    /// The side to move.
    pub turn: Turn,
    // if any pawn can be captured by en passant(just made double move)
    // position that enemy should attack at will be recorded here
    /// The square an en-passant capture may land on, or empty if none is
    /// available (set when a pawn has just made a double step).
    pub en_passant: Bitboard,
    /// Half-moves since the last capture or pawn move (the fifty-move clock).
    pub halfmove_count: u8,
    /// The full-move number, incremented after each black move.
    pub fullmove_count: u16,
    /// Which castles are still available to each side.
    pub castle_rights: CastleRights,
    /// The Zobrist hash of the current position, maintained incrementally.
    pub(crate) zobrist_key: ZobristHash,
    /// Undo stack: the pre-move snapshot for every move played from this board.
    pub(crate) history: Vec<StateDelta>,
    /// Cached high-level [`GameState`].
    pub(crate) game_state: GameState,
}

impl Board {
    const fn new_empty() -> Self {
        Self {
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

    /// Builds a board from explicit piece bitboards and state, then derives the
    /// aggregate boards and the initial Zobrist hash. The twelve `piece_boards`
    /// must follow the `get_bb_index` layout.
    #[must_use]
    pub fn new_from_bitboards(
        piece_boards: [Bitboard; PLAYER_COUNT * PIECE_COUNT],
        turn: Turn,
        en_passant: Bitboard,
        halfmove: u8,
        fullmove: u16,
        castle_rights: CastleRights,
    ) -> Self {
        let mut board = Self::new_empty();
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
    /// Returns the standard chess starting position.
    ///
    /// # Errors
    ///
    /// Returns `Err` only if the built-in start-position FEN fails to parse,
    /// which should never happen.
    ///
    /// ```
    /// use chess_engine::chess_engine::board::{Board, WHITE};
    /// use chess_engine::chess_engine::piece::Piece;
    /// use chess_engine::chess_engine::position::Position;
    ///
    /// let board = Board::new_start_pos().unwrap();
    /// assert_eq!(board.turn, WHITE);
    /// // a1 holds a white rook
    /// assert_eq!(board.get_piece_at(Position::new(0)), Some((Piece::Rook, WHITE)));
    /// ```
    pub fn new_start_pos() -> Result<Self, String> {
        Self::from_fen(START_POS_FEN)
    }

    fn compute_initial_zobrist(&mut self) {
        self.zobrist_key = ZOBRIST_TABLE.hash_position(self);
    }

    /// Returns `true` when neither side has enough material to deliver mate:
    /// K vs K, K+B vs K, K+N vs K, or K+B vs K+B with both bishops on the
    /// same square colour.
    ///
    /// ```
    /// use chess_engine::chess_engine::board::Board;
    /// // bare kings — a dead draw
    /// let kk = Board::from_fen("8/8/8/4k3/8/8/4K3/8 w - - 0 1").unwrap();
    /// assert!(kk.is_insufficient_material());
    /// // the opening position has plenty of material
    /// assert!(!Board::new_start_pos().unwrap().is_insufficient_material());
    /// ```
    #[must_use]
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
    /// Drives twofold/threefold repetition detection.
    pub(crate) fn get_count_of_current_position_reached(&self) -> usize {
        self.history
            .iter()
            .filter(|s| s.zobrist_hash == self.zobrist_key)
            .count()
    }

    /// Placeholder for refreshing the cached [`GameState`]. Currently a no-op:
    /// terminal results are detected inside the search instead (see the
    /// commented-out reference implementation below).
    #[allow(clippy::unused_self)]
    pub(crate) const fn update_game_result(&self) {
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

    /// Removes `piece` of colour `turn` from `pos`, keeping the Zobrist hash in
    /// sync.
    pub(crate) fn remove_piece(&mut self, turn: Turn, piece: Piece, pos: Position) {
        // remove piece from bitboard and zobrist key, keeping the per-colour
        // player_boards aggregate in sync incrementally. empty_tiles is derived
        // from player_boards once per move (see refresh_empty_tiles) because a
        // square can transiently hold two pieces during a capture sequence,
        // which a naive set/clear here would get wrong.
        let bb_index = Self::get_bb_index(piece, turn);
        let square = pos.as_usize();
        self.piece_boards[bb_index].clear_square(square);
        self.player_boards[usize::from(turn)].clear_square(square);
        self.xor_piece_from_zobrist(turn, piece, pos);
    }

    /// Adds `piece` of colour `turn` at `pos`, keeping the Zobrist hash and the
    /// per-colour player board in sync.
    pub(crate) fn add_piece(&mut self, turn: Turn, piece: Piece, pos: Position) {
        // see remove_piece for why empty_tiles is not touched here
        let bb_index = Self::get_bb_index(piece, turn);
        let square = pos.as_usize();
        self.piece_boards[bb_index].set_square(square);
        self.player_boards[usize::from(turn)].set_square(square);
        self.xor_piece_from_zobrist(turn, piece, pos);
    }

    /// Recomputes [`empty_tiles`](Board::empty_tiles) from the per-colour
    /// [`player_boards`](Board::player_boards). Cheap (one OR + one NOT) and used
    /// after a move's piece edits settle, since `player_boards` is kept correct
    /// incrementally while `empty_tiles` is not.
    pub(crate) fn refresh_empty_tiles(&mut self) {
        self.empty_tiles = !(self.player_boards[0] | self.player_boards[1]);
    }

    /// Toggles a single `(colour, piece, square)` entry in the Zobrist hash;
    /// XOR-ing the same entry twice cancels out, which is what makes the hash
    /// cheap to maintain incrementally.
    pub(crate) fn xor_piece_from_zobrist(&mut self, turn: Turn, piece: Piece, pos: Position) {
        self.zobrist_key ^=
            ZOBRIST_TABLE.piece_square[usize::from(turn)][piece as usize][pos.as_usize()];
    }

    /// Toggles the en-passant-file contribution to the Zobrist hash (a no-op
    /// when there is no en-passant square).
    pub(crate) fn xor_en_pass_from_zobrist(&mut self, en_passant: Bitboard) {
        if en_passant.is_not_empty() {
            let pos = Position::new(en_passant.trailing_zeros());
            let (file, _) = pos.get_file_and_rank();
            self.zobrist_key ^= ZOBRIST_TABLE.en_passant_file[file];
        }
    }

    /// Revokes one castling right (if currently held) and updates the Zobrist
    /// hash to match.
    pub(crate) fn remove_castle(&mut self, turn: Turn, king_side: bool) {
        // remove castle rights
        // also updates zobrist key
        if self.castle_rights.can_castle(turn, king_side) {
            self.zobrist_key ^=
                ZOBRIST_TABLE.castle_rights[self.castle_rights.castle_index(turn, king_side)];
            self.castle_rights.remove_castle_right(turn, king_side);
        }
    }

    /// Recomputes the derived [`player_boards`](Board::player_boards) and
    /// [`empty_tiles`](Board::empty_tiles) from the per-piece bitboards.
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

    /// Returns the bitboard of all `piece`s belonging to `turn`.
    #[must_use]
    pub fn get_piece_bitboard(&self, piece: Piece, turn: Turn) -> Bitboard {
        self.piece_boards[Self::get_bb_index(piece, turn)]
    }

    /// Maps a `(piece, colour)` pair to its index into
    /// [`piece_boards`](Board::piece_boards): `piece as usize + colour * 6`.
    pub(crate) fn get_bb_index(piece: Piece, turn: Turn) -> usize {
        piece as usize + (usize::from(turn) * PIECE_COUNT)
    }

    /// Inverse of `get_bb_index`: recovers the
    /// `(piece, colour)` pair from a `piece_boards` index.
    pub(crate) fn get_piece_information_index(index: usize) -> (Piece, Turn) {
        (Piece::from(index % PIECE_COUNT), index >= PIECE_COUNT)
    }

    /// Returns `true` if `attacking_player` attacks `tile`. Rather than
    /// enumerating that side's moves, this projects each piece type's attacks
    /// *outward from `tile`* and intersects with the matching enemy pieces — a
    /// pawn attacks `tile` iff an enemy pawn sits on a square this side's pawn on
    /// `tile` would capture, a knight iff an enemy knight is a knight-jump away,
    /// and so on for the king and (via the magic tables) the sliders.
    #[must_use]
    pub fn tile_under_attack(&self, tile: Position, attacking_player: Turn) -> bool {
        self.is_square_attacked(tile.as_usize(), attacking_player)
    }

    /// Returns `true` if any piece belonging to `by` attacks square `sq`
    /// (a `0..64` index). This is the allocation-free core of check detection;
    /// see [`tile_under_attack`](Self::tile_under_attack).
    #[must_use]
    pub fn is_square_attacked(&self, sq: usize, by: Turn) -> bool {
        self.is_square_attacked_occ(sq, by, !self.empty_tiles)
    }

    /// Like [`is_square_attacked`](Self::is_square_attacked) but takes an explicit
    /// `occupancy` for the sliding-piece rays. King-move legality passes the
    /// occupancy with the moving king removed, so a slider's attack is seen to
    /// pass through the square the king is vacating.
    #[must_use]
    pub fn is_square_attacked_occ(&self, sq: usize, by: Turn, occupancy: Bitboard) -> bool {
        // knights
        if (KNIGHT_MOVES[sq] & self.get_piece_bitboard(Piece::Knight, by)).is_not_empty() {
            return true;
        }
        // king adjacency
        if (KING_RING_MOVES[sq] & self.get_piece_bitboard(Piece::King, by)).is_not_empty() {
            return true;
        }
        // pawns: a `by`-pawn attacks `sq` from the squares a pawn of the
        // opposite colour on `sq` would attack
        let pawn_sources = PAWN_ATTACKS[usize::from(!by)][sq];
        if (pawn_sources & self.get_piece_bitboard(Piece::Pawn, by)).is_not_empty() {
            return true;
        }

        let queens = self.get_piece_bitboard(Piece::Queen, by);

        // bishops / queens along diagonals
        let bishop_entry = BISHOP_MAGICS[sq];
        let bishop_attacks = BISHOP_ATTACKS
            [bishop_entry.magic_index(occupancy & BISHOP_BLOCKERS[sq]) + bishop_entry.offset];
        if (bishop_attacks & (self.get_piece_bitboard(Piece::Bishop, by) | queens)).is_not_empty() {
            return true;
        }

        // rooks / queens along ranks and files
        let rook_entry = ROOK_MAGICS[sq];
        let rook_attacks =
            ROOK_ATTACKS[rook_entry.magic_index(occupancy & ROOK_BLOCKERS[sq]) + rook_entry.offset];
        (rook_attacks & (self.get_piece_bitboard(Piece::Rook, by) | queens)).is_not_empty()
    }

    /// Returns `true` if `turn`'s king is currently attacked.
    ///
    /// ```
    /// use chess_engine::chess_engine::board::{Board, WHITE};
    /// use chess_engine::chess_engine::utils::init_tables;
    ///
    /// init_tables();
    /// // black rook on a1 checks the white king on e1 along the first rank
    /// let board = Board::from_fen("4k3/8/8/8/8/8/8/r3K3 w - - 0 1").unwrap();
    /// assert!(board.in_check(WHITE));
    /// ```
    #[must_use]
    pub fn in_check(&self, turn: Turn) -> bool {
        let king_board = self.get_piece_bitboard(Piece::King, turn);
        let king_pos = Position::new(king_board.trailing_zeros());
        self.tile_under_attack(king_pos, !turn)
    }

    /// Returns `true` if playing `move_` would leave the mover's own king in
    /// check. Used to filter pseudo-legal moves down to legal ones; the move is
    /// applied and immediately undone, so `self` is unchanged on return.
    pub fn would_check(&mut self, move_: Move) -> bool {
        self.commit_verified_move(move_);
        let is_check = self.in_check(!self.turn);
        self.unmake_move();
        is_check
    }

    /// Returns the piece type occupying `pos` (ignoring colour), or
    /// [`Piece::None`] if the square is empty.
    pub(crate) fn get_piece_type_containing_position(&self, pos: Position) -> Piece {
        for (index, board) in self.piece_boards.iter().enumerate() {
            if board.is_square_set(pos.into()) {
                return Piece::from(index % PIECE_COUNT);
            }
        }
        Piece::None
    }

    /// Returns the `(piece, colour)` occupying `pos`, or `None` if the square is
    /// empty.
    #[must_use]
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

    /// Prints the position to stdout as an 8×8 grid (uppercase = white,
    /// lowercase = black, `.` = empty), rank 8 at the top.
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
