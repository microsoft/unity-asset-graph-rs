#[cfg(feature = "locstring")]
use crate::parser::{loc_override::LocOverrideParser, loc_text::LocStringParser};
use crate::{
    asset::{Asset, Relation},
    id::Id,
    parser::ParseError,
};
use regex::Regex;
use std::{io::BufRead, path::PathBuf, sync::LazyLock};
use uuid::Uuid;

static ID_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b([0-9a-f]{32})\b").expect("Failed to compile ID regex"));

pub fn parse(asset: &mut Asset, relative_to: Option<&PathBuf>) -> Result<Vec<Asset>, ParseError> {
    let path = match relative_to {
        Some(rel) => &rel.join(asset.path.as_ref().unwrap()),
        None => asset.path.as_ref().unwrap(),
    };

    let mut reader = match crate::util::read_file_no_bom(path) {
        Ok(file) => file,
        Err(e) => {
            return Err(ParseError::new(&path, format!("Failed to read prefab file: {}", e)));
        }
    };

    parse_reader(&mut reader, asset, relative_to)
}

fn parse_reader(
    reader: &mut dyn BufRead,
    asset: &mut Asset,
    relative_to: Option<&PathBuf>,
) -> Result<Vec<Asset>, ParseError> {
    let path = match relative_to {
        Some(rel) => &rel.join(asset.path.as_ref().unwrap()),
        None => asset.path.as_ref().unwrap(),
    };

    #[cfg(feature = "locstring")]
    let mut loctext_parser = LocStringParser::Start;
    #[cfg(feature = "locstring")]
    let mut locoverride_parser = LocOverrideParser::Start;

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => return Err(ParseError::new(path, format!("Failed to read line: {}", e))),
        };

        #[cfg(feature = "locstring")]
        {
            loctext_parser = loctext_parser.update(&line);
            if let LocStringParser::LocStringKey(id) = loctext_parser {
                asset.relations.insert(Relation::Uses(id));
                loctext_parser = LocStringParser::Start;
            }

            locoverride_parser = locoverride_parser.update(&line);
            if let LocOverrideParser::PropertyValue(value) = locoverride_parser {
                asset.relations.insert(Relation::Uses(Id::Loc(value)));
                locoverride_parser = LocOverrideParser::Modifications;
            }
        }

        if let Some(captures) = ID_REGEX.captures(&line)
            && let Some(id_str) = captures.get(1)
            && let Ok(uuid) = Uuid::parse_str(id_str.as_str())
        {
            asset.relations.insert(Relation::Uses(Id::Guid(uuid)));
        }
    }

    Ok(vec![])
}

#[cfg(test)]
mod test {
    use std::io::BufReader;

    use super::*;
    use crate::asset_type::AssetType;

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
--- !u!1001 &161518669942422494
PrefabInstance:
  m_ObjectHideFlags: 0
  serializedVersion: 2
  m_Modification:
    serializedVersion: 3
    m_TransformParent: {fileID: 215340965743735207}
    m_Modifications:
    - target: {fileID: 8229847291080121086, guid: d7698b5f08e39cc4aaf5e62e6972733b,
        type: 3}
      propertyPath: localizedString.key
      value: events_host_panel_hand_raised_label
      objectReference: {fileID: 0}
    - target: {fileID: 8229847291080121086, guid: d7698b5f08e39cc4aaf5e62e6972733b,
        type: 3}
      propertyPath: localizedString.fallbackValue
      value: Hand raised
      objectReference: {fileID: 0}
    - target: {fileID: 8229847291080121086, guid: d7698b5f08e39cc4aaf5e62e6972733b,
        type: 3}
      propertyPath: localizedString.key
      value: events_host_panel_broadcasting_label
      objectReference: {fileID: 0}
    m_RemovedComponents: []
    m_RemovedGameObjects: []
    m_AddedGameObjects: []
    m_AddedComponents: []
  m_SourcePrefab: {fileID: 100100000, guid: d7698b5f08e39cc4aaf5e62e6972733b, type: 3}
"#;

    #[test]
    fn test_parse_unity_reader() {
        let mut reader = BufReader::new(PREFAB.as_bytes());
        let mut asset = Asset::new(
            Id::Guid(Uuid::nil()),
            AssetType::Prefab,
            Some(PathBuf::from("test.prefab")),
            [],
        );
        let result = parse_reader(&mut reader, &mut asset, None);

        assert!(result.is_ok());
        assert!(asset.relations.contains(&Relation::Uses(Id::Guid(
            Uuid::parse_str("7c77678171dd7a24ead5c598179e6378").unwrap()
        ))));
        assert!(asset.relations.contains(&Relation::Uses(Id::Guid(
            Uuid::parse_str("05503c2c5cf7b7f45bec1113802f99a0").unwrap()
        ))));
        assert!(
            asset
                .relations
                .contains(&Relation::Uses(Id::Loc("people_panel_people_label".into())))
        );
        assert!(
            asset
                .relations
                .contains(&Relation::Uses(Id::Loc("events_host_panel_hand_raised_label".into())))
        );
        assert!(
            asset
                .relations
                .contains(&Relation::Uses(Id::Loc("events_host_panel_broadcasting_label".into())))
        );
    }
}
