use gpui::Image;
use std::path::PathBuf;

pub async fn write_image_to_temp_file(image: &Image) -> anyhow::Result<PathBuf> {
    let image_bytes = image.bytes();
    let image = image::load_from_memory(image_bytes)?;
    let directory = std::env::temp_dir().join("aioncore-gui");
    tokio::fs::create_dir_all(&directory).await?;

    let path = directory.join(format!(
        "clipboard-{}-{}.png",
        crate::utils::time::now_millis(),
        uuid::Uuid::new_v4()
    ));
    image.save_with_format(&path, image::ImageFormat::Png)?;

    Ok(path)
}
