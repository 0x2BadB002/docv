use std::{
    fs::File,
    io::{BufReader, Seek},
    path::PathBuf,
};

use pest::{iterators::Pair, Parser};
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "pdf2.pest"]
struct PDFParser;

#[derive(thiserror::Error, Debug)]
pub enum ParserError {
    #[error(transparent)]
    IOError(#[from] std::io::Error),
    #[error(transparent)]
    GrammarError(#[from] Box<pest::error::Error<Rule>>),
    #[error(transparent)]
    IntConvError(#[from] std::num::ParseIntError),
    #[error("Unexpected token {0}")]
    InvalidStartXref(String),
    #[error("Unexpected token {0}")]
    InvalidXref(String),
}

#[derive(Debug, Default)]
pub struct PDFFile {
    xref_table: XrefTable,
    size: usize,
    path: PathBuf,
}

#[derive(Debug, Default)]
pub struct XrefTable {
    entries: Vec<XrefEntry>,
    offset: usize,
}

#[derive(Debug, Default)]
pub struct XrefEntry {
    offset: usize,
    gen_num: usize,
    occupied: bool,
}

impl PDFFile {
    pub fn from_path(path: PathBuf) -> Self {
        Self {
            path,
            xref_table: XrefTable::default(),
            size: 0,
        }
    }

    pub fn read(&mut self) -> Result<(), ParserError> {
        let file = File::open(&self.path)?;

        self.size = file.metadata()?.len() as usize;

        let mut reader = BufReader::new(file);

        let xref_offset = self.read_startxref(&mut reader)?;
        self.read_info(&mut reader, xref_offset)?;

        Ok(())
    }

    fn read_startxref<T>(&mut self, reader: &mut T) -> Result<usize, ParserError>
    where
        T: Seek,
        T: std::io::Read,
    {
        let offset = ((self.size as f64).log10().floor() + 1.0) as usize + 23;

        reader.seek(std::io::SeekFrom::End(-(offset as i64)))?;

        let mut buff = vec![0u8; offset];
        reader.read_exact(&mut buff)?;
        let buff = unsafe { String::from_utf8_unchecked(buff) };

        let token = PDFParser::parse(Rule::startxref, &buff)
            .map_err(Box::new)?
            .next()
            .ok_or_else(|| ParserError::InvalidStartXref(buff.clone()))?;

        let data = token.to_string();
        let token = token
            .into_inner()
            .next()
            .ok_or_else(|| ParserError::InvalidStartXref(data))?;

        match token.as_rule() {
            Rule::last_xref_pos => Ok(token.as_str().parse::<usize>()?),
            _ => Err(ParserError::InvalidStartXref(token.to_string())),
        }
    }

    fn read_info<T>(&mut self, reader: &mut T, xref_offset: usize) -> Result<(), ParserError>
    where
        T: Seek,
        T: std::io::Read,
    {
        let startxref_size = 9 + ((xref_offset as f64).log10().floor() + 1.0) as usize + 5;

        self.xref_table.offset = xref_offset;
        reader.seek(std::io::SeekFrom::Start(self.xref_table.offset as u64))?;

        let offset = self.size - self.xref_table.offset - startxref_size;
        let mut buff = vec![0u8; offset];
        reader.read_exact(&mut buff)?;
        let buff = unsafe { String::from_utf8_unchecked(buff) };

        let token = PDFParser::parse(Rule::xref, &buff)
            .map_err(Box::new)?
            .next()
            .ok_or_else(|| ParserError::InvalidXref(buff.clone()))?;

        let data = token.to_string();
        let token = token
            .into_inner()
            .next()
            .ok_or_else(|| ParserError::InvalidXref(data))?;

        match token.as_rule() {
            Rule::xref_old => self.parse_xref_table(token),
            Rule::stream => todo!(), // TODO: Parse xref table as a dictionary
            _ => Err(ParserError::InvalidXref(token.to_string())),
        }
    }

    fn parse_xref_table(&mut self, token: Pair<Rule>) -> Result<(), ParserError> {
        for token in token.into_inner() {
            let mut token = token.into_inner().next().unwrap().into_inner();
            let entry = XrefEntry {
                offset: token
                    .next()
                    .ok_or(ParserError::InvalidXref("Expected offset".to_string()))?
                    .as_str()
                    .parse::<usize>()?,

                gen_num: token
                    .next()
                    .ok_or(ParserError::InvalidXref("Expected gen. number".to_string()))?
                    .as_str()
                    .parse::<usize>()?,

                occupied: token
                    .next()
                    .ok_or(ParserError::InvalidXref(
                        "Expected occupied flag".to_string(),
                    ))?
                    .as_str()
                    .eq("n"),
            };

            self.xref_table.entries.push(entry);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs::{self, File},
        path::PathBuf,
        sync::LazyLock,
    };

    use super::*;

    static EXAMPLES: LazyLock<PathBuf> = LazyLock::new(|| {
        let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        dir.pop();
        dir.push("examples");
        dir
    });

    #[test]
    fn parse_boolean() {
        struct Case {
            test: &'static str,
            should_pass: bool,
        }
        let cases = [
            Case {
                test: "true",
                should_pass: true,
            },
            Case {
                test: "false",
                should_pass: true,
            },
            Case {
                test: " true",
                should_pass: false,
            },
            Case {
                test: " false",
                should_pass: false,
            },
            Case {
                test: "fale",
                should_pass: false,
            },
            Case {
                test: "tue",
                should_pass: false,
            },
        ];

        for case in cases {
            if case.should_pass {
                let mut data =
                    PDFParser::parse(Rule::boolean, case.test).expect("Failed to parse bool.");
                assert_eq!(data.next().unwrap().as_str(), case.test);
            } else {
                PDFParser::parse(Rule::boolean, case.test).expect_err("Parsed incorrect bool.");
            }
        }
    }

    #[test]
    fn parse_numeric() {
        #[derive(Debug)]
        struct Case {
            test: &'static str,
            expected_rule: Option<Rule>,
            should_pass: bool,
        }
        let cases = [
            Case {
                test: "+1",
                should_pass: true,
                expected_rule: Some(Rule::integer),
            },
            Case {
                test: "11234",
                should_pass: true,
                expected_rule: Some(Rule::integer),
            },
            Case {
                test: "0",
                should_pass: true,
                expected_rule: Some(Rule::integer),
            },
            Case {
                test: "-10",
                should_pass: true,
                expected_rule: Some(Rule::integer),
            },
            Case {
                test: "0.0",
                should_pass: true,
                expected_rule: Some(Rule::real),
            },
            Case {
                test: "+0.1",
                should_pass: true,
                expected_rule: Some(Rule::real),
            },
            Case {
                test: "-3.45",
                should_pass: true,
                expected_rule: Some(Rule::real),
            },
            Case {
                test: "-343242342.34238324",
                should_pass: true,
                expected_rule: Some(Rule::real),
            },
            Case {
                test: "-.20",
                should_pass: true,
                expected_rule: Some(Rule::real),
            },
            Case {
                test: "4.",
                should_pass: true,
                expected_rule: Some(Rule::real),
            },
            Case {
                test: "+-.0",
                should_pass: false,
                expected_rule: None,
            },
            Case {
                test: ".",
                should_pass: false,
                expected_rule: None,
            },
            Case {
                test: " 1234",
                should_pass: false,
                expected_rule: None,
            },
            Case {
                test: " -1.23344",
                should_pass: false,
                expected_rule: None,
            },
            Case {
                test: "",
                should_pass: false,
                expected_rule: None,
            },
            Case {
                test: "..2",
                should_pass: false,
                expected_rule: None,
            },
        ];

        const TEST_RULE: Rule = Rule::numeric;
        for case in cases {
            if case.should_pass {
                let mut data = PDFParser::parse(TEST_RULE, case.test)
                    .expect(format!("Failed to parse numeric {}", case.test).as_str());
                let el = data.next().unwrap();
                assert_eq!(
                    el.as_str(),
                    case.test,
                    "Case {} parsed incorrectly",
                    case.test
                );
                assert_ne!(
                    case.expected_rule, None,
                    "Test case {:#?} written incorrectly.",
                    case
                );
                assert_eq!(
                    el.clone()
                        .into_inner()
                        .next()
                        .expect(format!("Expected existing inner rule in match {:#?}", el).as_str())
                        .as_rule(),
                    case.expected_rule.unwrap()
                );
            } else {
                assert_eq!(
                    case.expected_rule, None,
                    "Test case {:#?} written incorrectly.",
                    case
                );
                PDFParser::parse(TEST_RULE, case.test)
                    .expect_err(format!("Parser should fail to parse case {}", case.test).as_str());
            }
        }
    }

    #[test]
    fn parse_string() {
        #[derive(Debug)]
        struct Case {
            test: &'static str,
            expected_rule: Option<Rule>,
            should_pass: bool,
        }
        let cases = [
            Case {
                test: "<4E6F762073686D6F7A206B6120706F702E>",
                expected_rule: Some(Rule::hex_string),
                should_pass: true,
            },
            Case {
                test: "< 901FA3 >",
                expected_rule: Some(Rule::hex_string),
                should_pass: true,
            },
            Case {
                test: "< 90 1FA>",
                expected_rule: Some(Rule::hex_string),
                should_pass: true,
            },
            Case {
                test: "<ffff0000222d>",
                expected_rule: Some(Rule::hex_string),
                should_pass: true,
            },
            Case {
                test: "<>",
                expected_rule: Some(Rule::hex_string),
                should_pass: true,
            },
            Case {
                test: "(test_string)",
                expected_rule: Some(Rule::literal_string),
                should_pass: true,
            },
            Case {
                test: "(sdfnsnkjs1293i342349)",
                expected_rule: Some(Rule::literal_string),
                should_pass: true,
            },
            Case {
                test: "(fs\nsdfs\nffsf)",
                expected_rule: Some(Rule::literal_string),
                should_pass: true,
            },
            Case {
                test: "(())",
                expected_rule: Some(Rule::literal_string),
                should_pass: true,
            },
            Case {
                test: "(()(test)())",
                expected_rule: Some(Rule::literal_string),
                should_pass: true,
            },
            Case {
                test: "((test)\n(str))",
                expected_rule: Some(Rule::literal_string),
                should_pass: true,
            },
            Case {
                test: "( The following \x66 is an empty string . )",
                expected_rule: Some(Rule::literal_string),
                should_pass: true,
            },
            Case {
                test: "()",
                expected_rule: Some(Rule::literal_string),
                should_pass: true,
            },
            Case {
                test: "(It has zero ( 0 ) length .\n)",
                expected_rule: Some(Rule::literal_string),
                should_pass: true,
            },
            Case {
                test: "( Strings may contain balanced parentheses ( ) and \n special characters ( * ! & } ^ % and so on ) . )",
                expected_rule: Some(Rule::literal_string),
                should_pass: true,
            },
            Case {
                test: "(()",
                expected_rule: None,
                should_pass: false,
            },
            Case {
                test: "<ffffvg0000222d>",
                expected_rule: None,
                should_pass: false,
            },
        ];

        const TEST_RULE: Rule = Rule::string;
        for case in cases {
            if case.should_pass {
                let mut data = PDFParser::parse(TEST_RULE, case.test)
                    .map_err(|err| {
                        eprintln!("{}", err.to_string());
                        err
                    })
                    .expect(format!("Failed to parse string {}", case.test).as_str());
                let el = data.next().unwrap();
                assert_eq!(el.as_str(), case.test);
                assert_ne!(
                    case.expected_rule, None,
                    "Test case {:#?} written incorrectly.",
                    case
                );
                assert_eq!(
                    el.clone()
                        .into_inner()
                        .next()
                        .expect(format!("Expected existing inner rule in match {:#?}", el).as_str())
                        .as_rule(),
                    case.expected_rule.unwrap()
                );
            } else {
                assert_eq!(
                    case.expected_rule, None,
                    "Test case {:#?} written incorrectly.",
                    case
                );
                PDFParser::parse(TEST_RULE, case.test)
                    .map(|res| eprintln!("{:#?}", res))
                    .expect_err(format!("Error case {} passed", case.test).as_str());
            }
        }
    }

    #[test]
    fn parse_null() {
        struct Case {
            test: &'static str,
            should_pass: bool,
        }
        let cases = [
            Case {
                test: "null",
                should_pass: true,
            },
            Case {
                test: "nll ",
                should_pass: false,
            },
            Case {
                test: " null",
                should_pass: false,
            },
            Case {
                test: "",
                should_pass: false,
            },
        ];

        const TEST_RULE: Rule = Rule::null;
        for case in cases {
            if case.should_pass {
                let mut data = PDFParser::parse(TEST_RULE, case.test)
                    .map_err(|err| eprintln!("{}", err.to_string()))
                    .expect(format!("Case {} failed", case.test).as_str());
                assert_eq!(
                    data.next().unwrap().as_str(),
                    case.test,
                    "Case {} parsed incorrectly",
                    case.test
                );
            } else {
                PDFParser::parse(TEST_RULE, case.test)
                    .map(|res| eprintln!("{:#?}", res))
                    .expect_err(format!("Error case {} passed.", case.test).as_str());
            }
        }
    }

    #[test]
    fn parse_name() {
        struct Case {
            test: &'static str,
            should_pass: bool,
        }
        let cases = [
            Case {
                test: "/Name1",
                should_pass: true,
            },
            Case {
                test: "/ASomewhatLongerName",
                should_pass: true,
            },
            Case {
                test: "/A;Name_With-Various***Characters?",
                should_pass: true,
            },
            Case {
                test: "/1.2",
                should_pass: true,
            },
            Case {
                test: "/$$",
                should_pass: true,
            },
            Case {
                test: "/@pattern",
                should_pass: true,
            },
            Case {
                test: "/.notdef",
                should_pass: true,
            },
            Case {
                test: "/lime#20Green",
                should_pass: true,
            },
            Case {
                test: "/paired#28#29parentheses",
                should_pass: true,
            },
            Case {
                test: "/The_Key_of_F#23_Minor",
                should_pass: true,
            },
            Case {
                test: "/A#42",
                should_pass: true,
            },
            Case {
                test: "/",
                should_pass: true,
            },
        ];

        const TEST_RULE: Rule = Rule::name;
        for case in cases {
            if case.should_pass {
                let mut data = PDFParser::parse(TEST_RULE, case.test)
                    .map_err(|err| eprintln!("{}", err.to_string()))
                    .expect(format!("Case {} failed", case.test).as_str());
                assert_eq!(
                    data.next().unwrap().as_str(),
                    case.test,
                    "Case {} parsed incorrectly",
                    case.test
                );
            } else {
                PDFParser::parse(TEST_RULE, case.test)
                    .map(|res| eprintln!("{:#?}", res))
                    .expect_err(format!("Error case {} passed.", case.test).as_str());
            }
        }
    }

    #[test]
    fn parse_array() {
        #[derive(Debug)]
        struct Case {
            test: &'static str,
            should_pass: bool,
            expected_content_rules: Option<Vec<Rule>>,
        }
        let cases = [
            Case {
                test: "[]",
                should_pass: true,
                expected_content_rules: Some(vec![]),
            },
            Case {
                test: "[   \n \t   ]",
                should_pass: true,
                expected_content_rules: Some(vec![]),
            },
            Case {
                test: "[+1.1]",
                should_pass: true,
                expected_content_rules: Some(vec![Rule::numeric]),
            },
            Case {
                test: "[  \n1    ]",
                should_pass: true,
                expected_content_rules: Some(vec![Rule::numeric]),
            },
            Case {
                test: "[(test_str)]",
                should_pass: true,
                expected_content_rules: Some(vec![Rule::string]),
            },
            Case {
                test: "[(test_\nstr2)]",
                should_pass: true,
                expected_content_rules: Some(vec![Rule::string]),
            },
            Case {
                test: "[ 549 3.14 false ( Ralph ) /SomeName ]",
                should_pass: true,
                expected_content_rules: Some(vec![
                    Rule::numeric,
                    Rule::numeric,
                    Rule::boolean,
                    Rule::string,
                    Rule::name,
                ]),
            },
            Case {
                test: "[ [ ]   ]",
                should_pass: true,
                expected_content_rules: Some(vec![Rule::array]),
            },
            Case {
                test: "[ / null / ]",
                should_pass: true,
                expected_content_rules: Some(vec![Rule::name, Rule::null, Rule::name]),
            },
            Case {
                test: "[ [ ] null  ]",
                should_pass: true,
                expected_content_rules: Some(vec![Rule::array, Rule::null]),
            },
            Case {
                test: "[ [ [] ]   ]",
                should_pass: true,
                expected_content_rules: Some(vec![Rule::array]),
            },
            Case {
                test: "[ [ [] ]",
                should_pass: false,
                expected_content_rules: None,
            },
        ];

        const TEST_RULE: Rule = Rule::array;
        for case in cases {
            if case.should_pass {
                let mut data = PDFParser::parse(TEST_RULE, case.test)
                    .map_err(|err| eprintln!("{}", err.to_string()))
                    .expect(format!("Case {} failed", case.test).as_str());
                let data = data.next().unwrap();
                assert_eq!(
                    data.as_str(),
                    case.test,
                    "Case {} parsed incorrectly",
                    case.test
                );
                assert_ne!(
                    case.expected_content_rules, None,
                    "Case written incorrectly. \n{:#?}",
                    case
                );

                let expected_rules = case.expected_content_rules.unwrap();
                let result_rules = data.into_inner();
                assert_eq!(
                    result_rules.len(),
                    expected_rules.len(),
                    "parsed array len differs from expected"
                );

                for (object, expected_rule) in result_rules.zip(expected_rules.iter()) {
                    let object = object.into_inner().next().expect("object is empty.");
                    assert_eq!(object.as_rule(), *expected_rule);
                }
            } else {
                assert_eq!(
                    case.expected_content_rules, None,
                    "Case {:#?} written incorrectly.",
                    case
                );
                PDFParser::parse(TEST_RULE, case.test)
                    .map(|res| eprintln!("{:#?}", res))
                    .expect_err(format!("Error case {} passed.", case.test).as_str());
            }
        }
    }

    #[test]
    fn parse_dictionary() {
        #[derive(Debug)]
        struct Case {
            test: &'static str,
            should_pass: bool,
            expected_content_rules: Option<Vec<(Rule, Rule)>>,
        }
        let cases = [
            Case {
                test: "<<>>",
                should_pass: true,
                expected_content_rules: Some(vec![]),
            },
            Case {
                test: "<< /test  \n (With whitespace)>>",
                should_pass: true,
                expected_content_rules: Some(vec![(Rule::name, Rule::string)]),
            },
            Case {
                test: "<</ />>",
                should_pass: true,
                expected_content_rules: Some(vec![(Rule::name, Rule::name)]),
            },
            Case {
                test: "<</Multiple <</Dictionaries <</test <00ffaa> >> >> >>",
                should_pass: true,
                expected_content_rules: Some(vec![(Rule::name, Rule::dictionary)]),
            },
            Case {
                test: "<</Type /Example /Subtype /DictionaryExample /Version 0.01 /IntegerItem 12 /StringItem ( a string ) >>",
                should_pass: true,
                expected_content_rules: Some(vec![
                    (Rule::name, Rule::name),
                    (Rule::name, Rule::name),
                    (Rule::name, Rule::numeric),
                    (Rule::name, Rule::numeric),
                    (Rule::name, Rule::string)
                ]),
            },
            Case {
                test: "<<>",
                should_pass: false,
                expected_content_rules: None,
            },
            Case {
                test: "<< fasdfsdf >>",
                should_pass: false,
                expected_content_rules: None,
            },
            Case {
                test: "<</Type /Example /Subtype >>",
                should_pass: false,
                expected_content_rules: None,
            },
        ];

        const TEST_RULE: Rule = Rule::dictionary;
        for case in cases {
            if case.should_pass {
                assert_ne!(
                    case.expected_content_rules, None,
                    "Case written incorrectly. \n{:#?}",
                    case
                );
                let mut data = PDFParser::parse(TEST_RULE, case.test)
                    .map_err(|err| eprintln!("{}", err.to_string()))
                    .expect(format!("Case {} failed", case.test).as_str());
                let data = data.next().unwrap();
                assert_eq!(
                    data.as_str(),
                    case.test,
                    "Case {} parsed incorrectly",
                    case.test
                );

                let expected_rules = case.expected_content_rules.unwrap();
                let result_rules = data.into_inner();
                assert_eq!(
                    result_rules.len(),
                    expected_rules.len(),
                    "Case {}: parsed array len differs from expected",
                    case.test
                );

                for (result, expected) in result_rules.zip(expected_rules.iter()) {
                    let mut result = result.into_inner();
                    assert_eq!(
                        result
                            .next()
                            .expect("Result pair don't have name")
                            .as_rule(),
                        expected.0
                    );
                    assert_eq!(
                        result
                            .next()
                            .expect("Result pair don't have value")
                            .into_inner()
                            .next()
                            .expect("Result value object don't have inner children")
                            .as_rule(),
                        expected.1
                    );
                }
            } else {
                assert_eq!(
                    case.expected_content_rules, None,
                    "Case {:#?} written incorrectly",
                    case
                );
                PDFParser::parse(TEST_RULE, case.test)
                    .map(|res| eprintln!("{:#?}", res))
                    .expect_err(format!("Error case {} passed.", case.test).as_str());
            }
        }
    }

    #[test]
    fn parse_stream() {
        #[derive(Debug)]
        struct Case {
            test: &'static str,
            should_pass: bool,
            expected_content: Option<&'static str>,
        }
        let cases = [
            Case {
                test: "<<>>stream\nendstream",
                should_pass: true,
                expected_content: Some(""),
            },
            Case {
                test: "<</test1 /name>>stream\r\nSome data...\nendstream",
                should_pass: true,
                expected_content: Some("Some data...\n"),
            },
            Case {
                test: "<</test2 /name>>\n\nstream\r\nSome data...endstream",
                should_pass: true,
                expected_content: Some("Some data..."),
            },
            Case {
                test: "<</test3 /name>>\n  stream\r\nSome\n data   \x3f...endstream",
                should_pass: true,
                expected_content: Some("Some\n data   \x3f..."),
            },
            Case {
                test: "<<>>stream\rendstream",
                should_pass: false,
                expected_content: None,
            },
        ];

        const TEST_RULE: Rule = Rule::stream;
        for case in cases {
            if case.should_pass {
                assert_ne!(
                    case.expected_content, None,
                    "Case {:#?} written incorrectly",
                    case
                );
                let mut data = PDFParser::parse(TEST_RULE, case.test)
                    .map_err(|err| eprintln!("{}", err.to_string()))
                    .expect(format!("Case {} failed", case.test).as_str());
                let data = data.next().unwrap();
                assert_eq!(
                    data.as_str(),
                    case.test,
                    "Case {} parsed incorrectly",
                    case.test
                );

                let expected_content = case.expected_content.unwrap();
                let mut result = data.into_inner();
                result.next().expect("Result stream don't have dictionary");
                let result_content = result.next().expect("Result stream don't have data");

                assert_eq!(
                    result_content.as_rule(),
                    Rule::stream_data,
                    "Result_content is not stream_data. Got = {:#?}",
                    result_content
                );
                assert_eq!(
                    result_content.as_str(),
                    expected_content,
                    "Stream data differs from expected. Got = {:#?}, expected = {}",
                    result_content,
                    expected_content
                );
            } else {
                assert_eq!(
                    case.expected_content, None,
                    "Case {:#?} written incorrectly",
                    case
                );
                PDFParser::parse(TEST_RULE, case.test)
                    .map(|res| eprintln!("{:#?}", res))
                    .expect_err(format!("Error case {} passed.", case.test).as_str());
            }
        }
    }

    #[test]
    fn parse_indirect_definition() {
        #[derive(Debug)]
        struct Case {
            test: &'static str,
            should_pass: bool,
            expected_object: Option<Rule>,
        }
        let cases = [
            Case {
                test: "1 1 obj null endobj",
                should_pass: true,
                expected_object: Some(Rule::null),
            },
            Case {
                test: "1\n 2 obj \n (Test2) \r\n endobj",
                should_pass: true,
                expected_object: Some(Rule::string),
            },
            Case {
                test: "1\n obj \n (Test3) \r\n endobj",
                should_pass: false,
                expected_object: None,
            },
            Case {
                test: "1\n1 obej \n (Test4) \r\n endobj",
                should_pass: false,
                expected_object: None,
            },
        ];

        const TEST_RULE: Rule = Rule::indirect_definition;
        for case in cases {
            if case.should_pass {
                assert_ne!(
                    case.expected_object, None,
                    "Case {:#?} written incorrectly",
                    case
                );
                let mut data = PDFParser::parse(TEST_RULE, case.test)
                    .map_err(|err| eprintln!("{}", err.to_string()))
                    .expect(format!("Case {} failed", case.test).as_str());
                let data = data.next().unwrap();
                assert_eq!(
                    data.as_str(),
                    case.test,
                    "Case {} parsed incorrectly",
                    case.test
                );

                let mut data = data.into_inner();
                data.next()
                    .expect("indirect_definition don't have integer identifiers");
                data.next()
                    .expect("indirect_definition don't have second integer identifier");
                let object = data
                    .next()
                    .expect("indirect_definition don't have child object");
                let expected_object = case.expected_object.unwrap();
                assert_eq!(
                    object
                        .into_inner()
                        .next()
                        .expect("indirect_definition object don't have child")
                        .as_rule(),
                    expected_object
                );
            } else {
                assert_eq!(
                    case.expected_object, None,
                    "Case {:#?} written incorrectly",
                    case
                );
                PDFParser::parse(TEST_RULE, case.test)
                    .map(|res| eprintln!("{:#?}", res))
                    .expect_err(format!("Error case {} passed.", case.test).as_str());
            }
        }
    }

    #[test]
    fn parse_indirect_reference() {
        #[derive(Debug)]
        struct Case {
            test: &'static str,
            should_pass: bool,
        }
        let cases = [
            Case {
                test: "1 1 R",
                should_pass: true,
            },
            Case {
                test: "1\n 2 \r\n R",
                should_pass: true,
            },
            Case {
                test: "1\n R",
                should_pass: false,
            },
            Case {
                test: "1\n1",
                should_pass: false,
            },
        ];

        const TEST_RULE: Rule = Rule::indirect_reference;
        for case in cases {
            if case.should_pass {
                let mut data = PDFParser::parse(TEST_RULE, case.test)
                    .map_err(|err| eprintln!("{}", err.to_string()))
                    .expect(format!("Case {} failed", case.test).as_str());
                let data = data.next().unwrap();
                assert_eq!(
                    data.as_str(),
                    case.test,
                    "Case {} parsed incorrectly",
                    case.test
                );
            } else {
                PDFParser::parse(TEST_RULE, case.test)
                    .map(|res| eprintln!("{:#?}", res))
                    .expect_err(format!("Error case {} passed.", case.test).as_str());
            }
        }
    }

    #[test]
    fn read_example_files_startxref() {
        for example in fs::read_dir(EXAMPLES.clone()).expect("Failed to read examples dir.") {
            let example = example.unwrap();
            eprintln!("Reading file {}...", example.path().display());

            let mut pdf_file = PDFFile::from_path(example.path());

            let file = File::open(&pdf_file.path).expect("Failed to open file");

            pdf_file.size = file.metadata().expect("Failed to get file metadata").len() as usize;

            let mut reader = BufReader::new(file);

            pdf_file
                .read_startxref(&mut reader)
                .map_err(|err| {
                    eprintln!("Working with file {}", example.path().display());
                    eprintln!("{}", err.to_string());
                    err
                })
                .expect("Failed to parse startxref");
        }
    }

    #[test]
    fn read_example_files_info() {
        for example in fs::read_dir(EXAMPLES.clone()).expect("Failed to read examples dir.") {
            let example = example.unwrap();
            eprintln!("Reading file {}...", example.path().display());

            let mut pdf_file = PDFFile::from_path(example.path());

            let file = File::open(&pdf_file.path).expect("Failed to open file");

            pdf_file.size = file.metadata().expect("Failed to get file metadata").len() as usize;

            let mut reader = BufReader::new(file);

            let offset = pdf_file
                .read_startxref(&mut reader)
                .map_err(|err| {
                    eprintln!("Working with file {}", example.path().display());
                    eprintln!("{}", err.to_string());
                    err
                })
                .expect("Failed to parse startxref");

            pdf_file
                .read_info(&mut reader, offset)
                .map_err(|err| {
                    eprintln!("Working with file {}", example.path().display());
                    eprintln!("{}", err.to_string());
                    err
                })
                .expect("Failed to read info");
        }
    }

    #[test]
    fn read_example_files() {
        for example in fs::read_dir(EXAMPLES.clone()).expect("Failed to read examples dir.") {
            let example = example.unwrap();
            eprintln!("Reading file {}...", example.path().display());

            let mut pdf_file = PDFFile::from_path(example.path());

            pdf_file
                .read()
                .map_err(|err| {
                    eprintln!("Working with file {}", example.path().display());
                    eprintln!("{}", err.to_string());
                    err
                })
                .expect("Failed to read file");
        }
    }
}
