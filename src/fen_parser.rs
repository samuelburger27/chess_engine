use crate::board::Board;
use crate::board::Position;
use crate::board::Piece;

const EMPTY: [[(Piece, Option<bool>); 8]; 8] = [
    [(Piece::None, None),(Piece::None, None),(Piece::None, None),(Piece::None, None),(Piece::None, None),(Piece::None, None),(Piece::None, None),(Piece::None, None)],
    [(Piece::None, None),(Piece::None, None),(Piece::None, None),(Piece::None, None),(Piece::None, None),(Piece::None, None),(Piece::None, None),(Piece::None, None)],
    [(Piece::None, None),(Piece::None, None),(Piece::None, None),(Piece::None, None),(Piece::None, None),(Piece::None, None),(Piece::None, None),(Piece::None, None)],
    [(Piece::None, None),(Piece::None, None),(Piece::None, None),(Piece::None, None),(Piece::None, None),(Piece::None, None),(Piece::None, None),(Piece::None, None)],
    [(Piece::None, None),(Piece::None, None),(Piece::None, None),(Piece::None, None),(Piece::None, None),(Piece::None, None),(Piece::None, None),(Piece::None, None)],
    [(Piece::None, None),(Piece::None, None),(Piece::None, None),(Piece::None, None),(Piece::None, None),(Piece::None, None),(Piece::None, None),(Piece::None, None)],
    [(Piece::None, None),(Piece::None, None),(Piece::None, None),(Piece::None, None),(Piece::None, None),(Piece::None, None),(Piece::None, None),(Piece::None, None)],
    [(Piece::None, None),(Piece::None, None),(Piece::None, None),(Piece::None, None),(Piece::None, None),(Piece::None, None),(Piece::None, None),(Piece::None, None)],
    ];

pub fn starting_pos_fen() -> String {
    return "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1".to_string();
}

pub fn parse_fen(string: &str) -> Result<Board, String> {
    let mut board = EMPTY;
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
            }
            else {
                let color = if ch.is_ascii_lowercase() {Some(false)} else {Some(true)};
                let Ok(piece) = Piece::try_from(ch.to_string().as_str()) else {return Err("Invalid piece encoding".to_string())};
                board[7-rank_index][x] = (piece, color);
                x += 1;
                }
        }
    }
    // turn
    let turn = match parts[1] {
        "w" => true,
        "b" => false,
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
    // en passant
    let en_passant = 
    if parts[3] != "-" {
        match Position::try_from(parts[3]) {
            Ok(pos) => Some(pos),
            _ => return Err("Invalid en passant position".to_string())
        }
    } else {
        None
    };
    // halfmove count
    let halfmove_count = parts[4].parse::<u32>().map_err(|_| "Invalid halfmove clock")?;

    // fullmove
    let full_move = parts[5].parse::<u32>().map_err(|_| "Invalid fullmove")?;
    
    Ok(Board::new(board, turn, en_passant, halfmove_count, full_move, castle_rights))
}