pub type Comments = Vec<String>;

/// Writes each comment to the formatter on its own line, prefixed by ";;".
pub fn write_comments(comments: &Comments, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    for c in comments {
        writeln!(f, ";;{}", c)?;
    }
    Ok(())
}
