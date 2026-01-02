use std::path::PathBuf;
use std::sync::LazyLock;

use iced::widget::{self, container, text_input};
use iced::{Element, Length, Task};
use pest::Parser;
use pest_derive::Parser;
use snafu::{OptionExt, ResultExt, Snafu};

#[derive(Debug, Snafu)]
pub struct Error(error::Error);
type Result<T> = std::result::Result<T, Error>;

#[derive(Parser)]
#[grammar = "app/cmdline.pest"]
struct CmdlineParser {}

#[derive(Default, Debug)]
pub struct Cmdline {
    cmd: String,
}

#[derive(Debug, Clone)]
pub enum Action {
    Quit,
    Open(PathBuf),
    ShowErrors,
    ShowInfo,
    SetPage(usize),
}

#[derive(Debug, Clone)]
pub enum Message {
    Action(Action),
    OnCommandSubmit,
    OnCommandInput(String),
    FocusInput,
}

static INPUT_ID: LazyLock<widget::Id> = LazyLock::new(widget::Id::unique);

impl Cmdline {
    pub fn update(&mut self, message: Message) -> Task<crate::app::Message> {
        match message {
            Message::Action(action) => match action {
                Action::Quit => Task::done(crate::app::Message::Quit),
                Action::Open(filepath) => Task::done(crate::app::Message::OpenFile(filepath)),
                Action::ShowErrors => Task::done(crate::app::Message::ShowErrors),
                Action::ShowInfo => Task::done(crate::app::Message::ShowInfo),
                Action::SetPage(number) => Task::done(crate::app::Message::Document(
                    crate::app::document::Message::SetPageNumber(number),
                )),
            },
            Message::OnCommandInput(cmd) => {
                if cmd.is_empty() {
                    return Task::done(crate::app::Message::CleanScreen);
                }
                self.cmd = cmd;

                Task::none()
            }
            Message::OnCommandSubmit => Task::batch([
                Task::perform(parse_cmd(self.cmd.clone()), |res| match res {
                    Ok(action) => crate::app::Message::CmdLine(Message::Action(action)),
                    Err(err) => crate::app::Message::ErrorOccurred(crate::error::Error::Command {
                        source: err,
                    }),
                }),
                Task::done(crate::app::Message::CleanScreen),
            ]),
            Message::FocusInput => {
                self.cmd = String::from(":");

                widget::operation::focus(INPUT_ID.clone())
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        container(
            text_input("", &self.cmd)
                .id(INPUT_ID.clone())
                .on_input(Message::OnCommandInput)
                .on_submit(Message::OnCommandSubmit)
                .width(Length::Fill)
                .style(|theme, status| {
                    let mut style = text_input::default(theme, status);
                    if let iced::Background::Color(color) = style.background {
                        style.border = style.border.color(color).width(0.0).rounded(0.0);
                    }
                    style
                }),
        )
        .height(Length::Shrink)
        .width(Length::Fill)
        .into()
    }

    pub fn focus(&self) -> Task<Message> {
        Task::done(Message::FocusInput)
    }
}

async fn parse_cmd(cmd: String) -> Result<Action> {
    let mut cmd = CmdlineParser::parse(Rule::line, cmd.as_str())
        .map_err(|err| {
            tracing::error!("{}", err);
            error::Error::Parser {
                message: err.to_string(),
            }
        })?
        .next()
        .context(error::Parser {
            message: "No top token parsed",
        })?
        .into_inner();

    let first = cmd.next().context(error::Parser {
        message: "No verb token parsed",
    })?;

    match first.as_rule() {
        Rule::verb => {
            let inner_verb = first.into_inner().next().context(error::Parser {
                message: "No inner verb parsed",
            })?;

            match inner_verb.as_rule() {
                Rule::quit => Ok(Action::Quit),
                Rule::open => {
                    let filename = cmd.next().context(error::Parser {
                        message: "Unexpected filename argument",
                    })?;

                    let path = PathBuf::from(filename.as_str());

                    Ok(Action::Open(path))
                }
                Rule::errors => Ok(Action::ShowErrors),
                Rule::info => Ok(Action::ShowInfo),
                Rule::page_number => {
                    let number = inner_verb.as_str().parse().context(error::TypeConvertion)?;

                    Ok(Action::SetPage(number))
                }
                _ => Err(error::Error::Parser {
                    message: String::from("Unexpected token"),
                }
                .into()),
            }
        }
        _ => Err(error::Error::Parser {
            message: String::from("Unexpected token"),
        }
        .into()),
    }
}

mod error {
    use std::str::FromStr;

    use snafu::Snafu;

    #[derive(Debug, Snafu)]
    #[snafu(visibility(pub(super)), context(suffix(false)))]
    pub(super) enum Error {
        #[snafu(display("{message}"))]
        Parser { message: String },

        #[snafu(display("Can't convert type"))]
        TypeConvertion { source: <usize as FromStr>::Err },
    }
}
