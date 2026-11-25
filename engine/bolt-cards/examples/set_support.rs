//! Which cards of a set the engine fully plays: `cargo run --example set_support -- TDM`
fn main() {
    let code = std::env::args().nth(1).unwrap_or_else(|| "TDM".into());
    let s = bolt_cards::set(&code).expect("a known set");
    for (o, ok) in s.oracle.iter().zip(&s.supported) {
        println!("{}\t{}", if *ok { "yes" } else { "no" }, o.name);
    }
}
