An animated ASCII art rust library, implementing the [3a format](https://github.com/asciimoth/3a).  
Features:
- reading/writing the [new 3a format](https://github.com/asciimoth/3a/blob/main/3a.md)
- partial support for the [legacy 3a format](https://github.com/asciimoth/3a/blob/main/3a_legacy_spec.md)
- editing API
- conversion to:
    - SVG
    - [asciicast v2](https://docs.asciinema.org/manual/asciicast/v2/)
    - plain text with ANSI color [escape codes](https://en.wikipedia.org/wiki/ANSI_escape_code)

## Used in
- [aaa](https://github.com/asciimoth/aaa) – a TUI tool for rendering 3a files

## Examples of 3a art
- [3a art storage](https://github.com/asciimoth/3a_storage)
- [3a logo](https://github.com/asciimoth/3a/blob/main/logo.3a)

## Other 3a implementations
- [go3a](https://github.com/asciimoth/go3a)
- [py3a](https://github.com/asciimoth/py3a)

## Example
You can run this example with `cargo run --example edit-and-export`
```rust
use rs3a::{Art, font::Font, CSSColorMap};
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

    // Exporting to SVG
    let mut output = File::create("./examples/dna.svg").unwrap();
    write!(
        output, "{}",
        art.to_svg_frames(&CSSColorMap::default(), &Font::default())
    ).unwrap();

    // Exporting to asciicast (asciinema format).
    // You can play it with `asciinema play examples/dna.cast`.
    let mut output = File::create("./examples/dna.cast").unwrap();
    write!(output, "{}", art.to_asciicast2()).unwrap();

    // Printing as ANSI colored frames to stdout
    for frame in art.to_ansi_frames() {
        println!("{}\n", frame);
    }
}
```

## TODO
- art optimisation
- conversion to
    - image
    - gif
    - video

## License
This project is licensed under either of

- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or [apache.org/licenses/LICENSE-2.0](http://www.apache.org/licenses/LICENSE-2.0))
- MIT license ([LICENSE-MIT](LICENSE-MIT) or [opensource.org/licenses/MIT](http://opensource.org/licenses/MIT))

at your option.
