use std::sync::LazyLock;

use iced::widget::image;

pub const ICON_SIZES: [&str; 7] = [
    "512x512", "256x256", "128x128", "96x96", "64x64", "48x48", "32x32",
];

pub const ICON_EXTENSIONS: [&str; 2] = ["svg", "png"];

macro_rules! bake_icon {
    ($path:expr) => {
        LazyLock::new(|| iced::widget::image::Handle::from_bytes(include_bytes!($path).as_slice()))
    };
}

// #EBECF2 - Active icons color
// #808080 - Inactive icons color

// --- UI Icons ---
pub static MAGNIFIER: LazyLock<image::Handle> = bake_icon!("../../assets/icons/magnifier.png");
pub static ENTER: LazyLock<image::Handle> = bake_icon!("../../assets/icons/enter.png");

pub static STAR_ACTIVE: LazyLock<image::Handle> = bake_icon!("../../assets/icons/star-fill.png");
pub static STAR_INACTIVE: LazyLock<image::Handle> = bake_icon!("../../assets/icons/star-line.png");

pub static CUBE_ACTIVE: LazyLock<image::Handle> =
    bake_icon!("../../assets/icons/tabler--cube-active.png");
pub static CUBE_INACTIVE: LazyLock<image::Handle> =
    bake_icon!("../../assets/icons/tabler--cube.png");

pub static FOLDER_ACTIVE: LazyLock<image::Handle> =
    bake_icon!("../../assets/icons/proicons--folder.png");
pub static FOLDER_INACTIVE: LazyLock<image::Handle> =
    bake_icon!("../../assets/icons/proicons--folder-inactive.png");

pub static ICON_PLACEHOLDER: LazyLock<image::Handle> =
    bake_icon!("../../assets/icons/icon-placeholder.png");

// --- Mimetypes / Generic Icons ---
pub static APPLICATION_DEFAULT: LazyLock<image::Handle> =
    bake_icon!("../../assets/mimetypes/application-x-executable.png");
pub static FOLDER_DEFAULT: LazyLock<image::Handle> =
    bake_icon!("../../assets/mimetypes/inode-directory.png");
pub static TEXT_GENERIC: LazyLock<image::Handle> =
    bake_icon!("../../assets/mimetypes/text-x-generic.png");
pub static IMAGE_GENERIC: LazyLock<image::Handle> =
    bake_icon!("../../assets/mimetypes/image-x-generic.png");
pub static AUDIO_GENERIC: LazyLock<image::Handle> =
    bake_icon!("../../assets/mimetypes/audio-x-generic.png");
pub static VIDEO_GENERIC: LazyLock<image::Handle> =
    bake_icon!("../../assets/mimetypes/video-x-generic.png");
pub static FONT_GENERIC: LazyLock<image::Handle> =
    bake_icon!("../../assets/mimetypes/font-x-generic.png");
pub static MULTIPART_GENERIC: LazyLock<image::Handle> =
    bake_icon!("../../assets/mimetypes/package-x-generic.png");
pub static MODEL_GENERIC: LazyLock<image::Handle> = bake_icon!("../../assets/mimetypes/model.png");
