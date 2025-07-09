use pest::{Parser, iterators::Pair};
use pest_derive::Parser;

use crate::{Error, Result};

#[derive(Parser)]
#[grammar = "parser/pdf2.pest"]
pub struct PDFParser;

impl From<pest::error::Error<Rule>> for Error {
    fn from(e: pest::error::Error<Rule>) -> Self {
        match e.line_col {
            pest::error::LineColLocation::Pos((line, column)) => Error::Grammar {
                line,
                column,
                reason: e.with_path(file!()).to_string(),
            },
            pest::error::LineColLocation::Span(begin, _) => Error::Grammar {
                line: begin.0,
                column: begin.1,
                reason: e.with_path(file!()).to_string(),
            },
        }
    }
}

pub fn parse_startxref(buff: &str) -> Result<Pair<Rule>> {
    PDFParser::parse(Rule::startxref, buff)?
        .next()
        .ok_or_else(|| Error::InvalidStartXref(buff.to_string()))
}

pub fn parse_xref(buff: &str) -> Result<Pair<Rule>> {
    PDFParser::parse(Rule::xref, buff)?
        .next()
        .ok_or_else(|| Error::InvalidXref(buff.to_string()))
}

pub fn parse_data(buff: &str) -> Result<Pair<Rule>> {
    PDFParser::parse(Rule::indirect_definition, buff)?
        .next()
        .ok_or_else(|| Error::InvalidObject(buff.to_string()))
}

#[cfg(test)]
mod tests {
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
                    (Rule::name, Rule::string),
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
            Case {
                test: "<</Type/XRef/ID[<b59d8432643ea048378d7332001e4dd1><b59d8432643ea048378d7332001e4dd1>]/Root\r\n1 0 R/Info 2 0 R/Size 139/W[1 3 2]/Filter/FlateDecode/Length 348>>",
                should_pass: true,
                expected_content_rules: Some(vec![
                    (Rule::name, Rule::name),
                    (Rule::name, Rule::array),
                    (Rule::name, Rule::indirect_reference),
                    (Rule::name, Rule::indirect_reference),
                    (Rule::name, Rule::numeric),
                    (Rule::name, Rule::array),
                    (Rule::name, Rule::name),
                    (Rule::name, Rule::numeric),
                ]),
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
                test: "<<>>stream\n{ID0}\nendstream",
                should_pass: true,
                expected_content: Some("{ID0}"),
            },
            Case {
                test: "<</test1 /name>>stream\r\n{ID0}\nendstream",
                should_pass: true,
                expected_content: Some("{ID0}"),
            },
            Case {
                test: "<</test2 /name>>\n\nstream\r\n{ID1}\nendstream",
                should_pass: true,
                expected_content: Some("{ID1}"),
            },
            Case {
                test: "<</test3 /name>>\n  stream\r\n{ID3}\r\nendstream",
                should_pass: true,
                expected_content: Some("{ID3}"),
            },
            Case {
                test: "<<>>stream\rendstream",
                should_pass: false,
                expected_content: None,
            },
            Case {
                test: "<<>>stream\r\rendstream",
                should_pass: false,
                expected_content: None,
            },
            Case {
                test: "<<>>stream\r\r\nendstream",
                should_pass: false,
                expected_content: None,
            },
            Case {
                test: "<<>>stream\r\n\rendstream",
                should_pass: false,
                expected_content: None,
            },
            Case {
                test: "<<>>stream\r\n{ID0}endstream",
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
    fn test_parse_startxref() {
        struct Case {
            test: &'static str,
            expected: &'static str,
            should_pass: bool,
        }
        let cases = [
            Case {
                test: "startxref1234",
                expected: "startxref1234",
                should_pass: true,
            },
            Case {
                test: "startxref0",
                expected: "startxref0",
                should_pass: true,
            },
            Case {
                test: "  \t\nstartxref9012",
                expected: "  \t\nstartxref9012",
                should_pass: true,
            },
            Case {
                test: "data_before_startxref5678",
                expected: "data_before_startxref5678",
                should_pass: true,
            },
            Case {
                test: "startxref",
                expected: "",
                should_pass: false,
            },
            Case {
                test: "startxref12a3",
                expected: "",
                should_pass: false,
            },
            Case {
                test: "startXref1234",
                expected: "",
                should_pass: false,
            },
        ];

        for case in cases {
            let result = parse_startxref(case.test);
            if case.should_pass {
                let pair = result.expect(&format!("Case '{}' should pass", case.test));
                assert_eq!(
                    pair.as_str(),
                    case.expected,
                    "Case '{}' parsed as '{}', expected '{}'",
                    case.test,
                    pair.as_str(),
                    case.expected
                );
            } else {
                assert!(
                    result.is_err(),
                    "Case '{}' should fail but passed",
                    case.test
                );
            }
        }
    }

    #[test]
    fn test_parse_xref() {
        struct Case {
            test: &'static str,
            expected: &'static str,
            should_pass: bool,
        }
        let cases = [
            Case {
                test: "xref\n0 1\n0000000000 65535 f\r\ntrailer<<>>",
                expected: "xref\n0 1\n0000000000 65535 f\r\ntrailer<<>>",
                should_pass: true,
            },
            Case {
                test: "xref\n3 2\n0000000000 65535 f\r\n0000000018 00000 n\r\ntrailer<</Size 5>>",
                expected: "xref\n3 2\n0000000000 65535 f\r\n0000000018 00000 n\r\ntrailer<</Size 5>>",
                should_pass: true,
            },
            Case {
                test: "5 0 obj <</Length 10>>stream\n{ID0}\nendstream endobj",
                expected: "5 0 obj <</Length 10>>stream\n{ID0}\nendstream endobj",
                should_pass: true,
            },
            Case {
                test: "5 0 obj <</Type/XRef>> stream\n{ID0}\nendstream endobj",
                expected: "5 0 obj <</Type/XRef>> stream\n{ID0}\nendstream endobj",
                should_pass: true,
            },
            Case {
                test: "xref\n0 1\n0000000000 65535 f\r\n",
                expected: "",
                should_pass: false,
            },
            Case {
                test: "xref\ninvalid",
                expected: "",
                should_pass: false,
            },
            Case {
                test: "trailer<<>>",
                expected: "",
                should_pass: false,
            },
            Case {
                test: "stream\nendstream",
                expected: "",
                should_pass: false,
            },
            Case {
                test: "5 0 obj ...",
                expected: "",
                should_pass: false,
            },
        ];

        for case in cases {
            let result = parse_xref(case.test);
            if case.should_pass {
                assert!(
                    result.is_ok(),
                    "Case {} should pass but failed. Err = {}",
                    case.test,
                    result.unwrap_err()
                );

                let pair = result.unwrap();
                assert_eq!(
                    pair.as_str(),
                    case.expected,
                    "Case '{}' parsed as '{}', expected '{}'",
                    case.test,
                    pair.as_str(),
                    case.expected
                );
            } else {
                assert!(
                    result.is_err(),
                    "Case '{}' should fail but passed",
                    case.test
                );
            }
        }
    }
}
