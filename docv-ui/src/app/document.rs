use core::str;
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use iced::{
    Element, Length, Subscription, Task,
    keyboard::{self, Event, Key},
    widget::{column, container, scrollable, text},
};
use snafu::{ResultExt, Snafu};

#[derive(Debug, Snafu)]
pub struct Error(error::Error);
type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone)]
pub struct Document {
    pub title: Arc<str>,
    pub filename: Arc<str>,
    pub page_count: usize,
    current_page_index: usize,

    view: View,
    file: Arc<Mutex<docv_pdf::Document>>,
    pages: Arc<[docv_pdf::Page]>,
}

#[derive(Debug, Clone)]
pub enum View {
    RawData,
}

#[derive(Debug)]
pub enum Message {
    ChangeView(View),
    NextPage,
    PrevPage,
    SetPageNumber(usize),
}

impl Document {
    pub fn read_from_path(path: &PathBuf) -> Result<Self> {
        let mut file = docv_pdf::Document::from_path(&path).context(error::Pdf)?;

        let filename = path.file_name().unwrap().to_string_lossy().to_string();

        let title = file
            .info()
            .title
            .clone()
            .unwrap_or_else(|| filename.clone());

        let pages = file
            .pages()
            .collect::<std::result::Result<Vec<_>, docv_pdf::Error>>()
            .context(error::Pdf)?;
        let page_count = file.pages().count();

        Ok(Document {
            title: title.into(),
            filename: filename.into(),
            page_count,
            current_page_index: 0,

            view: View::RawData,
            file: Arc::new(Mutex::new(file)),
            pages: pages.into(),
        })
    }

    pub fn title(&self) -> &str {
        self.title.as_ref()
    }

    pub fn update(&mut self, msg: Message) -> Task<crate::app::Message> {
        match msg {
            Message::ChangeView(view) => {
                self.view = view;

                Task::none()
            }
            Message::NextPage => match self.view {
                View::RawData => {
                    if self.current_page_index >= self.page_count - 1 {
                        self.current_page_index = self.page_count - 1;

                        return Task::done(crate::app::Message::ErrorOccurred(
                            Error::from(error::Error::LastPage).into(),
                        ));
                    }

                    self.current_page_index = self.current_page_index.saturating_add(1);

                    Task::none()
                }
            },
            Message::PrevPage => match self.view {
                View::RawData => {
                    if self.current_page_index == 0 {
                        return Task::done(crate::app::Message::ErrorOccurred(
                            Error::from(error::Error::FirstPage).into(),
                        ));
                    }

                    self.current_page_index = self.current_page_index.saturating_sub(1);

                    Task::none()
                }
            },
            Message::SetPageNumber(number) => match self.view {
                View::RawData => {
                    if number > self.page_count {
                        return Task::done(crate::app::Message::ErrorOccurred(
                            Error::from(error::Error::SetPage).into(),
                        ));
                    }

                    self.current_page_index = number.saturating_sub(1);

                    Task::none()
                }
            },
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        match self.view {
            View::RawData => {
                let page = &self.pages[self.current_page_index];

                scrollable(container(text!("{}", page)).padding(20))
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .into()
            }
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        keyboard::listen().filter_map(|event| match event {
            Event::KeyPressed { modified_key, .. } => {
                if let Key::Character(c) = modified_key {
                    match c.as_str() {
                        "j" => Some(Message::NextPage),
                        "k" => Some(Message::PrevPage),
                        _ => None,
                    }
                } else {
                    None
                }
            }
            _ => None,
        })
    }

    pub fn current_page(&self) -> usize {
        match self.view {
            View::RawData => self.current_page_index + 1,
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

mod error {
    use snafu::Snafu;

    #[derive(Debug, Snafu)]
    #[snafu(visibility(pub(super)), context(suffix(false)))]
    pub(super) enum Error {
        #[snafu(display("Error working with PDF file"))]
        Pdf { source: docv_pdf::Error },

        #[snafu(display("You are already on first page"))]
        FirstPage,

        #[snafu(display("You are already on last page"))]
        LastPage,

        #[snafu(display("Incorrect page number"))]
        SetPage,
    }
}
