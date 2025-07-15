use std::sync::LazyLock;
use regex::Regex;
use crate::id::Id;

static KEY_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^    key: (.*)$").expect("Failed to compile key regex")
});

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
        match self {
            LocStringParser::Start if line.starts_with("---") => {
                LocStringParser::File
            },
            LocStringParser::File if line == "MonoBehaviour:" => {
                println!("MonoBehaviour detected");
                LocStringParser::MonoBehaviour
            },
            LocStringParser::MonoBehaviour if line == "  m_Script: {fileID: 11500000, guid: 05503c2c5cf7b7f45bec1113802f99a0, type: 3}" => {
                println!("Localized text detected");
                LocStringParser::LocalizedText
            },
            LocStringParser::LocalizedText if line == "  localizedString:" => {
                LocStringParser::LocalizedString
            },
            LocStringParser::LocalizedString if line == "    keepUnlocalized: 1" => {
                LocStringParser::LocStringNoKey
            },
            LocStringParser::LocalizedString => {
                if let Some(captures) = KEY_RE.captures(line)
                    && let Some(m) = captures.get(1)
                {
                    LocStringParser::LocStringKey(Id::Loc(m.as_str().into()))
                }
                else {
                    LocStringParser::LocalizedString
                }
            },
            _ => self,
        }
    }
}