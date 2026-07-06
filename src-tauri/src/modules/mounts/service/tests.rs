/*
 * 核心职责：验证远程挂载关键规则。
 * 业务痛点：拆分后必须保留目录和参数生成回归测试。
 * 能力边界：只包含挂载模块内部单元测试。
 */

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use crate::modules::mounts::{
        dto::{MountAdvancedOptions, MountProfile, MountProtocol, MountStatus},
        service::{
            normalize::{
                normalize_tls_mode, now_millis, read_remote_option, remove_blank_remote_option,
                remove_remote_option,
            },
            profiles::{password_update_from_input, MountPasswordUpdate},
            rclone_config::{build_config_args, config_options},
            storage::default_mount_dir_name,
            target::{prepare_directory_mount_target, suggested_mount_target},
        },
    };

    fn ftp_profile() -> MountProfile {
        MountProfile {
            id: "profile-1".to_string(),
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
    fn config_args_keep_non_interactive_inside_config_subcommand() {
        let profile = ftp_profile();
        let args = build_config_args(
            Path::new("rclone.conf"),
            "create",
            &profile,
            &[
                ("host".to_string(), "127.0.0.1".to_string()),
                ("user".to_string(), "user".to_string()),
                ("pass".to_string(), "secret".to_string()),
            ],
        );

        assert_eq!(args[0], "--config");
        assert_eq!(args[2], "config");
        assert_eq!(args[3], "--obscure");
        assert_eq!(args[4], "--non-interactive");
        assert_eq!(args[5], "create");
        assert_eq!(args[6], "remote_ftp");
        assert_eq!(args[7], "ftp");
    }

    #[test]
    fn ftp_config_args_include_supplied_endpoint_and_credentials() {
        let mut profile = ftp_profile();
        profile.host = Some("192.168.88.186".to_string());
        profile.port = Some(21);
        profile.username = Some("was".to_string());

        let mut options = config_options(&profile);
        options.push(("pass".to_string(), "123456Aa".to_string()));
        let args = build_config_args(Path::new("rclone.conf"), "create", &profile, &options);

        assert!(args
            .windows(2)
            .any(|pair| pair == ["host", "192.168.88.186"]));
        assert!(args.windows(2).any(|pair| pair == ["port", "21"]));
        assert!(args.windows(2).any(|pair| pair == ["user", "was"]));
        assert!(args.windows(2).any(|pair| pair == ["pass", "123456Aa"]));
        assert_eq!(args[2], "config");
        assert_eq!(args[3], "--obscure");
        assert_eq!(args[5], "create");
        assert_eq!(args[7], "ftp");
    }

    #[test]
    fn remove_blank_remote_option_only_cleans_target_blank_value() {
        let config_path = std::env::temp_dir().join(format!(
            "local-efficiency-rclone-config-{}.conf",
            now_millis()
        ));
        fs::write(
            &config_path,
            "[remote_ftp]\ntype = ftp\npass = \nuser = was\n\n[other]\npass = \n",
        )
        .expect("write config");

        remove_blank_remote_option(&config_path, "remote_ftp", "pass").expect("clean blank pass");
        let content = fs::read_to_string(&config_path).expect("read config");

        assert!(content.contains("[remote_ftp]\ntype = ftp\nuser = was"));
        assert!(content.contains("[other]\npass = "));
        let _ = fs::remove_file(config_path);
    }

    #[test]
    fn read_remote_option_returns_target_value_only() {
        let config_path = std::env::temp_dir().join(format!(
            "local-efficiency-rclone-read-{}.conf",
            now_millis()
        ));
        fs::write(
            &config_path,
            "[other]\npass = wrong\n\n[remote_ftp]\ntype = ftp\npass = secret\n",
        )
        .expect("write config");

        let value = read_remote_option(&config_path, "remote_ftp", "pass").expect("read option");

        assert_eq!(value.as_deref(), Some("secret"));
        let _ = fs::remove_file(config_path);
    }

    #[test]
    fn password_update_distinguishes_unchanged_set_and_clear() {
        assert_eq!(
            password_update_from_input(None),
            MountPasswordUpdate::Unchanged
        );
        assert_eq!(
            password_update_from_input(Some(" secret ")),
            MountPasswordUpdate::Set("secret".to_string())
        );
        assert_eq!(
            password_update_from_input(Some("  ")),
            MountPasswordUpdate::Clear
        );
    }

    #[test]
    fn normalize_tls_mode_defaults_to_disabled() {
        assert_eq!(normalize_tls_mode(&MountProtocol::Ftp, None), None);
        assert_eq!(
            normalize_tls_mode(&MountProtocol::Ftp, Some("none".to_string())),
            None
        );
        assert_eq!(
            normalize_tls_mode(&MountProtocol::Ftp, Some("EXPLICIT".to_string())),
            Some("explicit".to_string())
        );
        assert_eq!(
            normalize_tls_mode(&MountProtocol::Sftp, Some("implicit".to_string())),
            None
        );
    }

    #[test]
    fn remove_remote_option_deletes_only_target_option() {
        let config_path = std::env::temp_dir().join(format!(
            "local-efficiency-rclone-remove-{}.conf",
            now_millis()
        ));
        fs::write(
            &config_path,
            "[remote_ftp]\ntype = ftp\npass = broken\nuser = was\n\n[other]\npass = keep\n",
        )
        .expect("write config");

        remove_remote_option(&config_path, "remote_ftp", "pass").expect("remove option");
        let content = fs::read_to_string(&config_path).expect("read config");

        assert!(content.contains("[remote_ftp]\ntype = ftp\nuser = was"));
        assert!(content.contains("[other]\npass = keep"));
        let _ = fs::remove_file(config_path);
    }

    #[test]
    fn default_mount_dir_name_sanitizes_invalid_path_chars() {
        assert_eq!(default_mount_dir_name("na<s"), "na_s");
        assert_eq!(default_mount_dir_name("   "), "remote");
    }

    #[test]
    fn default_mount_dir_name_does_not_append_id_suffix() {
        assert_eq!(default_mount_dir_name("nas"), "nas");
    }

    #[cfg(windows)]
    #[test]
    fn prepare_directory_mount_target_removes_existing_empty_directory() {
        let target =
            std::env::temp_dir().join(format!("local-efficiency-empty-mount-{}", now_millis()));
        fs::create_dir_all(&target).expect("create empty mount target");

        prepare_directory_mount_target(&target).expect("remove empty target");

        assert!(!target.exists());
    }

    #[cfg(windows)]
    #[test]
    fn prepare_directory_mount_target_rejects_non_empty_directory() {
        let target =
            std::env::temp_dir().join(format!("local-efficiency-non-empty-mount-{}", now_millis()));
        fs::create_dir_all(&target).expect("create non-empty mount target");
        fs::write(target.join("existing.txt"), "keep").expect("create existing file");

        let error = prepare_directory_mount_target(&target).expect_err("reject non-empty target");

        assert_eq!(error.code, "mount_target_exists");
        assert!(target.exists());
        let _ = fs::remove_dir_all(&target);
    }

    #[cfg(windows)]
    #[test]
    fn suggested_mount_target_skips_existing_suffixes() {
        let root =
            std::env::temp_dir().join(format!("local-efficiency-suggest-mount-{}", now_millis()));
        let target = root.join("nas");
        fs::create_dir_all(root.join("nas-1")).expect("create first suffix");
        fs::create_dir_all(root.join("nas-2")).expect("create second suffix");

        let suggested = suggested_mount_target(&target);

        assert_eq!(suggested, root.join("nas-3"));
        let _ = fs::remove_dir_all(&root);
    }
}
