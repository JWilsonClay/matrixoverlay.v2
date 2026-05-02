fn main() {
    if let Err(e) = matrix_overlay::core::main::run() {
        eprintln!("Error: {:?}", e);
        std::process::exit(1);
    }
}
