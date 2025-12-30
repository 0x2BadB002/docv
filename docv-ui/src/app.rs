use std::path::PathBuf;

use iced::{
    Alignment, Element, Length, Subscription, Task,
    alignment::{Horizontal, Vertical},
    keyboard::{self, Key, key::Named},
    widget::{column, container, row, scrollable, stack, text},
};
use snafu::ResultExt;

use crate::{Error, Result, app::document::Document};

pub mod cmdline;
pub mod document;

#[derive(Debug)]
pub enum Message {
    CmdLine(cmdline::Message),
    Document(document::Message),

    OpenFile(PathBuf),
    DocumentReady(Document),

    SetTheme(iced::Theme),
    Quit,
    ShowErrors,
    ShowInfo,
    ShowCmdline,
    CleanScreen,

    ErrorOccurred(Error),
}

#[derive(Default, Debug)]
struct App {
    document: Option<document::Document>,
    cmdline: cmdline::Cmdline,
    popup: Popup,
    notification_area: NotificationArea,

    theme: Option<iced::Theme>,

    errors: Vec<Error>,
}

#[derive(Debug, Default)]
enum Popup {
    #[default]
    None,
    Info,
    Errors,
}

#[derive(Debug, Default)]
enum NotificationArea {
    #[default]
    None,
    Info(&'static str),
    Error(String),
    Cmdline,
}

pub fn run(filename: Option<PathBuf>) -> Result<()> {
    iced::application(
        move || {
            let filename = filename.clone();
            let mut tasks = vec![Task::done(Message::SetTheme(iced::Theme::Nord))];

            if let Some(filename) = filename {
                tasks.push(Task::done(Message::OpenFile(filename)));
            }

            (App::default(), Task::batch(tasks))
        },
        App::update,
        App::view,
    )
    .subscription(App::subscription)
    .theme(App::theme)
    .title(App::title)
    .resizable(true)
    .centered()
    .run()
    .context(crate::error::IcedSnafu)
}

impl App {
    fn title(&self) -> String {
        match self.document.as_ref() {
            Some(doc) => doc.title(),
            None => "DocV",
        }
        .to_string()
    }

    fn update(&mut self, msg: Message) -> Task<Message> {
        match msg {
            Message::CmdLine(msg) => self.cmdline.update(msg),
            Message::Document(msg) => match self.document.as_mut() {
                Some(doc) => doc.update(msg),
                None => Task::none(),
            },
            Message::OpenFile(filepath) => Task::perform(
                async move { Document::read_from_path(filepath) },
                |res| match res {
                    Ok(doc) => Message::DocumentReady(doc),
                    Err(err) => Message::ErrorOccurred(err),
                },
            ),
            Message::DocumentReady(doc) => {
                self.document = Some(doc);

                Task::none()
            }
            Message::Quit => iced::exit(),
            Message::ShowErrors => {
                if !self.errors.is_empty() {
                    self.popup = Popup::Errors;
                } else {
                    self.notification_area = NotificationArea::Info("No errors");
                }

                iced::widget::operation::focus_previous()
            }
            Message::ShowInfo => {
                self.popup = Popup::Info;

                iced::widget::operation::focus_previous()
            }
            Message::ShowCmdline => {
                self.notification_area = NotificationArea::Cmdline;

                self.cmdline.focus().map(Message::CmdLine)
            }
            Message::ErrorOccurred(error) => {
                self.notification_area = NotificationArea::Error(format!("{error}"));

                self.errors.push(error);

                Task::none()
            }
            Message::CleanScreen => {
                self.popup = Popup::None;
                self.notification_area = NotificationArea::None;

                iced::widget::operation::focus_previous()
            }
            Message::SetTheme(theme) => {
                self.theme = Some(theme);

                Task::none()
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let document = self
            .document
            .as_ref()
            .map(|doc| doc.view().map(Message::Document))
            .unwrap_or_else(|| {
                container(text("No file opened").style(text::primary))
                    .padding(20)
                    .into()
            });

        let main_view = container(document).height(Length::Fill).width(Length::Fill);

        let info = container(self.notification_area.view(self))
            .align_bottom(Length::Fill)
            .width(Length::Fill);

        let popup = container(
            container(container(self.popup.view(self)).style(container::rounded_box))
                .padding(40)
                .align_x(Horizontal::Center)
                .align_y(Vertical::Center),
        )
        .height(Length::Fill)
        .width(Length::Fill);

        stack![main_view, popup, info].into()
    }

    fn subscription(&self) -> Subscription<Message> {
        keyboard::listen().filter_map(|event| match event {
            keyboard::Event::KeyPressed { modified_key, .. } => {
                if modified_key == Key::Character(":".into()) {
                    return Some(Message::ShowCmdline);
                }
                if modified_key == Key::Named(Named::Escape) {
                    return Some(Message::CleanScreen);
                }
                None
            }
            _ => None,
        })
    }

    fn theme(&self) -> Option<iced::Theme> {
        self.theme.clone()
    }
}

impl Popup {
    fn view<'a>(&'a self, app: &'a App) -> Element<'a, Message> {
        match self {
            Popup::None => column![].into(),
            Popup::Info => match app.document.as_ref() {
                Some(doc) => doc.view_info().map(Message::Document),
                None => column![].into(),
            },
            Popup::Errors => {
                scrollable(
                    column(app.errors.iter().map(|err| {
                        container(text!("{:#?}", snafu::Report::from_error(err))).into()
                    }))
                    .width(Length::Fill)
                    .padding(10),
                )
                .into()
            }
        }
    }
}

impl NotificationArea {
    fn view<'a>(&'a self, app: &'a App) -> Element<'a, Message> {
        match self {
            NotificationArea::None => row![].into(),
            NotificationArea::Info(msg) => text!("{msg}")
                .align_y(Alignment::Center)
                .style(text::secondary)
                .into(),
            NotificationArea::Error(err) => text!("{err}")
                .align_y(Alignment::Center)
                .style(text::danger)
                .into(),
            NotificationArea::Cmdline => app.cmdline.view().map(Message::CmdLine),
        }
    }
}
