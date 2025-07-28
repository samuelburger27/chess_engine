use crate::board_representation::{bitboard::Bitboard, r#const::EMPTY_BIT_B};

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Position(usize);

impl Position {
    pub const fn new(index: usize) -> Position {
        assert!(index < 64, "Index must be between 0 and 63");
        Self(index)
    }
    
    pub const fn from_file_and_rank(file: usize, rank: usize) -> Position {
        assert!(file < 8 && rank < 8, "File and rank must be between 0 and 7");
        Self(rank * 8 + file)
    }

    pub fn get_file_and_rank(&self) -> (usize, usize) {
        let file = self.0 % 8;
        let rank = self.0 / 8;
        return (file, rank);
    }

    pub const fn try_offset(&self, d_file: i8, d_rank: i8) -> Option<Self> {
        let file = (self.0 % 8) as i8 + d_file;
        let rank = (self.0 / 8) as i8 + d_rank;
        if file >= 0 && rank >= 0 && file < 8 && rank < 8 {
            return Some(Position::from_file_and_rank(file as usize, rank as usize));
        }
        None
    }

    pub const fn as_usize(&self) -> usize {
        return self.0;
    }

    pub const fn bitboard(&self) -> Bitboard {
        let mut board = EMPTY_BIT_B;
        board.set_square(self.as_usize());
        board
    }

    pub fn algebraic_notation(&self) -> String {
        let (file, rank) = self.get_file_and_rank();
        let file_str = match file {
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

        return file_str + &(rank + 1).to_string();
    }
}

impl TryFrom<&str> for Position {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if value.len() != 2 {
            return Err(());
        }
        let file: usize = match value.chars().nth(0) {
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
        return Ok(Position::from_file_and_rank(file, (rank - 1) as usize));
    }
}

impl From<Position> for usize {
    fn from(pos: Position) -> Self {
        pos.0
    }
}
