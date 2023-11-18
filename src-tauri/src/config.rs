use indoc::indoc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::fs::{create_dir_all, File};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::{
    plugin::{Plugin, Result as PluginResult},
    AppHandle, Invoke, Manager, PageLoadPayload, RunEvent, Runtime, Window,
};

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

const DEFAULT_CONFIG_JSON: &str = include_str!("defaultState.json");

pub const CONFIG_FILENAME: &str = "settings.json";

type TauriManagedConfig = Arc<Mutex<Config>>;

pub struct ConfigPlugin {
    managed_config: Option<TauriManagedConfig>,
}

impl Default for ConfigPlugin {
    fn default() -> Self {
        Self { managed_config: None }
    }
}

impl<R: Runtime> Plugin<R> for ConfigPlugin {
    fn name(&self) -> &'static str {
        "configPlugin"
    }

    fn initialize(&mut self, app: &AppHandle<R>, _: serde_json::Value) -> PluginResult<()> {
        // load default config no matter what, to act as validation
        let mut config: Config = serde_json::from_str(DEFAULT_CONFIG_JSON)
            .expect("invalid default configuration in defaultState.json");

        if let Some(app_dir) = app.path_resolver().app_config_dir() {
            let config_path = app_dir.join(CONFIG_FILENAME);
            if config_path.exists() {
                if let Ok(config_json) = tauri::api::file::read_string(config_path) {
                    if let Ok(user_config) = serde_json::from_str(config_json.as_str()) {
                        config = user_config
                    }
                }
            }
        }

        let managed_config = Arc::new(Mutex::new(config));

        app.manage::<TauriManagedConfig>(managed_config.clone());
        self.managed_config = Some(managed_config);

        Ok(())
    }

    fn initialization_script(&self) -> Option<String> {
        let managed_config = self
            .managed_config
            .as_ref()
            .expect("initial_config is None when creating initialization_script");

        let config = managed_config.lock().unwrap();

        let config_json = serde_json::to_string::<Config>(&config)
            .expect("failed to serialise config into string");

        let script = format!("window.configPlugin = {config_json};");

        Some(script)
    }

    fn on_event(&mut self, app: &AppHandle<R>, event: &RunEvent) {
        if let RunEvent::Exit = event {
            if let Some(app_dir) = app.path_resolver().app_config_dir() {
                let config_path = app_dir.join(CONFIG_FILENAME);
                let managed_config = app.state::<TauriManagedConfig>();
                let config = managed_config.lock().unwrap();
                let config_json = serde_json::to_string_pretty::<Config>(&config)
                    .expect("failed to serialise config into string");

                // create dir, then write file if dir was created successfully
                let _ = create_dir_all(app_dir).and_then(|_| fs::write(config_path, config_json));
            }
        }
        ()
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
        serde_json::from_str::<Config>(DEFAULT_CONFIG_JSON).unwrap();
    }
}
