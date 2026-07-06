/*
 * 核心职责：提供媒体运行时路径工具。
 * 业务痛点：平台路径解析需要集中，避免命令执行路径不一致。
 * 能力边界：只提供路径、可执行文件和展示字符串工具。
 */

use super::*;

pub(super) fn app_ffmpeg_dir(app: &AppHandle) -> AppResult<PathBuf> {
    let dir = app.path().app_data_dir()?.join("ffmpeg");
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub(super) fn ffmpeg_runtime_dir(app: &AppHandle) -> AppResult<PathBuf> {
    let dir = app_ffmpeg_dir(app)?.join("runtime");
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub(super) fn ffmpeg_binary_path(app: &AppHandle) -> AppResult<PathBuf> {
    Ok(ffmpeg_runtime_dir(app)?.join(ffmpeg_executable_name()))
}

pub(super) fn ffprobe_binary_path(app: &AppHandle) -> AppResult<PathBuf> {
    Ok(ffmpeg_runtime_dir(app)?.join(ffprobe_executable_name()))
}

pub(super) fn resolve_executable_path(name: &str) -> Option<PathBuf> {
    let direct = Path::new(name);
    if direct.components().count() > 1 || direct.is_absolute() {
        return executable_file(direct);
    }

    let path_var = env::var_os("PATH")?;
    for dir in env::split_paths(&path_var) {
        for candidate in executable_candidates(&dir, name) {
            if let Some(path) = executable_file(&candidate) {
                return Some(path);
            }
        }
    }
    None
}

#[cfg(windows)]
pub(super) fn executable_candidates(dir: &Path, name: &str) -> Vec<PathBuf> {
    let mut candidates = vec![dir.join(name)];
    if Path::new(name).extension().is_none() {
        let pathext = env::var("PATHEXT").unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".to_string());
        for extension in pathext
            .split(';')
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            let extension = if extension.starts_with('.') {
                extension.to_string()
            } else {
                format!(".{extension}")
            };
            candidates.push(dir.join(format!("{name}{extension}")));
        }
    }
    candidates
}

#[cfg(not(windows))]
pub(super) fn executable_candidates(dir: &Path, name: &str) -> Vec<PathBuf> {
    vec![dir.join(name)]
}

pub(super) fn executable_file(path: &Path) -> Option<PathBuf> {
    let metadata = fs::metadata(path).ok()?;
    if !metadata.is_file() {
        return None;
    }
    #[cfg(unix)]
    if metadata.permissions().mode() & 0o111 == 0 {
        return None;
    }
    Some(fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf()))
}

pub(super) fn hidden_command<S: AsRef<std::ffi::OsStr>>(program: S) -> Command {
    let mut command = Command::new(program);
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
}

pub(super) fn display_path(path: &Path) -> String {
    let text = path.to_string_lossy();
    #[cfg(windows)]
    {
        if let Some(stripped) = text.strip_prefix(r"\\?\UNC\") {
            return format!(r"\\{stripped}");
        }
        if let Some(stripped) = text.strip_prefix(r"\\?\") {
            return stripped.to_string();
        }
    }
    text.to_string()
}
