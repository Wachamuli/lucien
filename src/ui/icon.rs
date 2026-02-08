use iced::widget::image::Handle;

pub const ICON_SIZES: [&str; 7] = [
    "512x512", "256x256", "128x128", "96x96", "64x64", "48x48", "32x32",
];

pub const ICON_EXTENSIONS: [&str; 2] = ["svg", "png"];

// #EBECF2
//
pub static MAGNIFIER: &[u8] = include_bytes!("../../assets/icons/magnifier.png");
pub static ENTER: &[u8] = include_bytes!("../../assets/icons/enter.png");

pub static STAR_ACTIVE: &[u8] = include_bytes!("../../assets/icons/star-fill.png");
pub static CUBE_ACTIVE: &[u8] = include_bytes!("../../assets/icons/tabler--cube-active.png");
pub static FOLDER_ACTIVE: &[u8] = include_bytes!("../../assets/icons/proicons--folder.png");

// // #808080
pub static CUBE_INACTIVE: &[u8] = include_bytes!("../../assets/icons/tabler--cube.png");
pub static FOLDER_INACTIVE: &[u8] =
    include_bytes!("../../assets/icons/proicons--folder-inactive.png");
pub static STAR_INACTIVE: &[u8] = include_bytes!("../../assets/icons/star-line.png");
// static CLIPBOARD_INACTIVE: &[u8] = include_bytes!("../assets/icons/tabler--clipboard.png");

pub static APPLICATION_EXECUTABLE: &[u8] =
    include_bytes!("../../assets/mimetypes/application-x-executable.png");

#[derive(Debug, Clone)]
pub struct BakedIcons {
    pub magnifier: Handle,
    pub star_active: Handle,
    pub star_inactive: Handle,
    pub enter: Handle,
}
