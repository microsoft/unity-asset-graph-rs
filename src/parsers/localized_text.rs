use regex::Regex;
use crate::id::Id;

pub enum LocStringParser {
    Start,
    File,
    MonoBehaviour,
    LocalizedText,
    LocalizedString,
    LocStringNoKey,
    LocStringKey(Id),
}

impl LocStringParser {
    pub fn update(self, line: &str) -> Self {
        let key_re = Regex::new(r"key: (.*)$").unwrap();
        match self {
            LocStringParser::Start if line.starts_with("---") => {
                LocStringParser::File
            },
            LocStringParser::File if line.starts_with("MonoBehaviour:") => {
                LocStringParser::MonoBehaviour
            },
            LocStringParser::MonoBehaviour if line.contains("m_Script: {fileID: 11500000, guid: 05503c2c5cf7b7f45bec1113802f99a0, type: 3}") => {
                LocStringParser::LocalizedText
            },
            LocStringParser::LocalizedText if line.contains("localizedString:") => {
                LocStringParser::LocalizedString
            },
            LocStringParser::LocalizedString if line.contains("keepUnlocalized: 1") => {
                LocStringParser::LocStringNoKey
            },
            LocStringParser::LocalizedString => {
                if let Some(captures) = key_re.captures(line)
                    && let Some(m) = captures.get(1)
                {
                    LocStringParser::LocStringKey(Id::Loc(m.as_str().to_string()))
                }
                else {
                    LocStringParser::LocalizedString
                }
            },
            _ => self,
        }
    }
}