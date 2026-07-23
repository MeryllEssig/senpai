mod cli;
mod manifest;

fn main() {
    let code = cli::run(std::env::args().skip(1).collect());
    std::process::exit(code);
}
