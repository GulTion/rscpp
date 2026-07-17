use rscpp_parser::parse;
use std::env;
use std::fs;

fn main() {
    let path = env::args().nth(1).expect("path");
    let src = fs::read_to_string(&path).unwrap();
    match parse(&src) {
        Ok(_) => println!("parse ok"),
        Err(e) => {
            let start = e.span.start as usize;
            let end = e.span.end as usize;
            let line = src[..start].bytes().filter(|&b| b == b'\n').count() + 1;
            let col = start - src[..start].rfind('\n').map(|i| i + 1).unwrap_or(0) + 1;
            println!("{}:{}:{}: {}", path, line, col, e.message);
            let line_start = src[..start].rfind('\n').map(|i| i + 1).unwrap_or(0);
            let line_end = src[start..]
                .find('\n')
                .map(|i| start + i)
                .unwrap_or(src.len());
            println!("| {}", &src[line_start..line_end]);
            println!(
                "| {}{}",
                " ".repeat(start - line_start),
                "^".repeat((end - start).max(1))
            );
            // context ±2 lines
            let lines: Vec<_> = src.lines().collect();
            let lo = line.saturating_sub(2);
            let hi = (line + 1).min(lines.len());
            for i in lo..hi {
                println!("{:4}| {}", i + 1, lines[i]);
            }
        }
    }
}
