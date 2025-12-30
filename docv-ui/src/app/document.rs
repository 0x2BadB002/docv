use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use iced::{
    Element, Length, Task,
    widget::{column, container, scrollable, text},
};
use snafu::ResultExt;

use crate::Error;

#[derive(Debug)]
pub enum Message {}

#[derive(Debug)]
pub struct Document {
    file: Arc<Mutex<docv_pdf::Document>>,
    title: String,
    pages: Vec<docv_pdf::Page>,
    page_count: usize,

    view: View,
}

#[derive(Debug, Default)]
enum View {
    #[default]
    RawData,
}

impl Document {
    pub fn read_from_path(path: PathBuf) -> Result<Self, Error> {
        let mut file = docv_pdf::Document::from_path(&path).context(crate::error::PdfSnafu)?;

        let title = file
            .info()
            .title
            .clone()
            .unwrap_or_else(|| path.file_name().unwrap().to_string_lossy().to_string());

        let pages = file
            .pages()
            .collect::<std::result::Result<Vec<_>, docv_pdf::Error>>()
            .context(crate::error::PdfSnafu)?;
        let page_count = file.pages().count();

        Ok(Document {
            file: Arc::new(Mutex::new(file)),
            pages,
            page_count,
            title,

            view: View::default(),
        })
    }

    pub fn title(&self) -> &str {
        self.title.as_ref()
    }

    pub fn update(&mut self, msg: Message) -> Task<crate::app::Message> {
        match msg {}
    }

    pub fn view(&self) -> Element<'_, Message> {
        match self.view {
            View::RawData => scrollable(container(text!("{:#?}", self.pages)).padding(20))
                .width(Length::Fill)
                .height(Length::Fill)
                .into(),
        }
    }

    pub fn view_info(&self) -> Element<'_, Message> {
        let file = self.file.lock().unwrap();
        let info = file.info();

        container(
            column![
                container(
                    column![text!("Pages count: {}", self.page_count),]
                        .padding(10)
                        .spacing(10)
                )
                .style(container::rounded_box),
                container(
                    column![
                        text!("Title: {}", info.title.as_deref().unwrap_or("Unavailable")),
                        text!(
                            "Subject: {}",
                            info.subject.as_deref().unwrap_or("Unavailable")
                        ),
                        text!(
                            "Keywords: {}",
                            info.keywords.as_deref().unwrap_or("Unavailable")
                        ),
                    ]
                    .padding(10)
                    .spacing(10)
                )
                .style(container::rounded_box),
                container(
                    column![
                        text!(
                            "Author: {}",
                            info.author.as_deref().unwrap_or("Unavailable")
                        ),
                        text!(
                            "Creator: {}",
                            info.creator.as_deref().unwrap_or("Unavailable")
                        ),
                        text!(
                            "Producer: {}",
                            info.producer.as_deref().unwrap_or("Unavailable")
                        ),
                    ]
                    .padding(10)
                    .spacing(10)
                )
                .style(container::rounded_box),
                container(
                    column![
                        text(format!(
                            "Creation date: {}",
                            info.creation_date.unwrap_or_default()
                        )),
                        text(format!(
                            "Modified date: {}",
                            info.mod_date.unwrap_or_default()
                        )),
                    ]
                    .padding(10)
                    .spacing(10)
                )
                .style(container::rounded_box),
                container(
                    column![
                        text(format!("Version: {}", file.version())),
                        text(format!("Trapped: {}", info.trapped)),
                        text(format!(
                            "File size: {:.2} Mib",
                            file.filesize() as f64 / ((1024 * 1024) as f64)
                        )),
                        text(
                            file.hash()
                                .map(|hash| { format!("File hash: {hash}") })
                                .unwrap_or_else(|| "Hash wasn't provided".to_string())
                        )
                    ]
                    .padding(10)
                    .spacing(10)
                )
                .style(container::rounded_box),
            ]
            .spacing(15)
            .padding(10),
        )
        .into()
    }
}
