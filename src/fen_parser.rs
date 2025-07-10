use std::u16;

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

fn parse_fen(string: &str) -> Result<Board, ()> {
    let mut board = Board { board: EMPTY, white_turn: true, en_passant: None, halfmove_count: 0, possible_castle: [false, false, false, false]};    

    let mut iter = string.chars();
    let mut x: usize = 0;
    let mut y: usize = 7;
    // set board positions
    while let Some(ch) = iter.next() {
        if ch.is_whitespace() {
            break;
        }
        if ch == '/' {
            x = 0;
            y-=1;
        }
        if let Some(digit) = ch.to_digit(10) {
            x += digit as usize;
        }
        else {
            let color = if ch.is_ascii_lowercase() {Some(false)} else {Some(true)};
            let piece = match ch.to_ascii_lowercase() {
                'p' => Piece::Pawn,
                'n' => Piece::Knight,
                'b' => Piece::Bishop,
                'q' => Piece::Queen,
                'k' => Piece::King,
                _ => return Err(()),
            };
            board.board[y][x] = (piece, color);
            x += 1;
        }
    }
    // turn 
    if let Some(ch) = iter.next() {
        match ch {
            'w' => board.white_turn = true,
            'b' => board.white_turn = false,
            _ => return Err(())
        }
    }
    iter.next();

    // castle rights
    while let Some(ch) = iter.next() {
        if ch.is_whitespace() {
            break;
        }
        match ch {
            'K' => board.possible_castle[0] = true,
            'Q' => board.possible_castle[1] = true,
            'k' => board.possible_castle[2] = true,
            'q' => board.possible_castle[3] = true,
            _ => return Err(())
        }
    }
    // en passant
    if let Some(ch) = iter.next() {
        if ch != '-' {
            let mut notation: String = ch.into();
            let Some(ch) = iter.next() else {
                return Err(())
            };
            notation.push(ch);
            board.en_passant = match Position::try_from(notation.as_str()) {
                Ok(pos) => Some(pos),
                _ => return Err(()),  
            };
        } 
    }

    iter.next();

    // halfmoves count

    while let Some(ch) = iter.next() {
        
    }

    return Ok(board);
}