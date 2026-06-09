#[cfg(test)]
mod hermes_tts_voice_config_tests {
    use super::{build_hermes_tts_voice_config_values, merge_hermes_tts_voice_config};

    #[test]
    fn tts_voice_values_have_upstream_defaults() {
        let config: serde_yaml::Value = serde_yaml::from_str("{}").unwrap();
        let values = build_hermes_tts_voice_config_values(&config);
        assert_eq!(values["ttsProvider"], "edge");
        assert_eq!(values["ttsEdgeVoice"], "en-US-AriaNeural");
        assert_eq!(values["ttsOpenaiModel"], "gpt-4o-mini-tts");
        assert_eq!(values["ttsOpenaiVoice"], "alloy");
        assert_eq!(values["ttsElevenlabsVoiceId"], "pNInz6obpgDQGcFmaJgB");
        assert_eq!(values["ttsElevenlabsModelId"], "eleven_multilingual_v2");
        assert_eq!(values["ttsXaiVoiceId"], "eve");
        assert_eq!(values["ttsXaiLanguage"], "en");
        assert_eq!(values["ttsXaiSampleRate"], 24000);
        assert_eq!(values["ttsXaiBitRate"], 128000);
        assert_eq!(values["ttsMistralModel"], "voxtral-mini-tts-2603");
        assert_eq!(values["ttsMistralVoiceId"], "c69964a6-ab8b-4f8a-9465-ec0925096ec8");
        assert_eq!(values["ttsPiperVoice"], "en_US-lessac-medium");
        assert_eq!(values["voiceRecordKey"], "ctrl+b");
        assert_eq!(values["voiceMaxRecordingSeconds"], 120);
        assert_eq!(values["voiceAutoTts"], false);
        assert_eq!(values["voiceBeepEnabled"], true);
        assert_eq!(values["voiceSilenceThreshold"], 200);
        assert_eq!(values["voiceSilenceDuration"], 3.0);
    }

    #[test]
    fn tts_voice_values_read_yaml_fields() {
        let config: serde_yaml::Value = serde_yaml::from_str(
            r#"
tts:
  provider: openai
  edge:
    voice: zh-CN-XiaoxiaoNeural
  openai:
    model: gpt-4o-mini-tts
    voice: nova
  elevenlabs:
    voice_id: voice-123
    model_id: eleven_turbo_v2_5
  xai:
    voice_id: custom-eve
    language: zh
    sample_rate: 48000
    bit_rate: 192000
  mistral:
    model: voxtral-mini-tts-2603
    voice_id: mistral-voice
  piper:
    voice: zh_CN-huayan-medium
voice:
  record_key: ctrl+shift+v
  max_recording_seconds: 240
  auto_tts: true
  beep_enabled: false
  silence_threshold: 350
  silence_duration: 1.5
"#,
        )
        .unwrap();
        let values = build_hermes_tts_voice_config_values(&config);
        assert_eq!(values["ttsProvider"], "openai");
        assert_eq!(values["ttsEdgeVoice"], "zh-CN-XiaoxiaoNeural");
        assert_eq!(values["ttsOpenaiVoice"], "nova");
        assert_eq!(values["ttsElevenlabsVoiceId"], "voice-123");
        assert_eq!(values["ttsXaiLanguage"], "zh");
        assert_eq!(values["ttsXaiSampleRate"], 48000);
        assert_eq!(values["ttsMistralVoiceId"], "mistral-voice");
        assert_eq!(values["ttsPiperVoice"], "zh_CN-huayan-medium");
        assert_eq!(values["voiceRecordKey"], "ctrl+shift+v");
        assert_eq!(values["voiceAutoTts"], true);
        assert_eq!(values["voiceBeepEnabled"], false);
        assert_eq!(values["voiceSilenceDuration"], 1.5);
    }

    #[test]
    fn merge_tts_voice_config_preserves_unknown_fields() {
        let mut config: serde_yaml::Value = serde_yaml::from_str(
            r#"
model:
  provider: anthropic
tts:
  provider: edge
  custom_flag: keep-tts
  openai:
    custom_flag: keep-openai
  piper:
    voices_dir: /cache/piper
voice:
  custom_flag: keep-voice
streaming:
  enabled: true
"#,
        )
        .unwrap();

        merge_hermes_tts_voice_config(
            &mut config,
            &crate::jv!({
                "ttsProvider": "openai",
                "ttsEdgeVoice": "zh-CN-XiaoxiaoNeural",
                "ttsOpenaiModel": "gpt-4o-mini-tts",
                "ttsOpenaiVoice": "nova",
                "ttsElevenlabsVoiceId": "voice-123",
                "ttsElevenlabsModelId": "eleven_turbo_v2_5",
                "ttsXaiVoiceId": "eve-pro",
                "ttsXaiLanguage": "zh",
                "ttsXaiSampleRate": "48000",
                "ttsXaiBitRate": "192000",
                "ttsMistralModel": "voxtral-mini-tts-2603",
                "ttsMistralVoiceId": "mistral-voice",
                "ttsPiperVoice": "zh_CN-huayan-medium",
                "voiceRecordKey": "ctrl+shift+v",
                "voiceMaxRecordingSeconds": "240",
                "voiceAutoTts": true,
                "voiceBeepEnabled": false,
                "voiceSilenceThreshold": "350",
                "voiceSilenceDuration": "1.5",
            }),
        )
        .unwrap();

        assert_eq!(config["model"]["provider"].as_str(), Some("anthropic"));
        assert_eq!(config["streaming"]["enabled"].as_bool(), Some(true));
        assert_eq!(config["tts"]["provider"].as_str(), Some("openai"));
        assert_eq!(config["tts"]["edge"]["voice"].as_str(), Some("zh-CN-XiaoxiaoNeural"));
        assert_eq!(config["tts"]["openai"]["voice"].as_str(), Some("nova"));
        assert_eq!(config["tts"]["openai"]["custom_flag"].as_str(), Some("keep-openai"));
        assert_eq!(config["tts"]["elevenlabs"]["voice_id"].as_str(), Some("voice-123"));
        assert_eq!(config["tts"]["xai"]["sample_rate"].as_i64(), Some(48000));
        assert_eq!(config["tts"]["xai"]["bit_rate"].as_i64(), Some(192000));
        assert_eq!(config["tts"]["mistral"]["voice_id"].as_str(), Some("mistral-voice"));
        assert_eq!(config["tts"]["piper"]["voice"].as_str(), Some("zh_CN-huayan-medium"));
        assert_eq!(config["tts"]["piper"]["voices_dir"].as_str(), Some("/cache/piper"));
        assert_eq!(config["tts"]["custom_flag"].as_str(), Some("keep-tts"));
        assert_eq!(config["voice"]["record_key"].as_str(), Some("ctrl+shift+v"));
        assert_eq!(config["voice"]["max_recording_seconds"].as_i64(), Some(240));
        assert_eq!(config["voice"]["auto_tts"].as_bool(), Some(true));
        assert_eq!(config["voice"]["beep_enabled"].as_bool(), Some(false));
        assert_eq!(config["voice"]["silence_threshold"].as_i64(), Some(350));
        assert_eq!(config["voice"]["silence_duration"].as_f64(), Some(1.5));
        assert_eq!(config["voice"]["custom_flag"].as_str(), Some("keep-voice"));
    }

    #[test]
    fn merge_tts_voice_config_removes_empty_optional_overrides() {
        let mut config: serde_yaml::Value = serde_yaml::from_str(
            r#"
tts:
  edge:
    voice: custom-edge
  elevenlabs:
    voice_id: voice-123
    model_id: model-123
  piper:
    voice: custom-piper
    voices_dir: /cache/piper
voice:
  record_key: ctrl+shift+v
  custom_flag: keep-voice
"#,
        )
        .unwrap();

        merge_hermes_tts_voice_config(
            &mut config,
            &crate::jv!({
                "ttsEdgeVoice": "",
                "ttsElevenlabsVoiceId": " ",
                "ttsElevenlabsModelId": "",
                "ttsPiperVoice": "",
                "voiceRecordKey": "",
            }),
        )
        .unwrap();

        assert!(config["tts"]["edge"]["voice"].is_null());
        assert!(config["tts"]["elevenlabs"]["voice_id"].is_null());
        assert!(config["tts"]["elevenlabs"]["model_id"].is_null());
        assert!(config["tts"]["piper"]["voice"].is_null());
        assert_eq!(config["tts"]["piper"]["voices_dir"].as_str(), Some("/cache/piper"));
        assert!(config["voice"]["record_key"].is_null());
        assert_eq!(config["voice"]["custom_flag"].as_str(), Some("keep-voice"));
    }

    #[test]
    fn merge_tts_voice_config_rejects_invalid_values() {
        let mut config = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());
        let err = merge_hermes_tts_voice_config(&mut config, &crate::jv!({ "ttsProvider": "bad" })).unwrap_err();
        assert!(err.contains("tts.provider"));
        let err = merge_hermes_tts_voice_config(&mut config, &crate::jv!({ "ttsOpenaiVoice": "robot" })).unwrap_err();
        assert!(err.contains("tts.openai.voice"));
        let err = merge_hermes_tts_voice_config(&mut config, &crate::jv!({ "ttsXaiSampleRate": "0" })).unwrap_err();
        assert!(err.contains("tts.xai.sample_rate"));
        let err = merge_hermes_tts_voice_config(&mut config, &crate::jv!({ "voiceMaxRecordingSeconds": "0" })).unwrap_err();
        assert!(err.contains("voice.max_recording_seconds"));
        let err = merge_hermes_tts_voice_config(&mut config, &crate::jv!({ "voiceSilenceDuration": "-1" })).unwrap_err();
        assert!(err.contains("voice.silence_duration"));
    }
}