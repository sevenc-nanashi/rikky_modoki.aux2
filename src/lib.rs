mod audiobuffer;
mod glass;
mod image;
mod material_ex;
mod module;
mod objectsound;
mod progress;

pub static EDIT_HANDLE: aviutl2::generic::GlobalEditHandle =
    aviutl2::generic::GlobalEditHandle::new();

#[aviutl2::plugin(GenericPlugin)]
pub struct RikkyModoki {
    module: aviutl2::generic::SubPlugin<module::RikkyModokiMod2>,
    sound_module: aviutl2::generic::SubPlugin<objectsound::ObjectSoundAuf2>,
    audio_buffer_module: aviutl2::generic::SubPlugin<audiobuffer::AudioBufferAuf2>,
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
            module: aviutl2::generic::SubPlugin::new_script_module(&info)?,
            sound_module: aviutl2::generic::SubPlugin::new_filter_plugin(&info)?,
            audio_buffer_module: aviutl2::generic::SubPlugin::new_filter_plugin(&info)?,
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
        registry.register_script_module(None, &self.module);
        registry.register_filter_plugin(&self.sound_module);
        registry.register_filter_plugin(&self.audio_buffer_module);
    }

    fn on_project_save(&mut self, project: &mut aviutl2::generic::ProjectFile) {
        let path = project.get_path();
        let mut project_path = module::PROJECT_PATH.lock().unwrap();
        *project_path = path;
    }

    fn on_project_load(&mut self, _project: &mut aviutl2::generic::ProjectFile) {
        crate::module::COUNTER.store(0, std::sync::atomic::Ordering::SeqCst);
        objectsound::reset();
        audiobuffer::reset();
        progress::end();
    }

    fn event_update_object_info(&mut self) {
        objectsound::invalidate();
        audiobuffer::invalidate();
    }

    fn event_change_scene_info(&mut self) {
        objectsound::invalidate();
        audiobuffer::invalidate();
    }

    fn on_clear_cache(&mut self, _edit: &aviutl2::generic::EditSection) {
        objectsound::reset();
        audiobuffer::reset();
        progress::end();
    }
}

impl Drop for RikkyModoki {
    fn drop(&mut self) {
        objectsound::shutdown();
        audiobuffer::shutdown();
        progress::end();
    }
}

aviutl2::register_generic_plugin!(RikkyModoki);
