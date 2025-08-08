use crate::board_representation::{
    bitboard::Bitboard,
    board::{Board, Turn, BLACK, WHITE},
    game_state::StateDelta,
    piece::{Piece, PIECE_COUNT},
    position::Position,
    r#const::{
        B_KING_ROOK_START, B_KING_SIDE_BISHOP_START, B_QUEEN_ROOK_START, B_QUEEN_START, NORTH,
        SOUTH, W_KING_ROOK_START, W_KING_SIDE_BISHOP_START, W_QUEEN_ROOK_START, W_QUEEN_START,
    },
    r#move::{Move, SpecialMove},
};

type BitboardMutIter<'a> = std::iter::Take<std::iter::Skip<std::slice::IterMut<'a, Bitboard>>>;

impl Board {
    fn get_player_bit_boards_iter(&mut self, turn: Turn) -> BitboardMutIter {
        self.piece_boards
            .iter_mut()
            .skip(turn as usize * PIECE_COUNT)
            .take(PIECE_COUNT)
    }

    pub fn commit_verified_move(&mut self, move_: Move) {
        // commit move
        // move should be verified before
        let (origin, destination) = move_.get_org_and_dest();
        let mut captured_piece = if !self.empty_tiles.is_square_set(destination.into()) {
            Some(self.get_piece_type_containing_position(destination))
        } else {
            None
        };

        let moving_piece = self.get_piece_type_containing_position(origin);

        let b_b_index = Board::get_bb_index(moving_piece, self.turn);

        self.piece_boards[b_b_index].clear_square(origin.as_usize());
        self.piece_boards[b_b_index].set_square(destination.as_usize());

        for enemy_board in self.get_player_bit_boards_iter(!self.turn) {
            enemy_board.clear_square(destination.into());
        }

        match move_.get_special_move() {
            SpecialMove::Promotion => {
                self.piece_boards[b_b_index].clear_square(destination.as_usize());
                let promote_to = move_.get_promotion();
                let promote_bb_index = Board::get_bb_index(promote_to, self.turn);
                self.piece_boards[promote_bb_index].set_square(destination.into());
            }

            SpecialMove::Castle => {
                let (dest_file, _) = destination.get_file_and_rank();
                let rook_bb_i = Board::get_bb_index(Piece::Rook, self.turn);
                // queen side
                if dest_file == 2 {
                    if self.turn == WHITE {
                        self.piece_boards[rook_bb_i].clear_square(W_QUEEN_ROOK_START.into());
                        self.piece_boards[rook_bb_i].set_square(W_QUEEN_START.into());
                    } else {
                        self.piece_boards[rook_bb_i].clear_square(B_QUEEN_ROOK_START.into());
                        self.piece_boards[rook_bb_i].set_square(B_QUEEN_START.into());
                    }
                }
                // king side
                else {
                    if self.turn == WHITE {
                        self.piece_boards[rook_bb_i].clear_square(W_KING_ROOK_START.into());
                        self.piece_boards[rook_bb_i].set_square(W_KING_SIDE_BISHOP_START.into());
                    } else {
                        self.piece_boards[rook_bb_i].clear_square(B_KING_ROOK_START.into());
                        self.piece_boards[rook_bb_i].set_square(B_KING_SIDE_BISHOP_START.into());
                    }
                }
            }

            SpecialMove::EnPassant => {
                let backward = if self.turn == WHITE { SOUTH } else { NORTH };
                let enemy_pawn_index = Board::get_bb_index(Piece::Pawn, !self.turn);
                self.piece_boards[enemy_pawn_index]
                    .clear_square((destination.as_usize() as i8 + backward) as usize);
                captured_piece = Some(Piece::Pawn);
            }

            SpecialMove::NormalMove => (),
        }

        self.history.push(StateDelta::new(
            move_,
            captured_piece,
            self.en_passant,
            self.castle_rights,
            self.halfmove_count,
        ));

        let pawn_moved = moving_piece == Piece::Pawn;

        // update board state
        self.en_passant = Bitboard::new();
        match moving_piece {
            Piece::Pawn => {
                // en passant
                if (origin.as_usize() as i8 - destination.as_usize() as i8).abs() == 2 * NORTH {
                    // middle between des and origin
                    let en_passant_square = (origin.as_usize() + destination.as_usize()) / 2;

                    self.en_passant.set_square(en_passant_square);
                }
            }
            // castle rights
            Piece::King => {
                self.castle_rights.remove_castle_right(self.turn, true);
                self.castle_rights.remove_castle_right(self.turn, false);
            }

            Piece::Rook => {
                if (origin == W_KING_ROOK_START && self.turn == WHITE)
                    || (origin == B_KING_ROOK_START && self.turn == BLACK)
                {
                    self.castle_rights.remove_castle_right(self.turn, true);
                }
                if (origin == W_QUEEN_ROOK_START && self.turn == WHITE)
                    || (origin == B_QUEEN_ROOK_START && self.turn == BLACK)
                {
                    self.castle_rights.remove_castle_right(self.turn, false);
                }
            }

            _ => (),
        }

        self.captured_rook_remove_castle_rights();

        self.fullmove_count += 1;
        if let None = captured_piece {
            if !pawn_moved {
                self.halfmove_count += 1;
            }
        }
        self.turn = !self.turn;

        self.compute_bitboards();

        self.update_game_state();
    }

    fn captured_rook_remove_castle_rights(&mut self) {
        // capture rook, remove castle right
        let enemy_rooks = self.get_piece_bitboard(Piece::Rook, !self.turn);
        let enemy_queen_rook = if self.turn == WHITE {
            B_QUEEN_ROOK_START
        } else {
            W_QUEEN_ROOK_START
        };
        let enemy_king_rook = if self.turn == WHITE {
            B_KING_ROOK_START
        } else {
            W_KING_ROOK_START
        };
        if !enemy_rooks.is_square_set(enemy_king_rook.as_usize()) {
            self.castle_rights.remove_castle_right(!self.turn, true);
        }
        if !enemy_rooks.is_square_set(enemy_queen_rook.as_usize()) {
            self.castle_rights.remove_castle_right(!self.turn, false);
        }
    }

    pub(crate) fn unmake_move(&mut self) {
        let Some(move_delta) = self.history.pop() else {
            return;
        };
        self.en_passant = move_delta.en_pass;
        self.castle_rights = move_delta.castle_rights;
        self.halfmove_count = move_delta.halfmove;
        self.fullmove_count -= 1;
        self.turn = !self.turn;

        let last_move = move_delta.move_;
        let (origin, dest) = last_move.get_org_and_dest();

        let pawn_bb_index = Board::get_bb_index(Piece::Pawn, self.turn);

        let moving_piece = self.get_piece_type_containing_position(dest);
        let b_b_index = Board::get_bb_index(moving_piece, self.turn);

        self.piece_boards[b_b_index].set_square(origin.as_usize());
        self.piece_boards[b_b_index].clear_square(dest.as_usize());

        match last_move.get_special_move() {
            SpecialMove::NormalMove => {
                if let Some(captured_piece) = move_delta.captured_piece {
                    let cap_bb_index = Board::get_bb_index(captured_piece, !self.turn);
                    self.piece_boards[cap_bb_index].set_square(dest.as_usize());
                }
            }

            SpecialMove::Promotion => {
                if let Some(captured_piece) = move_delta.captured_piece {
                    let cap_bb_index = Board::get_bb_index(captured_piece, !self.turn);
                    self.piece_boards[cap_bb_index].set_square(dest.as_usize());
                }
                self.piece_boards[b_b_index].clear_square(origin.as_usize());
                self.piece_boards[pawn_bb_index].set_square(origin.as_usize());
            }

            SpecialMove::EnPassant => {
                let backward = if self.turn == WHITE { SOUTH } else { NORTH };
                if let Some(pawn_pos) = dest.try_offset(backward) {
                    let enemy_pawn_index = Board::get_bb_index(Piece::Pawn, !self.turn);
                    self.piece_boards[enemy_pawn_index].set_square(pawn_pos.as_usize());
                }
            }

            SpecialMove::Castle => {
                let (dest_file, _) = dest.get_file_and_rank();
                let rook_bb_i = Board::get_bb_index(Piece::Rook, self.turn);
                // queen side
                if dest_file == 2 {
                    if self.turn == WHITE {
                        self.piece_boards[rook_bb_i].set_square(W_QUEEN_ROOK_START.into());
                        self.piece_boards[rook_bb_i].clear_square(W_QUEEN_START.into());
                    } else {
                        self.piece_boards[rook_bb_i].set_square(B_QUEEN_ROOK_START.into());
                        self.piece_boards[rook_bb_i].clear_square(B_QUEEN_START.into());
                    }
                }
                // king side
                else {
                    if self.turn == WHITE {
                        self.piece_boards[rook_bb_i].set_square(W_KING_ROOK_START.into());
                        self.piece_boards[rook_bb_i].clear_square(W_KING_SIDE_BISHOP_START.into());
                    } else {
                        self.piece_boards[rook_bb_i].set_square(B_KING_ROOK_START.into());
                        self.piece_boards[rook_bb_i].clear_square(B_KING_SIDE_BISHOP_START.into());
                    }
                }
            }
        }
        self.compute_bitboards();
        self.update_game_state();
    }

    fn make_input_move(&mut self, origin: Position, dest: Position, promote: Piece) -> bool {
        let moves = self.generate_moves(self.turn);
        for move_ in moves {
            if move_.get_origin() == origin
                && move_.get_dest() == dest
                && move_.get_promotion() == promote
            {
                self.commit_verified_move(move_);
                return true;
            }
        }
        return false;
    }

    pub fn play_string_move(&mut self, s_move: &str) -> bool {
        if s_move.len() != 4 && s_move.len() != 5 {
            return false;
        }
        let promote = if s_move.len() == 5 {
            if let Ok(piece) = Piece::try_from(&s_move[4..5]) {
                piece
            } else {
                return false;
            }
        } else {
            Piece::Queen
        };

        let (from_s, to_s) = (&s_move[..2], &s_move[2..4]);

        let (Ok(origin), Ok(dest)) = (Position::try_from(from_s), Position::try_from(to_s)) else {
            return false;
        };
        return self.make_input_move(origin, dest, promote);
    }
}
