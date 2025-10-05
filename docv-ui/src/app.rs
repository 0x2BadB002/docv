use std::{path::PathBuf, sync::Arc};

use iced::{
    Alignment, Element, Length, Subscription, Task, Theme,
    keyboard::{self, Key, Modifiers},
    widget::{column, container, scrollable, stack, text},
};

use crate::{Error, Result};
use docv_pdf::Document;

mod cmdline;

#[derive(Debug)]
enum Message {
    CmdLine(cmdline::Message),

    OpenFile(PathBuf),
    SetTheme(iced::Theme),
    Quit,
    ShowErrors,

    ShowCmdline,
    FileOpened(Arc<Document>),
    ErrorOccurred(Error),
}

#[derive(Default, Debug)]
struct App {
    file: Option<Arc<Document>>,
    page_count: usize,
    error: Option<Arc<Error>>,
    prev_error: Option<Arc<Error>>,
    error_backtrace: bool,

    cmdline: cmdline::Cmdline,
    theme: iced::Theme,
}

pub fn run(filename: Option<PathBuf>) -> Result<()> {
    iced::application("DocV", App::update, App::view)
        .subscription(App::subscription)
        .theme(App::theme)
        .resizable(true)
        .centered()
        .run_with(|| {
            let mut tasks = vec![Task::done(Message::SetTheme(Theme::Nord))];

            if let Some(filename) = filename {
                tasks.push(Task::done(Message::OpenFile(filename)));
            }

            (App::default(), Task::batch(tasks))
        })
        .map_err(Error::Iced)
}

impl App {
    fn update(&mut self, msg: Message) -> Task<Message> {
        match msg {
            Message::CmdLine(msg) => self.cmdline.update(msg),
            Message::OpenFile(filepath) => Task::perform(read_file(filepath), |res| {
                res.map(Message::FileOpened)
                    .unwrap_or_else(Message::ErrorOccurred)
            }),
            Message::Quit => iced::exit(),
            Message::ShowErrors => {
                self.error_backtrace = true;

                Task::none()
            }
            Message::ShowCmdline => {
                self.error = None;
                self.error_backtrace = false;

                self.cmdline.show().map(Message::CmdLine)
            }
            Message::FileOpened(mut file) => {
                self.error = None;
                self.error_backtrace = false;

                self.page_count = Arc::<Document>::get_mut(&mut file)
                    .unwrap()
                    .pages()
                    .map(|pages| pages.count())
                    .unwrap_or(0);

                self.file = Some(file);

                Task::none()
            }
            Message::ErrorOccurred(error) => {
                self.error_backtrace = false;
                self.prev_error = self.error.clone();
                self.error = Some(Arc::new(error));
                if self.prev_error.is_none() {
                    self.prev_error = self.error.clone();
                }

                Task::none()
            }
            Message::SetTheme(theme) => {
                self.theme = theme;

                Task::none()
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let no_info = "Unavailable".to_string();
        let main_content = self
            .file
            .as_ref()
            .map(|file| {
                let info = file.info();

                container(
                    column![
                        container(
                            column![text(format!("Pages count: {}", self.page_count)),]
                                .padding(10)
                                .spacing(10)
                        )
                        .style(container::rounded_box),
                        container(
                            column![
                                text(format!(
                                    "Title: {}",
                                    info.title.as_ref().unwrap_or(&no_info)
                                )),
                                text(format!(
                                    "Subject: {}",
                                    info.subject.as_ref().unwrap_or(&no_info)
                                )),
                                text(format!(
                                    "Keywords: {}",
                                    info.keywords.as_ref().unwrap_or(&no_info)
                                )),
                            ]
                            .padding(10)
                            .spacing(10)
                        )
                        .style(container::rounded_box),
                        container(
                            column![
                                text(format!(
                                    "Author: {}",
                                    info.author.as_ref().unwrap_or(&no_info)
                                )),
                                text(format!(
                                    "Creator: {}",
                                    info.creator.as_ref().unwrap_or(&no_info)
                                )),
                                text(format!(
                                    "Producer: {}",
                                    info.producer.as_ref().unwrap_or(&no_info)
                                )),
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
            })
            .unwrap_or_else(|| container(text("No file opened")))
            .padding(5);

        let interface = container(
            column![self.error.as_ref().map_or_else(
                || self.cmdline.view().map(Message::CmdLine),
                |err| {
                    container(
                        text(if self.error_backtrace {
                            format!("{err:?}")
                        } else {
                            format!("{err}")
                        })
                        .style(text::danger),
                    )
                    .height(Length::Shrink)
                    .width(Length::Fill)
                    .padding(5)
                    .into()
                }
            ),]
            .spacing(0)
            .padding(0)
            .align_x(Alignment::Start),
        )
        .width(Length::Fill)
        .align_bottom(Length::Fill)
        .padding(0);

        stack![
            scrollable(main_content)
                .height(Length::Fill)
                .width(Length::Fill),
            interface
        ]
        .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            keyboard::on_key_press(|key, key_mod| {
                if let Key::Character(key) = key.as_ref()
                    && key == ";"
                    && key_mod == Modifiers::SHIFT
                {
                    return Some(Message::ShowCmdline);
                }
                None
            }),
            self.cmdline.subscription().map(Message::CmdLine),
        ])
    }

    fn theme(&self) -> iced::Theme {
        self.theme.clone()
    }
}

async fn read_file(filepath: PathBuf) -> Result<Arc<Document>> {
    let file = Document::from_path(&filepath)?;

    Ok(Arc::new(file))
}
