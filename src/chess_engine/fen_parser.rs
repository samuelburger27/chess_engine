//! [Forsyth–Edwards Notation][fen] parsing for [`Board`].
//!
//! A FEN string has six space-separated fields: piece placement (rank 8 first),
//! side to move, castling rights, en-passant target, half-move clock, and
//! full-move number. This parser requires all six fields to be present.
//!
//! [fen]: https://www.chessprogramming.org/Forsyth-Edwards_Notation

use super::r#const::EMPTY_BIT_B;
use crate::chess_engine::board::{Board, Turn, BLACK, PLAYER_COUNT, WHITE};
use crate::chess_engine::castle_rights::CastleRights;
use crate::chess_engine::piece::{Piece, PIECE_COUNT};
use crate::chess_engine::position::Position;

/// FEN of the standard starting position.
pub const START_POS_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

impl Board {
    /// Parses a six-field FEN string into a [`Board`].
    ///
    /// # Errors
    ///
    /// Returns `Err` with a human-readable message if the string does not have
    /// exactly six fields, has more than eight ranks, or contains an invalid
    /// piece letter, side-to-move, castling character, en-passant square, or
    /// move counter.
    ///
    /// ```
    /// use chess_engine::chess_engine::board::{Board, BLACK, WHITE};
    /// use chess_engine::chess_engine::piece::Piece;
    /// use chess_engine::chess_engine::position::Position;
    ///
    /// // position after 1. e4
    /// let board =
    ///     Board::from_fen("rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1")
    ///         .unwrap();
    /// assert_eq!(board.turn, BLACK);
    /// assert_eq!(board.get_piece_at(Position::new(28)), Some((Piece::Pawn, WHITE))); // e4
    ///
    /// // every field must be present
    /// assert!(Board::from_fen("garbage").is_err());
    /// ```
    pub fn from_fen(string: &str) -> Result<Board, String> {
        let mut piece_boards = [EMPTY_BIT_B; PIECE_COUNT * PLAYER_COUNT];

        let parts: Vec<&str> = string.trim().split_whitespace().collect();
        if parts.len() != 6 {
            return Err("FEN must have 6 fields".to_string());
        }

        // set board positions
        for (rank_index, rank) in parts[0].split('/').enumerate() {
            let mut x: usize = 0;
            if rank_index > 7 {
                return Err("Board must have 8 ranks".to_string());
            }
            for ch in rank.chars() {
                if let Some(blank_tiles) = ch.to_digit(10) {
                    x += blank_tiles as usize;
                } else {
                    let turn: Turn = if ch.is_lowercase() { BLACK } else { WHITE };
                    let Ok(piece) = Piece::try_from(ch.to_string().as_str()) else {
                        return Err("Invalid piece encoding".to_string());
                    };
                    let position = Position::from_file_and_rank(x, 7 - rank_index);

                    let piece_index = Board::get_bb_index(piece, turn);
                    piece_boards[piece_index].set_square(position.as_usize());
                    x += 1;
                }
            }
        }
        // turn
        let turn = match parts[1] {
            "w" => WHITE,
            "b" => BLACK,
            _ => return Err("Invalid side to move".to_string()),
        };

        // castle rights
        let mut castle_rights = [false, false, false, false];
        for ch in parts[2].chars() {
            match ch {
                'K' => castle_rights[0] = true,
                'Q' => castle_rights[1] = true,
                'k' => castle_rights[2] = true,
                'q' => castle_rights[3] = true,
                '-' => (),
                _ => return Err(format!("Invalid castle character: {}", ch)),
            }
        }
        let castle = CastleRights::make(
            castle_rights[0],
            castle_rights[1],
            castle_rights[2],
            castle_rights[3],
        );
        // en passant
        let mut en_passant = EMPTY_BIT_B;
        if parts[3] != "-" {
            match Position::try_from(parts[3]) {
                Ok(pos) => en_passant.set_square(pos.as_usize()),
                _ => return Err("Invalid en passant position".to_string()),
            }
        }
        // halfmove count
        let halfmove_count = parts[4]
            .parse::<u8>()
            .map_err(|_| "Invalid halfmove clock")?;

        // fullmove
        let full_move = parts[5].parse::<u16>().map_err(|_| "Invalid fullmove")?;

        Ok(Board::new_from_bitboards(
            piece_boards,
            turn,
            en_passant,
            halfmove_count,
            full_move,
            castle,
        ))
    }
}

#[cfg(test)]
mod tests {
    use crate::chess_engine::board::{Board, BLACK, WHITE};
    use crate::chess_engine::piece::Piece;
    use crate::chess_engine::position::Position;

    #[test]
    fn start_pos_fen_places_pieces() {
        let board = Board::new_start_pos().unwrap();
        assert_eq!(board.turn, WHITE);
        assert_eq!(board.fullmove_count, 1);
        // back-rank corners hold rooks of the right colour
        assert_eq!(
            board.get_piece_at(Position::new(0)),
            Some((Piece::Rook, WHITE))
        ); // a1
        assert_eq!(
            board.get_piece_at(Position::new(63)),
            Some((Piece::Rook, BLACK))
        ); // h8
           // the centre is empty
        assert_eq!(board.get_piece_at(Position::new(28)), None); // e4
    }

    #[test]
    fn parses_state_fields() {
        let board =
            Board::from_fen("rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1").unwrap();
        assert_eq!(board.turn, BLACK);
        assert_eq!(board.halfmove_count, 0);
        // e3 (square 20) is the recorded en-passant target
        assert!(board.en_passant.is_square_set(20));
    }

    #[test]
    fn rejects_malformed_fen() {
        assert!(Board::from_fen("garbage").is_err());
        assert!(Board::from_fen("8/8/8/8/8/8/8/8 x - - 0 1").is_err());
    }
}
