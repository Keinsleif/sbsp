// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! What the web build says to the host, and how it reads the answer.
//!
//! The native remote reuses the backend's handles, which send the host's
//! protocol messages as a side effect. The browser cannot link the backend
//! (audio and file code), so this does the same translation directly:
//! the UI's `Control` and `ModelEdit` become the protocol's `WsCommand`s,
//! in the order they must arrive. No I/O here; the socket is elsewhere.

use std::path::PathBuf;

use sbsp_backend::{
    action::{AudioAction, CueAction},
    api::{
        AuthInfo, WsCommand, WsFeedback,
        auth::{generate_authentication_string, generate_secret},
    },
    controller::ControllerCommand,
    manager::{InsertPosition, ModelCommand},
    model::cue::audio::Decibels,
    asset_processor::AssetProcessorCommand,
};

use super::{Control, ModelEdit};

fn model(command: ModelCommand) -> WsCommand {
    WsCommand::Model(Box::new(command))
}

/// The commands for playback controls, in order.
pub fn controls(controls: Vec<Control>) -> Vec<WsCommand> {
    controls
        .into_iter()
        .map(|control| {
            WsCommand::Control(match control {
                Control::Execute(id) => ControllerCommand::Execute(id),
                Control::Load(id) => ControllerCommand::Load(id),
                Control::Pause(id) => ControllerCommand::Pause(id),
                Control::Resume(id) => ControllerCommand::Resume(id),
                Control::Stop(id) => ControllerCommand::Stop(id),
                Control::SoftStop(id) => ControllerCommand::SoftStop(id),
                Control::HardStop(id) => ControllerCommand::HardStop(id),
                Control::SeekBy(id, amount) => ControllerCommand::SeekBy(id, amount),
                Control::SeekTo(id, position) => ControllerCommand::SeekTo(id, position),
                Control::ToggleRepeat(id) => {
                    ControllerCommand::PerformAction(id, CueAction::Audio(AudioAction::ToggleRepeat))
                }
                Control::SetVolume(id, volume) => ControllerCommand::PerformAction(
                    id,
                    CueAction::Audio(AudioAction::SetVolume(Decibels::from(volume as f32))),
                ),
                Control::PauseAll => ControllerCommand::PauseAll,
                Control::ResumeAll => ControllerCommand::ResumeAll,
                Control::StopAll => ControllerCommand::StopAll,
                Control::SoftStopAll => ControllerCommand::SoftStopAll,
                Control::HardStopAll => ControllerCommand::HardStopAll,
            })
        })
        .collect()
}

/// The commands for a model edit, in order (a new group is added first and
/// then takes its members, so it is two).
pub fn edit(edit: ModelEdit) -> Vec<WsCommand> {
    match edit {
        ModelEdit::AddCues { mut cues, position } => match cues.len() {
            0 => Vec::new(),
            1 => vec![model(ModelCommand::AddCue {
                cue: cues.remove(0),
                position,
            })],
            _ => vec![model(ModelCommand::AddCues { cues, position })],
        },
        ModelEdit::AddGroup {
            group,
            position,
            members,
        } => {
            let group_id = group.id;
            let mut commands = vec![model(ModelCommand::AddCue {
                cue: group,
                position,
            })];
            if !members.is_empty() {
                commands.push(model(ModelCommand::MoveCues {
                    cue_ids: members.into_iter().collect(),
                    position: InsertPosition::Inside {
                        target: Some(group_id),
                        index: Some(0),
                    },
                }));
            }
            commands
        }
        ModelEdit::RemoveCues(ids) => vec![model(ModelCommand::RemoveCues {
            cue_ids: ids.into_iter().collect(),
        })],
        ModelEdit::MoveCues { ids, position } => vec![model(ModelCommand::MoveCues {
            cue_ids: ids.into_iter().collect(),
            position,
        })],
        ModelEdit::RenumberCues {
            ids,
            start_from,
            increment,
            prefix,
            suffix,
        } => vec![model(ModelCommand::RenumberCues {
            cues: ids,
            start_from,
            increment,
            prefix,
            suffix,
        })],
        ModelEdit::UpdateCue(cue) => vec![model(ModelCommand::UpdateCue(cue))],
        ModelEdit::UpdateShowSettings(settings) => {
            vec![model(ModelCommand::UpdateSettings(Box::new(settings)))]
        }
        ModelEdit::RenameShow(name) => vec![model(ModelCommand::UpdateModelName(name))],
    }
}

/// Asks the host to analyze an audio file.
pub fn request_asset(path: PathBuf) -> WsCommand {
    WsCommand::AssetProcessor(AssetProcessorCommand::RequestFileAssetData { path })
}

/// The answer to the host's challenge (none when no password was given).
pub fn authenticate(password: Option<&str>, auth: &AuthInfo) -> WsCommand {
    let response = password.map(|password| {
        let secret = generate_secret(&password.to_string(), &auth.salt);
        generate_authentication_string(&secret, &auth.challenge)
    });
    WsCommand::Authenticate { response }
}

pub fn encode(command: &WsCommand) -> Option<String> {
    serde_json::to_string(command).ok()
}

pub fn decode(text: &str) -> Result<WsFeedback, serde_json::Error> {
    serde_json::from_str(text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sbsp_backend::model::cue::Uuid;
    use serde_json::{Value, json};

    fn a_cue() -> sbsp_backend::model::cue::Cue {
        sbsp_frontend_settings::TemplateSettings::default().wait
    }

    fn value(command: &WsCommand) -> Value {
        serde_json::from_str(&encode(command).unwrap()).unwrap()
    }

    #[test]
    fn controls_go_out_in_order_in_the_hosts_wire_form() {
        let id = Uuid::nil();
        let commands = controls(vec![Control::Load(id), Control::Execute(id), Control::StopAll]);
        assert_eq!(commands.len(), 3);

        let first = value(&commands[0]);
        assert_eq!(first["type"], "control");
        assert_eq!(first["command"], "load");
        assert_eq!(first["params"], json!(id));
        assert_eq!(value(&commands[1])["command"], "execute");
        assert_eq!(value(&commands[2])["command"], "stopAll");
    }

    #[test]
    fn volume_and_repeat_are_cue_actions() {
        let id = Uuid::nil();
        let commands = controls(vec![Control::SetVolume(id, -6.0), Control::ToggleRepeat(id)]);
        assert_eq!(value(&commands[0])["command"], "performAction");
        assert_eq!(value(&commands[1])["command"], "performAction");
    }

    #[test]
    fn a_new_group_is_added_and_then_filled() {
        let group = a_cue();
        let group_id = group.id;
        let member = Uuid::new_v4();
        let commands = edit(ModelEdit::AddGroup {
            group,
            position: InsertPosition::LAST,
            members: vec![member],
        });
        assert_eq!(commands.len(), 2);
        assert_eq!(value(&commands[0])["command"], "addCue");
        let fill = value(&commands[1]);
        assert_eq!(fill["command"], "moveCues");
        assert_eq!(fill["params"]["position"]["target"], json!(group_id));
    }

    #[test]
    fn adding_nothing_sends_nothing_and_one_cue_is_not_a_batch() {
        let none = edit(ModelEdit::AddCues {
            cues: Vec::new(),
            position: InsertPosition::LAST,
        });
        assert!(none.is_empty());

        let one = edit(ModelEdit::AddCues {
            cues: vec![a_cue()],
            position: InsertPosition::LAST,
        });
        assert_eq!(value(&one[0])["command"], "addCue");
    }

    #[test]
    fn the_password_answers_the_challenge_the_way_the_host_checks_it() {
        let auth = AuthInfo {
            challenge: "challenge".into(),
            salt: "salt".into(),
        };
        let WsCommand::Authenticate { response } = authenticate(Some("pass"), &auth) else {
            panic!("not an authenticate command");
        };
        let secret = generate_secret(&"pass".to_string(), &auth.salt);
        assert_eq!(
            response,
            Some(generate_authentication_string(&secret, &auth.challenge))
        );

        let WsCommand::Authenticate { response } = authenticate(None, &auth) else {
            panic!("not an authenticate command");
        };
        assert_eq!(response, None);
    }

    #[test]
    fn what_the_host_sends_reads_back() {
        let text = r#"{"type":"level","data":{"left":0.5,"right":0.25}}"#;
        assert!(matches!(
            decode(text),
            Ok(WsFeedback::Level { left, right }) if left == 0.5 && right == 0.25
        ));
        assert!(decode("not json").is_err());
    }
}
