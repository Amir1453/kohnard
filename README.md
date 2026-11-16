# Execution

To build:
`cargo build`

To run:
`cargo run --release {filename}`

To test:
`cargo test`

# Dependencies

- logos: Lexer
- lalrpop: Parser Generator
- compact_str: String with SSO
- petgraph: Graphs
- anyhow: Error handling
- thiserror: To generate error enums
- hashbrown: Hashing
- parking_lot: Fast Mutex
- scoped_tls: To have scoped TLS 
- thin_vec: Word length Vector
- typed-index-collections: Provides IndexVec<I: Idx, T>
- smallvec: Inline storage Vector
