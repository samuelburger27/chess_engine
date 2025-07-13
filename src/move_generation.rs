use crate::board::Board;
use crate::board::Piece;
use crate::board::Position;

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum SpecialMove {
    None,
    Promotion,
    KingCastle,
    QueenCastle,
    EnPassant,
}

#[derive(PartialEq, Clone, Debug)]
pub struct Move {
    // maybe in future remake into a 16 bit representation
    pub origin: Position,
    pub dest: Position,
    pub promote: Piece,
    pub special_move: SpecialMove,
}
impl Move {
    fn new_default(origin: Position, dest: Position) -> Move {
        return Move {
            origin: origin,
            dest: dest,
            promote: Piece::Queen,
            special_move: SpecialMove::None,
        };
    }

    fn new_promote(origin: Position, dest: Position) -> [Move; 4] {
        [
            Move {
                origin: origin,
                dest: dest,
                promote: Piece::Queen,
                special_move: SpecialMove::Promotion,
            },
            Move {
                origin: origin,
                dest: dest,
                promote: Piece::Knight,
                special_move: SpecialMove::Promotion,
            },
            Move {
                origin: origin,
                dest: dest,
                promote: Piece::Bishop,
                special_move: SpecialMove::Promotion,
            },
            Move {
                origin: origin,
                dest: dest,
                promote: Piece::Rook,
                special_move: SpecialMove::Promotion,
            },
        ]
    }

    fn new(origin: Position, dest: Position, promote: Piece, special_move: SpecialMove) -> Move {
        return Move {
            origin: origin,
            dest: dest,
            promote: promote,
            special_move: special_move,
        };
    }
}

impl ToString for Move {
    fn to_string(&self) -> String {
        let mut result = self.origin.algebraic_notation() + &self.dest.algebraic_notation();
        if self.special_move == SpecialMove::Promotion {
            result += &self.promote.to_notation();
        }
        return result;
    }
}

const ORTHOGONAL: [(i32, i32); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];
const DIAGONAL: [(i32, i32); 4] = [(-1, -1), (-1, 1), (1, -1), (1, 1)];
const KNIGHT: [(i32, i32); 8] = [
    (2, 1),
    (-2, 1),
    (1, 2),
    (1, -2),
    (2, -1),
    (-2, -1),
    (-1, 2),
    (-1, -2),
];

pub fn get_moves(board: &Board, origin: Position) -> Vec<Move> {
    get_unchecked_moves(board, origin)
        // filter out check moves
        .into_iter()
        .filter(|m| !would_check(board, m))
        .collect()
}

pub fn get_unchecked_moves(board: &Board, origin: Position) -> Vec<Move> {
    // get all moves but dont verify that king is under attack,
    //used in check_detection to not get infinite loop

    // TODO rewrite to not assume board turn is the color of the piece
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
        Piece::None => Vec::new(),
    }
}

fn pawn_moves(board: &Board, origin: Position, piece_color: bool) -> Vec<Move> {
    let mut result: Vec<Move> = Vec::new();
    let forward_dir = if piece_color { 1 } else { -1 };

    if let Ok(front) = origin.add_scalars((0, forward_dir)) {
        if let (_, None) = board.get_piece_and_color(front) {
            // double move
            let start_y: usize = if piece_color { 1 } else { 6 };
            if start_y == origin.y {
                if let Ok(d_front) = front.add_scalars((0, forward_dir)) {
                    if let (_, None) = board.get_piece_and_color(d_front) {
                        result.push(Move::new_default(origin, d_front));
                    }
                }
            }
            // promotion
            if front.y == 0 || front.y == 7 {
                result.extend_from_slice(&Move::new_promote(origin, front))
            } else {
                result.push(Move::new_default(origin, front));
            }
        }
    }
    for direction in [-1, 1] {
        let Ok(diag) = origin.add_scalars((direction, forward_dir)) else {
            continue;
        };
        match board.get_piece_and_color(diag) {
            // enemy tile on diagonals
            (_, Some(color)) if color != piece_color => {
                result.push(Move::new_default(origin, diag))
            }
            _ => (),
        }

        // en passant
        if let Some(enemy_double_move) = board.en_passant {
            if let Ok(neighbor) = origin.add_scalars((direction, 0)) {
                if enemy_double_move == neighbor {
                    result.push(Move::new(
                        origin,
                        diag,
                        Piece::Queen,
                        SpecialMove::EnPassant,
                    ));
                }
            }
        }
    }
    return result;
}

fn sliding_piece_moves(
    board: &Board,
    origin: Position,
    piece_color: bool,
    directions: &[(i32, i32)],
) -> Vec<Move> {
    let mut result: Vec<Move> = Vec::new();
    for (x_dir, y_dir) in directions {
        for i in 1..8 {
            let Ok(slide) = origin.add_scalars((i * x_dir, i * y_dir)) else {
                break;
            };
            match board.get_piece_and_color(slide) {
                // empty piece
                (_, None) => result.push(Move::new_default(origin, slide)),
                // enemy piece
                (_, Some(color)) if color != piece_color => {
                    result.push(Move::new_default(origin, slide));
                    break;
                }
                // our piece
                _ => break,
            }
        }
    }
    return result;
}

fn rook_moves(board: &Board, origin: Position, piece_color: bool) -> Vec<Move> {
    return sliding_piece_moves(board, origin, piece_color, &ORTHOGONAL);
}

fn bishop_moves(board: &Board, origin: Position, piece_color: bool) -> Vec<Move> {
    return sliding_piece_moves(board, origin, piece_color, &DIAGONAL);
}

fn queen_moves(board: &Board, origin: Position, piece_color: bool) -> Vec<Move> {
    let mut moves = bishop_moves(board, origin, piece_color);
    moves.append(&mut rook_moves(board, origin, piece_color));
    return moves;
}

fn specified_positions_moves(
    board: &Board,
    origin: Position,
    piece_color: bool,
    positions: &[(i32, i32)],
) -> Vec<Move> {
    let mut result: Vec<Move> = Vec::new();
    for pos in positions {
        let Ok(new_position) = origin.add_scalars(*pos) else {
            continue;
        };
        match board.get_piece_and_color(new_position) {
            // our piece
            (_, Some(color)) if color == piece_color => (),
            (_, Some(_)) => result.push(Move::new_default(origin, new_position)),
            _ => result.push(Move::new_default(origin, new_position)),
        }
    }
    return result;
}

fn knight_moves(board: &Board, origin: Position, piece_color: bool) -> Vec<Move> {
    return specified_positions_moves(board, origin, piece_color, &KNIGHT);
}

fn would_check(board: &Board, move_: &Move) -> bool {
    let mut board_cpy = *board;

    board_cpy.commit_verified_move(move_);
    return board_cpy.in_check(Some(board.white_turn));
}

fn king_moves(board: &Board, origin: Position, piece_color: bool) -> Vec<Move> {
    let mut moves = specified_positions_moves(board, origin, piece_color, &DIAGONAL);
    moves.append(&mut &mut specified_positions_moves(
        board,
        origin,
        piece_color,
        &ORTHOGONAL,
    ));

    // castle
    if !board.in_check(None) {
        if board.can_castle(false) {
            let mut empty_space = true;
            for i in 1..4 {
                if let (_, Some(_)) = board.get_piece_and_color(Position {
                    x: i as usize,
                    y: origin.y,
                }) {
                    empty_space = false;
                }
            }
            // square king moves over is not under attack
            if empty_space
                && !would_check(
                    board,
                    &Move {
                        origin: origin,
                        dest: Position { x: 3, y: origin.y },
                        promote: Piece::Queen,
                        special_move: SpecialMove::None,
                    },
                )
            {
                moves.push(Move::new(
                    origin,
                    Position { x: 2, y: origin.y },
                    Piece::Queen,
                    SpecialMove::QueenCastle,
                ));
            }
        }
        if board.can_castle(true) {
            let mut empty_space = true;
            for i in 5..7 {
                if let (_, Some(_)) = board.get_piece_and_color(Position {
                    x: i as usize,
                    y: origin.y,
                }) {
                    empty_space = false;
                }
            }
            // the square king moves over is not under attack
            if empty_space
                && !would_check(
                    board,
                    &Move {
                        origin: origin,
                        dest: Position { x: 5, y: origin.y },
                        promote: Piece::Queen,
                        special_move: SpecialMove::None,
                    },
                )
            {
                moves.push(Move::new(
                    origin,
                    Position { x: 6, y: origin.y },
                    Piece::Queen,
                    SpecialMove::KingCastle,
                ));
            }
        }
    }
    return moves;
}
