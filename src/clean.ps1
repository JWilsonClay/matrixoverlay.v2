# 1. Force clean the package to remove old artifacts
cargo clean -p matrix-overlay

# 2. Rebuild release binary
cargo build --release

# 3. Run and check for the new log message
RUST_LOG=info ./target/release/matrix-overlay
