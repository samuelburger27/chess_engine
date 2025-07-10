use std::ops::Add;

#[derive(Clone, Copy)]
pub struct Position {
    pub x: usize,
    pub y: usize,
}

impl Position {
    pub fn from_bit_index(index: usize) -> Position {

        return Position { x: index % 8, y: index / 8 };
    }

    pub fn get_bit_index(&self) -> usize {
        return 8 * self.y + self.x;
    }

    pub fn add_scalars(& self, (add_x, add_y): (i32, i32)) -> Result<Position, ()>{
        let casted_x = i32::try_from(self.x).unwrap();
        let casted_y = i32::try_from(self.y).unwrap();

        let Ok(x) = usize::try_from(casted_x + add_x) else {
            return Err(());
        };

        let Ok(y) = usize::try_from(casted_y + add_y) else {
            return Err(());
        };
        if x >= 8 || y >= 8 {
            return Err(());
        }
        
        return Ok(Position {x: x, y: y});
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
            _ => "-"            
        }.to_string();

        return file + &self.y.to_string();
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
        if (rank > 8) {
            return Err(());
        }
        return Ok(Position { x: x, y: (rank - 1) as usize })
    
    }
}

#[derive(Clone, Copy)]
pub enum Piece {
    Pawn,
    Rook,
    Knight,
    Bishop,
    King,
    Queen,
    None,
}

//const NOT_A_FILE: u64 = 0xfefefefefefefefe;
//const NOT_H_FILE: u64 = 0x7f7f7f7f7f7f7f7f;

type Color = Option<bool>;

pub struct Board {
    pub board: [[(Piece, Color); 8]; 8],
    // white_bitboards: [u64; 6],
    // black_bitboards: [u64; 6],
    pub white_turn: bool,
    // if any pawn can be captured by en passant(just made double move) it will be recorded in this variable
    pub en_passant: Option<Position>,

    pub halfmove_count: u32,
    // [white_king_side, white_queen_side, black_king_side, black_queen_side]
    pub possible_castle: [bool; 4],
}

impl Board {
    
    // fn is_position_in_bitboard(pos: Position, bitboard: u64) -> bool {
    //     let mask = 1u64 << pos.get_bit_index();
    //     return (bitboard & mask) > 0;
    // }

    // fn match_index_to_piece(index: usize) -> Piece {
    //     match index {
    //         0 => Piece::Pawn,
    //         1 => Piece::Rook,
    //         2 => Piece::Knight,
    //         3 => Piece::Bishop,
    //         4 => Piece::King,
    //         5 => Piece::Queen,
    //         _ => Piece::None
    //     }
    // }

    fn can_castle(&self, king_side: bool) -> bool {
        let index = 2 * usize::from(self.white_turn) + usize::from(!king_side);
        return self.possible_castle[index];
    }

    pub fn get_piece_and_color(&self, pos: Position) -> (Piece, Color) {
        return self.board[pos.y][pos.x];
    }
}
