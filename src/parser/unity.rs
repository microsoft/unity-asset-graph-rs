use std::{
    io::BufRead,
    path::PathBuf,
    sync::LazyLock,
};
use regex::Regex;
use uuid::Uuid;
use crate::{
    asset::Asset,
    id::Id,
    parser::{
        loc_text::LocStringParser,
        loc_manager::LocManagerParser,
        ParseError,
    },
};

static ID_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b([0-9a-f]{32})\b").expect("Failed to compile ID regex")
});

pub fn parse_unity(asset: &mut Asset, relative_to: Option<&PathBuf>) -> Result<Vec<Asset>, ParseError> {
    let path = match relative_to {
        Some(rel) => &rel.join(&asset.path),
        None => &asset.path,
    };

    let mut reader = match crate::util::read_file_no_bom(path) {
        Ok(file) => file,
        Err(e) => return Err(ParseError {
            message: format!("Failed to read prefab file: {}", e),
            inner: None,
        }),
    };

    parse_unity_reader(&mut reader, asset)
}

fn parse_unity_reader(reader: &mut dyn BufRead, asset: &mut Asset) -> Result<Vec<Asset>, ParseError> {
    let mut loctext_parser = LocStringParser::Start;
    let mut locmgr_parser = LocManagerParser::Start;

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => return Err(ParseError {
                message: format!("Failed to read line: {}", e),
                inner: None,
            }),
        };

        loctext_parser = loctext_parser.update(&line);
        if let LocStringParser::LocStringKey(id) = loctext_parser {
            asset.dependencies.insert(id);
            loctext_parser = LocStringParser::Start;
        }

        locmgr_parser = locmgr_parser.update(&line);
        if let LocManagerParser::Names(names) = locmgr_parser {
            asset.loc_roots = names;
            locmgr_parser = LocManagerParser::Start;
        }

        if let Some(captures) = ID_REGEX.captures(&line)
            && let Some(id_str) = captures.get(1)
            && let Ok(uuid) = Uuid::parse_str(id_str.as_str())
        {
            asset.dependencies.insert(Id::Guid(uuid));
        }
    }

    Ok(vec![])
}

#[cfg(test)]
mod test {
    use std::io::BufReader;

    use super::*;

    const PREFAB: &str = r#"%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!1 &3561149108886604914 stripped
GameObject:
  m_CorrespondingSourceObject: {fileID: 2746916251226383531, guid: 7c77678171dd7a24ead5c598179e6378,
    type: 3}
  m_PrefabInstance: {fileID: 1690186840591182041}
  m_PrefabAsset: {fileID: 0}
--- !u!114 &4538655452972157934
MonoBehaviour:
  m_ObjectHideFlags: 0
  m_CorrespondingSourceObject: {fileID: 0}
  m_PrefabInstance: {fileID: 0}
  m_PrefabAsset: {fileID: 0}
  m_GameObject: {fileID: 3561149108886604914}
  m_Enabled: 1
  m_EditorHideFlags: 0
  m_Script: {fileID: 11500000, guid: 05503c2c5cf7b7f45bec1113802f99a0, type: 3}
  m_Name: 
  m_EditorClassIdentifier: 
  localizedString:
    keepUnlocalized: 0
    key: people_panel_people_label
  canvasText: {fileID: 7690883829489752066}
  tmpText: {fileID: 0}
  accessibilityComponent: {fileID: 0}
  allowEmptyKey: 0
  OnTextChanged:
    m_PersistentCalls:
      m_Calls: []
"#;

    #[test]
    fn test_parse_unity_reader() {
        let mut reader = BufReader::new(PREFAB.as_bytes());
        let mut asset = Asset::new_with_path(Id::Guid(Uuid::nil()), PathBuf::from("test.prefab"));
        let result = parse_unity_reader(&mut reader, &mut asset);

        assert!(result.is_ok());
        assert!(asset.dependencies.contains(&Id::Guid(Uuid::parse_str("7c77678171dd7a24ead5c598179e6378").unwrap())));
        assert!(asset.dependencies.contains(&Id::Guid(Uuid::parse_str("05503c2c5cf7b7f45bec1113802f99a0").unwrap())));
        assert!(asset.dependencies.contains(&Id::Loc("people_panel_people_label".into())));
    }
}