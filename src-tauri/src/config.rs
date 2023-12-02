use indoc::indoc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::fs::{create_dir_all, File};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::{
    plugin::{Plugin, Result as PluginResult},
    AppHandle, Invoke, LogicalSize, Manager, PageLoadPayload, PhysicalPosition, RunEvent, Runtime,
    Window, WindowEvent,
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

    fn set_window_position<R>(&self, window: &Window<R>) -> tauri::Result<()>
    where
        R: Runtime,
    {
        if let Some(dimensions) = &self.dimensions {
            window.set_position(PhysicalPosition::new(dimensions.x, dimensions.y))?;
            window.set_size(LogicalSize::new(dimensions.width, dimensions.height))?;
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

/// You must call this function before exiting in the frontend (*).
/// * Only needed if you exit using `appWindow.close()`
#[tauri::command]
fn update_window_size_config<R: Runtime>(
    state: tauri::State<'_, TauriManagedConfig>,
    window: Window<R>,
) -> tauri::Result<()> {
    let mut config = state.lock().unwrap();
    config.update_from_window(&window)?;
    Ok(())
}

impl<R: Runtime> Default for ConfigPlugin<R> {
    fn default() -> Self {
        Self {
            managed_config: None,
            invoke_handler: Box::new(tauri::generate_handler![update_window_size_config]),
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

    fn created(&mut self, window: Window<R>) {
        let managed_config = self
            .managed_config
            .as_ref()
            .expect("config is still empty when webview is created")
            .clone();
        let window_clone = window.clone();

        // apply window size and position
        {
            let config = managed_config.lock().unwrap();
            config
                .set_window_position(&window)
                .expect("failed to set window position");
        }

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

    fn extend_api(&mut self, message: Invoke<R>) {
        (self.invoke_handler)(message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;

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
