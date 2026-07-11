//! The [`Piece`] enum and its conversions to/from notation and indices.
//!
//! The six real piece types are given explicit discriminants `0..6`. That
//! ordering is load-bearing: [`Board`](super::board::Board) stores one bitboard
//! per `(piece, colour)` pair and indexes it as `piece as usize + colour * 6`,
//! so white pieces occupy slots `0..6` and black pieces `6..12`. The extra
//! [`Piece::None`] variant represents "no piece" (e.g. an empty square) and is
//! not part of that 0–5 range.

/// A chess piece type. The discriminants `0..6` double as bitboard indices
/// (see the [module documentation](self)); [`None`](Piece::None) means "no piece".
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Piece {
    /// Pawn (index 0).
    Pawn = 0,
    /// Rook (index 1).
    Rook = 1,
    /// Knight (index 2).
    Knight = 2,
    /// Bishop (index 3).
    Bishop = 3,
    /// King (index 4).
    King = 4,
    /// Queen (index 5).
    Queen = 5,
    /// Absence of a piece (e.g. an empty square); not a bitboard index.
    None,
}

/// The number of distinct piece types (pawn through queen), i.e. the number of
/// bitboards per colour.
pub const PIECE_COUNT: usize = 6;

impl Piece {
    /// Returns a relative material value for the piece in arbitrary units
    /// ([`Piece::None`] is `0`). This is a simple piece-worth scale separate from
    /// the search's positional [evaluation](super::engine).
    ///
    /// ```
    /// use sabertooth::chess_engine::piece::Piece;
    /// assert_eq!(Piece::Queen.get_piece_value(), 900);
    /// assert_eq!(Piece::None.get_piece_value(), 0);
    /// ```
    #[must_use]
    pub const fn get_piece_value(&self) -> i32 {
        match self {
            Self::Pawn => 200,
            Self::Rook => 500,
            Self::Knight => 320,
            Self::Bishop => 330,
            Self::King => 2000,
            Self::Queen => 900,
            Self::None => 0,
        }
    }
}

impl Piece {
    /// Returns the lowercase FEN/algebraic letter for the piece (`"p"`, `"r"`,
    /// `"n"`, `"b"`, `"k"`, `"q"`), or `"-"` for [`Piece::None`].
    ///
    /// ```
    /// use sabertooth::chess_engine::piece::Piece;
    /// assert_eq!(Piece::Knight.to_notation(), "n");
    /// ```
    #[must_use]
    pub fn to_notation(&self) -> String {
        match self {
            Self::Pawn => "p",
            Self::Rook => "r",
            Self::Knight => "n",
            Self::Bishop => "b",
            Self::King => "k",
            Self::Queen => "q",
            Self::None => "-",
        }
        .to_string()
    }
}

/// Parses a single piece letter. Both cases are accepted (`"N"` and `"n"` are
/// both a knight); `" "` and `"-"` map to [`Piece::None`]. Returns `Err(())`
/// for anything else.
///
/// ```
/// use sabertooth::chess_engine::piece::Piece;
/// assert_eq!(Piece::try_from("q"), Ok(Piece::Queen));
/// assert_eq!(Piece::try_from("-"), Ok(Piece::None));
/// assert!(Piece::try_from("x").is_err());
/// ```
impl TryFrom<&str> for Piece {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "P" | "p" => Ok(Self::Pawn),
            "R" | "r" => Ok(Self::Rook),
            "N" | "n" => Ok(Self::Knight),
            "B" | "b" => Ok(Self::Bishop),
            "Q" | "q" => Ok(Self::Queen),
            "K" | "k" => Ok(Self::King),
            " " | "-" => Ok(Self::None),
            _ => Err(()),
        }
    }
}

/// Maps a bitboard index `0..6` back to its piece type; any other value yields
/// [`Piece::None`].
///
/// ```
/// use sabertooth::chess_engine::piece::Piece;
/// assert_eq!(Piece::from(2), Piece::Knight);
/// assert_eq!(Piece::from(99), Piece::None);
/// ```
impl From<usize> for Piece {
    fn from(value: usize) -> Self {
        match value {
            0 => Self::Pawn,
            1 => Self::Rook,
            2 => Self::Knight,
            3 => Self::Bishop,
            4 => Self::King,
            5 => Self::Queen,
            _ => Self::None,
        }
    }
}
