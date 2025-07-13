use crate::move_generation::get_moves;
use crate::move_generation::get_unchecked_moves;
use crate::move_generation::Move;
use crate::move_generation::SpecialMove;

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Position {
    pub x: usize,
    pub y: usize,
}

impl Position {
    pub fn from_bit_index(index: usize) -> Position {
        return Position {
            x: index % 8,
            y: index / 8,
        };
    }

    pub fn get_bit_index(&self) -> usize {
        return 8 * self.y + self.x;
    }

    pub fn add_scalars(&self, (add_x, add_y): (i32, i32)) -> Result<Position, ()> {
        let x: i32 = self.x as i32 + add_x;
        let y: i32 = self.y as i32 + add_y;
        if x >= 8 || y >= 8 || x < 0 || y < 0 {
            return Err(());
        }
        return Ok(Position {
            x: x as usize,
            y: y as usize,
        });
    }

    pub fn add_bit_scalar(&mut self, scalar: i32) {
        let casted = i32::try_from(self.get_bit_index()).unwrap();
        if casted + scalar < 0 {
            self.x = 0;
            self.y = 0;
            return;
        }
        *self = Position::from_bit_index(usize::try_from(casted + scalar).unwrap());
    }
    pub fn algebraic_notation(&self) -> String {
        let file = match self.x {
            0 => "a",
            1 => "b",
            2 => "c",
            3 => "d",
            4 => "e",
            5 => "f",
            6 => "g",
            7 => "h",
            // should never happen
            _ => "-",
        }
        .to_string();

        return file + &(self.y + 1).to_string();
    }
}

impl TryFrom<&str> for Position {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if value.len() != 2 {
            return Err(());
        }
        let x: usize = match value.chars().nth(0) {
            Some('a') => 0,
            Some('b') => 1,
            Some('c') => 2,
            Some('d') => 3,
            Some('e') => 4,
            Some('f') => 5,
            Some('g') => 6,
            Some('h') => 7,
            _ => return Err(()),
        };

        let Some(ch) = value.chars().nth(1) else {
            return Err(());
        };
        let Some(rank) = ch.to_digit(10) else {
            return Err(());
        };
        if rank > 8 {
            return Err(());
        }
        return Ok(Position {
            x: x,
            y: (rank - 1) as usize,
        });
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Piece {
    Pawn,
    Rook,
    Knight,
    Bishop,
    King,
    Queen,
    None,
}
impl Piece {
    pub fn to_notation(&self) -> String {
        match self {
            Piece::Pawn => "p",
            Piece::Rook => "r",
            Piece::Knight => "n",
            Piece::Bishop => "b",
            Piece::King => "k",
            Piece::Queen => "q",
            Piece::None => "-",
        }
        .to_string()
    }
}

impl TryFrom<&str> for Piece {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "P" | "p" => Ok(Piece::Pawn),
            "R" | "r" => Ok(Piece::Rook),
            "N" | "n" => Ok(Piece::Knight),
            "B" | "b" => Ok(Piece::Bishop),
            "Q" | "q" => Ok(Piece::Queen),
            "K" | "k" => Ok(Piece::King),
            " " | "-" => Ok(Piece::None),
            _ => Err(()),
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum GameState {
    Normal,
    WhiteWon,
    BlackWon,
    Draw,
    WhiteCheck,
    BlackCheck,
}

//const NOT_A_FILE: u64 = 0xfefefefefefefefe;
//const NOT_H_FILE: u64 = 0x7f7f7f7f7f7f7f7f;

type Color = Option<bool>;
type Tile = (Piece, Color);

const EMPTY_TILE: Tile = (Piece::None, None);
#[derive(Clone, Copy)]
pub struct Board {
    pub board: [[Tile; 8]; 8],
    // white_bitboards: [u64; 6],
    // black_bitboards: [u64; 6],
    pub game_state: GameState,
    pub white_turn: bool,
    // if any pawn can be captured by en passant(just made double move) it will be recorded in this variable
    pub en_passant: Option<Position>,

    pub halfmove_count: u32,
    pub fullmove_count: u32,
    // [white_king_side, white_queen_side, black_king_side, black_queen_side]
    pub possible_castle: [bool; 4],
}

pub struct BoardIter<'a> {
    x: usize,
    y: usize,
    board: &'a Board,
}

impl<'a> BoardIter<'a> {
    fn new(board: &'a Board) -> Self {
        BoardIter {
            x: 0,
            y: 0,
            board: board,
        }
    }
}

impl<'a> Iterator for BoardIter<'a> {
    type Item = (Position, Tile);

    fn next(&mut self) -> Option<Self::Item> {
        if self.x >= 8 || self.y >= 8 {
            return None;
        }
        let tile = self.board.board[self.y][self.x];
        let position = Position {
            x: self.x,
            y: self.y,
        };
        self.x += 1;
        if self.x >= 8 {
            self.x = 0;
            self.y += 1;
        }
        Some((position, tile))
    }
}

impl<'a> IntoIterator for &'a Board {
    type Item = (Position, Tile);
    type IntoIter = BoardIter<'a>;
    fn into_iter(self) -> Self::IntoIter {
        BoardIter::new(self)
    }
}

impl Board {
    pub fn new(
        board: [[Tile; 8]; 8],
        turn: bool,
        en_passant: Option<Position>,
        halfmove: u32,
        full_move: u32,
        castle_right: [bool; 4],
    ) -> Board {
        let mut board = Board {
            board: board,
            game_state: GameState::Normal,
            white_turn: turn,
            en_passant: en_passant,
            halfmove_count: halfmove,
            fullmove_count: full_move,
            possible_castle: castle_right,
        };
        board.update_game_state();
        board
    }

    pub fn can_castle(&self, king_side: bool) -> bool {
        let index = 2 * usize::from(self.white_turn) + usize::from(!king_side);
        return self.possible_castle[index];
    }

    fn update_castle_right(&mut self, king_side: bool) {
        let index = 2 * usize::from(self.white_turn) + usize::from(!king_side);
        self.possible_castle[index] = false;
    }

    fn update_game_state(&mut self) {
        // this method should be called when creating board and when committing a move
        // if self.in_check(Some(true)) {

        // }

        // TODO, draw, stalemate
    }

    pub fn get_piece_and_color(&self, pos: Position) -> Tile {
        return self.board[pos.y][pos.x];
    }

    fn get_tile_ref(&mut self, pos: Position) -> &mut Tile {
        return &mut self.board[pos.y][pos.x];
    }

    pub fn legal_moves(&self) -> Vec<Move> {
        let mut moves = Vec::new();
        for (pos, (_, color)) in self {
            match color {
                Some(c) if c == self.white_turn => {
                    moves.append(&mut get_moves(self, pos));
                }
                _ => continue,
            }
        }

        return moves;
    }

    pub fn commit_verified_move(&mut self, move_: &Move) {
        // commit move
        // move should be verified before
        let capture: bool = if let (Piece::None, _) = self.get_piece_and_color(move_.dest) {
            false
        } else {
            true
        };
        let to_move = *self.get_tile_ref(move_.origin);
        *self.get_tile_ref(move_.dest) = to_move;
        *self.get_tile_ref(move_.origin) = EMPTY_TILE;

        let rook_rank: usize = if self.white_turn { 0 } else { 7 };
        match move_.special_move {
            SpecialMove::Promotion => {
                let (_, orignal_color) = self.get_piece_and_color(move_.dest);
                *self.get_tile_ref(move_.dest) = (move_.promote, orignal_color);
            }
            SpecialMove::EnPassant => {
                if let Ok(enemy_pos) = move_.dest.add_scalars((0, -1)) {
                    *self.get_tile_ref(enemy_pos) = EMPTY_TILE;
                }
            }
            SpecialMove::KingCastle => {
                if let Ok(new_rook_pos) = move_.dest.add_scalars((-1, 0)) {
                    let rook = *self.get_tile_ref(Position { x: 7, y: 0 });
                    *self.get_tile_ref(new_rook_pos) = rook;
                    *self.get_tile_ref(Position { x: 7, y: rook_rank }) = EMPTY_TILE;
                }
            }
            SpecialMove::QueenCastle => {
                if let Ok(new_rook_pos) = move_.dest.add_scalars((1, 0)) {
                    let rook = *self.get_tile_ref(Position { x: 0, y: 0 });
                    *self.get_tile_ref(new_rook_pos) = rook;
                    *self.get_tile_ref(Position { x: 0, y: rook_rank }) = EMPTY_TILE;
                }
            }
            SpecialMove::None => (),
        }

        // update board state
        let (moved_piece, _) = self.get_piece_and_color(move_.dest);
        let mut pawn_moved = false;
        self.en_passant = None;
        match moved_piece {
            Piece::Pawn => {
                pawn_moved = true;
                // double move
                if move_.origin.x == move_.dest.x
                    && (move_.origin.y as i32 - move_.dest.y as i32).abs() == 2
                {
                    self.en_passant = Some(move_.dest);
                }
            }
            Piece::King => {
                self.update_castle_right(true);
                self.update_castle_right(false);
            }

            Piece::Rook => {
                if move_.origin == (Position { x: 0, y: rook_rank }) {
                    self.update_castle_right(false);
                } else if move_.origin == (Position { x: 7, y: rook_rank }) {
                    self.update_castle_right(true);
                }
            }
            _ => (),
        }

        self.fullmove_count += 1;
        if !capture && !pawn_moved {
            self.halfmove_count += 1
        }
        self.white_turn = !self.white_turn;

        self.update_game_state();
    }

    pub fn in_check(&self, king_color: Color) -> bool {
        // maybe keep king pos in the structure
        // if no color is provided use current player turn
        let col = if let Some(c) = king_color {
            c
        } else {
            self.white_turn
        };
        let mut king: Option<Position> = None;
        for (pos, (piece, color)) in self {
            let Some(is_white) = color else {
                continue;
            };
            if is_white == col && piece == Piece::King {
                king = Some(pos);
            }
        }
        let Some(king_pos) = king else { return false };

        for (pos, (piece, color)) in self {
            let Some(is_white) = color else {
                continue;
            };
            // our piece or king 
            if piece == Piece::King {
                continue;
            }
            // enemy piece
            if is_white != col {
                for move_ in get_unchecked_moves(self, pos) {
                    if move_.dest == king_pos {
                        return true;
                    }
                }
            }
        }
        return false;
    }

    fn make_input_move(&mut self, origin: Position, dest: Position, promote: Piece) -> bool {
        let moves = get_moves(self, origin);
        for move_ in moves {
            if move_.origin == origin && move_.dest == dest && move_.promote == promote {
                self.commit_verified_move(&move_);
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

    pub fn get_all_moves(&self, col: Color) -> Vec<Move> {
        // returns all legal moves that can be made by player
        // if no color is provided use current player turn
        let color = if let Some(c) = col {
            c
        } else {
            self.white_turn
        };
        let mut result = Vec::new();

        for (pos, (_, tile_color)) in self {
            match tile_color {
                Some(col) if col == color => result.append(&mut get_moves(self, pos)),
                _ => (),
            }
        }
        return result;
    }
}
