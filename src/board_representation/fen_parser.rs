use super::r#const::EMPTY_BIT_B;
use crate::board_representation::bitboard::Bitboard;
use crate::board_representation::board::{Board, Turn, BLACK, PLAYER_COUNT, WHITE};
use crate::board_representation::castle_rights::CastleRights;
use crate::board_representation::piece::{Piece, PIECE_COUNT};
use crate::board_representation::position::Position;

pub const START_POS_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

impl Board {
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
            .parse::<u32>()
            .map_err(|_| "Invalid halfmove clock")?;

        // fullmove
        let full_move = parts[5].parse::<u32>().map_err(|_| "Invalid fullmove")?;

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
