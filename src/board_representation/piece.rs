
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Piece {
    Pawn = 0,
    Rook = 1,
    Knight = 2,
    Bishop = 3,
    King = 4,
    Queen = 5,
    None,
}

pub const PIECE_COUNT: usize = 6;

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

impl From<usize> for Piece {
    fn from(value: usize) -> Self {
        match value {
            0 => Piece::Pawn,
            1 => Piece::Rook,
            2 => Piece::Knight,
            3 => Piece::Bishop,
            4 => Piece::King,
            5 => Piece::Queen,
            _ => Piece::None,
        }
    }
    
}