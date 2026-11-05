use crate::hash40;

#[derive(Clone, Debug, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Json>),
    Object(Vec<(String, Json)>),
}

impl Json {
    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Object(fields) => fields.iter().rev().find(|(name, _)| name == key).map(|(_, value)| value),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JsonError {
    pub offset: usize,
    pub message: &'static str,
}

struct Parser<'a> {
    bytes: &'a [u8],
    at: usize,
}

const MAX_DEPTH: usize = 64;

impl<'a> Parser<'a> {
    fn fail<T>(&self, message: &'static str) -> Result<T, JsonError> {
        Err(JsonError { offset: self.at, message })
    }

    fn skip_space(&mut self) {
        while let Some(byte) = self.bytes.get(self.at) {
            if matches!(byte, b' ' | b'\t' | b'\n' | b'\r') {
                self.at += 1;
            } else {
                break;
            }
        }
    }

    fn eat(&mut self, byte: u8) -> bool {
        if self.bytes.get(self.at) == Some(&byte) {
            self.at += 1;
            true
        } else {
            false
        }
    }

    fn literal(&mut self, word: &[u8], value: Json) -> Result<Json, JsonError> {
        if self.bytes[self.at..].starts_with(word) {
            self.at += word.len();
            Ok(value)
        } else {
            self.fail("unknown literal")
        }
    }

    fn value(&mut self, depth: usize) -> Result<Json, JsonError> {
        if depth > MAX_DEPTH {
            return self.fail("nested too deep");
        }
        self.skip_space();
        match self.bytes.get(self.at) {
            None => self.fail("unexpected end"),
            Some(b'{') => self.object(depth),
            Some(b'[') => self.array(depth),
            Some(b'"') => Ok(Json::String(self.string()?)),
            Some(b't') => self.literal(b"true", Json::Bool(true)),
            Some(b'f') => self.literal(b"false", Json::Bool(false)),
            Some(b'n') => self.literal(b"null", Json::Null),
            Some(b'-' | b'0'..=b'9') => self.number(),
            Some(_) => self.fail("unexpected character"),
        }
    }

    fn object(&mut self, depth: usize) -> Result<Json, JsonError> {
        self.at += 1;
        let mut fields = Vec::new();
        self.skip_space();
        if self.eat(b'}') {
            return Ok(Json::Object(fields));
        }
        loop {
            self.skip_space();
            if self.bytes.get(self.at) != Some(&b'"') {
                return self.fail("expected a key");
            }
            let key = self.string()?;
            self.skip_space();
            if !self.eat(b':') {
                return self.fail("expected ':'");
            }
            let value = self.value(depth + 1)?;
            fields.push((key, value));
            self.skip_space();
            if self.eat(b',') {
                continue;
            }
            if self.eat(b'}') {
                return Ok(Json::Object(fields));
            }
            return self.fail("expected ',' or '}'");
        }
    }

    fn array(&mut self, depth: usize) -> Result<Json, JsonError> {
        self.at += 1;
        let mut items = Vec::new();
        self.skip_space();
        if self.eat(b']') {
            return Ok(Json::Array(items));
        }
        loop {
            items.push(self.value(depth + 1)?);
            self.skip_space();
            if self.eat(b',') {
                continue;
            }
            if self.eat(b']') {
                return Ok(Json::Array(items));
            }
            return self.fail("expected ',' or ']'");
        }
    }

    fn hex4(&mut self) -> Result<u32, JsonError> {
        let digits = self.bytes.get(self.at..self.at + 4).ok_or(JsonError {
            offset: self.at,
            message: "short \\u escape",
        })?;
        let text = core::str::from_utf8(digits).map_err(|_| JsonError {
            offset: self.at,
            message: "bad \\u escape",
        })?;
        let value = u32::from_str_radix(text, 16).map_err(|_| JsonError {
            offset: self.at,
            message: "bad \\u escape",
        })?;
        self.at += 4;
        Ok(value)
    }

    fn string(&mut self) -> Result<String, JsonError> {
        self.at += 1;
        let mut out = Vec::new();
        loop {
            let Some(&byte) = self.bytes.get(self.at) else {
                return self.fail("unterminated string");
            };
            self.at += 1;
            match byte {
                b'"' => break,
                b'\\' => {
                    let Some(&escape) = self.bytes.get(self.at) else {
                        return self.fail("unterminated escape");
                    };
                    self.at += 1;
                    match escape {
                        b'"' => out.push(b'"'),
                        b'\\' => out.push(b'\\'),
                        b'/' => out.push(b'/'),
                        b'b' => out.push(0x08),
                        b'f' => out.push(0x0c),
                        b'n' => out.push(b'\n'),
                        b'r' => out.push(b'\r'),
                        b't' => out.push(b'\t'),
                        b'u' => {
                            let mut code = self.hex4()?;
                            if (0xd800..0xdc00).contains(&code) {
                                if !(self.eat(b'\\') && self.eat(b'u')) {
                                    return self.fail("lone surrogate");
                                }
                                let low = self.hex4()?;
                                if !(0xdc00..0xe000).contains(&low) {
                                    return self.fail("lone surrogate");
                                }
                                code = 0x10000 + ((code - 0xd800) << 10) + (low - 0xdc00);
                            }
                            let Some(character) = char::from_u32(code) else {
                                return self.fail("bad \\u escape");
                            };
                            let mut buffer = [0u8; 4];
                            out.extend_from_slice(character.encode_utf8(&mut buffer).as_bytes());
                        }
                        _ => return self.fail("unknown escape"),
                    }
                }
                0x00..=0x1f => return self.fail("control character in string"),
                _ => out.push(byte),
            }
        }
        String::from_utf8(out).map_err(|_| JsonError {
            offset: self.at,
            message: "string is not UTF-8",
        })
    }

    fn number(&mut self) -> Result<Json, JsonError> {
        let start = self.at;
        self.eat(b'-');
        if self.bytes.get(self.at) == Some(&b'0') && matches!(self.bytes.get(self.at + 1), Some(b'0'..=b'9')) {
            return self.fail("leading zero");
        }
        let digits = |parser: &mut Parser| {
            let from = parser.at;
            while matches!(parser.bytes.get(parser.at), Some(b'0'..=b'9')) {
                parser.at += 1;
            }
            parser.at > from
        };
        if !digits(self) {
            return self.fail("expected digits");
        }
        if self.eat(b'.') && !digits(self) {
            return self.fail("expected digits after '.'");
        }
        if self.eat(b'e') || self.eat(b'E') {
            if !self.eat(b'+') {
                self.eat(b'-');
            }
            if !digits(self) {
                return self.fail("expected exponent digits");
            }
        }
        let text = core::str::from_utf8(&self.bytes[start..self.at]).unwrap_or("");
        text.parse::<f64>()
            .map(Json::Number)
            .map_err(|_| JsonError { offset: start, message: "bad number" })
    }
}

pub fn parse(text: &str) -> Result<Json, JsonError> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut parser = Parser { bytes: text.as_bytes(), at: 0 };
    let value = parser.value(0)?;
    parser.skip_space();
    if parser.at != parser.bytes.len() {
        return parser.fail("trailing data");
    }
    Ok(value)
}

pub fn hash_value(value: &Json) -> Option<u64> {
    match value {
        Json::String(text) => match text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
            Some(hex) => u64::from_str_radix(hex, 16).ok(),
            None => Some(hash40(text)),
        },
        Json::Number(number) if *number >= 0.0 && number.fract() == 0.0 => Some(*number as u64),
        _ => None,
    }
}

fn signed_byte(value: &Json) -> Result<Option<i8>, ()> {
    let number = match value {
        Json::Number(number) => *number,
        Json::Object(_) => match value.get("Overwrite").or_else(|| value.get("Optional")) {
            Some(Json::Number(number)) => *number,
            Some(Json::Null) => return Ok(None),
            _ => return Err(()),
        },
        Json::Null => return Ok(None),
        _ => return Err(()),
    };
    if number.fract() != 0.0 || number < i8::MIN as f64 || number > i8::MAX as f64 {
        return Err(());
    }
    Ok(Some(number as i8))
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CharaOrders {
    pub orders: Vec<(u64, i8)>,
    pub refused: Vec<String>,
}

pub fn chara_disp_orders(text: &str) -> Result<CharaOrders, JsonError> {
    let root = parse(text)?;
    let mut out = CharaOrders::default();
    let Some(Json::Array(entries)) = root.get("character_database_entries") else {
        return Ok(out);
    };
    for entry in entries {
        let Some(order) = entry.get("disp_order") else {
            continue;
        };
        let Some(ui_chara) = entry.get("ui_chara_id").and_then(hash_value) else {
            out.refused.push("an entry with disp_order has no usable ui_chara_id".to_string());
            continue;
        };
        match signed_byte(order) {
            Ok(Some(order)) => {
                out.orders.retain(|(known, _)| *known != ui_chara);
                out.orders.push((ui_chara, order));
            }
            Ok(None) => {}
            Err(()) => out.refused.push(format!("{ui_chara:#x}: disp_order {order:?} is not a whole number from -128 to 127")),
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_csk_schema_with_labels_and_plain_values() {
        let text = r#"{
            "series_database_entries": [{"ui_series_id": "ui_series_brainage", "disp_order": 0}],
            "character_database_entries": [
                {"ui_chara_id": "ui_chara_kawashima", "disp_order": 12},
                {"ui_chara_id": "0x0FEDCBA987", "disp_order": -1},
                {"ui_chara_id": "ui_chara_mecha", "name_id": "mecha"}
            ]
        }"#;
        let orders = chara_disp_orders(text).unwrap();
        assert_eq!(
            orders.orders,
            vec![(hash40("ui_chara_kawashima"), 12), (0xfedcba987, -1)]
        );
        assert!(orders.refused.is_empty());
    }

    #[test]
    fn a_later_entry_for_the_same_row_wins() {
        let text = r#"{"character_database_entries": [
            {"ui_chara_id": "ui_chara_x", "disp_order": 3},
            {"ui_chara_id": "ui_chara_x", "disp_order": 9}
        ]}"#;
        assert_eq!(chara_disp_orders(text).unwrap().orders, vec![(hash40("ui_chara_x"), 9)]);
    }

    #[test]
    fn accepts_tagged_values_and_null() {
        let text = r#"{"character_database_entries": [
            {"ui_chara_id": "ui_chara_a", "disp_order": {"Overwrite": 5}},
            {"ui_chara_id": "ui_chara_b", "disp_order": {"Optional": 6}},
            {"ui_chara_id": "ui_chara_c", "disp_order": null}
        ]}"#;
        assert_eq!(
            chara_disp_orders(text).unwrap().orders,
            vec![(hash40("ui_chara_a"), 5), (hash40("ui_chara_b"), 6)]
        );
    }

    #[test]
    fn out_of_range_and_fractional_orders_are_refused_not_wrapped() {
        let text = r#"{"character_database_entries": [
            {"ui_chara_id": "ui_chara_a", "disp_order": 200},
            {"ui_chara_id": "ui_chara_b", "disp_order": 1.5},
            {"disp_order": 4}
        ]}"#;
        let orders = chara_disp_orders(text).unwrap();
        assert!(orders.orders.is_empty());
        assert_eq!(orders.refused.len(), 3);
    }

    #[test]
    fn files_without_character_entries_give_nothing() {
        assert_eq!(chara_disp_orders("{}").unwrap(), CharaOrders::default());
        assert_eq!(chara_disp_orders("[1, 2]").unwrap(), CharaOrders::default());
    }

    #[test]
    fn strict_like_serde_json() {
        assert!(parse(r#"{"a": 1,}"#).is_err());
        assert!(parse(r#"{"a": 1} x"#).is_err());
        assert!(parse(r#"{'a': 1}"#).is_err());
        assert!(parse("{\"a\": \"line\nbreak\"}").is_err());
        assert!(parse(r#"{"a": 01}"#).is_err());
        assert!(parse(r#"{"a": -0}"#).is_ok());
    }

    #[test]
    fn strings_numbers_and_bom() {
        let value = parse("\u{feff}{\"s\": \"a\\\"b\\\\c\\u00e9\\ud83d\\ude00\", \"n\": -1.25e2, \"t\": [true, false, null]}").unwrap();
        assert_eq!(value.get("s"), Some(&Json::String("a\"b\\c\u{e9}\u{1f600}".to_string())));
        assert_eq!(value.get("n"), Some(&Json::Number(-125.0)));
        assert_eq!(
            value.get("t"),
            Some(&Json::Array(vec![Json::Bool(true), Json::Bool(false), Json::Null]))
        );
    }

    #[test]
    fn depth_is_bounded() {
        let deep = "[".repeat(MAX_DEPTH + 2) + &"]".repeat(MAX_DEPTH + 2);
        assert!(parse(&deep).is_err());
    }

    #[test]
    fn real_pack_files_parse() {
        let springtrap = r#"{
    "series_database_entries": [
        {
            "ui_series_id": "ui_series_five_nights_at_freddys",
            "clone_from_series_id": "ui_series_mario",
            "name_id": "five_nights_at_freddys",
            "disp_order": 0,
            "disp_order_sound": 100,
            "dlc_chara_id": "",
            "is_use_amiibo_bg": false
        }
    ],
    "gametitle_database_entries": [
        {"ui_gametitle_id": "ui_gametitle_x", "test_disp_order": 15000, "record_type": "record_original"}
    ]
}"#;
        assert_eq!(chara_disp_orders(springtrap).unwrap(), CharaOrders::default());
    }
}
