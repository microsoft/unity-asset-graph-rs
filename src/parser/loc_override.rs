use regex::Regex;
use std::sync::LazyLock;

static VALUE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^      value: (.+)$").expect("Failed to compile value regex"));

#[derive(Debug, Clone)]
pub enum LocOverrideParser {
    Start,
    File,
    PrefabInstance,
    Modification,
    Modifications,
    PropertyPath,
    PropertyValue(String),
}

impl LocOverrideParser {
    pub fn update(self, line: &str) -> Self {
        match self {
            Self::Start if line.starts_with("---") => Self::File,
            Self::File if line == "PrefabInstance:" => Self::PrefabInstance,
            Self::PrefabInstance if line == "  m_Modification:" => Self::Modification,
            Self::Modification if line == "    m_Modifications:" => Self::Modifications,
            Self::Modifications if line == "      propertyPath: localizedString.key" => Self::PropertyPath,
            Self::PropertyPath => {
                if let Some(captures) = VALUE_RE.captures(line)
                    && let Some(m) = captures.get(1)
                {
                    Self::PropertyValue(m.as_str().into())
                } else {
                    Self::PropertyPath
                }
            }
            _ => {
                if line.starts_with("---") {
                    Self::File
                } else {
                    self
                }
            }
        }
    }
}
