use iced::{Task, futures::Stream};
use sqlx::Connection;

use crate::{
    launcher::Message,
    providers::{Provider, ScanRequest},
    ui::entry::Entry,
};

struct ClipboardProvider;

impl Provider for ClipboardProvider {
    fn scan(request: ScanRequest) -> impl Stream<Item = Message> {
        iced::stream::channel(100, async move |output| {
            let conn = sqlx::SqliteConnection::connect("/home/wachamuli/clipboard.db")
                .await
                .unwrap();

            todo!()
        })
    }

    fn launch(entry: &Entry) -> Task<Message> {
        todo!()
    }
}
