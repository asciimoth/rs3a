use rs3a::Art;
use std::fs::read_to_string;

fn main() {
    let text = read_to_string("./examples/dna.ansi").unwrap();
    let art = Art::from_ansi_text(&text);

    print!("{}", art);

    // Printing as ANSI colored frames to stdout
    for frame in art.to_ansi_frames() {
        println!("{}\n", frame);
    }
}
