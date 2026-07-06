/*
 * 核心职责：验证远程挂载高级兼容性参数。
 * 业务痛点：Windows 资源管理器与 FTP 视频读取依赖稳定的 VFS 参数组合。
 * 能力边界：只测试参数归一化、默认迁移和 rclone mount 参数生成。
 */

#[cfg(test)]
mod advanced_options_tests {
    use std::path::Path;

    use crate::modules::mounts::{
        dto::{MountAdvancedOptions, MountProfile, MountProtocol, MountStatus},
        service::{
            advanced_options::normalize_advanced_options, profiles::upgrade_profile_defaults,
            rclone_config::build_mount_args, storage::select_default_drive_letter,
        },
    };

    fn ftp_profile() -> MountProfile {
        MountProfile {
            id: "profile-advanced".to_string(),
            name: "FTP".to_string(),
            protocol: MountProtocol::Ftp,
            remote_name: "remote_ftp".to_string(),
            host: Some("127.0.0.1".to_string()),
            port: Some(21),
            username: Some("user".to_string()),
            password: None,
            url: None,
            vendor: None,
            key_file: None,
            remote_path: None,
            mount_point: Some("X:".to_string()),
            drive_letter: None,
            tls_mode: None,
            no_check_certificate: false,
            read_only: false,
            cache_mode: "full".to_string(),
            advanced_options: MountAdvancedOptions::default(),
            enabled: false,
            created_at: 0,
            updated_at: 0,
            mounted: false,
            status: MountStatus::Disabled,
            error: None,
        }
    }

    #[test]
    fn mount_args_include_recommended_compatibility_options() {
        let args = build_mount_args(Path::new("rclone.conf"), Path::new("cache"), &ftp_profile())
            .expect("build mount args");

        assert!(args
            .windows(2)
            .any(|pair| pair == ["--vfs-cache-mode", "full"]));
        assert!(args
            .windows(2)
            .any(|pair| pair == ["--vfs-cache-max-size", "5G"]));
        assert!(args
            .windows(2)
            .any(|pair| pair == ["--vfs-cache-max-age", "24h"]));
        assert!(args
            .windows(2)
            .any(|pair| pair == ["--vfs-read-chunk-size", "64M"]));
        assert!(args.windows(2).any(|pair| pair == ["--buffer-size", "32M"]));
        assert!(args.windows(2).any(|pair| pair == ["--poll-interval", "0"]));
        assert!(args.windows(2).any(|pair| pair == ["--contimeout", "5s"]));
        assert!(args.windows(2).any(|pair| pair == ["--timeout", "30s"]));
        assert!(args.windows(2).any(|pair| pair == ["--retries", "1"]));
        assert!(args
            .windows(2)
            .any(|pair| pair == ["--low-level-retries", "3"]));
        assert!(args
            .windows(2)
            .any(|pair| pair == ["--retries-sleep", "2s"]));
        assert!(args.iter().any(|arg| arg == "--links"));
    }

    #[cfg(windows)]
    #[test]
    fn windows_drive_mount_args_include_network_mode() {
        let mut profile = ftp_profile();
        profile.mount_point = None;
        profile.drive_letter = Some("Z:".to_string());
        profile.advanced_options.network_mode = true;

        let args = build_mount_args(Path::new("rclone.conf"), Path::new("cache"), &profile)
            .expect("build mount args");

        assert!(args.iter().any(|arg| arg == "--network-mode"));
    }

    #[cfg(windows)]
    #[test]
    fn windows_directory_mount_args_do_not_include_network_mode() {
        let mut profile = ftp_profile();
        profile.mount_point = Some(
            std::env::temp_dir()
                .join("nas")
                .to_string_lossy()
                .to_string(),
        );
        profile.drive_letter = None;
        profile.advanced_options.network_mode = true;

        let args = build_mount_args(Path::new("rclone.conf"), Path::new("cache"), &profile)
            .expect("build mount args");

        assert!(!args.iter().any(|arg| arg == "--network-mode"));
    }

    #[cfg(windows)]
    #[test]
    fn default_drive_letter_skips_used_and_occupied_letters() {
        let used = vec!["Z:".to_string(), "Y:".to_string()];
        let selected = select_default_drive_letter(&used, |letter| letter == 'X');

        assert_eq!(selected.as_deref(), Some("W:"));
    }

    #[test]
    fn advanced_options_reject_invalid_values() {
        let mut options = MountAdvancedOptions::default();
        options.buffer_size = "fast".to_string();

        let error = normalize_advanced_options(Some(options), true)
            .expect_err("reject invalid buffer size");

        assert_eq!(error.code, "mount_invalid_advanced_option");
    }

    #[test]
    fn old_ftp_writes_cache_is_upgraded_to_full() {
        let mut profile = ftp_profile();
        profile.cache_mode = "writes".to_string();
        let mut profiles = vec![profile];

        assert!(upgrade_profile_defaults(&mut profiles));
        assert_eq!(profiles[0].cache_mode, "full");
    }
}
