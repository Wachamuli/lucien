pub const ICON_SIZES: [&str; 7] = [
    "512x512", "256x256", "128x128", "96x96", "64x64", "48x48", "32x32",
];

pub const ICON_EXTENSIONS: [&str; 2] = ["svg", "png"];

// #EBECF2 - Active icons color
// #808080 - Inactive icons color

// UI Icons
pub static MAGNIFIER: &[u8] = include_bytes!("../../assets/icons/magnifier.png");
pub static ENTER: &[u8] = include_bytes!("../../assets/icons/enter.png");

pub static STAR_ACTIVE: &[u8] = include_bytes!("../../assets/icons/star-fill.png");
pub static CUBE_ACTIVE: &[u8] = include_bytes!("../../assets/icons/tabler--cube-active.png");
pub static FOLDER_ACTIVE: &[u8] = include_bytes!("../../assets/icons/proicons--folder.png");
pub static ICON_PLACEHOLDER: &[u8] = include_bytes!("../../assets/icons/icon-placeholder.png");

pub static CUBE_INACTIVE: &[u8] = include_bytes!("../../assets/icons/tabler--cube.png");
pub static FOLDER_INACTIVE: &[u8] =
    include_bytes!("../../assets/icons/proicons--folder-inactive.png");
pub static STAR_INACTIVE: &[u8] = include_bytes!("../../assets/icons/star-line.png");
// static CLIPBOARD_INACTIVE: &[u8] = include_bytes!("../assets/icons/tabler--clipboard.png");

// TODO: Convert every image to raster image formats
pub static APPLICATION_DEFAULT: &[u8] =
    include_bytes!("../../assets/mimetypes/application-x-executable.png");
pub static FOLDER_DEFAULT: &[u8] = include_bytes!("../../assets/mimetypes/inode-directory.png");
pub static TEXT_GENERIC: &[u8] = include_bytes!("../../assets/mimetypes/text-x-generic.png");
pub static IMAGE_GENERIC: &[u8] = include_bytes!("../../assets/mimetypes/image-x-generic.png");
pub static AUDIO_GENERIC: &[u8] = include_bytes!("../../assets/mimetypes/audio-x-generic.png");
pub static VIDEO_GENERIC: &[u8] = include_bytes!("../../assets/mimetypes/video-x-generic.png");
pub static FONT_GENERIC: &[u8] = include_bytes!("../../assets/mimetypes/font-x-generic.png");
pub static MULTIPART_GENERIC: &[u8] =
    include_bytes!("../../assets/mimetypes/package-x-generic.png");
pub static MODEL_GENERIC: &[u8] = include_bytes!("../../assets/mimetypes/model.png");

// TODO: I should have this struct declared and use in one place
#[derive(Debug, Clone)]
pub struct BakedIcons {
    pub magnifier: iced::widget::image::Handle,
    pub star_active: iced::widget::image::Handle,
    pub star_inactive: iced::widget::image::Handle,
    pub enter: iced::widget::image::Handle,
    pub icon_placeholder: iced::widget::image::Handle,
}
