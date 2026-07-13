/*
 * 核心职责：为探测到的远端目录计算跨平台本地目标建议。
 * 能力边界：只生成建议值，不预占盘符或创建目录。
 */

use super::super::*;

pub(super) fn apply_suggested_targets(
    app: &AppHandle,
    connection: &RemoteConnection,
    store: &MountStore,
    entries: &mut [ProbeShareEntry],
) -> AppResult<()> {
    #[cfg(windows)]
    {
        let _ = (app, connection);
        let mut used = store
            .workspaces
            .iter()
            .filter_map(|workspace| workspace.drive_letter.clone())
            .chain(store.workspaces.iter().flat_map(|workspace| {
                workspace
                    .bindings
                    .iter()
                    .filter_map(|binding| binding.drive_letter.clone())
            }))
            .collect::<Vec<_>>();
        for entry in entries {
            let suggested = super::super::storage::select_default_drive_letter(&used, |letter| {
                PathBuf::from(format!("{}:\\", letter)).exists()
            });
            if let Some(letter) = suggested {
                used.push(letter.clone());
                entry.suggested_drive_letter = Some(letter);
            }
        }
    }
    #[cfg(not(windows))]
    {
        let _ = store;
        let (root, _) = super::super::storage::default_mount_root(app)?;
        let workspace_root = root.join(super::super::storage::default_mount_dir_name(
            &connection.name,
        ));
        for entry in entries {
            entry.suggested_mount_point = Some(
                workspace_root
                    .join(super::super::storage::default_mount_dir_name(&entry.name))
                    .to_string_lossy()
                    .to_string(),
            );
        }
    }
    Ok(())
}
