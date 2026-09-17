use aviutl2::module::ScriptModuleFunctions;

pub static PROJECT_PATH: std::sync::Mutex<Option<std::path::PathBuf>> = std::sync::Mutex::new(None);
pub static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

#[aviutl2::plugin(ScriptModule)]
pub struct RikkyModokiMod2;

impl aviutl2::module::ScriptModule for RikkyModokiMod2 {
    fn new(_info: aviutl2::common::AviUtl2Info) -> aviutl2::common::AnyResult<Self> {
        Ok(Self)
    }

    fn plugin_info(&self) -> aviutl2::module::ScriptModuleTable {
        aviutl2::module::ScriptModuleTable {
            information: "rikky_modoki.mod2".to_string(),
            functions: Self::functions(),
        }
    }
}

#[aviutl2::module::functions]
impl RikkyModokiMod2 {
    fn project_path(&self, basename_only: bool) -> aviutl2::common::AnyResult<String> {
        let project_path = PROJECT_PATH.lock().unwrap();
        if let Some(path) = &*project_path {
            if basename_only {
                Ok(path
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default())
            } else {
                Ok(path.to_string_lossy().to_string())
            }
        } else {
            Ok("".to_string())
        }
    }

    fn edit_state(&self) -> aviutl2::common::AnyResult<String> {
        Ok(match crate::EDIT_HANDLE.get_edit_state()? {
            aviutl2::generic::EditState::Edit => "editing".to_string(),
            aviutl2::generic::EditState::Preview => "playing".to_string(),
            aviutl2::generic::EditState::Save => "saving".to_string(),
        })
    }

    fn aviutl2_dir(&self) -> aviutl2::common::AnyResult<String> {
        let path = std::env::current_exe()?;
        let dir = path.parent().ok_or_else(|| {
            anyhow::anyhow!("Failed to get parent directory of the executable path")
        })?;
        Ok(format!("{}\\", dir.to_string_lossy()))
    }

    fn desktop_dir(&self) -> aviutl2::common::AnyResult<String> {
        let desktop_dir = dirs::desktop_dir()
            .ok_or_else(|| anyhow::anyhow!("Failed to get the desktop directory"))?;
        Ok(format!("{}\\", desktop_dir.to_string_lossy()))
    }

    fn scene_id(&self) -> i32 {
        crate::EDIT_HANDLE.get_edit_info().scene_id
    }

    fn script_name_of(
        &self,
        layer: usize,
        frame: usize,
        read: &aviutl2::generic::ReadSection,
    ) -> aviutl2::common::AnyResult<String> {
        let object = read.find_object_after(layer - 1, frame)?.ok_or_else(|| {
            anyhow::anyhow!("No object found at layer {} and frame {}", layer, frame)
        })?;
        let first_effect = read.get_first_effect(object)?;
        let name = read.get_effect_name(first_effect)?;
        anyhow::Ok(name)
    }

    fn hwnd(&self) -> aviutl2::common::AnyResult<isize> {
        Ok(crate::EDIT_HANDLE.get_host_app_window_raw().map(|hwnd| hwnd.hwnd.get()).unwrap_or(0))
    }

    fn counter(&self) -> usize {
        COUNTER.load(std::sync::atomic::Ordering::SeqCst)
    }

    fn to_sjis(&self, input: String) -> aviutl2::common::AnyResult<Vec<u8>> {
        let (cow, _, _) = encoding_rs::SHIFT_JIS.encode(&input);
        Ok(cow.into_owned())
    }
}
