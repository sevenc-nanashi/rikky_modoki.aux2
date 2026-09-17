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

    fn rewrite_parameter(
        &self,
        script_name: String,
        extension: String,
        parameter_type: String,
        index: usize,
    ) -> aviutl2::common::AnyResult<()> {
        let script_file_path = find_script_file(&script_name, &extension)?;
        let mut script_content = encoding_rs::SHIFT_JIS
            .decode(&std::fs::read(&script_file_path)?)
            .0
            .into_owned();
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

        update_script_file(&script_file_path, &script_content)?;
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
        let mut script_content = encoding_rs::SHIFT_JIS
            .decode(&std::fs::read(&script_file_path)?)
            .0
            .into_owned();
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

        update_script_file(&script_file_path, &script_content)?;
        Ok(())
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
    let script_file_name = format!("{}.{}", script_name.split("@").last().unwrap(), extension);
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
        values.push(format!("--value@{name}:{},{value}", label.trim()));
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
    tracing::info!(
        "Writing modified script content back to file: {:?}",
        &script_path
    );
    let bak_path = script_path.with_added_extension("bak");
    if !bak_path.exists() {
        std::fs::copy(script_path, &bak_path).map_err(|e| {
            anyhow::anyhow!(
                "Failed to create backup file {:?}: {}",
                bak_path,
                e
            )
        })?;
    }
    std::fs::write(script_path, encoding_rs::SHIFT_JIS.encode(script_content).0)?;

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
    use super::expand_dialog;

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
