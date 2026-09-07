//! Conversion between 3a art and plain text that contains frames in a grid.

use crate::chars::{normalize_text, Char};
use crate::{Art, Cell, Error, Result};

/// Layout information for tiled plain text.
///
/// `columns` and `rows` are required. The other fields are optional parsing
/// hints. A gap is an area between cells that the parser ignores. Thus, the
/// gap can contain spaces, separator characters, or other text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TiledTextOptions {
    /// Maximum number of frame cells in one grid row.
    pub columns: usize,
    /// Maximum number of frame cells in one grid column.
    pub rows: usize,
    /// Width of one frame cell, in characters.
    pub cell_width: Option<usize>,
    /// Height of one frame cell, in lines.
    pub cell_height: Option<usize>,
    /// Width of the ignored area between frame cells.
    pub horizontal_gap: Option<usize>,
    /// Height of the ignored area between frame cells.
    pub vertical_gap: Option<usize>,
}

impl TiledTextOptions {
    /// Creates options with the required grid dimensions and no hints.
    pub fn new(columns: usize, rows: usize) -> Self {
        Self {
            columns,
            rows,
            cell_width: None,
            cell_height: None,
            horizontal_gap: None,
            vertical_gap: None,
        }
    }

    /// Adds exact frame cell dimensions.
    pub fn with_cell_size(mut self, width: usize, height: usize) -> Self {
        self.cell_width = Some(width);
        self.cell_height = Some(height);
        self
    }

    /// Adds exact gap dimensions.
    pub fn with_gaps(mut self, horizontal: usize, vertical: usize) -> Self {
        self.horizontal_gap = Some(horizontal);
        self.vertical_gap = Some(vertical);
        self
    }

    fn validate(&self) -> Result<()> {
        if self.columns == 0 || self.rows == 0 {
            return Err(Error::InvalidTiledTextOptions(
                "grid dimensions must be greater than zero".into(),
            ));
        }
        if self.cell_width == Some(0) || self.cell_height == Some(0) {
            return Err(Error::InvalidTiledTextOptions(
                "cell dimensions must be greater than zero".into(),
            ));
        }
        self.columns
            .checked_mul(self.rows)
            .ok_or_else(|| Error::InvalidTiledTextOptions("grid capacity is too large".into()))?;
        Ok(())
    }
}

impl Art {
    /// Parses colorless frames from plain text arranged in a grid.
    ///
    /// Frames are read from left to right and then from top to bottom. Missing
    /// characters are spaces. A short final grid row produces only the frame
    /// cells that occur in the input. If a dimension or gap is not supplied,
    /// the parser infers it from the longest line or from the line count.
    pub fn from_tiled_text(text: &str, options: TiledTextOptions) -> Result<Self> {
        options.validate()?;

        let mut lines: Vec<Vec<char>> = text
            .split('\n')
            .map(|line| {
                normalize_text(line.trim_end_matches('\r'))
                    .chars()
                    .collect()
            })
            .collect();
        if text.is_empty() {
            lines.clear();
        } else if text.ends_with('\n') {
            lines.pop();
        }

        let text_width = lines.iter().map(Vec::len).max().unwrap_or(0);
        let (cell_width, horizontal_gap) = infer_horizontal_axis(
            &lines,
            text_width,
            options.columns,
            options.cell_width,
            options.horizontal_gap,
        );
        let (cell_height, vertical_gap) = infer_vertical_axis(
            &lines,
            options.rows,
            options.cell_height,
            options.vertical_gap,
        );

        if lines.is_empty() || cell_width == 0 || cell_height == 0 {
            return Ok(Self::new(0, cell_width, cell_height, Cell::default()));
        }

        let row_stride = cell_height.saturating_add(vertical_gap);
        let column_stride = cell_width.saturating_add(horizontal_gap);
        let present_rows = ((lines.len() - 1) / row_stride + 1).min(options.rows);

        let mut cells_per_row = Vec::with_capacity(present_rows);
        for grid_row in 0..present_rows {
            let line_start = grid_row.saturating_mul(row_stride);
            let line_end = line_start.saturating_add(cell_height).min(lines.len());
            let extent = lines[line_start..line_end]
                .iter()
                .map(Vec::len)
                .max()
                .unwrap_or(0);
            let count = if extent == 0 || column_stride == 0 {
                0
            } else {
                ((extent - 1) / column_stride + 1).min(options.columns)
            };
            cells_per_row.push(count);
        }

        let frame_count = cells_per_row.iter().sum();
        let mut art = Self::new(frame_count, cell_width, cell_height, Cell::default());
        let mut frame = 0;
        for (grid_row, &cell_count) in cells_per_row.iter().enumerate() {
            let line_start = grid_row.saturating_mul(row_stride);
            for grid_column in 0..cell_count {
                let column_start = grid_column.saturating_mul(column_stride);
                for row in 0..cell_height {
                    let Some(line) = lines.get(line_start.saturating_add(row)) else {
                        continue;
                    };
                    for column in 0..cell_width {
                        let Some(&ch) = line.get(column_start.saturating_add(column)) else {
                            continue;
                        };
                        // normalize_text has already removed disallowed characters.
                        art.set(
                            frame,
                            column,
                            row,
                            Cell {
                                text: Char::new_must(ch),
                                color: None,
                            },
                        );
                    }
                }
                frame += 1;
            }
        }
        Ok(art)
    }

    /// Converts frames to colorless plain text arranged in a grid.
    ///
    /// The function uses spaces for horizontal gaps and blank lines for
    /// vertical gaps. If a gap is not supplied, its size is one. Cell size
    /// hints, if supplied, must match the art dimensions.
    pub fn to_tiled_text(&self, options: TiledTextOptions) -> Result<String> {
        options.validate()?;
        let capacity = options.columns * options.rows;
        if self.frames() > capacity {
            return Err(Error::InvalidTiledTextOptions(format!(
                "the grid has {} cells but the art has {} frames",
                capacity,
                self.frames()
            )));
        }
        if let Some(width) = options.cell_width {
            if width != self.width() {
                return Err(Error::InvalidTiledTextOptions(format!(
                    "cell width {} does not match art width {}",
                    width,
                    self.width()
                )));
            }
        }
        if let Some(height) = options.cell_height {
            if height != self.height() {
                return Err(Error::InvalidTiledTextOptions(format!(
                    "cell height {} does not match art height {}",
                    height,
                    self.height()
                )));
            }
        }
        if self.frames() == 0 {
            return Ok(String::new());
        }

        let horizontal_gap = options.horizontal_gap.unwrap_or(1);
        let vertical_gap = options.vertical_gap.unwrap_or(1);
        let used_grid_rows = (self.frames() - 1) / options.columns + 1;
        let mut output = Vec::new();

        for grid_row in 0..used_grid_rows {
            let first_frame = grid_row * options.columns;
            let frames_in_row = (self.frames() - first_frame).min(options.columns);
            for row in 0..self.height() {
                let mut line = String::new();
                for grid_column in 0..frames_in_row {
                    if grid_column > 0 {
                        line.push_str(&" ".repeat(horizontal_gap));
                    }
                    let frame = first_frame + grid_column;
                    for column in 0..self.width() {
                        line.push(self.get(frame, column, row, Cell::default()).text.into());
                    }
                }
                output.push(line);
            }
            if grid_row + 1 < used_grid_rows {
                for _ in 0..vertical_gap {
                    output.push(String::new());
                }
            }
        }

        Ok(output.join("\n"))
    }
}

fn infer_axis(
    extent: usize,
    cells: usize,
    size_hint: Option<usize>,
    gap_hint: Option<usize>,
) -> (usize, usize) {
    match (size_hint, gap_hint) {
        (Some(size), Some(gap)) => (size, gap),
        (Some(size), None) => {
            let occupied = size.saturating_mul(cells);
            let gap = if cells > 1 && extent > occupied {
                (extent - occupied) / (cells - 1)
            } else {
                0
            };
            (size, gap)
        }
        (None, Some(gap)) => {
            let gaps = gap.saturating_mul(cells.saturating_sub(1));
            let available = extent.saturating_sub(gaps);
            (div_ceil(available, cells), gap)
        }
        (None, None) => {
            if extent == 0 {
                return (0, 0);
            }
            if cells == 1 {
                return (extent, 0);
            }

            // Prefer the smallest gap that gives equal cell sizes. This makes
            // adjacent cells the default and detects common one-character gaps.
            for gap in 0..=extent / (cells - 1) {
                let gaps = gap * (cells - 1);
                let available = extent - gaps;
                if available > 0 && available.is_multiple_of(cells) {
                    return (available / cells, gap);
                }
            }
            (div_ceil(extent, cells), 0)
        }
    }
}

fn infer_horizontal_axis(
    lines: &[Vec<char>],
    extent: usize,
    cells: usize,
    size_hint: Option<usize>,
    gap_hint: Option<usize>,
) -> (usize, usize) {
    let inferred = infer_axis(extent, cells, size_hint, gap_hint);
    if size_hint.is_some() || gap_hint.is_some() {
        return inferred;
    }
    let Some(size) = default_gap_size(extent, cells) else {
        return inferred;
    };

    let mut found_separator = false;
    for boundary in 1..cells {
        let separator = boundary * size + boundary - 1;
        for line in lines {
            if let Some(&ch) = line.get(separator) {
                found_separator = true;
                if ch != ' ' {
                    return inferred;
                }
            }
        }
    }
    if found_separator {
        (size, 1)
    } else {
        inferred
    }
}

fn infer_vertical_axis(
    lines: &[Vec<char>],
    cells: usize,
    size_hint: Option<usize>,
    gap_hint: Option<usize>,
) -> (usize, usize) {
    let inferred = infer_axis(lines.len(), cells, size_hint, gap_hint);
    if size_hint.is_some() || gap_hint.is_some() {
        return inferred;
    }
    let Some(size) = default_gap_size(lines.len(), cells) else {
        return inferred;
    };

    let mut found_separator = false;
    for boundary in 1..cells {
        let separator = boundary * size + boundary - 1;
        if let Some(line) = lines.get(separator) {
            found_separator = true;
            if !line.is_empty() {
                return inferred;
            }
        }
    }
    if found_separator {
        (size, 1)
    } else {
        inferred
    }
}

fn default_gap_size(extent: usize, cells: usize) -> Option<usize> {
    if cells < 2 || extent <= cells - 1 {
        return None;
    }
    let size = div_ceil(extent - (cells - 1), cells);
    let full_extent = size.saturating_mul(cells).saturating_add(cells - 1);
    let missing = full_extent.saturating_sub(extent);
    if size > 0 && missing < size {
        Some(size)
    } else {
        None
    }
}

fn div_ceil(value: usize, divisor: usize) -> usize {
    if value == 0 {
        0
    } else {
        (value - 1) / divisor + 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infers_axes() {
        assert_eq!(infer_axis(11, 3, None, None), (3, 1));
        assert_eq!(infer_axis(9, 3, None, None), (3, 0));
        assert_eq!(infer_axis(5, 3, Some(1), None), (1, 1));
        assert_eq!(infer_axis(7, 3, None, Some(1)), (2, 1));
    }

    #[test]
    fn finds_default_gap_after_trimming() {
        let lines = vec!["1   2   3".chars().collect()];
        assert_eq!(infer_horizontal_axis(&lines, 9, 3, None, None), (3, 1));

        let lines = vec![
            vec!['x'],
            vec!['x'],
            Vec::new(),
            Vec::new(),
            vec!['y'],
            vec!['y'],
        ];
        assert_eq!(infer_vertical_axis(&lines, 2, None, None), (3, 1));
    }
}
