use actix_web::HttpRequest;

use crate::constants::http;
use crate::http_errors::HttpErrors;

pub(super) type KeyValueVec = Vec<(String, String)>;

pub fn parse_protocol_header(req: &HttpRequest) -> Result<(Vec<String>, KeyValueVec), HttpErrors> {
    let mut key_value: Vec<(String, String)> = vec![];
    let mut protocols: Vec<String> = vec![];
    let Some(header) = req.headers().get(http::SEC_WEBSOCKET_PROTOCOL_HEADER) else {
        return Err(HttpErrors::NoTokenProvided);
    };
    let header = header.to_str().map_err(|_| HttpErrors::NoTokenProvided)?;

    for elem in header
        .split(',')
        .map(str::trim)
        .filter(|elem| !elem.is_empty())
    {
        if let Some((key, value)) = elem.split_once('.') {
            if key.is_empty() || value.is_empty() {
                return Err(HttpErrors::NoTokenProvided);
            }
            key_value.push((key.to_owned(), value.to_owned()));
        } else {
            protocols.push(elem.to_owned());
        }
    }

    Ok((protocols, key_value))
}

#[cfg(test)]
mod tests {
    use super::parse_protocol_header;
    use crate::constants::http;
    use crate::http_errors::HttpErrors;
    use actix_web::test::TestRequest;

    #[test]
    fn parses_protocols_and_key_value_tokens() {
        let request = TestRequest::default()
            .insert_header((
                http::SEC_WEBSOCKET_PROTOCOL_HEADER,
                "auth.jwt, password.secret, ping",
            ))
            .to_http_request();

        let (protocols, key_values) = parse_protocol_header(&request).unwrap();
        assert_eq!(protocols, vec!["ping"]);
        assert_eq!(
            key_values,
            vec![
                ("auth".to_owned(), "jwt".to_owned()),
                ("password".to_owned(), "secret".to_owned()),
            ]
        );
    }

    #[test]
    fn rejects_missing_or_malformed_tokens() {
        let missing = TestRequest::default().to_http_request();
        assert!(matches!(
            parse_protocol_header(&missing),
            Err(HttpErrors::NoTokenProvided)
        ));

        let malformed = TestRequest::default()
            .insert_header((http::SEC_WEBSOCKET_PROTOCOL_HEADER, "auth."))
            .to_http_request();
        assert!(matches!(
            parse_protocol_header(&malformed),
            Err(HttpErrors::NoTokenProvided)
        ));
    }
}
