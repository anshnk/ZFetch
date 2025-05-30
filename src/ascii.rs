use include_dir::{include_dir, Dir};

static LOGOS_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/logos");

pub async fn get_ascii_logo(distro_id: &str) -> String {
    let filename = format!("{}.txt", distro_id);
    if let Some(file) = LOGOS_DIR.get_file(&filename) {
        file.contents_utf8().unwrap_or("Logo not found").to_string()
    } else {
        "Logo not found".to_string()
    }
}