use std::path::PathBuf;

use iced::Task;
use iced::widget::image;

use crate::{
    launcher::Message,
    providers::{app::AppProvider, file::FileProvider},
};

pub mod app;
pub mod file;

#[derive(Debug, Clone, Copy)]
pub enum ProviderKind {
    App(AppProvider),
    File(FileProvider),
}

impl ProviderKind {
    // TODO: Replace dynamic dispatch with monomorphization
    pub fn handler(&self) -> &dyn Provider {
        match self {
            ProviderKind::App(p) => p,
            ProviderKind::File(p) => p,
        }
    }
}

pub trait Provider {
    // Maybe this function should return a Task<Message::PopulateEntries>?
    fn scan(&self, dir: &PathBuf) -> Vec<Entry>;
    // Maybe, launch could consume self? But I have to get rid of dynamic dispatch first.
    // I could avoid couple clones doing this.
    fn launch(&self, id: &str) -> Task<Message>;
    fn get_icon(&self, path: &PathBuf, size: u32) -> Option<image::Handle>;
}

#[derive(Debug, Clone)]
pub struct Entry {
    pub id: String,
    pub main: String,
    pub secondary: Option<String>,
    pub icon: Option<PathBuf>,
}

impl Entry {
    fn new(id: String, main: String, secondary: Option<String>, icon: Option<PathBuf>) -> Self {
        Self {
            id,
            main,
            secondary,
            icon,
        }
    }
}
