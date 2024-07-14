use std::io::{BufReader, Read, Seek};

use pest::Parser;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "parser/pdf2.pest"]
struct PDFParser;

#[derive(thiserror::Error, Debug)]
pub enum ParserError {
    #[error("IO error")]
    IOError(#[from] std::io::Error),
    #[error("Parsing grammar error")]
    GrammarError(#[from] pest::error::Error<Rule>),
    #[error("Parsing usize error")]
    IntConvError(#[from] std::num::ParseIntError),
    #[error("Unexpected token")]
    InvalidStartxref(String),
}

pub fn read_startxref<T>(reader: &mut BufReader<T>, file_size: usize) -> Result<usize, ParserError>
where
    T: Seek,
    T: std::io::Read,
{
    let offset = ((file_size as f64).log10().floor() + 1.0) as usize + 23;

    reader.seek(std::io::SeekFrom::End(-(offset as i64)))?;

    let mut buff = String::with_capacity(offset);
    reader.read_to_string(&mut buff)?;

    let data = PDFParser::parse(Rule::startxref, &buff)?.next().unwrap();
    let token = data.into_inner().next().unwrap();

    match token.as_rule() {
        Rule::last_xref_pos => Ok(token.as_str().parse::<usize>()?),
        _ => Err(ParserError::InvalidStartxref(token.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs::{self, File},
        path::PathBuf,
    };

    use super::*;

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
    fn parse_example_files_startxref() {
        let examples = {
            let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            dir.pop();
            dir.push("examples");
            dir
        };

        for example in fs::read_dir(examples).expect("Failed to read examples dir.") {
            let example = example.unwrap();
            eprintln!("Reading file {}...", example.path().display());

            let file = File::open(example.path()).expect("Failed to open file");
            let size = file.metadata().expect("Failed to get file metadata").len() as usize;

            let mut reader = BufReader::new(file);

            read_startxref(&mut reader, size)
                .map_err(|err| {
                    eprintln!("Working with file {}", example.path().display());
                    eprintln!("{}", err.to_string());
                    err
                })
                .expect("Failed to parse startxref");
        }
    }
}
