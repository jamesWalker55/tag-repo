use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
enum FixedComponent {
    ItemList,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
enum PanelComponent {
    FolderTree,
    ItemProperties,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
#[serde(untagged)]
enum Component {
    Fixed(FixedComponent),
    Panel(PanelComponent),
}

structstruck::strike! {
    #[strikethrough[derive(Serialize, Deserialize, PartialEq, Debug)]]
    #[strikethrough[serde(rename_all = "camelCase")]]
    struct Config {
        last_open_path: Option<String>,
        dimensions: Option<struct DimensionsConfig {
            x: f64,
            y: f64,
            width: f64,
            height: f64,
            fullscreen: bool,
        }>,
        audio_preview: struct AudioPreviewConfig {
            enabled: bool,
            volume: f64,
        },
        layout: struct LayoutConfig {
            left: struct PanelConfig {
                component: Option<PanelComponent>,
                size: i32,
            },
            right: PanelConfig,
            bottom: PanelConfig,
        },
        components: struct ComponentsConfig {
            item_list: struct ItemListConfig {
                columns: Vec<struct ItemListColumn {
                    r#type: enum ItemListColumnType {
                        Path,
                        Name,
                        Tags,
                        Extension,
                    },
                    width: i32,
                }>,
            },
            folder_tree: struct FolderTreeConfig {
                recursive: bool,
            }
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            last_open_path: None,
            dimensions: None,
            audio_preview: AudioPreviewConfig { enabled: true, volume: 0.5 },
            layout: LayoutConfig {
                left: PanelConfig {
                    component: Some(PanelComponent::FolderTree),
                    size: 200,
                },
                right: PanelConfig {
                    component: Some(PanelComponent::ItemProperties),
                    size: 250,
                },
                bottom: PanelConfig { component: None, size: 160 },
            },
            components: ComponentsConfig {
                item_list: ItemListConfig {
                    columns: vec![
                        ItemListColumn { r#type: ItemListColumnType::Name, width: 300 },
                        ItemListColumn { r#type: ItemListColumnType::Tags, width: 160 },
                        ItemListColumn { r#type: ItemListColumnType::Extension, width: 60 },
                        ItemListColumn { r#type: ItemListColumnType::Path, width: 500 },
                    ],
                },
                folder_tree: FolderTreeConfig { recursive: true },
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;

    #[test]
    fn test_config() {
        let config = Config {
            last_open_path: Some("hello world".into()),
            dimensions: Some(DimensionsConfig {
                x: 0.0,
                y: 0.0,
                width: 0.0,
                height: 0.0,
                fullscreen: false,
            }),
            audio_preview: AudioPreviewConfig { enabled: true, volume: 1.0 },
            layout: LayoutConfig {
                left: PanelConfig {
                    component: Some(PanelComponent::FolderTree),
                    size: 134,
                },
                right: PanelConfig {
                    component: Some(PanelComponent::ItemProperties),
                    size: 888,
                },
                bottom: PanelConfig { component: None, size: 0 },
            },
            components: ComponentsConfig {
                item_list: ItemListConfig {
                    columns: vec![
                        ItemListColumn { r#type: ItemListColumnType::Name, width: 123 },
                        ItemListColumn { r#type: ItemListColumnType::Path, width: 456 },
                    ],
                },
                folder_tree: FolderTreeConfig { recursive: true },
            },
        };
        let serialized = serde_json::to_string_pretty(&config).unwrap();
        let deserialized: Config = serde_json::from_str(&serialized).unwrap();
        assert_eq!(config, deserialized);
    }

    #[test]
    fn default_config() {
        let config = Config::default();
        let serialized = serde_json::to_string_pretty(&config).unwrap();
        let expected_serialized = indoc! {r#"
            {
              "lastOpenPath": null,
              "dimensions": null,
              "audioPreview": {
                "enabled": true,
                "volume": 0.5
              },
              "layout": {
                "left": {
                  "component": "FolderTree",
                  "size": 200
                },
                "right": {
                  "component": "ItemProperties",
                  "size": 250
                },
                "bottom": {
                  "component": null,
                  "size": 160
                }
              },
              "components": {
                "itemList": {
                  "columns": [
                    {
                      "type": "name",
                      "width": 300
                    },
                    {
                      "type": "tags",
                      "width": 160
                    },
                    {
                      "type": "extension",
                      "width": 60
                    },
                    {
                      "type": "path",
                      "width": 500
                    }
                  ]
                },
                "folderTree": {
                  "recursive": true
                }
              }
            }
        "#};
        let deserialized: Config = serde_json::from_str(&serialized).unwrap();
        assert_eq!(config, deserialized);
    }
}
