use aviutl2::module::ScriptModuleFunctions;
use lazy_regex::regex;

pub static PROJECT_PATH: std::sync::Mutex<Option<std::path::PathBuf>> = std::sync::Mutex::new(None);
pub static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

static PARAMETER_REPLACE_NOTIFIED: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

// 置換ルールを変更したときに増やす。プラグインのバージョンとは独立。
const REPLACEMENT_VERSION: u32 = 5;

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
    #[allow(clippy::too_many_arguments)]
    fn draw_constants(
        &self,
        width: usize,
        height: usize,
        pose: crate::glass::Pose,
        camera: crate::glass::Camera,
        args: Vec<f64>,
        groups: Vec<f64>,
    ) -> aviutl2::common::AnyResult<Vec<f64>> {
        crate::glass::constants(width, height, pose, camera, args, groups)
    }
    fn material_layer_position(
        &self,
        position: Vec<f64>,
        groups: Vec<f64>,
    ) -> aviutl2::common::AnyResult<Vec<f64>> {
        crate::glass::layer_position(position, groups)
    }
    fn material_light_constants(
        &self,
        light: crate::material_ex::LightInput,
        textured: bool,
    ) -> aviutl2::common::AnyResult<Vec<f64>> {
        crate::material_ex::constants(light, textured)
    }

    fn is_development(&self) -> bool {
        cfg!(debug_assertions)
    }

    fn progress_start(&self, title: String, color: u32) -> bool {
        let Some(owner) = crate::EDIT_HANDLE.get_host_app_window_raw() else {
            tracing::error!("AviUtl2のウィンドウを取得できません");
            return false;
        };
        crate::progress::start(title, color, owner.hwnd)
    }

    fn progress_processing(&self, percent: f64) -> bool {
        crate::progress::processing(percent)
    }

    fn progress_end(&self) -> bool {
        crate::progress::end()
    }

    fn sound_receiving(&self, frame: u32) -> bool {
        crate::objectsound::receiving(frame)
    }

    fn audio_buffer_info(&self) -> (u32, u32) {
        crate::audiobuffer::info()
    }

    fn audio_buffer_pcm(
        &self,
        frame: i64,
        size: Option<usize>,
    ) -> aviutl2::common::AnyResult<(Vec<f64>, Vec<f64>)> {
        crate::audiobuffer::pcm(frame, size)
    }

    fn audio_buffer_fourier(
        &self,
        frame: i64,
        resolution: usize,
        monaural: bool,
    ) -> aviutl2::common::AnyResult<(Vec<f64>, Vec<f64>)> {
        crate::audiobuffer::fourier(frame, resolution, monaural)
    }

    fn sound_length(&self, file: String) -> aviutl2::common::AnyResult<Option<f64>> {
        crate::objectsound::length(&file)
    }

    #[allow(clippy::too_many_arguments)] // soundregisterの引数に描画フレームを添えて渡す。
    fn sound_register(
        &self,
        origin_frame: u32,
        file: String,
        frame: f64,
        volume: f64,
        speed: f64,
        pan: f64,
        reverse: bool,
    ) -> aviutl2::common::AnyResult<bool> {
        crate::objectsound::register(
            origin_frame,
            crate::objectsound::Sound {
                file,
                frame,
                volume,
                speed,
                pan,
                reverse,
            },
        )
    }

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

    fn is_effect_focused(
        &self,
        effect_id: i64,
        read: &aviutl2::generic::ReadSection,
    ) -> aviutl2::common::AnyResult<bool> {
        let Some(focused_object) = read.get_focused_object()? else {
            return Ok(false);
        };

        let effects = read.get_effects(focused_object)?;
        for effect in effects {
            let id = read.get_effect_id(effect)?;
            if id == effect_id {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn dir(
        &self,
        directory: String,
        extensions: Vec<String>,
    ) -> aviutl2::common::AnyResult<Vec<String>> {
        anyhow::ensure!(
            !extensions.is_empty(),
            "Expected at least one extension or directory mode"
        );
        let folders = extensions[0].is_empty();
        let all_files = extensions[0] == "*all";
        let mut paths = Vec::new();
        for entry in std::fs::read_dir(std::path::absolute(directory)?)? {
            let path = entry?.path();
            let metadata = path.metadata()?;
            let matches = if folders {
                metadata.is_dir()
            } else {
                metadata.is_file()
                    && (all_files
                        || path
                            .extension()
                            .and_then(|extension| extension.to_str())
                            .is_some_and(|extension| {
                                extensions.iter().any(|filter| {
                                    extension.eq_ignore_ascii_case(
                                        filter.trim_start_matches("*.").trim_start_matches('.'),
                                    )
                                })
                            }))
            };
            if matches {
                paths.push(
                    path.into_os_string().into_string().map_err(|_| {
                        anyhow::anyhow!("Directory entry path is not valid Unicode")
                    })?,
                );
            }
        }
        paths.sort_unstable();
        Ok(paths)
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

    fn text_of(
        &self,
        layer: usize,
        frame: usize,
        read: &aviutl2::generic::ReadSection,
    ) -> aviutl2::common::AnyResult<String> {
        let object = read.find_object_after(layer - 1, frame)?.ok_or_else(|| {
            anyhow::anyhow!("No object found at layer {} and frame {}", layer, frame)
        })?;
        let text = read.get_object_effect_item(object, "テキスト", 0, "テキスト")?;
        Ok(text)
    }

    fn hwnd(&self) -> aviutl2::common::AnyResult<isize> {
        Ok(crate::EDIT_HANDLE
            .get_host_app_window_raw()
            .map(|hwnd| hwnd.hwnd.get())
            .unwrap_or(0))
    }

    fn counter(&self) -> usize {
        COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    }

    fn to_sjis(&self, input: String) -> aviutl2::common::AnyResult<Vec<u8>> {
        let (cow, _, _) = encoding_rs::SHIFT_JIS.encode(&input);
        Ok(cow.into_owned())
    }

    fn convert_to_codes(
        &self,
        input: Vec<u8>,
        unicode: bool,
    ) -> aviutl2::common::AnyResult<Vec<u16>> {
        let text = encoding_rs::SHIFT_JIS
            .decode_without_bom_handling_and_without_replacement(&input)
            .ok_or_else(|| anyhow::anyhow!("Invalid Shift-JIS sequence"))?;
        Ok(if unicode {
            text.encode_utf16().collect()
        } else {
            text.bytes().map(u16::from).collect()
        })
    }

    fn convert_from_codes(
        &self,
        input: Vec<u16>,
        unicode: bool,
    ) -> aviutl2::common::AnyResult<Vec<u8>> {
        let text = if unicode {
            String::from_utf16(&input)?
        } else {
            let bytes = input
                .into_iter()
                .map(u8::try_from)
                .collect::<Result<Vec<_>, _>>()?;
            String::from_utf8(bytes)?
        };
        let (bytes, _, errors) = encoding_rs::SHIFT_JIS.encode(&text);
        anyhow::ensure!(!errors, "Text cannot be represented in Shift-JIS");
        Ok(bytes.into_owned())
    }

    fn image_write(
        &self,
        id: String,
        data: *const u8,
        width: usize,
        height: usize,
        alpha: f64,
        only_empty: bool,
    ) -> aviutl2::common::AnyResult<bool> {
        // SAFETY: Lua側がgetpixeldataの結果、または呼び出し元のuserdataと寸法を渡す。
        unsafe { crate::image::write(id, data, width, height, alpha, only_empty) }
    }

    fn image_save_file(
        &self,
        file: String,
        format: String,
        data: *const u8,
        width: usize,
        height: usize,
        quality: u32,
    ) -> aviutl2::common::AnyResult<()> {
        // SAFETY: Lua側でgetpixeldataの直後に呼び出し、その間画像を変更しない。
        unsafe { crate::image::save_file(&file, &format, data, width, height, quality) }
    }

    fn image_copy(&self, destination: String, source: String, only_empty: bool) -> bool {
        crate::image::copy(destination, &source, only_empty)
    }

    fn image_read(&self, id: String, export: bool) -> Option<crate::image::ImageRead> {
        crate::image::read(&id, export)
    }

    fn image_release(
        &self,
        lease: aviutl2::module::ScriptModuleUserData<crate::image::ImageLease>,
    ) {
        crate::image::release(lease);
    }

    fn image_delete(&self, id: Option<String>) -> bool {
        crate::image::delete(id.as_deref())
    }

    fn image_ids(&self, count: usize) -> aviutl2::common::AnyResult<Vec<String>> {
        crate::image::ids(count)
    }

    fn image_merge(
        &self,
        back: String,
        front: String,
        x: i32,
        y: i32,
        export: bool,
    ) -> Option<crate::image::ImageRead> {
        crate::image::merge(&back, &front, x, y, export)
    }

    fn image_channels(
        &self,
        lease: aviutl2::module::ScriptModuleUserData<crate::image::ImageLease>,
    ) -> aviutl2::common::AnyResult<(Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>)> {
        crate::image::channels(lease)
    }

    fn image_pixel(
        &self,
        data: *const u8,
        width: usize,
        height: usize,
        index: f64,
    ) -> aviutl2::common::AnyResult<Option<(u8, u8, u8, u8)>> {
        // SAFETY: 取得中の画像はLua側のlease、外部userdataは呼び出し元が保持する。
        unsafe { crate::image::pixel(data, width, height, index) }
    }

    #[allow(clippy::too_many_arguments)] // 公開Lua APIの開始座標と判定条件を渡す。
    fn fillarea(
        &self,
        data: *const u8,
        width: usize,
        height: usize,
        x: usize,
        y: usize,
        mode: usize,
        threshold: f64,
    ) -> aviutl2::common::AnyResult<Option<crate::image::FillAreaRead>> {
        // SAFETY: Lua側でgetpixeldataの直後に呼び出し、その間画像を変更しない。
        unsafe { crate::image::fillarea(data, width, height, [x, y], mode, threshold) }
    }

    fn bordering(
        &self,
        data: *const u8,
        width: usize,
        height: usize,
        skip: usize,
        threshold: f64,
        hq: bool,
    ) -> aviutl2::common::AnyResult<(Vec<usize>, Vec<usize>)> {
        // SAFETY: Lua側でgetpixeldataの直後に呼び出し、その間画像を変更しない。
        unsafe { crate::image::bordering(data, width, height, skip, threshold, hq) }
    }

    fn linedetection(
        &self,
        data: *const u8,
        width: usize,
        height: usize,
        scale: f64,
        background: u32,
    ) -> aviutl2::common::AnyResult<Vec<f64>> {
        // SAFETY: Lua側でgetpixeldataの直後に呼び出し、その間画像を変更しない。
        unsafe { crate::image::linedetection(data, width, height, scale, background) }
    }

    fn rewrite_parameter(
        &self,
        script_name: String,
        extension: String,
        parameter_type: String,
        index: usize,
    ) -> aviutl2::common::AnyResult<()> {
        replace_parameter_with_check(&script_name, &extension, index, || {
            let script_file_path = find_script_file(&script_name, &extension)?;
            let Some(mut content) = read_script_for_replacement(&script_file_path)? else {
                return Ok(());
            };
            let range = script_section(&content, &script_name)?;
            let mut script_content = content[range.clone()].to_owned();
            let dialog_info = expand_dialog(&mut script_content)?;

            let name = dialog_info.get(index.wrapping_sub(1)).ok_or_else(|| {
                anyhow::anyhow!(
                    "Index {} is out of bounds for dialog parameters (max index: {})",
                    index,
                    dialog_info.len() - 1
                )
            })?;
            let pattern = format!("--value@{}:", name);
            let replacement = format!("--{}@{}:", parameter_type, name);
            if !script_content.contains(&pattern) {
                if script_content.contains(&format!("--{}@{}:", parameter_type, name)) {
                    tracing::debug!(
                        "Parameter '{}' has already been replaced in the script content",
                        name
                    );
                    // --dialogの展開で/col等が既に目的の種類になっていても保存する。
                    if content[range.clone()] != script_content {
                        content.replace_range(range, &script_content);
                        update_script_file(&script_file_path, &content)?;
                    }
                    return Ok(());
                }
                return Err(anyhow::anyhow!(
                    "Parameter '{}' not found in the script content",
                    name
                ));
            }
            script_content = script_content.replace(&pattern, &replacement);

            content.replace_range(range, &script_content);
            update_script_file(&script_file_path, &content)?;
            Ok(())
        })
    }

    fn rewrite_select_parameter(
        &self,
        script_name: String,
        extension: String,
        index: usize,
        choices: Vec<String>,
    ) -> aviutl2::common::AnyResult<()> {
        replace_parameter_with_check(&script_name, &extension, index, || {
            let script_file_path = find_script_file(&script_name, &extension)?;
            let Some(mut content) = read_script_for_replacement(&script_file_path)? else {
                return Ok(());
            };
            let range = script_section(&content, &script_name)?;
            let mut script_content = content[range.clone()].to_owned();
            let dialog_info = expand_dialog(&mut script_content)?;

            let name = dialog_info.get(index.wrapping_sub(1)).ok_or_else(|| {
                anyhow::anyhow!(
                    "Index {} is out of bounds for dialog parameters (max index: {})",
                    index,
                    dialog_info.len() - 1
                )
            })?;

            let pattern = regex::Regex::new(&format!(
                r"--value@{}:([^,]+),([^,\r\n]+)",
                regex::escape(name)
            ))?;
            let Some((_, label, default)) = pattern.captures(&script_content).map(|caps| {
                (
                    caps.get(0).unwrap().as_str().to_string(),
                    caps.get(1).unwrap().as_str().to_string(),
                    caps.get(2).unwrap().as_str().to_string(),
                )
            }) else {
                if script_content.contains(&format!("--select@tmp_{}:", name)) {
                    tracing::debug!(
                        "Select parameter '{}' has already been replaced in the script content",
                        name
                    );
                    return Ok(());
                }
                return Err(anyhow::anyhow!(
                    "Select parameter '{}' not found in the script content",
                    name
                ));
            };

            let wants_numeric_value = label.starts_with("*");
            let default_int = if wants_numeric_value {
                default.parse::<usize>().ok()
            } else {
                let default = unescape_string(&default);
                choices
                    .iter()
                    .position(|choice| choice == &default)
                    .map(|i| i + 1)
            };
            let default_int = match default_int {
                Some(value) => value,
                None => {
                    tracing::warn!(
                        "Default value '{}' not found in choices for parameter '{}'",
                        default,
                        name
                    );
                    1
                }
            };

            let select_line = format!(
                "--select@tmp_{}:{}={},{}",
                name,
                label,
                default_int,
                choices
                    .iter()
                    .enumerate()
                    .map(|(i, choice)| format!("{}={}", choice, i + 1))
                    .collect::<Vec<_>>()
                    .join(",")
            );
            let mapping_line = if label.starts_with("*") {
                format!("local {} = tmp_{}", name, name)
            } else {
                format!(
                    "local {} = ({{ {} }})[tmp_{}] or {}",
                    name,
                    choices
                        .iter()
                        .map(|choice| format!("{:?}", choice))
                        .collect::<Vec<_>>()
                        .join(","),
                    name,
                    default
                )
            };

            script_content = pattern
                .replace(
                    &script_content,
                    &format!("{}\n{}", select_line, mapping_line),
                )
                .to_string();

            content.replace_range(range, &script_content);
            update_script_file(&script_file_path, &content)?;
            Ok(())
        })
    }

    fn rewrite_group_parameter(
        &self,
        script_name: String,
        extension: String,
        index: usize,
        entries: Vec<String>,
    ) -> aviutl2::common::AnyResult<()> {
        replace_parameter_with_check(&script_name, &extension, index, || {
            let path = find_script_file(&script_name, &extension)?;
            let Some(mut content) = read_script_for_replacement(&path)? else {
                return Ok(());
            };
            if expand_parameter_group(&mut content, &script_name, index, &entries)? {
                update_script_file(&path, &content)?;
            }
            Ok(())
        })
    }

    fn group_parameter_needs_rewrite(
        &self,
        script_name: String,
        extension: String,
        index: usize,
    ) -> aviutl2::common::AnyResult<bool> {
        let request = ParamReplaceRequest {
            script_name,
            extension,
            index,
        };
        let mut replaced = REPLACED_PARAMETERS.lock().unwrap();
        if replaced.contains(&request) {
            return Ok(false);
        }
        let path = find_script_file(&request.script_name, &request.extension)?;
        let content = read_script(&path)?;
        if group_needs_rewrite(&content, &request.script_name, index)? {
            return Ok(true);
        }
        replaced.insert(request);
        Ok(false)
    }
}

fn expand_parameter_group(
    content: &mut String,
    script_name: &str,
    index: usize,
    entries: &[String],
) -> anyhow::Result<bool> {
    anyhow::ensure!(
        !entries.is_empty() && entries.len().is_multiple_of(4),
        "Parameter definitions must contain groups of four values"
    );
    let range = script_section(content, script_name)?;
    let mut section = content[range.clone()].to_owned();
    let names = expand_dialog(&mut section)?;
    let name = names
        .get(index.wrapping_sub(1))
        .ok_or_else(|| anyhow::anyhow!("Invalid dialog parameter index: {index}"))?;
    let marker = format!("--rikky_modoki:parameter={name}");
    if section.lines().any(|line| line == marker) {
        return Ok(false);
    }
    let pattern = regex::Regex::new(&format!(
        r"(?m)^--value@{}:([^,\r\n]+),[^\r\n]*",
        regex::escape(name)
    ))?;
    let captures = pattern
        .captures(&section)
        .ok_or_else(|| anyhow::anyhow!("Value parameter '{name}' not found"))?;
    let parameter_range = captures.get(0).unwrap().range();
    let label = &captures[1];
    let newline = if section.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let mut lines = vec![marker, format!("--group:{label},false")];
    let mut variables = Vec::new();
    for (i, entry) in entries.chunks_exact(4).enumerate() {
        let variable = format!("__rikky_parameter_{name}_{}", i + 1);
        anyhow::ensure!(
            !section.contains(&format!("@{variable}:")),
            "Generated parameter '{variable}' already exists"
        );
        lines.push(group_parameter_line(&variable, entry)?);
        variables.push(variable);
    }
    lines.push("--group".to_owned());
    let assignment = format!("local {name} = {{{}}}{newline}", variables.join(", "));
    section.replace_range(parameter_range, &lines.join(newline));
    let body = script_body_start(&section)?;
    if body == section.len() && !section.ends_with('\n') {
        section.push_str(newline);
        section.push_str(&assignment);
    } else {
        section.insert_str(body, &assignment);
    }
    content.replace_range(range, &section);
    Ok(true)
}

fn group_parameter_line(variable: &str, entry: &[String]) -> anyhow::Result<String> {
    let [label, default, min, max] = entry else {
        unreachable!()
    };
    anyhow::ensure!(
        !label.is_empty() && !label.contains([',', '\r', '\n']) && !default.contains(['\r', '\n']),
        "Parameter labels and defaults must fit on one declaration line"
    );
    let min: f64 = min.parse()?;
    let max: f64 = max.parse()?;
    anyhow::ensure!(
        min.is_finite() && max.is_finite() && min <= max,
        "Invalid parameter range"
    );
    let mut label = label.as_str();
    let mut kind = "value";
    let mut step = "1";
    if let Some((text, suffix)) = label.rsplit_once('*') {
        match suffix {
            "file" | "folder" | "font" | "color" => {
                kind = suffix;
                label = text;
            }
            "1" | "0.1" | "0.01" | "0.001" => {
                kind = "track";
                step = suffix;
                label = text;
            }
            _ => anyhow::bail!("Unknown parameter suffix: {suffix}"),
        }
    }
    anyhow::ensure!(!label.is_empty(), "Parameter label is empty");
    let default = default.trim();
    anyhow::ensure!(!default.is_empty(), "Parameter default is empty");
    let quoted = default.starts_with('"');
    if kind == "value" {
        kind = if min != 0.0 || max != 0.0 {
            "track"
        } else if default == "true" || default == "false" {
            "check"
        } else if quoted {
            "string"
        } else {
            "value"
        };
    }
    let settings = match kind {
        "track" => {
            let value: f64 = default.parse()?;
            anyhow::ensure!(
                value.is_finite() && min <= value && value <= max,
                "Default is outside the parameter range"
            );
            format!("{min},{max},{default},{step}")
        }
        "string" | "file" | "folder" | "font" => {
            anyhow::ensure!(
                quoted && min == 0.0 && max == 0.0,
                "String parameters require a string default and zero bounds"
            );
            decode_parameter_string(default)?
                .replace('\r', "\\r")
                .replace('\n', "\\n")
        }
        "color" => {
            anyhow::ensure!(
                min == 0.0 && max == 0.0,
                "Color parameters require zero bounds"
            );
            if default != "nil" {
                let color: u32 = default.parse()?;
                anyhow::ensure!(color <= 0xffffff, "Invalid color default");
            }
            default.to_owned()
        }
        _ => default.to_owned(),
    };
    let declaration = format!("--{kind}@{variable}:{variable}::{label}");
    // AviUtl2 のファイル・フォルダ選択には初期値の指定がない。
    if kind == "file" || kind == "folder" {
        Ok(declaration)
    } else {
        Ok(format!("{declaration},{settings}"))
    }
}

// Lua 側で string.format("%q") に正規化した文字列を読み戻す。
fn decode_parameter_string(value: &str) -> anyhow::Result<String> {
    let value = value
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .ok_or_else(|| anyhow::anyhow!("Invalid string default"))?;
    let mut result = String::new();
    let mut chars = value.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\\' {
            result.push(c);
            continue;
        }
        let escaped = chars
            .next()
            .ok_or_else(|| anyhow::anyhow!("Incomplete string escape"))?;
        result.push(match escaped {
            'a' => '\x07',
            'b' => '\x08',
            'f' => '\x0c',
            'n' => '\n',
            'r' => '\r',
            't' => '\t',
            'v' => '\x0b',
            '\\' => '\\',
            '"' => '"',
            '0'..='9' => {
                let mut number = escaped.to_digit(10).unwrap();
                for _ in 0..2 {
                    if let Some(digit) = chars.peek().and_then(|c| c.to_digit(10)) {
                        chars.next();
                        number = number * 10 + digit;
                    } else {
                        break;
                    }
                }
                // %q は非ASCII文字をそのまま出力するため、数値エスケープは制御文字のみ。
                anyhow::ensure!(number < 128, "Unexpected non-ASCII string escape");
                char::from_u32(number).unwrap()
            }
            _ => anyhow::bail!("Invalid string escape: {escaped}"),
        });
    }
    anyhow::ensure!(
        !result.contains('\0'),
        "NUL cannot be used in a parameter default"
    );
    Ok(result)
}

fn script_section(content: &str, script_name: &str) -> anyhow::Result<std::ops::Range<usize>> {
    let Some((name, _)) = script_name.rsplit_once('@') else {
        return Ok(0..content.len());
    };
    let mut sections = regex!(r"(?m)^@([^\r\n]+)\r?$").captures_iter(content);
    while let Some(section) = sections.next() {
        if &section[1] == name {
            let start = section.get(0).unwrap().start();
            let end = match sections.next() {
                Some(next) => next.get(0).unwrap().start(),
                None => content.len(),
            };
            return Ok(start..end);
        }
    }
    anyhow::bail!("Script section '{name}' not found")
}

fn script_body_start(section: &str) -> anyhow::Result<usize> {
    let mut rest = section;
    loop {
        rest = rest.trim_start();
        if let Some((open, equals)) = lazy_regex::regex_captures!(r"^--\[(=*)\[", rest) {
            let close = format!("]{equals}]");
            let end = rest[open.len()..]
                .find(&close)
                .ok_or_else(|| anyhow::anyhow!("Unterminated header comment"))?;
            rest = &rest[open.len() + end + close.len()..];
        } else if rest.starts_with("--") || rest.starts_with('@') {
            rest = match rest.find('\n') {
                Some(end) => &rest[end + 1..],
                None => "",
            };
        } else {
            return Ok(section.len() - rest.len());
        }
    }
}

fn unescape_string(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();

    if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
        chars.next();
        chars.next_back();
    }
    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(next_char) = chars.peek() {
                match next_char {
                    'n' => {
                        result.push('\n');
                        chars.next();
                    }
                    'r' => {
                        result.push('\r');
                        chars.next();
                    }
                    't' => {
                        result.push('\t');
                        chars.next();
                    }
                    '\\' => {
                        result.push('\\');
                        chars.next();
                    }
                    '"' => {
                        result.push('"');
                        chars.next();
                    }
                    '\'' => {
                        result.push('\'');
                        chars.next();
                    }
                    _ => result.push(c),
                }
            } else {
                result.push(c);
            }
        } else {
            result.push(c);
        }
    }
    result
}

fn find_script_file(script_name: &str, extension: &str) -> anyhow::Result<std::path::PathBuf> {
    let script_dir = aviutl2::config::app_data_path().join("Script");
    let script_file_name = match script_name.rsplit_once('@') {
        Some((_, file)) => format!("@{file}.{extension}"),
        None => format!("{script_name}.{extension}"),
    };
    let script_file_path = std::fs::read_dir(script_dir)?
        .filter_map(|entry| entry.ok())
        .flat_map(|entry| {
            let path = entry.path();
            if path.is_file() {
                vec![path]
            } else if path.is_dir() {
                let entries = match std::fs::read_dir(path) {
                    Ok(entries) => entries,
                    Err(e) => {
                        tracing::warn!("Failed to read directory: {}", e);
                        return vec![];
                    }
                };

                entries
                    .into_iter()
                    .filter_map(|entry| entry.ok())
                    .map(|entry| entry.path())
                    .collect::<Vec<_>>()
            } else {
                vec![]
            }
        })
        .find(|path| {
            path.file_name()
                .map(|name| name.to_str().is_some_and(|name| name == script_file_name))
                .unwrap_or(false)
        })
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Script file '{}' not found in the Script directory",
                script_file_name
            )
        })?;
    Ok(script_file_path)
}

fn expand_dialog(script_content: &mut String) -> anyhow::Result<Vec<String>> {
    if let Some((_, names)) = lazy_regex::regex_captures!(
        r"(?m)^--rikky_modoki:dialog_info=([^\r\n]+)\r?$",
        script_content
    ) {
        return Ok(names.split(';').map(str::to_owned).collect());
    }

    let captures = lazy_regex::regex!(r"(?m)^[\t ]*--dialog:([^\r\n]*)")
        .captures(script_content)
        .ok_or_else(|| anyhow::anyhow!("--dialog declaration not found"))?;
    let range = captures.get(0).unwrap().range();
    let mut remaining = captures.get(1).unwrap().as_str().trim();
    let mut names = Vec::new();
    let mut values = Vec::new();
    while !remaining.is_empty() {
        let (header, label, name) = lazy_regex::regex_captures!(
            r"^([^,;]+),[\t ]*(?:local[\t ]+)?([A-Za-z_][A-Za-z_0-9]*)[\t ]*=[\t ]*",
            remaining
        )
        .ok_or_else(|| anyhow::anyhow!("Invalid --dialog item: {remaining}"))?;
        remaining = &remaining[header.len()..];
        let end = dialog_value_end(remaining)?;
        let value = remaining[..end].trim();
        anyhow::ensure!(!value.is_empty(), "Missing default value for {name}");
        names.push(name.to_owned());
        let label = label.trim();
        let (kind, label) = if let Some(label) = label.strip_suffix("/chk") {
            ("check", label)
        } else if let Some(label) = label.strip_suffix("/col") {
            ("color", label)
        } else if let Some(label) = label.strip_suffix("/fig") {
            ("figure", label)
        } else {
            ("value", label)
        };
        values.push((kind, name, label.to_owned(), value));
        if end == remaining.len() {
            break;
        }
        remaining = remaining[end + 1..].trim();
    }
    anyhow::ensure!(!names.is_empty(), "--dialog declaration is empty");
    anyhow::ensure!(names.len() <= 16, "--dialog supports at most 16 items");

    // AviUtl2のパラメーター重複回避処理の再現。本当にワケのわからない動きをしている...
    let mut track_names = std::collections::HashSet::new();
    for capture in lazy_regex::regex_captures_iter!(r"(?m)^--track[0-3]:([^,]+),", script_content) {
        let name = &capture[1];
        track_names.insert(name.to_owned());
    }

    let mut labels = std::collections::HashSet::new();
    let mut labels_counter = std::collections::HashMap::new();
    for (_, _, label, _) in values.iter_mut() {
        let counter = labels_counter.entry(label.clone()).or_insert(0);
        *counter += 1;
    }

    for (_, _, label, _) in values.iter_mut().rev() {
        if !labels.insert(label.clone()) {
            label.insert_str(0, "dialog::");
        } else if track_names.contains(label) {
            if labels_counter[label] > 1 {
                label.insert_str(0, "dialog::dialog::");
            } else {
                label.insert_str(0, "dialog::");
            }
        }
    }

    let values = values
        .into_iter()
        .map(|(kind, name, label, value)| format!("--{kind}@{name}:{label},{value}"))
        .collect::<Vec<_>>();

    let newline = if script_content.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let replacement = format!(
        "--rikky_modoki:dialog_info={}{newline}{}",
        names.join(";"),
        values.join(newline)
    );
    script_content.replace_range(range, &replacement);
    Ok(names)
}

// Lua の初期値は評価せず、文字列・括弧の外にあるセミコロンだけを区切りにする。
fn dialog_value_end(value: &str) -> anyhow::Result<usize> {
    let bytes = value.as_bytes();
    let mut closing = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            quote @ (b'\'' | b'"') => {
                index += 1;
                while index < bytes.len() && bytes[index] != quote {
                    if bytes[index] == b'\\' {
                        index += 1;
                    }
                    index += 1;
                }
                anyhow::ensure!(index < bytes.len(), "Unterminated string in --dialog");
            }
            b'[' => {
                if let Some((open, equals)) =
                    lazy_regex::regex_captures!(r"^\[(=*)\[", &value[index..])
                {
                    let close = format!("]{equals}]");
                    let end = value[index + open.len()..]
                        .find(&close)
                        .ok_or_else(|| anyhow::anyhow!("Unterminated long string in --dialog"))?;
                    index += open.len() + end + close.len() - 1;
                } else {
                    closing.push(b']');
                }
            }
            b'{' => closing.push(b'}'),
            b'(' => closing.push(b')'),
            close @ (b']' | b'}' | b')') => {
                anyhow::ensure!(
                    closing.pop() == Some(close),
                    "Mismatched brackets in --dialog"
                );
            }
            b';' if closing.is_empty() => return Ok(index),
            _ => {}
        }
        index += 1;
    }
    anyhow::ensure!(closing.is_empty(), "Unclosed brackets in --dialog");
    Ok(value.len())
}

fn replacement_version(content: &str) -> anyhow::Result<Option<u32>> {
    let mut versions =
        regex!(r"(?m)^--rikky_modoki:replacement_version=([^\r\n]*)\r?$").captures_iter(content);
    if let Some(version) = versions.next() {
        let version = version[1].parse::<u32>()?;
        anyhow::ensure!(
            versions.next().is_none(),
            "Duplicate replacement version markers"
        );
        return Ok(Some(version));
    }
    // バージョン導入前に生成したファイルだけを0として扱う。
    if regex!(r"(?m)^--rikky_modoki:(?:dialog_info|parameter)=").is_match(content) {
        Ok(Some(0))
    } else {
        Ok(None)
    }
}

fn read_script(path: &std::path::Path) -> anyhow::Result<String> {
    let bytes = std::fs::read(path)?;
    decode_script(&bytes, path)
}

fn decode_script(bytes: &[u8], path: &std::path::Path) -> anyhow::Result<String> {
    let (content, _, errors) = encoding_rs::SHIFT_JIS.decode(bytes);
    anyhow::ensure!(
        !errors,
        "Script contains invalid Shift-JIS: {}",
        path.display()
    );
    Ok(content.into_owned())
}

fn read_script_for_replacement(path: &std::path::Path) -> anyhow::Result<Option<String>> {
    read_script_for_replacement_in(path, &backup_directory()?)
}

fn backup_directory() -> anyhow::Result<std::path::PathBuf> {
    let path = process_path::get_dylib_path()
        .ok_or_else(|| anyhow::anyhow!("Failed to get plugin DLL path"))?;
    let parent = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Plugin DLL has no parent directory"))?;
    Ok(parent.join("backups"))
}

fn original_hash(content: &str) -> anyhow::Result<Option<String>> {
    let mut markers =
        regex!(r"(?m)^--rikky_modoki:original_hash=([^\r\n]*)\r?$").captures_iter(content);
    let Some(marker) = markers.next() else {
        return Ok(None);
    };
    anyhow::ensure!(markers.next().is_none(), "Duplicate original hash markers");
    let hash = &marker[1];
    anyhow::ensure!(
        regex!(r"^[0-9a-f]{16}$").is_match(hash),
        "Invalid original hash: {hash}"
    );
    Ok(Some(hash.to_owned()))
}

fn script_hash(bytes: &[u8]) -> String {
    format!("{:016x}", xxhash_rust::xxh3::xxh3_64(bytes))
}

fn original_script_bytes(
    path: &std::path::Path,
    content: &str,
    backups: &std::path::Path,
) -> anyhow::Result<Vec<u8>> {
    let hash = original_hash(content)?;
    let version = replacement_version(content)?;
    if hash.is_none() && version.is_none() {
        return Ok(std::fs::read(path)?);
    }
    let backup = if let Some(hash) = &hash {
        backups.join(format!("{hash}.bak"))
    } else {
        // original_hash導入前の形式のみ、スクリプト横のbakから移行する。
        anyhow::ensure!(version.is_some_and(|v| v < 3), "Missing original hash");
        path.with_added_extension("bak")
    };
    let bytes = std::fs::read(&backup).map_err(|error| {
        anyhow::anyhow!(
            "元のスクリプトを復元できません: {}: {error}",
            backup.display()
        )
    })?;
    if let Some(hash) = hash {
        anyhow::ensure!(
            script_hash(&bytes) == hash,
            "Backup hash mismatch: {}",
            backup.display()
        );
    }
    let restored = decode_script(&bytes, &backup)?;
    anyhow::ensure!(
        replacement_version(&restored)?.is_none() && original_hash(&restored)?.is_none(),
        "バックアップが置換済みです。元のスクリプトが必要です: {}",
        backup.display()
    );
    Ok(bytes)
}

fn read_script_for_replacement_in(
    path: &std::path::Path,
    backups: &std::path::Path,
) -> anyhow::Result<Option<String>> {
    let content = read_script(path)?;
    if further_replacement_disabled(&content) {
        tracing::debug!(
            "Further parameter replacement is disabled for script '{}'",
            path.display()
        );
        return Ok(None);
    }
    if replacement_version(&content)?.is_some_and(|version| version < REPLACEMENT_VERSION) {
        let bytes = original_script_bytes(path, &content, backups)?;
        let restored = decode_script(&bytes, path)?;
        tracing::info!("バックアップから再置換します: {}", path.display());
        // 再置換が成功するまでは元ファイルもバックアップも変更しない。
        Ok(Some(restored))
    } else {
        Ok(Some(content))
    }
}

fn group_needs_rewrite(content: &str, script_name: &str, index: usize) -> anyhow::Result<bool> {
    match replacement_version(content)? {
        Some(version) if version < REPLACEMENT_VERSION => return Ok(true),
        None => return Ok(false),
        _ => {}
    }
    let range = script_section(content, script_name)?;
    let mut section = content[range].to_owned();
    let names = expand_dialog(&mut section)?;
    let name = names
        .get(index.wrapping_sub(1))
        .ok_or_else(|| anyhow::anyhow!("Invalid dialog parameter index: {index}"))?;
    Ok(!section
        .lines()
        .any(|line| line == format!("--rikky_modoki:parameter={name}")))
}

fn versioned_script(content: &str) -> anyhow::Result<String> {
    if let Some(version) = replacement_version(content)?
        && version >= REPLACEMENT_VERSION
    {
        return Ok(content.to_owned());
    }
    anyhow::ensure!(
        !regex!(r"(?m)^--rikky_modoki:replacement_version=").is_match(content),
        "Old replacement version must be restored before writing"
    );
    let newline = if content.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    Ok(format!(
        "--rikky_modoki:replacement_version={REPLACEMENT_VERSION}{newline}{content}"
    ))
}

fn write_replaced_script(
    script_path: &std::path::Path,
    script_content: &str,
) -> anyhow::Result<()> {
    write_replaced_script_in(script_path, script_content, &backup_directory()?)
}

fn write_replaced_script_in(
    script_path: &std::path::Path,
    script_content: &str,
    backups: &std::path::Path,
) -> anyhow::Result<()> {
    use std::io::Write;
    let original = original_script_bytes(script_path, &read_script(script_path)?, backups)?;
    let hash = script_hash(&original);
    let mut content = versioned_script(script_content)?;

    let newline = if content.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };

    if let Some(existing) = original_hash(&content)? {
        anyhow::ensure!(existing == hash, "Original hash changed during replacement");
    } else {
        content = format!("--rikky_modoki:original_hash={hash}{newline}{content}");
    }

    static ALERT: &[&str] = &[
        "このスクリプトはrikky_modoki.aux2によって編集処理が行われました。",
        "このスクリプトを編集すると、将来のrikky_modoki.aux2の変更で上書きされる可能性があります。",
        "このスクリプトの編集を無効化するには、以下の行の「no」を「yes」に変更してください。",
    ];
    if !regex!(r"(?m)^--rikky_modoki:disable_further_replacement=(?:yes|no)\r?$").is_match(&content)
    {
        content = format!(
            "--{newline}-- {}{newline}--rikky_modoki:disable_further_replacement=no{newline}--{newline}{content}",
            ALERT.join(&format!("{newline}-- "))
        );
    }

    let (encoded, _, errors) = encoding_rs::SHIFT_JIS.encode(&content);
    anyhow::ensure!(!errors, "Script cannot be encoded as Shift-JIS");
    tracing::info!(
        "Writing modified script content back to file: {:?}",
        &script_path
    );
    std::fs::create_dir_all(backups)?;
    let bak_path = backups.join(format!("{hash}.bak"));
    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&bak_path)
    {
        Ok(mut file) => {
            file.write_all(&original)?;
            file.sync_all()?;
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            anyhow::ensure!(
                std::fs::read(&bak_path)? == original,
                "Backup contents differ: {}",
                bak_path.display()
            );
        }
        Err(error) => return Err(error.into()),
    }
    std::fs::write(script_path, encoded)?;
    Ok(())
}

fn update_script_file(script_path: &std::path::Path, script_content: &str) -> anyhow::Result<()> {
    write_replaced_script(script_path, script_content)?;
    if !PARAMETER_REPLACE_NOTIFIED.swap(true, std::sync::atomic::Ordering::SeqCst) {
        native_dialog::DialogBuilder::message()
            .set_title("rikky_modoki")
            .set_text("パラメーターが書き換えられました。AviUtl2を再起動すると反映されます。")
            .set_owner(&unsafe { crate::EDIT_HANDLE.get_host_app_window().unwrap() })
            .alert()
            .show()
            .map_err(|e| anyhow::anyhow!("Failed to show dialog: {}", e))?;
    }
    Ok(())
}

static REPLACED_PARAMETERS: std::sync::LazyLock<
    std::sync::Mutex<std::collections::HashSet<ParamReplaceRequest>>,
> = std::sync::LazyLock::new(|| std::sync::Mutex::new(std::collections::HashSet::new()));

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ParamReplaceRequest {
    script_name: String,
    extension: String,
    index: usize,
}

fn replace_parameter_with_check<F, R, E>(
    script_name: &str,
    extension: &str,
    index: usize,
    f: F,
) -> Result<R, E>
where
    R: std::default::Default,
    F: FnOnce() -> Result<R, E>,
{
    let request = ParamReplaceRequest {
        script_name: script_name.to_owned(),
        extension: extension.to_owned(),
        index,
    };
    let mut replaced = REPLACED_PARAMETERS.lock().unwrap();
    if replaced.contains(&request) {
        tracing::debug!(
            "Parameter replacement already performed for {:?}, skipping",
            request
        );
        return Ok(R::default());
    }
    let result = f();
    if result.is_ok() {
        replaced.insert(request);
    }
    result
}

fn further_replacement_disabled(content: &str) -> bool {
    lazy_regex::regex!(r"(?m)^--rikky_modoki:disable_further_replacement=yes\r?$").is_match(content)
}
