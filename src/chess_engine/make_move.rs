//! Applying and undoing moves on a [`Board`].
//!
//! [`Board::commit_verified_move`] mutates the position in place and is the
//! single point where all four [`SpecialMove`] cases (normal, promotion,
//! castle, en passant) are handled, along with the bookkeeping that hangs off a
//! move: captures, en-passant square, castling rights, the half-move/full-move
//! clocks, and the side to move. Before touching anything it pushes a
//! [`StateDelta`] capturing the pre-move state — including the full Zobrist hash
//! — so [`unmake_move`](Board::unmake_move) can restore the position exactly.
//!
//! The Zobrist hash is kept up to date incrementally: each
//! [`add_piece`](Board::add_piece)/[`remove_piece`](Board::remove_piece) XORs the
//! relevant table entry. `unmake_move` restores the stored hash wholesale rather
//! than replaying those XORs. The unit tests at the bottom of this file assert
//! the incremental hash always matches a full recompute.

use crate::chess_engine::{
    bitboard::Bitboard,
    board::{Board, Turn, BLACK, WHITE},
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
    /// Applies `move_` to the board, updating piece placement, the Zobrist hash,
    /// castling rights, en-passant square, the move clocks, and the side to move,
    /// and records a `StateDelta` so the move can be undone.
    ///
    /// The move must already be known to be legal (e.g. produced by
    /// [`generate_moves`](Board::generate_moves)); it is not re-validated here.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn commit_verified_move(&mut self, move_: Move) {
        // commit move
        // move should be verified before

        let (origin, destination) = move_.get_org_and_dest();
        let captured_piece = if move_.get_special_move() == SpecialMove::EnPassant {
            Some(Piece::Pawn)
        } else if !self.empty_tiles.is_square_set(destination.into()) {
            Some(self.get_piece_type_containing_position(destination))
        } else {
            None
        };

        let moving_piece = self.get_piece_type_containing_position(origin);

        // record state before any mutation; the stored zobrist must be the
        // full pre-move hash so repetition detection can compare against it
        self.history.push(StateDelta::new(
            move_,
            captured_piece,
            self.en_passant,
            self.castle_rights,
            self.halfmove_count,
            self.zobrist_key,
        ));

        // xor old en_pass file
        self.xor_en_pass_from_zobrist(self.en_passant);

        // remove captured piece from bitboard and hash
        // en passant captures are handled in the match below
        if move_.get_special_move() != SpecialMove::EnPassant {
            if let Some(cap_piece) = captured_piece {
                self.remove_piece(!self.turn, cap_piece, destination);
            }
        }

        match move_.get_special_move() {
            SpecialMove::Promotion => {
                let promote_to = move_.get_promotion();
                self.remove_piece(self.turn, moving_piece, origin);
                self.add_piece(self.turn, promote_to, destination);
            }

            SpecialMove::Castle => {
                self.remove_piece(self.turn, moving_piece, origin);
                self.add_piece(self.turn, moving_piece, destination);
                let (rook_origin, rook_dest) =
                    Self::get_castle_rook_origin_dest(self.turn, destination);
                self.remove_piece(self.turn, Piece::Rook, rook_origin);
                self.add_piece(self.turn, Piece::Rook, rook_dest);
            }

            SpecialMove::EnPassant => {
                self.remove_piece(self.turn, moving_piece, origin);
                self.add_piece(self.turn, moving_piece, destination);
                let backward = if self.turn == WHITE { SOUTH } else { NORTH };
                let captured_pawn_pos = (destination.as_usize() as i8 + backward) as usize;
                self.remove_piece(!self.turn, Piece::Pawn, Position::new(captured_pawn_pos));
            }

            SpecialMove::NormalMove => {
                self.remove_piece(self.turn, moving_piece, origin);
                self.add_piece(self.turn, moving_piece, destination);
            }
        }

        let pawn_moved = moving_piece == Piece::Pawn;

        // update board state
        self.en_passant = Bitboard::new();
        match moving_piece {
            Piece::Pawn => {
                // en passant
                if (origin.as_usize() as i8 - destination.as_usize() as i8).abs() == 2 * NORTH {
                    // middle between des and origin
                    let en_passant_pos =
                        Position::new(usize::midpoint(origin.as_usize(), destination.as_usize()));
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

        if self.turn == BLACK {
            self.fullmove_count += 1;
        }
        if captured_piece.is_none() && !pawn_moved {
            self.halfmove_count += 1;
        } else {
            self.halfmove_count = 0;
        }
        self.turn = !self.turn;
        self.zobrist_key ^= ZOBRIST_TABLE.white_to_move;

        self.compute_bitboards();
        self.update_game_result();
    }

    /// After a rook is captured, revokes the opponent's castling right on that
    /// rook's side if the rook is no longer on its starting square.
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

    /// Reverts the most recently committed move, restoring the board to its
    /// previous state from the top [`StateDelta`] on the history stack. Does
    /// nothing if no move has been made.
    pub(crate) fn unmake_move(&mut self) {
        let Some(move_delta) = self.history.pop() else {
            return;
        };

        self.en_passant = move_delta.en_pass;
        self.castle_rights = move_delta.castle_rights;
        self.halfmove_count = move_delta.halfmove;
        self.turn = !self.turn;
        // fullmove counter only advances after black's move
        if self.turn == BLACK {
            self.fullmove_count -= 1;
        }

        let last_move = move_delta.move_;
        let (origin, dest) = last_move.get_org_and_dest();
        let moving_piece = self.get_piece_type_containing_position(dest);

        match last_move.get_special_move() {
            SpecialMove::NormalMove => {
                self.add_piece(self.turn, moving_piece, origin);
                self.remove_piece(self.turn, moving_piece, dest);
                if let Some(captured_piece) = move_delta.captured_piece {
                    self.add_piece(!self.turn, captured_piece, dest);
                }
            }

            SpecialMove::Promotion => {
                if let Some(captured_piece) = move_delta.captured_piece {
                    self.add_piece(!self.turn, captured_piece, dest);
                }
                self.remove_piece(self.turn, moving_piece, dest);
                self.add_piece(self.turn, Piece::Pawn, origin);
            }

            SpecialMove::EnPassant => {
                self.add_piece(self.turn, moving_piece, origin);
                self.remove_piece(self.turn, moving_piece, dest);
                let backward = if self.turn == WHITE { SOUTH } else { NORTH };
                if let Some(pawn_pos) = dest.try_offset(backward) {
                    self.add_piece(!self.turn, Piece::Pawn, pawn_pos);
                }
            }

            SpecialMove::Castle => {
                self.add_piece(self.turn, moving_piece, origin);
                self.remove_piece(self.turn, moving_piece, dest);
                let (rook_origin, rook_dest) = Self::get_castle_rook_origin_dest(self.turn, dest);
                self.remove_piece(self.turn, Piece::Rook, rook_dest);
                self.add_piece(self.turn, Piece::Rook, rook_origin);
            }
        }

        // add_piece/remove_piece above xor the hash; the stored pre-move hash
        // is exact, so restore it wholesale instead of replaying xors
        self.zobrist_key = move_delta.zobrist_hash;

        self.compute_bitboards();
        self.update_game_result();
    }

    /// Finds the legal move matching `(origin, dest, promote)` and commits it,
    /// returning whether such a move existed.
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
        false
    }

    /// Parses a move in long algebraic / UCI notation (e.g. `"e2e4"`, or
    /// `"e7e8q"` with a trailing promotion letter) and plays it if it is legal
    /// in the current position. Returns `false` for malformed strings or illegal
    /// moves, leaving the board unchanged.
    ///
    /// ```
    /// use chess_engine::chess_engine::board::Board;
    /// use chess_engine::chess_engine::utils::init_tables;
    ///
    /// init_tables();
    /// let mut board = Board::new_start_pos().unwrap();
    /// assert!(board.play_string_move("e2e4")); // legal opening move
    /// assert!(!board.play_string_move("xyz")); // malformed input
    /// ```
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
        self.make_input_move(origin, dest, promote)
    }

    /// Returns the `(origin, destination)` squares of the rook involved in a
    /// castle, inferred from the king's destination square `king_des`.
    const fn get_castle_rook_origin_dest(turn: Turn, king_des: Position) -> (Position, Position) {
        // return dest and origin of a rook that is moved during castle
        let (file, _) = king_des.get_file_and_rank();
        // queen side
        if file == 2 {
            if turn == WHITE {
                return (W_QUEEN_ROOK_START, W_QUEEN_START);
            }
            return (B_QUEEN_ROOK_START, B_QUEEN_START);
        }
        // king side
        else if turn == WHITE {
            return (W_KING_ROOK_START, W_KING_SIDE_BISHOP_START);
        }
        (B_KING_ROOK_START, B_KING_SIDE_BISHOP_START)
    }
}

#[cfg(test)]
mod tests {
    use crate::chess_engine::board::Board;
    use crate::chess_engine::computed_boards::ZOBRIST_TABLE;

    fn assert_incremental_hash_matches(board: &Board) {
        assert_eq!(
            board.zobrist_key,
            ZOBRIST_TABLE.hash_position(board),
            "incremental zobrist key diverged from a full recompute"
        );
    }

    fn play_and_check(board: &mut Board, moves: &[&str]) {
        for mv in moves {
            assert!(board.play_string_move(mv), "illegal move in test: {mv}");
            assert_incremental_hash_matches(board);
        }
    }

    #[test]
    fn zobrist_stays_consistent_through_special_moves() {
        let mut board = Board::new_start_pos().unwrap();
        assert_incremental_hash_matches(&board);
        // double pawn pushes (en passant squares), en passant capture,
        // development, castling both sides
        play_and_check(
            &mut board,
            &[
                "e2e4", "a7a6", "e4e5", "d7d5", "e5d6", // en passant capture
                "e7d6", "g1f3", "g8f6", "f1e2", "f8e7", "e1g1", // white castles short
                "e8g8", // black castles short
            ],
        );
    }

    #[test]
    fn zobrist_stays_consistent_through_promotion() {
        // white pawn on g7 promotes by pushing; black pawn on b2 promotes by
        // capturing the rook on a1
        let mut board = Board::from_fen("8/6P1/7k/8/8/8/1p6/R6K w - - 0 1").unwrap();
        assert_incremental_hash_matches(&board);
        play_and_check(&mut board, &["g7g8q", "b2a1n"]);
    }

    #[test]
    fn unmake_restores_board_and_hash() {
        let mut board = Board::new_start_pos().unwrap();
        let original = board.clone();
        play_and_check(
            &mut board,
            &["e2e4", "a7a6", "e4e5", "d7d5", "e5d6", "e7d6", "g1f3"],
        );
        for _ in 0..7 {
            board.unmake_move();
            assert_incremental_hash_matches(&board);
        }
        assert!(
            board == original,
            "unmake did not restore the original board"
        );
    }

    #[test]
    fn halfmove_and_fullmove_counters() {
        let mut board = Board::new_start_pos().unwrap();
        board.play_string_move("g1f3"); // quiet knight move: halfmove 1
        assert_eq!(board.halfmove_count, 1);
        assert_eq!(board.fullmove_count, 1);
        board.play_string_move("g8f6");
        assert_eq!(board.halfmove_count, 2);
        assert_eq!(board.fullmove_count, 2); // black moved: fullmove advances
        board.play_string_move("e2e4"); // pawn move resets the clock
        assert_eq!(board.halfmove_count, 0);
        assert_eq!(board.fullmove_count, 2);
    }

    #[test]
    fn repetition_is_detected() {
        let mut board = Board::new_start_pos().unwrap();
        // shuffle knights back and forth: position repeats
        for mv in ["g1f3", "g8f6", "f3g1", "f6g8"] {
            assert!(board.play_string_move(mv));
        }
        // back to the start position, which occurred once before
        assert_eq!(board.get_count_of_current_position_reached(), 1);
    }
}
