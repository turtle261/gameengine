fn main() {
    if let Err(error) = gameengine::cli::run_from_env() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
