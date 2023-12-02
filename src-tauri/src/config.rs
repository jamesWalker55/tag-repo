use serde::{Deserialize, Serialize};
use std::fs;
use std::fs::create_dir_all;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tauri::{
    plugin::{Plugin, Result as PluginResult},
    AppHandle, Invoke, Manager, RunEvent, Runtime, Window, WindowEvent,
};

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub enum FixedComponent {
    ItemList,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub enum PanelComponent {
    FolderTree,
    ItemProperties,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
#[serde(untagged)]
pub enum Component {
    Fixed(FixedComponent),
    Panel(PanelComponent),
}

structstruck::strike! {
    #[strikethrough[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]]
    #[strikethrough[serde(rename_all = "camelCase")]]
    pub struct Config {
        pub path: Option<String>,
        pub dimensions: Option<pub struct DimensionsConfig {
            pub x: i32,
            pub y: i32,
            pub width: u32,
            pub height: u32,
        }>,
        pub audio_preview: struct AudioPreviewConfig {
            pub enabled: bool,
            pub volume: f64,
        },
        pub layout: pub struct LayoutConfig {
            pub left: pub struct PanelConfig {
                pub component: Option<PanelComponent>,
                pub size: i32,
            },
            pub right: PanelConfig,
            pub bottom: PanelConfig,
        },
        pub components: pub struct ComponentsConfig {
            pub item_list: pub struct ItemListConfig {
                pub columns: Vec<pub struct ItemListColumn {
                    pub r#type: enum ItemListColumnType {
                        Path,
                        Name,
                        Tags,
                        Extension,
                    },
                    pub width: i32,
                }>,
            },
            pub folder_tree: pub struct FolderTreeConfig {
                pub recursive: bool,
            }
        }
    }
}

impl Config {
    fn update_from_window<R>(&mut self, window: &Window<R>) -> tauri::Result<()>
    where
        R: Runtime,
    {
        let position = window.inner_position()?;
        let size = window.inner_size()?;
        match &mut self.dimensions {
            None => {
                self.dimensions = Some(DimensionsConfig {
                    x: position.x,
                    y: position.y,
                    width: size.width,
                    height: size.height,
                })
            }
            Some(ref mut dimensions) => {
                dimensions.x = position.x;
                dimensions.y = position.y;
                dimensions.width = size.width;
                dimensions.height = size.height;
            }
        }
        Ok(())
    }
}

const DEFAULT_CONFIG_JSON: &str = include_str!("defaultState.json");

pub const CONFIG_FILENAME: &str = "settings.json";

pub type TauriManagedConfig = Arc<Mutex<Config>>;

pub struct ConfigPlugin<R: Runtime> {
    managed_config: Option<TauriManagedConfig>,
    invoke_handler: Box<dyn Fn(Invoke<R>) + Send + Sync>,
}

impl<R: Runtime> ConfigPlugin<R> {
    fn load(app_config_dir: &Path) -> Option<Config> {
        let config_path = app_config_dir.join(CONFIG_FILENAME);
        let Ok(config_json) = tauri::api::file::read_string(config_path) else {
            return None;
        };

        let Ok(config) = serde_json::from_str(config_json.as_str()) else {
            return None;
        };

        Some(config)
    }

    fn save(app_config_dir: &Path, config: &Config) {
        let config_path = app_config_dir.join(CONFIG_FILENAME);
        let config_json = serde_json::to_string_pretty::<Config>(config)
            .expect("failed to serialise config into string");

        // create dir, then write file if dir was created successfully
        let _ = create_dir_all(app_config_dir).and_then(|_| fs::write(config_path, config_json));
    }
}

/// You must call this function before exiting in the frontend (*).
/// * Only needed if you exit using `appWindow.close()`
#[tauri::command]
fn set_dimensions<R: Runtime>(
    state: tauri::State<'_, TauriManagedConfig>,
    window: Window<R>,
) -> tauri::Result<()> {
    let mut config = state.lock().unwrap();
    config.update_from_window(&window)?;
    Ok(())
}

#[tauri::command]
fn set_audio_preview(
    state: tauri::State<'_, TauriManagedConfig>,
    audio_preview: AudioPreviewConfig,
) {
    let mut config = state.lock().unwrap();
    config.audio_preview = audio_preview;
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
#[serde(rename_all = "camelCase")]
enum LayoutSide {
    Left,
    Right,
    Bottom,
}

#[tauri::command]
fn set_layout(
    state: tauri::State<'_, TauriManagedConfig>,
    side: LayoutSide,
    panel_config: PanelConfig,
) {
    let mut config = state.lock().unwrap();
    match side {
        LayoutSide::Left => {
            config.layout.left = panel_config;
        }
        LayoutSide::Right => {
            config.layout.right = panel_config;
        }
        LayoutSide::Bottom => {
            config.layout.bottom = panel_config;
        }
    }
}

#[tauri::command]
fn set_item_list(state: tauri::State<'_, TauriManagedConfig>, item_list: ItemListConfig) {
    let mut config = state.lock().unwrap();
    config.components.item_list = item_list;
}

#[tauri::command]
fn set_folder_tree(state: tauri::State<'_, TauriManagedConfig>, folder_tree: FolderTreeConfig) {
    let mut config = state.lock().unwrap();
    config.components.folder_tree = folder_tree;
}

#[tauri::command]
fn save<R: Runtime>(state: tauri::State<'_, TauriManagedConfig>, app_handle: tauri::AppHandle<R>) {
    let config = state.lock().unwrap();
    let Some(app_dir) = app_handle.path_resolver().app_config_dir() else {
        return;
    };
    ConfigPlugin::<R>::save(&app_dir, &config);
}

#[tauri::command]
fn load<R: Runtime>(app_handle: tauri::AppHandle<R>) -> Option<Config> {
    let Some(app_dir) = app_handle.path_resolver().app_config_dir() else {
        return None;
    };
    ConfigPlugin::<R>::load(&app_dir)
}

impl<R: Runtime> Default for ConfigPlugin<R> {
    fn default() -> Self {
        Self {
            managed_config: None,
            invoke_handler: Box::new(tauri::generate_handler![
                set_dimensions,
                set_audio_preview,
                set_layout,
                set_item_list,
                set_folder_tree,
                save,
                load,
            ]),
        }
    }
}

impl<R: Runtime> Plugin<R> for ConfigPlugin<R> {
    fn name(&self) -> &'static str {
        "configPlugin"
    }

    fn initialize(&mut self, app: &AppHandle<R>, _: serde_json::Value) -> PluginResult<()> {
        // load default config no matter what, to act as validation
        let mut config: Config = serde_json::from_str(DEFAULT_CONFIG_JSON)
            .expect("invalid default configuration in defaultState.json");

        if let Some(app_dir) = app.path_resolver().app_config_dir() {
            if let Some(user_config) = Self::load(&app_dir) {
                config = user_config
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

    fn created(&mut self, window: Window<R>) {
        let managed_config = self
            .managed_config
            .as_ref()
            .expect("config is still empty when webview is created")
            .clone();
        let window_clone = window.clone();

        // setup callback to update config (part 1)
        window.on_window_event(move |e| {
            // IMPORTANT: This will not run when closing the window through script: appWindow.close()
            // They refuse to fix this which is fucking stupid:
            // https://github.com/tauri-apps/plugins-workspace/issues/701
            // You need to manually update the state if you're using script to close the window
            if let WindowEvent::CloseRequested { .. } = e {
                let mut config = managed_config.lock().unwrap();
                config
                    .update_from_window(&window_clone)
                    .expect("failed to update config from closing window");
            }
        });
    }

    fn on_event(&mut self, app: &AppHandle<R>, event: &RunEvent) {
        let RunEvent::Exit = event else {
            return;
        };
        let Some(app_dir) = app.path_resolver().app_config_dir() else {
            return;
        };

        let managed_config = app.state::<TauriManagedConfig>();
        let config = managed_config.lock().unwrap();
        Self::save(&app_dir, &config);
    }

    fn extend_api(&mut self, message: Invoke<R>) {
        (self.invoke_handler)(message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config() {
        let config = Config {
            path: Some("hello world".into()),
            dimensions: Some(DimensionsConfig { x: 0, y: 0, width: 0, height: 0 }),
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
