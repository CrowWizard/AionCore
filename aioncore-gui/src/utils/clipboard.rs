use std::path::PathBuf;

use gpui::Image;

#[derive(Clone, Debug)]
pub struct ClipboardImage {
    pub path: PathBuf,
    pub filename: String,
}

pub async fn image_to_file(image: Image) -> anyhow::Result<ClipboardImage> {
    let path = crate::utils::file::write_image_to_temp_file(&image).await?;
    let filename = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("clipboard-image.png")
        .to_owned();

    Ok(ClipboardImage { path, filename })
}

pub async fn remove_file(path: PathBuf) {
    if let Err(error) = tokio::fs::remove_file(path).await {
        if error.kind() != std::io::ErrorKind::NotFound {
            log::warn!("failed to remove clipboard image temporary file: {error}");
        }
    }
}

pub fn remove_file_sync(path: &std::path::Path) {
    if let Err(error) = std::fs::remove_file(path) {
        if error.kind() != std::io::ErrorKind::NotFound {
            log::warn!("failed to remove clipboard image temporary file: {error}");
        }
    }
}
