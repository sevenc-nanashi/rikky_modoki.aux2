mod module;

pub static EDIT_HANDLE: aviutl2::generic::GlobalEditHandle =
    aviutl2::generic::GlobalEditHandle::new();

#[aviutl2::plugin(GenericPlugin)]
pub struct RikkyModoki {
    plugin: aviutl2::generic::SubPlugin<module::RikkyModokiMod2>,
}

impl aviutl2::generic::GenericPlugin for RikkyModoki {
    fn new(info: aviutl2::common::AviUtl2Info) -> aviutl2::common::AnyResult<Self> {
        aviutl2::tracing_subscriber::fmt()
            .with_max_level(if cfg!(debug_assertions) {
                tracing::Level::DEBUG
            } else {
                tracing::Level::INFO
            })
            .event_format(aviutl2::logger::AviUtl2Formatter)
            .with_writer(aviutl2::logger::AviUtl2LogWriter)
            .init();

        tracing::info!("This is an info log message using tracing.");
        Ok(Self {
            plugin: aviutl2::generic::SubPlugin::new_script_module(&info)?,
        })
    }

    fn plugin_info(&self) -> aviutl2::generic::GenericPluginTable {
        aviutl2::generic::GenericPluginTable {
            name: "rikky_modoki.aux2".to_string(),
            information: "rikky_modoki.anm2".to_string(),
        }
    }

    fn register(&mut self, registry: &mut aviutl2::generic::HostAppHandle) {
        EDIT_HANDLE.init(registry.create_edit_handle());
        registry.register_script_module(None, &self.plugin);
    }

    fn on_project_save(&mut self, project: &mut aviutl2::generic::ProjectFile) {
        let path = project.get_path();
        let mut project_path = module::PROJECT_PATH.lock().unwrap();
        *project_path = path;
    }

    fn on_project_load(&mut self, _project: &mut aviutl2::generic::ProjectFile) {
        crate::module::COUNTER.store(0, std::sync::atomic::Ordering::SeqCst);
    }
}

aviutl2::register_generic_plugin!(RikkyModoki);
