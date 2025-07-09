use std::io::{BufRead, Seek, SeekFrom};

use pest::iterators::Pair;

use crate::{
    Error, Result,
    parser::{Rule, parse_data, parse_dictionary, parse_literal_string, parse_name},
};

#[derive(Debug, Default)]
pub enum Trapped {
    True,
    False,
    #[default]
    Unknown,
}

#[derive(Debug, Default)]
pub struct Info {
    title: Option<String>,
    author: Option<String>,
    subject: Option<String>,
    keywords: Option<String>,
    creator: Option<String>,
    producer: Option<String>,
    creation_date: Option<String>,
    modified_date: Option<String>,
    trapped: Trapped,
}

impl Info {
    pub fn read<T>(&mut self, reader: &mut T, offset: u64) -> Result<()>
    where
        T: Seek + BufRead,
    {
        reader.seek(SeekFrom::Start(offset))?;

        let mut buff = String::with_capacity(1024);
        for line in reader.lines() {
            let line = line?;
            buff += &line;
            if line.contains("endobj") {
                break;
            }
        }

        eprintln!("{}", buff);
        let token = parse_data(&buff)?;
        match token.as_rule() {
            Rule::object => self.parse_info(token),
            _ => Err(Error::InvalidObject(token.as_str().to_string())),
        }
    }

    fn parse_info(&mut self, token: Pair<Rule>) -> Result<()> {
        *self = parse_dictionary(token, |data: &mut Info, key, object| match key {
            "Title" => {
                let title = parse_literal_string(object)?;

                data.title = Some(title.to_string());

                Ok(())
            }
            "Author" => {
                let author = parse_literal_string(object)?;

                data.author = Some(author.to_string());

                Ok(())
            }
            "Subject" => {
                let subject = parse_literal_string(object)?;

                data.subject = Some(subject.to_string());

                Ok(())
            }
            "Keywords" => {
                let keywords = parse_literal_string(object)?;

                data.keywords = Some(keywords.to_string());

                Ok(())
            }
            "Creator" => {
                let creator = parse_literal_string(object)?;

                data.creator = Some(creator.to_string());

                Ok(())
            }
            "Producer" => {
                let producer = parse_literal_string(object)?;

                data.producer = Some(producer.to_string());

                Ok(())
            }
            // TODO: Creation date
            // TODO: Modification date
            "Trapped" => {
                let name = parse_name(object)?;

                match name {
                    "True" => {
                        data.trapped = Trapped::True;

                        Ok(())
                    }
                    "False" => {
                        data.trapped = Trapped::False;

                        Ok(())
                    }
                    "Unknown" => {
                        data.trapped = Trapped::Unknown;

                        Ok(())
                    }
                    _ => {
                        eprintln!("Unrecognized name: {}", name);

                        Err(Error::UnexpectedName(name.to_string()))
                    }
                }
            }
            _ => Ok(()),
        })?;

        Ok(())
    }
}
