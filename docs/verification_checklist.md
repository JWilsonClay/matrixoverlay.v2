# Final Verification Checklist & Run Instructions

## 1. Build & Unit Tests
- [ ] **Clean Build**: `cargo clean && cargo build --release`
- [ ] **Unit Tests**: `cargo test` (Should pass all metrics/config tests)
- [ ] **Binary Size**: Check `target/release/matrix-overlay` size (Expect ~4-6MB unstripped).

## 2. Integration Tests (Automated)
Run the following commands in order. Ensure an X11 session is active.

### Hardware & Sensors
```bash
# Verifies NVIDIA/AMD detection and sensor reading latency
./tests/test_scripts/hardware_test.sh
