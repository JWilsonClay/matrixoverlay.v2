fn main() {
    // [HARDENING] Integrated startup info and process governance
    matrix_overlay::core::version::print_startup_info();

    if let Err(e) = matrix_overlay::core::main::run() {
        log::error!("CRITICAL: Matrix Overlay failed: {:?}", e);
        eprintln!("Sovereign Failure: {:?}", e);
        std::process::exit(1);
    }
}
