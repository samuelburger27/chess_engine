pub use chess_engine::{
    board_representation::{board::Board, position::Position},
    perf::perft,
};

struct PerftTestCase<'a> {
    fen: &'a str,
    depth: u32,
    expected_nodes: u64,
    description: &'a str,
}

#[test]
fn run_perft_tests() {
    // test taken from https://www.chessprogramming.org/Perft_Results

    let tests = [
        PerftTestCase {
            description: "Initial position, depth 5",
            fen: "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
            depth: 5,
            expected_nodes: 4865609,
        },
        PerftTestCase {
            description: "Kiwipete position, depth 2",
            fen: "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
            depth: 2,
            expected_nodes: 2039,
        },
        PerftTestCase {
            description: "Kiwipete position, depth 4",
            fen: "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
            depth: 4,
            expected_nodes: 4085603,
        },
        PerftTestCase {
            description: "Position 3 depth 5",
            fen: "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1 ",
            depth: 5,
            expected_nodes: 674624,
        },
        PerftTestCase {
            description: "Position 4 depth 4",
            fen: "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
            depth: 4,
            expected_nodes: 422333,
        },
    ];

    for test in tests.iter() {
        if let Ok(board) = Board::from_fen(test.fen) {
            let result = perft(&board, test.depth);
            assert_eq!(
                result, test.expected_nodes,
                "Failed test: {} (FEN: {})",
                test.description, test.fen
            );
        }
    }
}
