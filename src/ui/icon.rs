use iced::widget::image::Handle;

// #EBECF2
pub static MAGNIFIER: &[u8] = include_bytes!("../../assets/magnifier.png");
pub static ENTER: &[u8] = include_bytes!("../../assets/enter.png");

pub static STAR_ACTIVE: &[u8] = include_bytes!("../../assets/star-fill.png");
pub static CUBE_ACTIVE: &[u8] = include_bytes!("../../assets/tabler--cube-active.png");
pub static FOLDER_ACTIVE: &[u8] = include_bytes!("../../assets/proicons--folder.png");

// // #808080
pub static CUBE_INACTIVE: &[u8] = include_bytes!("../../assets/tabler--cube.png");
pub static FOLDER_INACTIVE: &[u8] = include_bytes!("../../assets/proicons--folder-inactive.png");
pub static STAR_INACTIVE: &[u8] = include_bytes!("../../assets/star-line.png");
// static CLIPBOARD_INACTIVE: &[u8] = include_bytes!("../assets/tabler--clipboard.png");

#[derive(Debug, Clone)]
pub struct BakedIcons {
    pub magnifier: Handle,
    pub star_active: Handle,
    pub star_inactive: Handle,
    pub enter: Handle,
}
