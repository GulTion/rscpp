use std::env;
use std::fs;
fn main() {
    let path = env::args().nth(1).unwrap();
    let src = fs::read_to_string(&path).unwrap();
    let tu = match rscpp_parser::parse(&src) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("parse: {e}");
            std::process::exit(1);
        }
    };
    let r = rscpp_sema::analyze(&tu);
    for e in &r.errors {
        let start = e.span.start as usize;
        let line = src[..start].bytes().filter(|&b| b == b'\n').count() + 1;
        println!("{}:{}: {}", path, line, e.message);
    }
    if r.errors.is_empty() {
        println!("sema ok");
    }
}
