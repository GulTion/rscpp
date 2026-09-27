use rscpp_parser::parse;
use std::env;
use std::fs;

fn main() {
    let path = env::args().nth(1).expect("path");
    let src = fs::read_to_string(&path).unwrap();
    match parse(&src) {
        Ok(_) => println!("parse ok"),
        Err(e) => {
            eprint!("{}", e.format_with_source(&src));
        }
    }
}
