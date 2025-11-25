use std::path::PathBuf;

use iced::{
    Alignment, Element, Length, Subscription, Task, Theme,
    alignment::{Horizontal, Vertical},
    keyboard::{self, Key, Modifiers, key::Named},
    widget::{column, container, horizontal_space, row, scrollable, stack, text, vertical_space},
};

use crate::{Error, Result, app::document::Document};

mod cmdline;
mod document;

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

    theme: iced::Theme,

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
    iced::application(App::title, App::update, App::view)
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

                iced::widget::focus_previous()
            }
            Message::ShowInfo => {
                self.popup = Popup::Info;

                iced::widget::focus_previous()
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

                iced::widget::focus_previous()
            }
            Message::SetTheme(theme) => {
                self.theme = theme;

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
            column![
                vertical_space(),
                row![
                    horizontal_space(),
                    container(self.popup.view(self))
                        .style(container::rounded_box)
                        .width(Length::FillPortion(8)),
                    horizontal_space(),
                ]
                .height(Length::FillPortion(8))
                .align_y(Vertical::Center),
                vertical_space(),
            ]
            .height(Length::Fill)
            .width(Length::Fill)
            .align_x(Horizontal::Center),
        )
        .height(Length::Fill)
        .width(Length::Fill)
        .align_x(Horizontal::Center);

        stack![main_view, popup, info].into()
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            keyboard::on_key_press(|key, key_mod| match key.as_ref() {
                Key::Character(key) => {
                    if key == ";" && key_mod == Modifiers::SHIFT {
                        return Some(Message::ShowCmdline);
                    }
                    None
                }
                Key::Named(Named::Escape) => Some(Message::CleanScreen),
                _ => None,
            }),
            keyboard::on_key_release(|key, _| match key.as_ref() {
                Key::Named(Named::Escape) => Some(Message::CleanScreen),
                _ => None,
            }),
        ])
    }

    fn theme(&self) -> iced::Theme {
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
            Popup::Errors => scrollable(
                column(
                    app.errors
                        .iter()
                        .map(|err| container(text!("{:#?}", err)).into()),
                )
                .width(Length::Fill)
                .padding(10),
            )
            .into(),
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
