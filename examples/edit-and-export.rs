use rs3a::{font::Font, Art, CSSColorMap};
use std::fs::File;
use std::io::Write;

fn main() {
    // Reading
    let mut art = Art::from_file("./examples/dna.3a").unwrap();

    // Add new color mapping
    let color_pair = "fg:black bg:yellow".parse().unwrap();
    let color = art.search_or_create_color_map(color_pair);

    // Editing example: add frame numbers
    for frame in 0..art.frames() {
        art.print(frame, 0, 0, &format!("{}", frame), Some(Some(color)));
    }

    // Saving
    art.to_file("./examples/edited_dna.3a").unwrap();

    // Exporting to JSON
    let mut output = File::create("./examples/dna.json").unwrap();
    write!(output, "{}", art.to_json()).unwrap();

    // Exporting to SVG
    let mut output = File::create("./examples/dna.svg").unwrap();
    write!(
        output,
        "{}",
        art.to_svg_frames(&CSSColorMap::default(), &Font::default())
    )
    .unwrap();

    // Exporting to asciicast (asciinema format).
    // You can play it with `asciinema play examples/dna.cast`.
    let mut output = File::create("./examples/dna.cast").unwrap();
    write!(output, "{}", art.to_asciicast2()).unwrap();

    // Printing as ANSI colored frames to stdout
    for frame in art.to_ansi_frames() {
        println!("{}\n", frame);
    }
}
