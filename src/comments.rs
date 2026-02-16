pub type Comments = Vec<String>;

pub fn write_comments(comments: &Comments, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    for c in comments {
        writeln!(f, ";;{}", c)?;
    }
    Ok(())
}
