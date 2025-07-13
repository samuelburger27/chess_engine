pub use chess_engine::{
    board::{Board, Position},
    fen_parser::{parse_fen, starting_pos_fen},
    move_generation::get_moves,
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
    let tests = [
        PerftTestCase {
            description: "Kiwipete position, depth 2",
            fen: "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
            depth: 2,
            expected_nodes: 2039,
        },
        // PerftTestCase {
        //     description: "Initial position, depth 1",
        //     fen: "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        //     depth: 1,
        //     expected_nodes: 20,
        // },
        // PerftTestCase {
        //     description: "Initial position, depth 2",
        //     fen: "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        //     depth: 2,
        //     expected_nodes: 400,
        // },
        // PerftTestCase {
        //     description: "Position after 1.e4, depth 1",
        //     fen: "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1",
        //     depth: 1,
        //     expected_nodes: 20,
        // },
        // PerftTestCase {
        //     description: "Kiwipete position, depth 3",
        //     fen: "r3k2r/p1pp1pb1/bn2Qnp1/2qPN3/1p2P3/2N2P2/PPPBB1PP/R3K2R w KQkq - 0 1",
        //     depth: 3,
        //     expected_nodes: 97862,
        // },
    ];

    for test in tests.iter() {
        if let Ok(board) = parse_fen(test.fen) {
            let result = perft(&board, test.depth);
            assert_eq!(
                result, test.expected_nodes,
                "Failed test: {} (FEN: {})",
                test.description, test.fen
            );
        }
    }
}
