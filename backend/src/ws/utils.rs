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
