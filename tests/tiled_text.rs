use rs3a::{chars::Char, Art, Cell, TiledTextOptions};

fn assert_same_text(actual: &Art, expected: &Art) {
    assert_eq!(actual.frames(), expected.frames());
    assert_eq!(actual.width(), expected.width());
    assert_eq!(actual.height(), expected.height());
    assert!(!actual.color());

    for frame in 0..expected.frames() {
        for row in 0..expected.height() {
            for column in 0..expected.width() {
                assert_eq!(
                    actual.get(frame, column, row, Cell::default()).text,
                    expected.get(frame, column, row, Cell::default()).text,
                    "different text at frame {frame}, column {column}, row {row}"
                );
            }
        }
    }
}

#[test]
fn converts_to_tiled_text_and_back() {
    let mut expected = Art::new(5, 4, 3, Cell::default());
    for frame in 0..expected.frames() {
        expected.print(frame, 0, 0, &format!("F{frame}  "), None);
        expected.print(frame, 0, 1, "/\\  ", None);
        expected.print(frame, 0, 2, "____", None);
    }
    expected.set(
        0,
        0,
        0,
        Cell {
            text: Char::new_must('F'),
            color: Some(Char::new_must('x')),
        },
    );

    let options = TiledTextOptions::new(3, 2);
    let text = expected.to_tiled_text(options).unwrap();
    let actual = Art::from_tiled_text(&text, options).unwrap();

    assert_same_text(&actual, &expected);
}

#[test]
fn accepts_stripped_trailing_spaces() {
    let mut expected = Art::new(5, 3, 2, Cell::default());
    for frame in 0..expected.frames() {
        expected.print(frame, 0, 0, &format!("{}", frame + 1), None);
        expected.print(frame, 0, 1, &format!("{}x", frame + 1), None);
    }
    let options = TiledTextOptions::new(3, 2)
        .with_cell_size(3, 2)
        .with_gaps(2, 1);
    let text = expected.to_tiled_text(options).unwrap();
    let stripped = text
        .split('\n')
        .map(|line| line.trim_end())
        .collect::<Vec<_>>()
        .join("\n");

    let actual = Art::from_tiled_text(&stripped, options).unwrap();

    assert_same_text(&actual, &expected);
}

#[test]
fn round_trip_trimmed_text_with_only_grid_dimensions() {
    let mut expected = Art::new(5, 3, 3, Cell::default());
    for frame in 0..expected.frames() {
        expected.print(frame, 0, 0, &format!("{}", frame + 1), None);
        expected.print(frame, 0, 1, &format!("{}x", frame + 1), None);
        // The last row and the end of each other row remain spaces.
    }
    let options = TiledTextOptions::new(3, 2);
    let text = expected.to_tiled_text(options).unwrap();
    let stripped = text
        .split('\n')
        .map(|line| line.trim_end())
        .collect::<Vec<_>>()
        .join("\n");

    let actual = Art::from_tiled_text(&stripped, options).unwrap();

    assert_same_text(&actual, &expected);
}

#[test]
fn uses_best_effort_for_larger_grid_than_input() {
    let options = TiledTextOptions::new(3, 3)
        .with_cell_size(1, 1)
        .with_gaps(0, 0);
    let actual = Art::from_tiled_text("XY", options).unwrap();

    assert_eq!(actual.frames(), 2);
    assert_eq!(actual.width(), 1);
    assert_eq!(actual.height(), 1);
    assert_eq!(actual.get(0, 0, 0, Cell::default()).text.to_string(), "X");
    assert_eq!(actual.get(1, 0, 0, Cell::default()).text.to_string(), "Y");
}

#[test]
fn uses_small_cell_hint_without_panicking() {
    let options = TiledTextOptions::new(3, 1).with_cell_size(1, 1);
    let actual = Art::from_tiled_text("AA|BB|CC", options).unwrap();

    assert_eq!(actual.frames(), 3);
    assert_eq!(actual.get(0, 0, 0, Cell::default()).text.to_string(), "A");
    assert_eq!(actual.get(1, 0, 0, Cell::default()).text.to_string(), "B");
    assert_eq!(actual.get(2, 0, 0, Cell::default()).text.to_string(), "C");
}

#[test]
fn detects_tile_count_in_each_grid_row() {
    let options = TiledTextOptions::new(3, 2)
        .with_cell_size(1, 1)
        .with_gaps(1, 1);
    let actual = Art::from_tiled_text("A B\n---\nC D", options).unwrap();

    assert_eq!(actual.frames(), 4);
    for (frame, expected) in ['A', 'B', 'C', 'D'].iter().enumerate() {
        assert_eq!(
            actual.get(frame, 0, 0, Cell::default()).text.to_string(),
            expected.to_string()
        );
    }
}

#[test]
fn rejects_zero_grid_dimension() {
    let error = Art::from_tiled_text("A", TiledTextOptions::new(0, 1)).unwrap_err();
    assert!(error.to_string().contains("grid dimensions"));
}

#[test]
fn reads_delimited_irregular_golden_vector() {
    let expected: Art = include_str!("vectors/tiled/delimited.3a").parse().unwrap();
    let options = TiledTextOptions::new(3, 2)
        .with_cell_size(2, 2)
        .with_gaps(1, 1);

    let actual =
        Art::from_tiled_text(include_str!("vectors/tiled/delimited.txt"), options).unwrap();

    assert_same_text(&actual, &expected);
    assert_eq!(actual.frames(), 5);
}

#[test]
fn reads_and_writes_compact_golden_vector() {
    let text = include_str!("vectors/tiled/compact.txt").trim_end_matches('\n');
    let expected: Art = include_str!("vectors/tiled/compact.3a").parse().unwrap();
    let options = TiledTextOptions::new(2, 2).with_gaps(0, 0);

    let actual = Art::from_tiled_text(text, options).unwrap();

    assert_same_text(&actual, &expected);
    assert_eq!(expected.to_tiled_text(options).unwrap(), text);
}

#[test]
fn reads_trimmed_trailing_whitespace_golden_vector() {
    let text = include_str!("vectors/tiled/trailing-whitespace.txt");
    // The 3a vector uses non-breaking spaces so that source control preserves
    // its trailing whitespace. The 3a parser normalizes them to ASCII spaces.
    let expected: Art = include_str!("vectors/tiled/trailing-whitespace.3a")
        .parse()
        .unwrap();
    let options = TiledTextOptions::new(3, 2);

    let actual = Art::from_tiled_text(text, options).unwrap();

    assert_same_text(&actual, &expected);
    assert_eq!(actual.frames(), 5);
    assert_eq!((actual.width(), actual.height()), (3, 3));
}
