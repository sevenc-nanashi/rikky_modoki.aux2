use aviutl2::module::ScriptModuleFunctions;
use lazy_regex::regex;

pub static PROJECT_PATH: std::sync::Mutex<Option<std::path::PathBuf>> = std::sync::Mutex::new(None);
pub static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

static PARAMETER_REPLACED: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

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
        crate::sound::receiving(frame)
    }

    fn sound_length(&self, file: String) -> aviutl2::common::AnyResult<Option<f64>> {
        crate::sound::length(&file)
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
        crate::sound::register(
            origin_frame,
            crate::sound::Sound {
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
        let script_file_path = find_script_file(&script_name, &extension)?;
        let mut content = encoding_rs::SHIFT_JIS
            .decode(&std::fs::read(&script_file_path)?)
            .0
            .into_owned();
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
    }

    fn rewrite_select_parameter(
        &self,
        script_name: String,
        extension: String,
        index: usize,
        choices: Vec<String>,
    ) -> aviutl2::common::AnyResult<()> {
        let script_file_path = find_script_file(&script_name, &extension)?;
        let mut content = encoding_rs::SHIFT_JIS
            .decode(&std::fs::read(&script_file_path)?)
            .0
            .into_owned();
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

        let select_line = format!(
            "--select@tmp_{}:{}={},{}",
            name,
            label,
            unescape_string(&default),
            choices
                .iter()
                .enumerate()
                .map(|(i, choice)| format!("{}={}", choice, i + 1))
                .collect::<Vec<_>>()
                .join(",")
        );
        let mapping_line = if label.starts_with("*") {
            format!("local {} = tostring(tmp_{})", name, name)
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
    }

    fn rewrite_group_parameter(
        &self,
        script_name: String,
        extension: String,
        index: usize,
        entries: Vec<String>,
    ) -> aviutl2::common::AnyResult<()> {
        let path = find_script_file(&script_name, &extension)?;
        let bytes = std::fs::read(&path)?;
        let (content, _, errors) = encoding_rs::SHIFT_JIS.decode(&bytes);
        anyhow::ensure!(!errors, "Script contains invalid Shift-JIS");
        let mut content = content.into_owned();
        if expand_parameter_group(&mut content, &script_name, index, &entries)? {
            update_script_file(&path, &content)?;
        }
        Ok(())
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
    let mut lines = vec![marker, format!("--group:{label},true")];
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
    // 既存のグループ内にある項目を展開した場合は、後続項目の所属を戻す。
    if let Some(group) = section[..parameter_range.start]
        .lines()
        .rev()
        .find(|line| *line == "--group" || line.starts_with("--group:"))
        .filter(|line| line.starts_with("--group:"))
    {
        lines.push(group.to_owned());
    }
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
        values.push(format!("--{kind}@{name}:{label},{value}"));
        if end == remaining.len() {
            break;
        }
        remaining = remaining[end + 1..].trim();
    }
    anyhow::ensure!(!names.is_empty(), "--dialog declaration is empty");
    anyhow::ensure!(names.len() <= 16, "--dialog supports at most 16 items");

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

fn update_script_file(script_path: &std::path::Path, script_content: &str) -> anyhow::Result<()> {
    let (encoded, _, errors) = encoding_rs::SHIFT_JIS.encode(script_content);
    anyhow::ensure!(!errors, "Script cannot be encoded as Shift-JIS");
    tracing::info!(
        "Writing modified script content back to file: {:?}",
        &script_path
    );
    let bak_path = script_path.with_added_extension("bak");
    if !bak_path.exists() {
        std::fs::copy(script_path, &bak_path)
            .map_err(|e| anyhow::anyhow!("Failed to create backup file {:?}: {}", bak_path, e))?;
    }
    std::fs::write(script_path, encoded)?;

    if !PARAMETER_REPLACED.swap(true, std::sync::atomic::Ordering::SeqCst) {
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

#[cfg(test)]
mod tests {
    use super::{expand_dialog, expand_parameter_group};

    #[test]
    fn expands_parameter_groups_without_changing_other_scripts() {
        let entries: Vec<String> = [
            "数値",
            "0",
            "-100",
            "100",
            "細かさ*0.001",
            "10",
            "0",
            "100",
            "オンオフ",
            "false",
            "0",
            "0",
            "文字",
            r#""日本語,\"引用\"\\パス""#,
            "0",
            "0",
            "配列",
            r#"{1; false; "a,b"}"#,
            "0",
            "0",
            "自由",
            "0",
            "0",
            "0",
            "ファイル*file",
            r#""C:\\画像.png""#,
            "0",
            "0",
            "フォルダ*folder",
            r#""C:\\素材""#,
            "0",
            "0",
            "フォント*font",
            r#""ＭＳ ゴシック""#,
            "0",
            "0",
            "色*color",
            "16711680",
            "0",
            "0",
            "透明*color",
            "nil",
            "0",
            "0",
        ]
        .map(str::to_owned)
        .to_vec();
        let declarations = [
            "--track@__rikky_parameter_val_1:__rikky_parameter_val_1::数値,-100,100,0,1",
            "--track@__rikky_parameter_val_2:__rikky_parameter_val_2::細かさ,0,100,10,0.001",
            "--check@__rikky_parameter_val_3:__rikky_parameter_val_3::オンオフ,false",
            r#"--string@__rikky_parameter_val_4:__rikky_parameter_val_4::文字,日本語,"引用"\パス"#,
            r#"--value@__rikky_parameter_val_5:__rikky_parameter_val_5::配列,{1; false; "a,b"}"#,
            "--value@__rikky_parameter_val_6:__rikky_parameter_val_6::自由,0",
            "--file@__rikky_parameter_val_7:__rikky_parameter_val_7::ファイル",
            "--folder@__rikky_parameter_val_8:__rikky_parameter_val_8::フォルダ",
            "--font@__rikky_parameter_val_9:__rikky_parameter_val_9::フォント,ＭＳ ゴシック",
            "--color@__rikky_parameter_val_10:__rikky_parameter_val_10::色,16711680",
            "--color@__rikky_parameter_val_11:__rikky_parameter_val_11::透明,nil",
        ];
        for newline in ["\n", "\r\n"] {
            let before = format!("@別{newline}--dialog:別,val=\"\"{newline}obj.draw(){newline}");
            let after = format!("@次{newline}--dialog:次,val=\"\"{newline}obj.draw()");
            let section = [
                "@対象",
                "--group:既存,false",
                r#"--dialog:設定,local val="古い保存値";後,next_value=0;設定2,local val2="""#,
                "--[=[",
                "説明",
                "]=]",
                "require(\"rikky_module\")",
                "obj.draw()",
                "",
            ]
            .join(newline);
            let mut script = format!("{before}{section}{after}");
            assert!(expand_parameter_group(&mut script, "対象@まとめ", 1, &entries).unwrap());
            assert!(script.starts_with(&before) && script.ends_with(&after));
            for declaration in declarations {
                assert!(
                    script.lines().any(|line| line == declaration),
                    "{declaration}"
                );
            }
            assert!(!script.contains("古い保存値"));
            assert!(script.contains("--rikky_modoki:dialog_info=val;next_value;val2"));
            assert!(script.contains(&format!(
                "--group{newline}--group:既存,false{newline}--value@next_value:後,0"
            )));
            let assignment = script.find("local val = {").unwrap();
            assert!(assignment > script.find("]=]").unwrap());
            assert!(assignment < script.find("require(\"rikky_module\")").unwrap());
            let rewritten = script.clone();
            assert!(!expand_parameter_group(&mut script, "対象@まとめ", 1, &entries).unwrap());
            assert_eq!(script, rewritten);
            assert!(expand_parameter_group(&mut script, "対象@まとめ", 3, &entries).unwrap());
            assert!(script.contains("--rikky_modoki:parameter=val2"));
            assert!(script.contains("local val2 = {__rikky_parameter_val2_1,"));
            assert!(script.starts_with(&before) && script.ends_with(&after));
            if newline == "\r\n" {
                assert!(!script.replace("\r\n", "").contains('\n'));
            }
        }
        let mut no_newline = "--dialog:設定,val=\"\"".to_owned();
        assert!(expand_parameter_group(&mut no_newline, "test", 1, &entries).unwrap());
        assert!(no_newline.contains("--group\nlocal val = {"));
        for comment in ["--[[説明]] ", "--[=[\n説明\n]=] --[[補足]] "] {
            let mut script = format!("--dialog:設定,val=\"\"\n{comment}obj.draw()");
            assert!(expand_parameter_group(&mut script, "test", 1, &entries).unwrap());
            assert!(script.contains(&format!("{comment}local val = {{")));
            assert!(script.ends_with("}\nobj.draw()"));
        }

        for invalid in [
            vec![],
            vec!["数値", "0", "-1"],
            vec!["数値", "10", "0", "1"],
            vec!["数値", "0", "2", "1"],
            vec!["数値", "0", "NaN", "1"],
            vec!["数値*0.02", "0", "0", "1"],
            vec!["ファイル*file", "0", "0", "0"],
            vec!["色*color", "16777216", "0", "0"],
            vec!["文字\n--file@x:x", "0", "0", "0"],
        ] {
            let mut script = "--dialog:設定,val=\"\"\nobj.draw()".to_owned();
            let original = script.clone();
            let invalid = invalid.into_iter().map(str::to_owned).collect::<Vec<_>>();
            assert!(expand_parameter_group(&mut script, "test", 1, &invalid).is_err());
            assert_eq!(script, original);
        }
        for (name, index) in [("test", 0), ("test", 2), ("不存在@test", 1)] {
            let mut script = "--dialog:設定,val=\"\"".to_owned();
            let original = script.clone();
            assert!(expand_parameter_group(&mut script, name, index, &entries).is_err());
            assert_eq!(script, original);
        }
    }

    #[test]
    fn expands_dialog_and_preserves_defaults() {
        let declaration = r#"--dialog:サイズ,size=100;色/col,local color=0xff0000;図形/fig,local fig="四角形";文字,text="a;\"b,c=d";座標,pos={1; {2, 3}; label='x;y'};長文,long=[==[a;],=b]==];通常,local_name=1"#;
        let names = ["size", "color", "fig", "text", "pos", "long", "local_name"];
        let expected = [
            "--rikky_modoki:dialog_info=size;color;fig;text;pos;long;local_name",
            "--value@size:サイズ,100",
            "--value@color:色/col,0xff0000",
            r#"--value@fig:図形/fig,"四角形""#,
            r#"--value@text:文字,"a;\"b,c=d""#,
            "--value@pos:座標,{1; {2, 3}; label='x;y'}",
            "--value@long:長文,[==[a;],=b]==]",
            "--value@local_name:通常,1",
        ];
        for newline in ["\n", "\r\n"] {
            for terminator in ["", ";"] {
                for suffix in ["".to_owned(), format!("{newline}obj.draw(){newline}")] {
                    let prefix = format!("--track0:速度,0,100,10{newline}");
                    let mut script = format!("{prefix}{declaration}{terminator}{suffix}");
                    assert_eq!(expand_dialog(&mut script).unwrap(), names);
                    assert_eq!(
                        script,
                        format!("{prefix}{}{suffix}", expected.join(newline))
                    );
                    let rewritten = script.clone();
                    assert_eq!(expand_dialog(&mut script).unwrap(), names);
                    assert_eq!(script, rewritten);
                }
            }
        }

        for declaration in [
            "--dialog:",
            "--dialog:値,num=",
            "--dialog:値,1num=0",
            "--dialog:値,num='unterminated",
            "--dialog:値,num=[=[unterminated",
            "--dialog:値,num={1,2",
            "--dialog:値,num=(1]",
        ] {
            let mut script = declaration.to_owned();
            assert!(expand_dialog(&mut script).is_err(), "{declaration}");
            assert_eq!(script, declaration);
        }
    }
}
