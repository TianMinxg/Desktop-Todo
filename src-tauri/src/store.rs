use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

static IO: Mutex<()> = Mutex::new(());

fn io_lock() -> std::sync::MutexGuard<'static, ()> {
    IO.lock().unwrap_or_else(|poison| poison.into_inner())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Todo {
    pub id: String,
    pub title: String,
    pub completed: bool,
    pub reminder_at: Option<String>,
    pub reminded: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct Settings {
    #[serde(default)]
    data_dir: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataDirUpdate {
    pub data_dir: String,
    pub todos: Vec<Todo>,
}

fn app_config_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Desktop Todo Tauri")
}

fn settings_file() -> PathBuf {
    app_config_dir().join("settings.json")
}

fn ensure_dir(path: &Path) -> Result<(), String> {
    fs::create_dir_all(path).map_err(|e| format!("无法创建目录：{e}"))
}

fn display_path(path: &Path) -> String {
    let canonical = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let text = canonical.to_string_lossy();
    let stripped = if let Some(rest) = text.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{rest}")
    } else if let Some(rest) = text.strip_prefix(r"\\?\") {
        rest.to_string()
    } else {
        text.into_owned()
    };
    stripped.trim_end_matches(['\\', '/']).to_string()
}

fn same_dir(a: &Path, b: &Path) -> bool {
    match (fs::canonicalize(a), fs::canonicalize(b)) {
        (Ok(left), Ok(right)) => left == right,
        _ => display_path(a).eq_ignore_ascii_case(&display_path(b)),
    }
}

fn load_settings_unlocked() -> Settings {
    let Ok(raw) = fs::read_to_string(settings_file()) else {
        return Settings::default();
    };
    serde_json::from_str(&raw).unwrap_or_default()
}

fn save_settings_unlocked(settings: &Settings) -> Result<(), String> {
    ensure_dir(&app_config_dir())?;
    let raw = serde_json::to_string_pretty(settings).map_err(|e| format!("无法保存设置：{e}"))?;
    fs::write(settings_file(), raw).map_err(|e| format!("无法保存设置：{e}"))
}

fn configured_data_dir_unlocked() -> PathBuf {
    if let Some(dir) = load_settings_unlocked().data_dir {
        let path = PathBuf::from(dir.trim());
        if path.is_absolute() {
            return path;
        }
    }
    app_config_dir()
}

fn todos_file_unlocked() -> PathBuf {
    configured_data_dir_unlocked().join("todos.json")
}

fn load_todos_unlocked() -> Vec<Todo> {
    let path = todos_file_unlocked();
    if !path.exists() {
        return Vec::new();
    }
    match fs::read_to_string(path) {
        Ok(raw) => serde_json::from_str(&raw).unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

fn save_todos_unlocked(todos: &[Todo]) {
    let path = todos_file_unlocked();
    if let Some(parent) = path.parent() {
        let _ = ensure_dir(parent);
    }
    if let Ok(raw) = serde_json::to_string_pretty(todos) {
        let _ = fs::write(path, raw);
    }
}

fn parse_data_dir(raw: &str) -> Result<PathBuf, String> {
    let trimmed = raw.trim().trim_end_matches(['\\', '/']);
    if trimmed.is_empty() {
        return Err("请填写数据目录".into());
    }
    let path = PathBuf::from(trimmed);
    if !path.is_absolute() {
        return Err("请填写完整路径，例如 D:\\Todos".into());
    }
    if path.exists() && !path.is_dir() {
        return Err("该路径是文件，请改成文件夹".into());
    }
    Ok(path)
}

fn move_file(from: &Path, to: &Path) -> Result<(), String> {
    if let Some(parent) = to.parent() {
        ensure_dir(parent)?;
    }
    if fs::rename(from, to).is_ok() {
        return Ok(());
    }
    fs::copy(from, to).map_err(|e| format!("无法移动数据文件：{e}"))?;
    fs::remove_file(from).map_err(|e| format!("无法移动数据文件：{e}"))?;
    Ok(())
}

pub fn load_todos() -> Vec<Todo> {
    let _guard = io_lock();
    load_todos_unlocked()
}

pub fn merge_and_save(incoming: Vec<Todo>) -> Vec<Todo> {
    let _guard = io_lock();
    let existing = load_todos_unlocked();
    let merged: Vec<Todo> = incoming
        .into_iter()
        .map(|todo| {
            if let Some(prev) = existing.iter().find(|t| t.id == todo.id) {
                if prev.reminded && !todo.reminded && prev.reminder_at == todo.reminder_at {
                    return Todo {
                        reminded: true,
                        ..todo
                    };
                }
            }
            todo
        })
        .collect();
    save_todos_unlocked(&merged);
    merged
}

pub fn current_data_dir() -> String {
    let _guard = io_lock();
    let path = configured_data_dir_unlocked();
    let _ = ensure_dir(&path);
    display_path(&path)
}

pub fn set_data_dir(raw: &str) -> Result<DataDirUpdate, String> {
    let _guard = io_lock();
    let target = if raw.trim().is_empty() {
        app_config_dir()
    } else {
        parse_data_dir(raw)?
    };
    ensure_dir(&app_config_dir())?;
    ensure_dir(&target)?;

    let old_file = todos_file_unlocked();
    let new_file = target.join("todos.json");
    let moved = if !same_dir(old_file.parent().unwrap_or(Path::new("")), &target)
        && old_file.exists()
        && !new_file.exists()
    {
        move_file(&old_file, &new_file)?;
        true
    } else {
        false
    };

    let mut settings = load_settings_unlocked();
    settings.data_dir = if same_dir(&target, &app_config_dir()) {
        None
    } else {
        Some(display_path(&target))
    };
    if let Err(err) = save_settings_unlocked(&settings) {
        if moved {
            let _ = move_file(&new_file, &old_file);
        }
        return Err(err);
    }

    Ok(DataDirUpdate {
        data_dir: display_path(&target),
        todos: load_todos_unlocked(),
    })
}

pub fn take_due_reminders(now_ms: i64) -> Vec<Todo> {
    let _guard = io_lock();
    let mut todos = load_todos_unlocked();
    let due: Vec<Todo> = todos
        .iter()
        .filter(|todo| {
            if todo.completed || todo.reminded {
                return false;
            }
            let Some(reminder) = &todo.reminder_at else {
                return false;
            };
            match chrono::DateTime::parse_from_rfc3339(reminder) {
                Ok(dt) => dt.timestamp_millis() <= now_ms,
                Err(_) => false,
            }
        })
        .cloned()
        .collect();
    if due.is_empty() {
        return due;
    }
    let ids: Vec<String> = due.iter().map(|todo| todo.id.clone()).collect();
    for todo in &mut todos {
        if ids.iter().any(|id| id == &todo.id) {
            todo.reminded = true;
        }
    }
    save_todos_unlocked(&todos);
    due
}
