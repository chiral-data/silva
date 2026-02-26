use std::path::Path;

/// Recursively copies a directory and its contents.
///
/// # Returns
///
/// Returns the number of files copied, or an error if the copy fails.
pub fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<usize> {
    use std::fs;

    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }

    let mut file_count = 0;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let dest_path = dst.join(entry.file_name());

        if path.is_dir() {
            file_count += copy_dir_recursive(&path, &dest_path)?;
        } else {
            fs::copy(&path, &dest_path)?;
            file_count += 1;
        }
    }

    Ok(file_count)
}
