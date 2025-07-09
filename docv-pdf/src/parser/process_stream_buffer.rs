use crate::{Error, Result};

pub fn process_bytes(data: Vec<u8>) -> Result<(String, Vec<Vec<u8>>)> {
    let mut position = 0;

    let mut output = String::new();
    let mut chunks = Vec::new();

    while position < data.len() {
        let pos = data[position..]
            .windows(6)
            .position(|window| window == b"stream");

        if pos.is_none() {
            let valid_slice = &data[position..];
            let valid_str = std::str::from_utf8(valid_slice)?;
            output.push_str(valid_str);
            break;
        }

        let pos = pos.unwrap();
        let stream_start = position + pos;

        if stream_start > position {
            let valid_slice = &data[position..stream_start];
            let valid_str = std::str::from_utf8(valid_slice)?;
            output.push_str(valid_str);
        }

        output.push_str("stream");
        position = stream_start + 6;

        handle_endline_markers(&data, &mut position, &mut output);

        let pos = data[position..]
            .windows(9)
            .position(|window| window == b"endstream")
            .ok_or(Error::InvalidStreamDeclaration("endstream".to_string()))?;

        let mut endstream_start = position + pos;
        while data[endstream_start - 1] == b'\n' || data[endstream_start - 1] == b'\r' {
            endstream_start -= 1;
        }

        let chunk = &data[position..endstream_start];

        let id = format!("{{ID{}}}", chunks.len());
        chunks.push(chunk.to_vec());

        output.push_str(&id);

        position = endstream_start;

        while data[position] == b'\r' || data[position] == b'\n' {
            output.push(data[position] as char);
            position += 1;
        }

        output.push_str("endstream");
        position += 9;

        handle_endline_markers(&data, &mut position, &mut output);
    }

    Ok((output, chunks))
}

fn handle_endline_markers(data: &[u8], position: &mut usize, output: &mut String) {
    if *position >= data.len() {
        return;
    }

    match data[*position] {
        b'\r' => {
            output.push('\r');
            *position += 1;
            if *position < data.len() && data[*position] == b'\n' {
                output.push('\n');
                *position += 1;
            }
        }
        b'\n' => {
            output.push('\n');
            *position += 1;
        }
        _ => (),
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_process_bytes() {
        #[derive(Debug)]
        struct Case {
            data: &'static [u8],
            should_pass: bool,
            expected_output: Option<&'static str>,
            expected_data: Option<Vec<Vec<u8>>>,
            description: &'static str,
        }

        let cases = vec![
            Case {
                data: b"Hello stream\r\nThis is a test \nendstream world!",
                should_pass: true,
                expected_output: Some("Hello stream\r\n{ID0}\nendstream world!"),
                expected_data: Some(vec![b"This is a test ".to_vec()]),
                description: "Normal case with stream and endstream",
            },
            Case {
                data: b"Data before stream some data without end marker",
                should_pass: false,
                expected_output: Some("Data before stream some data without end marker"),
                expected_data: Some(vec![]),
                description: "Missing endstream after stream",
            },
            Case {
                data: b"Valid text \xFF\xFE stream data endstream",
                should_pass: false,
                expected_output: None,
                expected_data: None,
                description: "Invalid UTF-8 outside streams",
            },
            Case {
                data: b"Just some normal text without any markers.",
                should_pass: true,
                expected_output: Some("Just some normal text without any markers."),
                expected_data: Some(vec![]),
                description: "Data with no stream markers",
            },
            Case {
                data: b"First stream\r\nData1 \nendstream between stream\nData2 \nendstream end.",
                should_pass: true,
                expected_output: Some(
                    "First stream\r\n{ID0}\nendstream between stream\n{ID1}\nendstream end.",
                ),
                expected_data: Some(vec![b"Data1 ".to_vec(), b"Data2 ".to_vec()]),
                description: "Multiple streams in data",
            },
            Case {
                data: b"Line1\nstream\r\nData\r\nendstream\rLine2\r\n",
                should_pass: true,
                expected_output: Some("Line1\nstream\r\n{ID0}\r\nendstream\rLine2\r\n"),
                expected_data: Some(vec![b"Data".to_vec()]),
                description: "Handling CRLF and LF after markers",
            },
            Case {
                data: b"stream\r\n\xFF\xFE\r\nendstream",
                should_pass: true,
                expected_output: Some("stream\r\n{ID0}\r\nendstream"),
                expected_data: Some(vec![b"\xFF\xFE".to_vec()]),
                description: "Invalid UTF-8 inside stream chunk",
            },
            Case {
                data: b"",
                should_pass: true,
                expected_output: Some(""),
                expected_data: Some(vec![]),
                description: "Empty data",
            },
        ];

        for case in cases {
            let result = process_bytes(case.data.to_vec());

            if case.should_pass {
                assert!(
                    result.is_ok(),
                    "Unexpected error {} in test {}",
                    result.unwrap_err(),
                    case.description
                );

                let (output, chunks) = result.unwrap();

                assert_eq!(
                    output,
                    case.expected_output.unwrap(),
                    "Output mismatch in test '{}'",
                    case.description
                );
                assert_eq!(
                    chunks,
                    case.expected_data.unwrap(),
                    "Chunks mismatch in test '{}'",
                    case.description
                );
            } else {
                assert!(
                    result.is_err(),
                    "Test {} passed but it shouldn't",
                    case.description
                );
            }
        }
    }
}
