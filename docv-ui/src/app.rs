use std::{
    env,
    path::{Path, PathBuf},
    sync::Arc,
};

use ashpd::desktop::file_chooser::{FileFilter, SelectedFiles};
use iced::{
    Alignment, Element, Length, Subscription, Task,
    alignment::Horizontal,
    keyboard::{self, Key, key::Named},
    widget::{button, column, container, row, scrollable, stack, text},
};
use snafu::{OptionExt, ResultExt};

use crate::{Error, Result, app::document::Document};

pub mod cmdline;
pub mod document;

#[derive(Debug, Clone)]
pub enum Message {
    CmdLine(cmdline::Message),
    Document(document::Message),

    OpenFile(Option<Arc<Path>>),
    DocumentReady(Document),

    SetTheme(iced::Theme),
    Quit,
    ShowErrors,
    ShowInfo,
    ShowCmdline,
    CleanScreen,

    ErrorOccurred(Arc<Error>),
}

#[derive(Default, Debug)]
pub struct App {
    document: Option<document::Document>,
    cmdline: cmdline::Cmdline,
    popup: Popup,
    action_area: ActionArea,

    theme: Option<iced::Theme>,

    errors: Vec<Arc<Error>>,
}

#[derive(Debug, Default)]
enum Popup {
    #[default]
    None,
    Info,
    Errors,
}

#[derive(Debug, Default)]
enum ActionArea {
    #[default]
    None,
    Info(&'static str),
    Error(String),
    Cmdline,
}

pub fn run(filename: Option<PathBuf>) -> Result<()> {
    let boot = move || {
        let file = filename
            .as_ref()
            .map(|path| Document::read_from_path(path))
            .transpose()
            .map_err(|err| crate::error::Error::Document { source: err });

        let (file, error_task) = if let Ok(file) = file {
            (file, Task::none())
        } else {
            (
                None,
                Task::done(Message::ErrorOccurred(file.unwrap_err().into())),
            )
        };

        (
            App {
                document: file,
                theme: Some(iced::Theme::Nord),
                ..Default::default()
            },
            error_task,
        )
    };

    Ok(iced::application(boot, App::update, App::view)
        .subscription(App::subscription)
        .theme(App::theme)
        .title(App::title)
        .resizable(true)
        .centered()
        .run()
        .context(crate::error::Iced)?)
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
            Message::OpenFile(filepath) => {
                if filepath.is_some() {
                    Task::perform(
                        async move {
                            Document::read_from_path(&filepath.unwrap())
                                .context(crate::error::Document)
                        },
                        |res| match res {
                            Ok(doc) => Message::DocumentReady(doc),
                            Err(err) => Message::ErrorOccurred(err.into()),
                        },
                    )
                } else {
                    Task::perform(get_file_with_dialog(), |res| match res {
                        Ok(doc) => Message::DocumentReady(doc),
                        Err(err) => Message::ErrorOccurred(err.into()),
                    })
                }
            }
            Message::DocumentReady(doc) => {
                self.errors.clear();
                self.document = Some(doc);

                Task::none()
            }
            Message::Quit => iced::exit(),
            Message::ShowErrors => {
                if !self.errors.is_empty() {
                    self.popup = Popup::Errors;
                } else {
                    self.action_area = ActionArea::Info("No errors");
                }

                iced::widget::operation::focus_previous()
            }
            Message::ShowInfo => {
                self.popup = Popup::Info;

                iced::widget::operation::focus_previous()
            }
            Message::ShowCmdline => {
                self.action_area = ActionArea::Cmdline;

                self.cmdline.focus().map(Message::CmdLine)
            }
            Message::ErrorOccurred(error) => {
                self.action_area = ActionArea::Error(error.to_string());

                self.errors.push(error);

                Task::none()
            }
            Message::CleanScreen => {
                self.popup = Popup::None;
                self.action_area = ActionArea::None;

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
                container(
                    column![
                        text("No file opened").style(text::primary),
                        button(container(text("   Open file...   ")).padding(3))
                            .style(button::primary)
                            .on_press_with(|| Message::OpenFile(None))
                    ]
                    .spacing(10)
                    .align_x(Horizontal::Center),
                )
                .center(Length::Fill)
                .padding(20)
                .into()
            });

        let status_line = match self.document.as_ref() {
            Some(doc) => {
                let current_page =
                    container(text!("  {}/{}  ", doc.current_page(), doc.page_count))
                        .center_y(Length::Fill)
                        .height(Length::Fill);

                let current_file = container(text!("  {}  ", doc.filename))
                    .style(container::success)
                    .center_y(Length::Fill)
                    .height(Length::Fill);

                container(row![current_page, current_file].spacing(4))
                    .center_y(Length::Fill)
                    .style(container::secondary)
                    .width(Length::Fill)
                    .height(30)
            }
            None => container(row![]),
        };

        let action_line = container(self.action_area.view(self))
            .center_y(Length::Fill)
            .width(Length::Fill)
            .height(30);

        let status_area = container(column![status_line, action_line]).height(Length::Shrink);

        let main_view = column![document, status_area].height(Length::Fill);

        let popup = container(
            container(container(self.popup.view(self)).style(container::rounded_box))
                .padding(40)
                .center(Length::Fill),
        )
        .height(Length::Fill)
        .width(Length::Fill);

        stack![main_view, popup].into()
    }

    fn subscription(&self) -> Subscription<Message> {
        let mut subscriptions = vec![keyboard::listen().filter_map(|event| match event {
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
        })];

        if self.document.is_some() {
            subscriptions.push(
                self.document
                    .as_ref()
                    .unwrap()
                    .subscription()
                    .map(Message::Document),
            );
        }

        if let ActionArea::Error(_) = self.action_area {
            subscriptions
                .push(iced::time::every(iced::time::seconds(5)).map(|_| Message::CleanScreen));
        }

        Subscription::batch(subscriptions)
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
            Popup::Errors => scrollable(
                column(app.errors.iter().map(|err| {
                    container(column![
                        text!("{:#?}", snafu::Report::from_error(err)),
                        iced::widget::rule::horizontal(2)
                    ])
                    .into()
                }))
                .spacing(4)
                .width(Length::Fill)
                .padding(10),
            )
            .into(),
        }
    }
}

impl ActionArea {
    fn view<'a>(&'a self, app: &'a App) -> Element<'a, Message> {
        match self {
            ActionArea::None => row![].into(),
            ActionArea::Info(msg) => text!(" {msg}")
                .align_y(Alignment::Center)
                .style(text::secondary)
                .into(),
            ActionArea::Error(err) => text!(" {err}")
                .align_y(Alignment::Center)
                .style(text::danger)
                .into(),
            ActionArea::Cmdline => app.cmdline.view().map(Message::CmdLine),
        }
    }
}

async fn get_file_with_dialog() -> Result<Document> {
    let file = SelectedFiles::open_file()
        .title("open a file to read")
        .accept_label("read")
        .current_folder(env::current_dir().unwrap_or(".".into()))
        .context(crate::error::ModalDialog)?
        .modal(true)
        .filter(FileFilter::new("PDF Document").mimetype("application/pdf"))
        .send()
        .await
        .context(crate::error::ModalDialog)?
        .response()
        .context(crate::error::ModalDialog)?;

    let path = file
        .uris()
        .iter()
        .next()
        .context(crate::error::NoFile)?
        .to_file_path()
        .ok()
        .context(crate::error::Path)?;

    Ok(Document::read_from_path(&path).context(crate::error::Document)?)
}
