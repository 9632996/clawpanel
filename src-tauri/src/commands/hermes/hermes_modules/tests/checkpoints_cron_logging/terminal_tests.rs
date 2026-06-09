#[cfg(test)]
mod hermes_terminal_config_tests {
    use super::{build_hermes_terminal_config_values, merge_hermes_terminal_config};

    #[test]
    fn terminal_values_have_upstream_defaults() {
        let config: serde_yaml::Value = serde_yaml::from_str("{}").unwrap();
        let values = build_hermes_terminal_config_values(&config);
        assert_eq!(values["terminalBackend"], "local");
        assert_eq!(values["terminalCwd"], ".");
        assert_eq!(values["terminalTimeout"], 180);
        assert_eq!(values["terminalLifetimeSeconds"], 300);
        assert_eq!(values["terminalShellInitFiles"], "");
        assert_eq!(values["terminalAutoSourceBashrc"], true);
        assert_eq!(values["terminalPersistentShell"], true);
        assert_eq!(values["terminalEnvPassthrough"], "");
        assert_eq!(values["terminalDockerMountCwdToWorkspace"], false);
        assert_eq!(values["terminalDockerRunAsHostUser"], false);
        assert_eq!(values["terminalContainerCpu"], 1);
        assert_eq!(values["terminalContainerMemory"], 5120);
        assert_eq!(values["terminalContainerDisk"], 51200);
        assert_eq!(values["terminalContainerPersistent"], true);
        assert_eq!(values["terminalDockerImage"], "");
        assert_eq!(values["terminalSingularityImage"], "");
        assert_eq!(values["terminalModalImage"], "");
        assert_eq!(values["terminalModalMode"], "auto");
        assert_eq!(values["terminalVercelRuntime"], "node24");
        assert_eq!(values["terminalDaytonaImage"], "");
        assert_eq!(values["terminalDockerForwardEnv"], "");
        assert_eq!(values["terminalDockerEnvJson"], "{}");
        assert_eq!(values["terminalDockerVolumes"], "");
        assert_eq!(values["terminalDockerExtraArgs"], "");
        assert_eq!(values["terminalSshHost"], "");
        assert_eq!(values["terminalSshUser"], "");
        assert_eq!(values["terminalSshPort"], 22);
        assert_eq!(values["terminalSshKey"], "");
    }

    #[test]
    fn terminal_values_read_yaml_fields() {
        let config: serde_yaml::Value = serde_yaml::from_str(
            r#"
terminal:
  backend: docker
  cwd: /workspace
  timeout: 600
  lifetime_seconds: 1800
  shell_init_files:
    - ~/.zshrc
    - ${HOME}/.config/hermes/env.sh
  auto_source_bashrc: false
  persistent_shell: false
  env_passthrough:
    - OPENROUTER_API_KEY
    - GITHUB_TOKEN
  docker_mount_cwd_to_workspace: true
  docker_run_as_host_user: true
  docker_image: nikolaik/python-nodejs:python3.11-nodejs20
  docker_forward_env:
    - GITHUB_TOKEN
    - NPM_TOKEN
  docker_env:
    PLAYWRIGHT_BROWSERS_PATH: /ms-playwright
    PIP_CACHE_DIR: /workspace/.cache/pip
  docker_volumes:
    - /data/projects:/workspace/projects
    - /data/cache:/cache
  docker_extra_args:
    - --network=host
    - --add-host=host.docker.internal:host-gateway
  singularity_image: docker://nikolaik/python-nodejs:python3.11-nodejs20
  modal_image: python:3.12
  modal_mode: managed
  vercel_runtime: python3.13
  daytona_image: ubuntu:24.04
  ssh_host: build.example.com
  ssh_user: deploy
  ssh_port: 2222
  ssh_key: ~/.ssh/hermes_ed25519
  container_cpu: 4
  container_memory: 8192
  container_disk: 102400
  container_persistent: false
"#,
        )
        .unwrap();
        let values = build_hermes_terminal_config_values(&config);
        assert_eq!(values["terminalBackend"], "docker");
        assert_eq!(values["terminalCwd"], "/workspace");
        assert_eq!(values["terminalTimeout"], 600);
        assert_eq!(values["terminalLifetimeSeconds"], 1800);
        assert_eq!(values["terminalShellInitFiles"], "~/.zshrc\n${HOME}/.config/hermes/env.sh");
        assert_eq!(values["terminalAutoSourceBashrc"], false);
        assert_eq!(values["terminalPersistentShell"], false);
        assert_eq!(values["terminalEnvPassthrough"], "OPENROUTER_API_KEY\nGITHUB_TOKEN");
        assert_eq!(values["terminalDockerMountCwdToWorkspace"], true);
        assert_eq!(values["terminalDockerRunAsHostUser"], true);
        assert_eq!(values["terminalDockerImage"], "nikolaik/python-nodejs:python3.11-nodejs20");
        assert_eq!(values["terminalDockerForwardEnv"], "GITHUB_TOKEN\nNPM_TOKEN");
        assert_eq!(
            values["terminalDockerEnvJson"],
            "{\n  \"PLAYWRIGHT_BROWSERS_PATH\": \"/ms-playwright\",\n  \"PIP_CACHE_DIR\": \"/workspace/.cache/pip\"\n}"
        );
        assert_eq!(values["terminalDockerVolumes"], "/data/projects:/workspace/projects\n/data/cache:/cache");
        assert_eq!(
            values["terminalDockerExtraArgs"],
            "--network=host\n--add-host=host.docker.internal:host-gateway"
        );
        assert_eq!(values["terminalSingularityImage"], "docker://nikolaik/python-nodejs:python3.11-nodejs20");
        assert_eq!(values["terminalModalImage"], "python:3.12");
        assert_eq!(values["terminalModalMode"], "managed");
        assert_eq!(values["terminalVercelRuntime"], "python3.13");
        assert_eq!(values["terminalDaytonaImage"], "ubuntu:24.04");
        assert_eq!(values["terminalSshHost"], "build.example.com");
        assert_eq!(values["terminalSshUser"], "deploy");
        assert_eq!(values["terminalSshPort"], 2222);
        assert_eq!(values["terminalSshKey"], "~/.ssh/hermes_ed25519");
        assert_eq!(values["terminalContainerCpu"], 4);
        assert_eq!(values["terminalContainerMemory"], 8192);
        assert_eq!(values["terminalContainerDisk"], 102400);
        assert_eq!(values["terminalContainerPersistent"], false);
    }

    #[test]
    fn merge_terminal_config_preserves_unknown_fields() {
        let mut config: serde_yaml::Value = serde_yaml::from_str(
            r#"
model:
  provider: anthropic
terminal:
  backend: local
  shell_init_files:
    - ~/.profile
  env_passthrough:
    - OLD_TOKEN
  docker_image: custom/python-node
  docker_forward_env:
    - OLD_TOKEN
  docker_env:
    OLD_FLAG: keep-old
  docker_volumes:
    - /old:/old
  docker_extra_args:
    - --old
  custom_flag: keep-terminal
streaming:
  enabled: true
"#,
        )
        .unwrap();

        merge_hermes_terminal_config(
            &mut config,
            &crate::jv!({
                "terminalBackend": "docker",
                "terminalCwd": "/workspace",
                "terminalTimeout": "900",
                "terminalLifetimeSeconds": "1200",
                "terminalShellInitFiles": "~/.zshrc\n${HOME}/.config/hermes/env.sh\n~/.zshrc",
                "terminalAutoSourceBashrc": false,
                "terminalPersistentShell": false,
                "terminalEnvPassthrough": "OPENROUTER_API_KEY\nGITHUB_TOKEN\nOPENROUTER_API_KEY",
                "terminalDockerMountCwdToWorkspace": true,
                "terminalDockerRunAsHostUser": true,
                "terminalDockerImage": "nikolaik/python-nodejs:python3.12-nodejs22",
                "terminalDockerForwardEnv": "GITHUB_TOKEN\nNPM_TOKEN\nGITHUB_TOKEN",
                "terminalDockerEnvJson": "{ \"PLAYWRIGHT_BROWSERS_PATH\": \"/ms-playwright\", \"PIP_CACHE_DIR\": \"/workspace/.cache/pip\" }",
                "terminalDockerVolumes": "/data/projects:/workspace/projects\n/data/cache:/cache\n/data/projects:/workspace/projects",
                "terminalDockerExtraArgs": "--network=host\n--add-host=host.docker.internal:host-gateway\n--network=host",
                "terminalSingularityImage": "docker://ubuntu:24.04",
                "terminalModalImage": "debian:bookworm",
                "terminalModalMode": "direct",
                "terminalVercelRuntime": "node22",
                "terminalDaytonaImage": "ubuntu:22.04",
                "terminalSshHost": "ssh.example.com",
                "terminalSshUser": "hermes",
                "terminalSshPort": "2200",
                "terminalSshKey": "~/.ssh/id_ed25519",
                "terminalContainerCpu": "2",
                "terminalContainerMemory": "6144",
                "terminalContainerDisk": "20480",
                "terminalContainerPersistent": false,
            }),
        )
        .unwrap();

        assert_eq!(config["model"]["provider"].as_str(), Some("anthropic"));
        assert_eq!(config["streaming"]["enabled"].as_bool(), Some(true));
        assert_eq!(config["terminal"]["backend"].as_str(), Some("docker"));
        assert_eq!(config["terminal"]["cwd"].as_str(), Some("/workspace"));
        assert_eq!(config["terminal"]["timeout"].as_i64(), Some(900));
        assert_eq!(config["terminal"]["lifetime_seconds"].as_i64(), Some(1200));
        assert_eq!(config["terminal"]["shell_init_files"][0].as_str(), Some("~/.zshrc"));
        assert_eq!(config["terminal"]["shell_init_files"][1].as_str(), Some("${HOME}/.config/hermes/env.sh"));
        assert_eq!(config["terminal"]["shell_init_files"].as_sequence().unwrap().len(), 2);
        assert_eq!(config["terminal"]["auto_source_bashrc"].as_bool(), Some(false));
        assert_eq!(config["terminal"]["persistent_shell"].as_bool(), Some(false));
        assert_eq!(config["terminal"]["env_passthrough"][0].as_str(), Some("OPENROUTER_API_KEY"));
        assert_eq!(config["terminal"]["env_passthrough"][1].as_str(), Some("GITHUB_TOKEN"));
        assert_eq!(config["terminal"]["env_passthrough"].as_sequence().unwrap().len(), 2);
        assert_eq!(config["terminal"]["docker_mount_cwd_to_workspace"].as_bool(), Some(true));
        assert_eq!(config["terminal"]["docker_run_as_host_user"].as_bool(), Some(true));
        assert_eq!(
            config["terminal"]["docker_image"].as_str(),
            Some("nikolaik/python-nodejs:python3.12-nodejs22")
        );
        assert_eq!(config["terminal"]["singularity_image"].as_str(), Some("docker://ubuntu:24.04"));
        assert_eq!(config["terminal"]["modal_image"].as_str(), Some("debian:bookworm"));
        assert_eq!(config["terminal"]["modal_mode"].as_str(), Some("direct"));
        assert_eq!(config["terminal"]["vercel_runtime"].as_str(), Some("node22"));
        assert_eq!(config["terminal"]["daytona_image"].as_str(), Some("ubuntu:22.04"));
        assert_eq!(config["terminal"]["ssh_host"].as_str(), Some("ssh.example.com"));
        assert_eq!(config["terminal"]["ssh_user"].as_str(), Some("hermes"));
        assert_eq!(config["terminal"]["ssh_port"].as_i64(), Some(2200));
        assert_eq!(config["terminal"]["ssh_key"].as_str(), Some("~/.ssh/id_ed25519"));
        assert_eq!(config["terminal"]["container_cpu"].as_i64(), Some(2));
        assert_eq!(config["terminal"]["container_memory"].as_i64(), Some(6144));
        assert_eq!(config["terminal"]["container_disk"].as_i64(), Some(20480));
        assert_eq!(config["terminal"]["container_persistent"].as_bool(), Some(false));
        assert_eq!(config["terminal"]["docker_forward_env"][0].as_str(), Some("GITHUB_TOKEN"));
        assert_eq!(config["terminal"]["docker_forward_env"][1].as_str(), Some("NPM_TOKEN"));
        assert_eq!(config["terminal"]["docker_forward_env"].as_sequence().unwrap().len(), 2);
        assert_eq!(
            config["terminal"]["docker_env"]["PLAYWRIGHT_BROWSERS_PATH"].as_str(),
            Some("/ms-playwright")
        );
        assert_eq!(config["terminal"]["docker_env"]["PIP_CACHE_DIR"].as_str(), Some("/workspace/.cache/pip"));
        assert_eq!(
            config["terminal"]["docker_volumes"][0].as_str(),
            Some("/data/projects:/workspace/projects")
        );
        assert_eq!(config["terminal"]["docker_volumes"][1].as_str(), Some("/data/cache:/cache"));
        assert_eq!(config["terminal"]["docker_volumes"].as_sequence().unwrap().len(), 2);
        assert_eq!(config["terminal"]["docker_extra_args"][0].as_str(), Some("--network=host"));
        assert_eq!(
            config["terminal"]["docker_extra_args"][1].as_str(),
            Some("--add-host=host.docker.internal:host-gateway")
        );
        assert_eq!(config["terminal"]["docker_extra_args"].as_sequence().unwrap().len(), 2);
        assert_eq!(config["terminal"]["custom_flag"].as_str(), Some("keep-terminal"));
    }

    #[test]
    fn merge_terminal_config_removes_empty_docker_advanced_fields() {
        let mut config: serde_yaml::Value = serde_yaml::from_str(
            r#"
terminal:
  docker_env:
    OLD_FLAG: "1"
  docker_volumes:
    - /old:/old
  docker_extra_args:
    - --old
  custom_flag: keep-terminal
"#,
        )
        .unwrap();

        merge_hermes_terminal_config(
            &mut config,
            &crate::jv!({
                "terminalDockerEnvJson": "{}",
                "terminalDockerVolumes": "  \n",
                "terminalDockerExtraArgs": "  \n",
            }),
        )
        .unwrap();

        assert!(config["terminal"]["docker_env"].is_null());
        assert!(config["terminal"]["docker_volumes"].is_null());
        assert!(config["terminal"]["docker_extra_args"].is_null());
        assert_eq!(config["terminal"]["custom_flag"].as_str(), Some("keep-terminal"));
    }

    #[test]
    fn merge_terminal_config_removes_empty_docker_forward_env() {
        let mut config: serde_yaml::Value = serde_yaml::from_str(
            r#"
terminal:
  docker_forward_env:
    - GITHUB_TOKEN
  custom_flag: keep-terminal
"#,
        )
        .unwrap();

        merge_hermes_terminal_config(
            &mut config,
            &crate::jv!({
                "terminalDockerForwardEnv": "  \n",
            }),
        )
        .unwrap();

        assert!(config["terminal"]["docker_forward_env"].is_null());
        assert_eq!(config["terminal"]["custom_flag"].as_str(), Some("keep-terminal"));
    }

    #[test]
    fn merge_terminal_config_removes_empty_shell_init_files() {
        let mut config: serde_yaml::Value = serde_yaml::from_str(
            r#"
terminal:
  shell_init_files:
    - ~/.bashrc
  custom_flag: keep-terminal
"#,
        )
        .unwrap();

        merge_hermes_terminal_config(
            &mut config,
            &crate::jv!({
                "terminalShellInitFiles": "  \n",
            }),
        )
        .unwrap();

        assert!(config["terminal"]["shell_init_files"].is_null());
        assert_eq!(config["terminal"]["custom_flag"].as_str(), Some("keep-terminal"));
    }

    #[test]
    fn merge_terminal_config_removes_empty_env_passthrough() {
        let mut config: serde_yaml::Value = serde_yaml::from_str(
            r#"
terminal:
  env_passthrough:
    - OPENROUTER_API_KEY
  custom_flag: keep-terminal
"#,
        )
        .unwrap();

        merge_hermes_terminal_config(
            &mut config,
            &crate::jv!({
                "terminalEnvPassthrough": "  \n",
            }),
        )
        .unwrap();

        assert!(config["terminal"]["env_passthrough"].is_null());
        assert_eq!(config["terminal"]["custom_flag"].as_str(), Some("keep-terminal"));
    }

    #[test]
    fn merge_terminal_config_removes_empty_images() {
        let mut config: serde_yaml::Value = serde_yaml::from_str(
            r#"
terminal:
  docker_image: old-docker
  singularity_image: old-singularity
  modal_image: old-modal
  daytona_image: old-daytona
  custom_flag: keep-terminal
"#,
        )
        .unwrap();

        merge_hermes_terminal_config(
            &mut config,
            &crate::jv!({
                "terminalDockerImage": "",
                "terminalSingularityImage": "  ",
                "terminalModalImage": "",
                "terminalDaytonaImage": " ",
            }),
        )
        .unwrap();

        assert!(config["terminal"]["docker_image"].is_null());
        assert!(config["terminal"]["singularity_image"].is_null());
        assert!(config["terminal"]["modal_image"].is_null());
        assert!(config["terminal"]["daytona_image"].is_null());
        assert_eq!(config["terminal"]["custom_flag"].as_str(), Some("keep-terminal"));
    }

    #[test]
    fn merge_terminal_config_removes_empty_ssh_fields() {
        let mut config: serde_yaml::Value = serde_yaml::from_str(
            r#"
terminal:
  ssh_host: old-host
  ssh_user: old-user
  ssh_port: 2200
  ssh_key: ~/.ssh/old
  custom_flag: keep-terminal
"#,
        )
        .unwrap();

        merge_hermes_terminal_config(
            &mut config,
            &crate::jv!({
                "terminalSshHost": "",
                "terminalSshUser": "  ",
                "terminalSshPort": "22",
                "terminalSshKey": "",
            }),
        )
        .unwrap();

        assert!(config["terminal"]["ssh_host"].is_null());
        assert!(config["terminal"]["ssh_user"].is_null());
        assert!(config["terminal"]["ssh_key"].is_null());
        assert_eq!(config["terminal"]["ssh_port"].as_i64(), Some(22));
        assert_eq!(config["terminal"]["custom_flag"].as_str(), Some("keep-terminal"));
    }

    #[test]
    fn merge_terminal_config_rejects_invalid_values() {
        let mut config = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());
        let err = merge_hermes_terminal_config(&mut config, &crate::jv!({ "terminalBackend": "unsafe" })).unwrap_err();
        assert!(err.contains("terminal.backend"));
        let err = merge_hermes_terminal_config(&mut config, &crate::jv!({ "terminalModalMode": "unsafe" })).unwrap_err();
        assert!(err.contains("terminal.modal_mode"));
        let err = merge_hermes_terminal_config(&mut config, &crate::jv!({ "terminalVercelRuntime": "ruby" })).unwrap_err();
        assert!(err.contains("terminal.vercel_runtime"));
        let err = merge_hermes_terminal_config(&mut config, &crate::jv!({ "terminalTimeout": 0 })).unwrap_err();
        assert!(err.contains("terminal.timeout"));
        let err = merge_hermes_terminal_config(&mut config, &crate::jv!({ "terminalLifetimeSeconds": -1 })).unwrap_err();
        assert!(err.contains("terminal.lifetime_seconds"));
        let err = merge_hermes_terminal_config(&mut config, &crate::jv!({ "terminalContainerCpu": 0 })).unwrap_err();
        assert!(err.contains("terminal.container_cpu"));
        let err = merge_hermes_terminal_config(&mut config, &crate::jv!({ "terminalContainerMemory": 127 })).unwrap_err();
        assert!(err.contains("terminal.container_memory"));
        let err = merge_hermes_terminal_config(&mut config, &crate::jv!({ "terminalSshPort": 0 })).unwrap_err();
        assert!(err.contains("terminal.ssh_port"));
        let err = merge_hermes_terminal_config(&mut config, &crate::jv!({ "terminalSshPort": 65536 })).unwrap_err();
        assert!(err.contains("terminal.ssh_port"));
        let err = merge_hermes_terminal_config(&mut config, &crate::jv!({ "terminalDockerForwardEnv": "GOOD_TOKEN\nBAD TOKEN" }))
            .unwrap_err();
        assert!(err.contains("terminal.docker_forward_env"));
        let err = merge_hermes_terminal_config(&mut config, &crate::jv!({ "terminalShellInitFiles": "valid.sh\nbad path.sh" }))
            .unwrap_err();
        assert!(err.contains("terminal.shell_init_files"));
        let err = merge_hermes_terminal_config(&mut config, &crate::jv!({ "terminalEnvPassthrough": "GOOD_TOKEN\nBAD TOKEN" }))
            .unwrap_err();
        assert!(err.contains("terminal.env_passthrough"));
        let err = merge_hermes_terminal_config(&mut config, &crate::jv!({ "terminalDockerEnvJson": "[]" })).unwrap_err();
        assert!(err.contains("terminal.docker_env"));
        let err =
            merge_hermes_terminal_config(&mut config, &crate::jv!({ "terminalDockerEnvJson": "{ \"BAD KEY\": \"value\" }" }))
                .unwrap_err();
        assert!(err.contains("terminal.docker_env"));
        let err = merge_hermes_terminal_config(&mut config, &crate::jv!({ "terminalDockerVolumes": "/host only" })).unwrap_err();
        assert!(err.contains("terminal.docker_volumes"));
        let err = merge_hermes_terminal_config(&mut config, &crate::jv!({ "terminalDockerExtraArgs": "bad arg" })).unwrap_err();
        assert!(err.contains("terminal.docker_extra_args"));
    }
}