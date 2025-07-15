use std::sync::LazyLock;
use regex::Regex;

static RESOURCE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^  - (.*)$").expect("Failed to compile key regex")
});

#[derive(Debug, Clone)]
pub enum LocManagerParser {
    Start,
    File,
    MonoBehaviour,
    LocManager,
    NamesWip(Vec<String>),
    Names(Vec<String>),
}

impl LocManagerParser {
    pub fn update(self, line: &str) -> Self {
        let _orig = self.clone();
        let ret = match self {
            LocManagerParser::Start if line.starts_with("---") => {
                LocManagerParser::File
            },
            LocManagerParser::File if line == "MonoBehaviour:" => {
                LocManagerParser::MonoBehaviour
            },
            LocManagerParser::MonoBehaviour if line == "  m_Script: {fileID: 11500000, guid: 65ce276a26b28f94b8abe63deebd2050, type: 3}" => {
                LocManagerParser::LocManager
            },
            LocManagerParser::LocManager if line == "  resourceNameBases:" => {
                LocManagerParser::NamesWip(vec![])
            },
            LocManagerParser::NamesWip(mut names) => {
                if let Some(captures) = RESOURCE_RE.captures(line)
                    && let Some(m) = captures.get(1)
                {
                    names.push(m.as_str().into());
                    LocManagerParser::NamesWip(names)
                }
                else {
                    LocManagerParser::Names(names)
                }
            },
            _ => self,
        };
        
        //println!("{:?} -> {:?}: {line}", &_orig, &ret);
        ret
    }
}

// #[cfg(test)]
// mod test {
//     use super::*;

//     const PREFAB: &str = r#"%YAML 1.1
// %TAG !u! tag:unity3d.com,2011:
// --- !u!1 &3561149108886604914 stripped
// GameObject:
//   m_CorrespondingSourceObject: {fileID: 2746916251226383531, guid: 7c77678171dd7a24ead5c598179e6378,
//     type: 3}
//   m_PrefabInstance: {fileID: 1690186840591182041}
//   m_PrefabAsset: {fileID: 0}
// --- !u!114 &4538655452972157934
// MonoBehaviour:
//   m_ObjectHideFlags: 0
//   m_CorrespondingSourceObject: {fileID: 0}
//   m_PrefabInstance: {fileID: 0}
//   m_PrefabAsset: {fileID: 0}
//   m_GameObject: {fileID: 3561149108886604914}
//   m_Enabled: 1
//   m_EditorHideFlags: 0
//   m_Script: {fileID: 11500000, guid: 05503c2c5cf7b7f45bec1113802f99a0, type: 3}
//   m_Name: 
//   m_EditorClassIdentifier: 
//   localizedString:
//     keepUnlocalized: 0
//     key: people_panel_people_label
//   canvasText: {fileID: 7690883829489752066}
//   tmpText: {fileID: 0}
//   accessibilityComponent: {fileID: 0}
//   allowEmptyKey: 0
//   OnTextChanged:
//     m_PersistentCalls:
//       m_Calls: []
// "#;

//     const PREFAB_OVERRIDE: &str = r#"%YAML 1.1
// %TAG !u! tag:unity3d.com,2011:
// --- !u!114 &8073721433681322272 stripped
// MonoBehaviour:
//   m_CorrespondingSourceObject: {fileID: 8229847291080121086, guid: d7698b5f08e39cc4aaf5e62e6972733b,
//     type: 3}
//   m_PrefabInstance: {fileID: 161518669942422494}
//   m_PrefabAsset: {fileID: 0}
//   m_GameObject: {fileID: 0}
//   m_Enabled: 1
//   m_EditorHideFlags: 0
//   m_Script: {fileID: 11500000, guid: 05503c2c5cf7b7f45bec1113802f99a0, type: 3}
//   m_Name: 
//   m_EditorClassIdentifier: 
// "#;

//     const PREFAB_NOKEY: &str = r#"%YAML 1.1
// %TAG !u! tag:unity3d.com,2011:
// --- !u!114 &3497495228035037355
// MonoBehaviour:
//   m_ObjectHideFlags: 0
//   m_CorrespondingSourceObject: {fileID: 0}
//   m_PrefabInstance: {fileID: 0}
//   m_PrefabAsset: {fileID: 0}
//   m_GameObject: {fileID: 8281751422924472755}
//   m_Enabled: 1
//   m_EditorHideFlags: 0
//   m_Script: {fileID: 11500000, guid: 05503c2c5cf7b7f45bec1113802f99a0, type: 3}
//   m_Name: 
//   m_EditorClassIdentifier: 
//   localizedString:
//     keepUnlocalized: 1
//     key: Tooltip
//   canvasText: {fileID: 6956349368374241523}
//   tmpText: {fileID: 0}
//   accessibilityComponent: {fileID: 0}
//   allowEmptyKey: 0
//   OnTextChanged:
//     m_PersistentCalls:
//       m_Calls: []
// "#;

//     #[test]
//     fn test_parse_locstring() {
//         let mut parser = LocManagerParser::Start;
//         for (i, line) in PREFAB.lines().enumerate() {
//             parser = parser.update(line);

//             match parser {
//                 LocManagerParser::Start if i < 2 => {},
//                 LocManagerParser::File if i < 9 => {},
//                 LocManagerParser::MonoBehaviour if i < 17 => {},
//                 LocManagerParser::LocManager if i < 20 => {},
//                 LocManagerParser::LocalizedString if i < 22 => {},
//                 LocManagerParser::LocStringKey(Id::Loc(ref key)) => {
//                     assert_eq!(key, "people_panel_people_label");
//                 },
//                 _ => panic!("Unexpected parser state: {:?} at line {i}", parser),
//             };
//         }
//     }

//     #[test]
//     fn test_parse_override() {
//         let mut parser = LocManagerParser::Start;
//         for (i, line) in PREFAB_OVERRIDE.lines().enumerate() {
//             parser = parser.update(line);

//             match parser {
//                 LocManagerParser::Start if i < 2 => {},
//                 LocManagerParser::File if i < 3 => {},
//                 LocManagerParser::MonoBehaviour if i < 11 => {},
//                 LocManagerParser::LocManager => {},
//                 _ => panic!("Unexpected parser state: {:?} at line {i}", parser),
//             };
//         }
//     }

//     #[test]
//     fn test_parse_unlocalized() {
//         let mut parser = LocManagerParser::Start;
//         for (i, line) in PREFAB_NOKEY.lines().enumerate() {
//             parser = parser.update(line);

//             match parser {
//                 LocManagerParser::Start if i < 2 => {},
//                 LocManagerParser::File if i < 3 => {},
//                 LocManagerParser::MonoBehaviour if i < 11 => {},
//                 LocManagerParser::LocManager if i < 14 => {},
//                 LocManagerParser::LocalizedString if i < 15 => {},
//                 LocManagerParser::LocStringNoKey => {},
//                 _ => panic!("Unexpected parser state: {:?} at line {i}", parser),
//             };
//         }
//     }
// }