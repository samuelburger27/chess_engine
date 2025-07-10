//mod board;
use crate::board::Board;
use crate::board::Position;
use crate::board::Piece;

enum SpecialMove {
    None,
    Promotion,
    KingCastle,
    QueenCastle,
    EnPassant,
}

struct Move {
    // maybe in future remake into a 16 bit representation
    origin: Position,
    dest: Position,
    promote: Piece,
    special_move: SpecialMove,
}
impl Move {

    fn new_implicit(origin: Position, dest: Position) -> Move {
        return Move { origin:origin, dest: dest, promote: Piece::Queen, special_move: SpecialMove::None }
    }

    fn new(origin: Position, dest: Position, promote: Piece, special_move: SpecialMove) -> Move {
        return Move { origin:origin, dest: dest, promote: promote, special_move: special_move }
    }
    
}

const ORTHOGONAL: [(i32,i32);4] = [(-1,0), (1,0), (0, -1), (0, 1)];
const DIAGONAL: [(i32,i32);4] = [(-1,-1), (-1,1), (1, -1), (1, 1)];
const KNIGHT: [(i32, i32); 8] = [(2, 1), (-2, 1), (1, 2), (1, -2),
                                (2, -1), (-2, -1), (-1, 2), (-1, -2)];


pub fn get_moves(board: &Board, origin: Position) -> Vec<Move>{
    let (piece, Some(color)) = board.get_piece_and_color(origin) else {
        return Vec::new();
    };
    match piece {
        Piece::Pawn => pawn_moves(board, origin, color),
        Piece::Rook => rook_moves(board, origin, color),
        Piece::Knight => knight_moves(board, origin, color),
        Piece::Bishop => bishop_moves(board, origin, color),
        Piece::Queen => queen_moves(board, origin, color),
        Piece::King => king_moves(board, origin, color),
        Piece::None =>  Vec::new(),
    }
}


fn pawn_moves(board: &Board, origin: Position, piece_color: bool) -> Vec<Move>{
    let mut result: Vec<Move> = Vec::new();
    let forward_dir = if piece_color {1} else {-1};
    
    if let Ok(front) = origin.add_scalars((0, forward_dir)) {
        if let (_, None) = board.get_piece_and_color(front) {
            result.push(Move::new_implicit(origin, front));
            // double move
            let start_y: usize = if piece_color {1} else {6};
            if start_y == origin.y {
                if let Ok(d_front) = front.add_scalars((0, forward_dir)) {
                    if let (_, None) = board.get_piece_and_color(front) {
                        result.push(Move::new_implicit(origin, d_front));
                    }   
                }
            }
        }
    }
    // enemy tile on diagonals 
    for direction in [-1, 1] {
        if let Ok(diag) = origin.add_scalars((direction, forward_dir)) {
            match board.get_piece_and_color(diag) {
                (_, Some(color)) if color != piece_color => 
                result.push(Move::new_implicit(origin, diag)),
                _ => ()
            }
        }
    }
    
    // TODO promotions, en passant
    
    return result;
    
}


fn sliding_piece_moves(board: &Board, origin: Position, piece_color: bool, directions: &[(i32, i32)]) -> Vec<Move>{
    let mut result: Vec<Move> = Vec::new();
    for (x_dir, y_dir) in directions {
        for i in 1..8 {
            let Ok(slide) = origin.add_scalars((i * x_dir, i * y_dir)) else {
                break;
            };
            match board.get_piece_and_color(slide) {
                // empty piece
                (_, None) => result.push(Move::new_implicit(origin, slide)),
                // enemy piece
                (_, Some(color)) if color != piece_color => {
                    result.push(Move::new_implicit(origin, slide));
                    break;
                },
                // our piece
                _ => break,
            }
        }
    }
    return result;
}


fn rook_moves(board: &Board, origin: Position, piece_color: bool) -> Vec<Move>{
    return sliding_piece_moves(board, origin, piece_color, &ORTHOGONAL);
}

fn bishop_moves(board: &Board, origin: Position, piece_color: bool) -> Vec<Move>{
    return sliding_piece_moves(board, origin, piece_color, &DIAGONAL);
}

fn queen_moves(board: &Board, origin: Position, piece_color: bool) -> Vec<Move> {
    let mut moves = bishop_moves(board, origin, piece_color);
    moves.append(&mut rook_moves(board, origin, piece_color));
    return moves;
}

fn specified_positions_moves(board: &Board, origin: Position, piece_color: bool, positions: &[(i32, i32)]) -> Vec<Move> {
    let mut result: Vec<Move> = Vec::new();
    for pos in positions {
        let Ok(new_position) = origin.add_scalars(*pos) else {
            continue;
        };
        match board.get_piece_and_color(new_position) {
            // our piece
            (_, Some(color)) if color == piece_color => (),
            _ => result.push(Move::new_implicit(origin, new_position)),
        }
    }
    return result
}


fn knight_moves(board: &Board, origin: Position, piece_color: bool) -> Vec<Move>{
    return sliding_piece_moves(board, origin, piece_color, &KNIGHT)
}

fn king_moves(board: &Board, origin: Position, piece_color: bool) -> Vec<Move>{
    let mut moves = specified_positions_moves(board, origin, piece_color, &DIAGONAL);
    moves.append(&mut &mut specified_positions_moves(board, origin, piece_color, &ORTHOGONAL));
    return moves;
    // TODO checks castle
}
