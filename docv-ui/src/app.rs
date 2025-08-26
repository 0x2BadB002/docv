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
            Message::FileOpened(file) => {
                self.error = None;
                self.error_backtrace = false;

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

    fn view(&self) -> Element<Message> {
        let main_content = container(
            scrollable(
                text(
                    self.file
                        .as_ref()
                        .map(|file| format!("{file:#?}"))
                        .or_else(|| {
                            if !self.error_backtrace {
                                return None;
                            }

                            self.prev_error
                                .as_ref()
                                .map(|err| format!("{err:#?}").to_string())
                        })
                        .unwrap_or_else(|| "No file openned".to_string()),
                )
                .center(),
            )
            .width(Length::Fill)
            .height(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fill);

        let interface = container(
            column![self.error.as_ref().map_or_else(
                || self.cmdline.view().map(Message::CmdLine),
                |err| {
                    container(text(err.to_string()).style(text::danger))
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

        stack![main_content, interface].into()
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            keyboard::on_key_press(|key, key_mod| {
                if let Key::Character(key) = key.as_ref() {
                    if key == ";" && key_mod == Modifiers::SHIFT {
                        return Some(Message::ShowCmdline);
                    }
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
    let mut file = Document::from_path(&filepath)?;

    file.read_xref()?;

    Ok(Arc::new(file))
}
