use crate::board_representation::{
    bitboard::Bitboard,
    board::{Board, BLACK, WHITE},
    computed_boards::ZOBRIST_TABLE,
    game_state::StateDelta,
    piece::Piece,
    position::Position,
    r#const::{
        B_KING_ROOK_START, B_KING_SIDE_BISHOP_START, B_QUEEN_ROOK_START, B_QUEEN_START, NORTH,
        SOUTH, W_KING_ROOK_START, W_KING_SIDE_BISHOP_START, W_QUEEN_ROOK_START, W_QUEEN_START,
    },
    r#move::{Move, SpecialMove},
};

impl Board {
    pub fn commit_verified_move(&mut self, move_: Move) {
        // commit move
        // move should be verified before

        // xor old en_pass file
        self.xor_en_pass_from_zobrist(self.en_passant);

        let (origin, destination) = move_.get_org_and_dest();
        let mut captured_piece = if !self.empty_tiles.is_square_set(destination.into()) {
            Some(self.get_piece_type_containing_position(destination))
        } else {
            None
        };

        let moving_piece = self.get_piece_type_containing_position(origin);

        self.remove_piece(self.turn, moving_piece, origin);
        self.add_piece(self.turn, moving_piece, destination);

        // remove captured piece from bitboard and hash
        // need to handle en passant separately
        if let Some(cap_piece) = captured_piece {
            self.remove_piece(!self.turn, cap_piece, destination);
        }

        match move_.get_special_move() {
            SpecialMove::Promotion => {
                let promote_to = move_.get_promotion();
                self.remove_piece(self.turn, moving_piece, destination);
                self.add_piece(self.turn, promote_to, destination);
            }

            SpecialMove::Castle => {
                let (dest_file, _) = destination.get_file_and_rank();
                // queen side
                if dest_file == 2 {
                    if self.turn == WHITE {
                        self.remove_piece(self.turn, Piece::Rook, W_QUEEN_ROOK_START);
                        self.add_piece(self.turn, Piece::Rook, W_QUEEN_START);
                    } else {
                        self.remove_piece(self.turn, Piece::Rook, B_QUEEN_ROOK_START);
                        self.add_piece(self.turn, Piece::Rook, B_QUEEN_START);
                    }
                }
                // king side
                else {
                    if self.turn == WHITE {
                        self.remove_piece(self.turn, Piece::Rook, W_KING_ROOK_START);
                        self.add_piece(self.turn, Piece::Rook, W_KING_SIDE_BISHOP_START);
                    } else {
                        self.remove_piece(self.turn, Piece::Rook, B_KING_ROOK_START);
                        self.add_piece(self.turn, Piece::Rook, B_KING_SIDE_BISHOP_START);
                    }
                }
            }

            SpecialMove::EnPassant => {
                captured_piece = Some(Piece::Pawn);
                let backward = if self.turn == WHITE { SOUTH } else { NORTH };
                let captured_pawn_pos = (destination.as_usize() as i8 + backward) as usize;
                self.remove_piece(!self.turn, Piece::Pawn, Position::new(captured_pawn_pos));
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
                    let en_passant_pos =
                        Position::new((origin.as_usize() + destination.as_usize()) / 2);
                    self.en_passant.set_square(en_passant_pos.as_usize());
                    self.xor_en_pass_from_zobrist(self.en_passant);
                }
            }
            // castle rights
            Piece::King => {
                self.remove_castle(self.turn, true);
                self.remove_castle(self.turn, false);
            }

            Piece::Rook => {
                if (origin == W_KING_ROOK_START && self.turn == WHITE)
                    || (origin == B_KING_ROOK_START && self.turn == BLACK)
                {
                    self.remove_castle(self.turn, true);
                }
                if (origin == W_QUEEN_ROOK_START && self.turn == WHITE)
                    || (origin == B_QUEEN_ROOK_START && self.turn == BLACK)
                {
                    self.remove_castle(self.turn, false);
                }
            }

            _ => (),
        }

        if Some(Piece::Rook) == captured_piece {
            self.captured_rook_remove_castle_rights();
        }

        self.fullmove_count += 1;
        if let None = captured_piece {
            if !pawn_moved {
                self.halfmove_count += 1;
            }
        }
        self.turn = !self.turn;
        self.zobrist_key ^= ZOBRIST_TABLE.white_to_move;

        self.compute_bitboards();
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
            self.remove_castle(!self.turn, true);
        }
        if !enemy_rooks.is_square_set(enemy_queen_rook.as_usize()) {
            self.remove_castle(!self.turn, false);
        }
    }

    pub(crate) fn unmake_move(&mut self) {
        let Some(move_delta) = self.history.pop() else {
            return;
        };

        if self.castle_rights != move_delta.castle_rights {
            // update castle rights in zobrist key
            for index in 0..4 {
                if self.castle_rights.castle_at_index(index)
                    != move_delta.castle_rights.castle_at_index(index)
                {
                    self.zobrist_key ^= ZOBRIST_TABLE.castle_rights[index];
                }
            }
        }
        self.xor_en_pass_from_zobrist(self.en_passant);
        self.xor_en_pass_from_zobrist(move_delta.en_pass);
        self.zobrist_key ^= ZOBRIST_TABLE.white_to_move;

        self.en_passant = move_delta.en_pass;
        self.castle_rights = move_delta.castle_rights;
        self.halfmove_count = move_delta.halfmove;
        self.fullmove_count -= 1;
        self.turn = !self.turn;

        let last_move = move_delta.move_;
        let (origin, dest) = last_move.get_org_and_dest();
        let moving_piece = self.get_piece_type_containing_position(dest);

        self.add_piece(self.turn, moving_piece, origin);
        self.remove_piece(self.turn, moving_piece, dest);

        match last_move.get_special_move() {
            SpecialMove::NormalMove => {
                if let Some(captured_piece) = move_delta.captured_piece {
                    self.add_piece(!self.turn, captured_piece, dest);
                }
            }

            SpecialMove::Promotion => {
                if let Some(captured_piece) = move_delta.captured_piece {
                    self.add_piece(!self.turn, captured_piece, dest);
                }
                self.remove_piece(self.turn, moving_piece, origin);
                self.add_piece(self.turn, Piece::Pawn, origin);
            }

            SpecialMove::EnPassant => {
                let backward = if self.turn == WHITE { SOUTH } else { NORTH };
                if let Some(pawn_pos) = dest.try_offset(backward) {
                    self.add_piece(!self.turn, Piece::Pawn, pawn_pos);
                }
            }

            SpecialMove::Castle => {
                let (dest_file, _) = dest.get_file_and_rank();
                // queen side
                if dest_file == 2 {
                    if self.turn == WHITE {
                        self.add_piece(self.turn, Piece::Rook, W_QUEEN_ROOK_START);
                        self.remove_piece(self.turn, Piece::Rook, W_QUEEN_START);
                    } else {
                        self.add_piece(self.turn, Piece::Rook, B_QUEEN_ROOK_START);
                        self.remove_piece(self.turn, Piece::Rook, B_QUEEN_START);
                    }
                }
                // king side
                else {
                    if self.turn == WHITE {
                        self.add_piece(self.turn, Piece::Rook, W_KING_ROOK_START);
                        self.remove_piece(self.turn, Piece::Rook, W_KING_SIDE_BISHOP_START);
                    } else {
                        self.add_piece(self.turn, Piece::Rook, B_KING_ROOK_START);
                        self.remove_piece(self.turn, Piece::Rook, B_KING_SIDE_BISHOP_START);
                    }
                }
            }
        }

        self.compute_bitboards();
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

    pub(crate) fn update_game_state(&mut self) {
        // this method should be called when creating board and when committing a move
        // TODO, draw, stalemate
    }
}
