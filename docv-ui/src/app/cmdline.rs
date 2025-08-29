use std::path::PathBuf;
use std::sync::LazyLock;

use iced::keyboard::Key;
use iced::keyboard::key::Named;
use iced::widget::{container, row, text_input};
use iced::{Element, Length, Subscription, Task};
use pest::Parser;
use pest_derive::Parser;

use crate::{Error, Result};

#[derive(Parser)]
#[grammar = "app/cmdline.pest"]
struct CmdlineParser {}

#[derive(Default, Debug)]
pub struct Cmdline {
    cmd: String,

    active: bool,
}

#[derive(Debug, Clone)]
pub enum Action {
    Quit,
    Open(PathBuf),
    ShowErrorStack,
}

#[derive(Debug, Clone)]
pub enum Message {
    Action(Action),
    OnCommandSubmit,
    OnCommandInput(String),
    HideCmdline,
    ShowCmdline,
}

static INPUT_ID: LazyLock<text_input::Id> = LazyLock::new(text_input::Id::unique);

impl Cmdline {
    pub fn update(&mut self, message: Message) -> Task<super::Message> {
        match message {
            Message::Action(action) => match action {
                Action::Quit => Task::done(super::Message::Quit),
                Action::Open(filepath) => Task::done(super::Message::OpenFile(filepath)),
                Action::ShowErrorStack => Task::done(super::Message::ShowErrors),
            },
            Message::OnCommandInput(cmd) => {
                if cmd.is_empty() {
                    return Task::done(super::Message::CmdLine(Message::HideCmdline));
                }
                self.cmd = cmd;

                Task::none()
            }
            Message::OnCommandSubmit => {
                self.active = false;

                Task::perform(parse_cmd(self.cmd.clone()), |res| match res {
                    Ok(action) => super::Message::CmdLine(Message::Action(action)),
                    Err(err) => super::Message::ErrorOccurred(err),
                })
            }
            Message::HideCmdline => {
                self.active = false;
                self.cmd.clear();

                Task::none()
            }
            Message::ShowCmdline => {
                self.active = true;
                self.cmd = String::from(":");

                text_input::focus(INPUT_ID.clone())
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        if !self.active {
            return row![].into();
        }

        container(
            text_input("", &self.cmd)
                .id(INPUT_ID.clone())
                .on_input_maybe(self.active.then_some(Message::OnCommandInput))
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
        .padding(4)
        .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        if !self.active {
            return Subscription::none();
        }

        iced::keyboard::on_key_release(|key, _| match key.as_ref() {
            Key::Named(Named::Escape) => Some(Message::HideCmdline),
            _ => None,
        })
    }

    pub fn show(&self) -> Task<Message> {
        Task::done(Message::ShowCmdline)
    }
}

async fn parse_cmd(cmd: String) -> Result<Action> {
    let mut cmd = CmdlineParser::parse(Rule::line, cmd.as_str())
        .map_err(|err| {
            tracing::error!("{}", err);
            Error::ParserError(cmd.clone())
        })?
        .next()
        .ok_or_else(|| Error::ParserError(String::from("No top token parsed")))?
        .into_inner();

    let first = cmd
        .next()
        .ok_or_else(|| Error::ParserError(String::from("No verb token parsed")))?;

    match first.as_rule() {
        Rule::verb => {
            let inner_verb = first
                .into_inner()
                .next()
                .ok_or_else(|| Error::ParserError(String::from("No inner verb parsed")))?;

            match inner_verb.as_rule() {
                Rule::quit => Ok(Action::Quit),
                Rule::open => {
                    let filename = cmd.next().ok_or_else(|| {
                        Error::ParserError(String::from("Expected filename argument"))
                    })?;

                    let path = PathBuf::from(filename.as_str());

                    Ok(Action::Open(path))
                }
                Rule::errors => Ok(Action::ShowErrorStack),
                _ => Err(Error::ParserError(String::from("Unexpected token"))),
            }
        }
        _ => Err(Error::ParserError(String::from("Unexpected token"))),
    }
}
