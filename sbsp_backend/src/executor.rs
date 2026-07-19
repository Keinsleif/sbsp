// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

mod command;
mod event;

#[cfg(test)]
mod tests;

pub use command::ExecutorCommand;
pub use command::StopMode;
pub use event::ExecutorEvent;

use std::collections::{HashMap, VecDeque};

use async_recursion::async_recursion;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::model::cue::CueChain;
use crate::{
    action::CueAction,
    controller::state::StateParam,
    engine::{
        EngineEvent, EngineType,
        audio_engine::{AudioCommand, AudioCommandData, AudioEngineEvent},
        wait_engine::{WaitCommand, WaitEvent, WaitType},
    },
    manager::ShowModelHandle,
    model::cue::{Cue, CueParam, audio::AudioCueParam, group::GroupMode},
};

#[derive(Debug)]
pub struct ActiveInstance {
    engine_type: EngineType,
    is_triggered: bool,  // specify loaded or triggered
    is_prewaiting: bool, // specify prewaiting or playing
    is_paused: bool,     // specify paused or playing
}

pub struct Executor {
    model_handle: ShowModelHandle,
    command_rx: mpsc::Receiver<ExecutorCommand>,
    audio_tx: mpsc::Sender<AudioCommand>,
    wait_tx: mpsc::Sender<WaitCommand>,
    executor_event_tx: mpsc::Sender<ExecutorEvent>,
    engine_event_rx: mpsc::Receiver<EngineEvent>,

    active_instances: HashMap<Uuid, ActiveInstance>,
}

impl Executor {
    pub fn new(
        model_handle: ShowModelHandle,
        command_rx: mpsc::Receiver<ExecutorCommand>,
        audio_tx: mpsc::Sender<AudioCommand>,
        wait_tx: mpsc::Sender<WaitCommand>,
        playback_event_tx: mpsc::Sender<ExecutorEvent>,
        engine_event_rx: mpsc::Receiver<EngineEvent>,
    ) -> Self {
        Self {
            model_handle,
            command_rx,
            audio_tx,
            wait_tx,
            executor_event_tx: playback_event_tx,
            engine_event_rx,
            active_instances: HashMap::new(),
        }
    }

    pub async fn run(mut self) {
        log::info!("Executor run loop started.");
        loop {
            tokio::select! {
                Some(command) = self.command_rx.recv() => {
                    log::debug!("Executor received command: {:?}", command);
                    if let Err(e) = self.process_command(command).await {
                        log::error!("Error processing executor command: {:?}", e);
                    }
                },
                Some(event) = self.engine_event_rx.recv() => {
                    if let Err(e) = self.handle_engine_event(event).await {
                        log::error!("Error handling engine event: {:?}", e);
                    }
                }
                else => break,
            }
        }
        log::info!("Executor run loop finished.");
    }

    #[async_recursion]
    async fn process_command(&mut self, command: ExecutorCommand) -> Result<(), anyhow::Error> {
        match command {
            ExecutorCommand::Load(cue_id) => {
                if let Some(cue) = self.model_handle.get_cue_by_id(&cue_id).await {
                    self.load_cue(&cue).await?;
                }
            }
            ExecutorCommand::Execute(cue_id) => {
                if let Some(active_instance) = self.active_instances.get(&cue_id)
                    && active_instance.is_triggered
                {
                    log::warn!("Cue already executed. cue_id={}", cue_id);
                } else if let Some(cue) = self.model_handle.get_cue_by_id(&cue_id).await {
                    self.check_and_start_parents(cue_id).await?;
                    self.executor_event_tx
                        .send(ExecutorEvent::Triggered { cue_id })
                        .await?;
                    if cue.pre_wait > 0.0 {
                        if !self.active_instances.contains_key(&cue_id) {
                            self.load_cue(&cue).await?;
                        }
                        if let Some(instance) = self.active_instances.get_mut(&cue_id) {
                            instance.is_prewaiting = true;
                            instance.is_triggered = true;
                        }
                        self.wait_tx
                            .send(WaitCommand::Start {
                                wait_type: WaitType::PreWait,
                                instance_id: cue_id,
                                duration: cue.pre_wait,
                            })
                            .await?;
                    } else {
                        self.execute_cue(&cue).await?;
                    }
                    self.resolve_after_start_chain(cue_id).await?;
                } else {
                    anyhow::bail!("EXECUTE: cue not found. cue_id={}", cue_id);
                }
            }
            ExecutorCommand::Pause(cue_id) => self.pause_cue(cue_id).await?,
            ExecutorCommand::Resume(cue_id) => self.resume_cue(cue_id).await?,
            ExecutorCommand::Stop(cue_id, stop_mode) => self.stop_cue(cue_id, stop_mode).await?,
            ExecutorCommand::SeekTo(cue_id, position) => self.seek_to_cue(cue_id, position).await?,
            ExecutorCommand::SeekBy(cue_id, amount) => self.seek_by_cue(cue_id, amount).await?,
            ExecutorCommand::PerformAction(cue_id, action) => {
                if let Some(active_instance) = self.active_instances.get(&cue_id) {
                    match (action, active_instance.engine_type) {
                        (CueAction::Audio(audio_action), EngineType::Audio) => {
                            self.audio_tx
                                .send(AudioCommand::PerformAction {
                                    id: cue_id,
                                    action: audio_action,
                                })
                                .await?;
                        }
                        _ => {
                            log::warn!("Action type isn't match active cue's type. ignoring...");
                        }
                    }
                }
            }
            ExecutorCommand::ReconfigureEngines(settings) => {
                self.audio_tx
                    .send(AudioCommand::Reconfigure(settings.audio))
                    .await?;
            }
        }
        Ok(())
    }

    async fn load_cue(&mut self, cue: &Cue) -> Result<(), anyhow::Error> {
        if self.active_instances.contains_key(&cue.id) {
            anyhow::bail!("Cue already loaded or executed. cue_id={}", cue.id);
        }
        match &cue.params {
            CueParam::Audio(AudioCueParam {
                target,
                start_time,
                fade_in_param,
                end_time,
                fade_out_param,
                volume,
                pan,
                repeat,
                sound_type,
                envelope,
            }) => {
                let filepath = self.model_handle.get_asset_standard_path(target).await?;

                self.audio_tx
                    .send(AudioCommand::Load {
                        id: cue.id,
                        data: AudioCommandData {
                            filepath,
                            volume: *volume,
                            pan: *pan,
                            start_time: *start_time,
                            fade_in_param: *fade_in_param,
                            end_time: *end_time,
                            fade_out_param: *fade_out_param,
                            repeat: *repeat,
                            sound_type: *sound_type,
                            envelope: envelope.clone(),
                        },
                    })
                    .await?;
                self.active_instances.insert(
                    cue.id,
                    ActiveInstance {
                        engine_type: EngineType::Audio,
                        is_prewaiting: false,
                        is_triggered: false,
                        is_paused: false,
                    },
                );
            }
            CueParam::Wait(params) => {
                self.wait_tx
                    .send(WaitCommand::Load {
                        wait_type: WaitType::Wait,
                        instance_id: cue.id,
                        duration: params.duration,
                    })
                    .await?;
                self.active_instances.insert(
                    cue.id,
                    ActiveInstance {
                        engine_type: EngineType::Wait,
                        is_prewaiting: false,
                        is_triggered: false,
                        is_paused: false,
                    },
                );
            }
            CueParam::Fade(params) => {
                self.wait_tx
                    .send(WaitCommand::Load {
                        wait_type: WaitType::FadeWait,
                        instance_id: cue.id,
                        duration: params.fade_param.duration,
                    })
                    .await?;
                self.active_instances.insert(
                    cue.id,
                    ActiveInstance {
                        engine_type: EngineType::Fade,
                        is_prewaiting: false,
                        is_triggered: false,
                        is_paused: false,
                    },
                );
            }
            CueParam::Start(_) | CueParam::Stop(_) | CueParam::Pause(_) | CueParam::Load(_) => {
                self.active_instances.insert(
                    cue.id,
                    ActiveInstance {
                        engine_type: EngineType::Playback,
                        is_prewaiting: false,
                        is_triggered: false,
                        is_paused: false,
                    },
                );
            }
            CueParam::Group { base, children } => match base.mode {
                GroupMode::Playlist { .. } | GroupMode::StartFirst { .. } => {
                    if let Some(first_id) = children.first() {
                        self.active_instances.insert(
                            cue.id,
                            ActiveInstance {
                                engine_type: EngineType::Group,
                                is_prewaiting: false,
                                is_triggered: false,
                                is_paused: false,
                            },
                        );
                        self.process_command(ExecutorCommand::Load(*first_id))
                            .await?;
                        self.executor_event_tx
                            .send(ExecutorEvent::Loaded {
                                cue_id: cue.id,
                                position: 0.0,
                                duration: 0.0,
                            })
                            .await?;
                    }
                }
                GroupMode::Concurrency => {
                    if !children.is_empty() {
                        self.active_instances.insert(
                            cue.id,
                            ActiveInstance {
                                engine_type: EngineType::Group,
                                is_prewaiting: false,
                                is_triggered: false,
                                is_paused: false,
                            },
                        );
                        for cue_id in children.iter() {
                            if let Err(e) =
                                self.process_command(ExecutorCommand::Load(*cue_id)).await
                            {
                                log::error!("Failed to load group child. e={}", e);
                            }
                        }
                        self.executor_event_tx
                            .send(ExecutorEvent::Loaded {
                                cue_id: cue.id,
                                position: 0.0,
                                duration: 0.0,
                            })
                            .await?;
                    }
                }
            },
        }
        Ok(())
    }

    async fn execute_cue(&mut self, cue: &Cue) -> Result<(), anyhow::Error> {
        match &cue.params {
            CueParam::Audio(AudioCueParam {
                target,
                start_time,
                fade_in_param,
                end_time,
                fade_out_param,
                volume,
                pan,
                repeat,
                sound_type,
                envelope,
            }) => {
                let filepath = self.model_handle.get_asset_standard_path(target).await?;

                let audio_command = AudioCommand::Play {
                    id: cue.id,
                    data: AudioCommandData {
                        filepath,
                        volume: *volume,
                        pan: *pan,
                        start_time: *start_time,
                        fade_in_param: *fade_in_param,
                        end_time: *end_time,
                        fade_out_param: *fade_out_param,
                        repeat: *repeat,
                        sound_type: *sound_type,
                        envelope: envelope.clone(),
                    },
                };
                self.audio_tx.send(audio_command).await?;
                self.active_instances.insert(
                    cue.id,
                    ActiveInstance {
                        engine_type: EngineType::Audio,
                        is_prewaiting: false,
                        is_triggered: true,
                        is_paused: false,
                    },
                );
            }
            CueParam::Wait(params) => {
                self.wait_tx
                    .send(WaitCommand::Start {
                        wait_type: WaitType::Wait,
                        instance_id: cue.id,
                        duration: params.duration,
                    })
                    .await?;
                self.active_instances.insert(
                    cue.id,
                    ActiveInstance {
                        engine_type: EngineType::Wait,
                        is_prewaiting: false,
                        is_triggered: true,
                        is_paused: false,
                    },
                );
            }
            CueParam::Fade(params) => {
                if let Some(cue) = self.model_handle.get_cue_by_id(&params.target).await
                    && self.active_instances.contains_key(&params.target)
                {
                    match cue.params {
                        CueParam::Audio(_) => {
                            if self
                                .audio_tx
                                .send(AudioCommand::FadeVolume {
                                    id: params.target,
                                    volume: params.volume,
                                    fade_param: params.fade_param,
                                })
                                .await
                                .is_err()
                            {
                                anyhow::bail!("cannot send AudioCommand");
                            }
                        }
                        CueParam::Group { .. } => {
                            let children = self
                                .model_handle
                                .get_all_children_by_id(&params.target)
                                .await;
                            for child in children {
                                if self.active_instances.contains_key(&child.id)
                                    && let CueParam::Audio(_) = child.params
                                    && let Err(e) = self
                                        .audio_tx
                                        .send(AudioCommand::FadeVolume {
                                            id: child.id,
                                            volume: params.volume,
                                            fade_param: params.fade_param,
                                        })
                                        .await
                                {
                                    log::error!("Failed to fade group child. e={}", e);
                                }
                            }
                        }
                        _ => {}
                    }
                }

                self.wait_tx
                    .send(WaitCommand::Start {
                        wait_type: WaitType::FadeWait,
                        instance_id: cue.id,
                        duration: params.fade_param.duration,
                    })
                    .await?;
                self.active_instances.insert(
                    cue.id,
                    ActiveInstance {
                        engine_type: EngineType::Fade,
                        is_prewaiting: false,
                        is_triggered: true,
                        is_paused: false,
                    },
                );
            }
            CueParam::Start(params) => {
                self.active_instances.insert(
                    cue.id,
                    ActiveInstance {
                        engine_type: EngineType::Playback,
                        is_prewaiting: false,
                        is_triggered: true,
                        is_paused: false,
                    },
                );
                if let Some(instance) = self.active_instances.get(&params.target)
                    && instance.is_triggered
                {
                    if instance.is_paused {
                        self.process_command(ExecutorCommand::Resume(params.target))
                            .await?;
                    }
                } else {
                    self.process_command(ExecutorCommand::Execute(params.target))
                        .await?;
                }

                self.executor_event_tx
                    .send(ExecutorEvent::Started {
                        cue_id: cue.id,
                        position: 0.0,
                        duration: 0.0,
                        initial_params: StateParam::None,
                    })
                    .await?;
                self.active_instances.remove(&cue.id);
                self.emit_completed(cue.id).await?;
            }
            CueParam::Stop(params) => {
                self.active_instances.insert(
                    cue.id,
                    ActiveInstance {
                        engine_type: EngineType::Playback,
                        is_prewaiting: false,
                        is_triggered: true,
                        is_paused: false,
                    },
                );
                if self.active_instances.contains_key(&params.target) {
                    let stop_mode = if params.hard {
                        StopMode::Hard
                    } else {
                        StopMode::Soft
                    };
                    self.process_command(ExecutorCommand::Stop(params.target, stop_mode))
                        .await?;
                }

                self.executor_event_tx
                    .send(ExecutorEvent::Started {
                        cue_id: cue.id,
                        position: 0.0,
                        duration: 0.0,
                        initial_params: StateParam::None,
                    })
                    .await?;
                self.active_instances.remove(&cue.id);
                self.emit_completed(cue.id).await?;
            }
            CueParam::Pause(params) => {
                self.active_instances.insert(
                    cue.id,
                    ActiveInstance {
                        engine_type: EngineType::Playback,
                        is_prewaiting: false,
                        is_triggered: true,
                        is_paused: false,
                    },
                );
                if let Some(instance) = self.active_instances.get(&params.target)
                    && instance.is_triggered
                    && !instance.is_paused
                {
                    self.process_command(ExecutorCommand::Pause(params.target))
                        .await?;
                }

                self.executor_event_tx
                    .send(ExecutorEvent::Started {
                        cue_id: cue.id,
                        position: 0.0,
                        duration: 0.0,
                        initial_params: StateParam::None,
                    })
                    .await?;
                self.active_instances.remove(&cue.id);
                self.emit_completed(cue.id).await?;
            }
            CueParam::Load(params) => {
                self.active_instances.insert(
                    cue.id,
                    ActiveInstance {
                        engine_type: EngineType::Playback,
                        is_prewaiting: false,
                        is_triggered: true,
                        is_paused: false,
                    },
                );
                if !self.active_instances.contains_key(&params.target) {
                    self.process_command(ExecutorCommand::Load(params.target))
                        .await?;
                }

                self.executor_event_tx
                    .send(ExecutorEvent::Started {
                        cue_id: cue.id,
                        position: 0.0,
                        duration: 0.0,
                        initial_params: StateParam::None,
                    })
                    .await?;
                self.active_instances.remove(&cue.id);
                self.emit_completed(cue.id).await?;
            }
            CueParam::Group { base, children } => match base.mode {
                GroupMode::Playlist { .. } | GroupMode::StartFirst { .. } => {
                    if let Some(first_id) = children.first() {
                        self.active_instances.insert(
                            cue.id,
                            ActiveInstance {
                                engine_type: EngineType::Group,
                                is_prewaiting: false,
                                is_triggered: true,
                                is_paused: false,
                            },
                        );
                        self.executor_event_tx
                            .send(ExecutorEvent::Started {
                                cue_id: cue.id,
                                position: 0.0,
                                duration: 0.0,
                                initial_params: StateParam::None,
                            })
                            .await?;
                        self.process_command(ExecutorCommand::Execute(*first_id))
                            .await?;
                    }
                }
                GroupMode::Concurrency => {
                    if !children.is_empty() {
                        self.active_instances.insert(
                            cue.id,
                            ActiveInstance {
                                engine_type: EngineType::Group,
                                is_prewaiting: false,
                                is_triggered: true,
                                is_paused: false,
                            },
                        );
                        self.executor_event_tx
                            .send(ExecutorEvent::Started {
                                cue_id: cue.id,
                                position: 0.0,
                                duration: 0.0,
                                initial_params: StateParam::None,
                            })
                            .await?;
                        for cue_id in children.iter() {
                            if let Err(e) = self
                                .process_command(ExecutorCommand::Execute(*cue_id))
                                .await
                            {
                                log::error!("Failed to execute group child. e={}", e);
                            }
                        }
                    }
                }
            },
        }
        Ok(())
    }

    async fn pause_cue(&mut self, cue_id: Uuid) -> Result<(), anyhow::Error> {
        if let Some(active_instance) = self.active_instances.get(&cue_id) {
            if active_instance.is_prewaiting {
                self.wait_tx
                    .send(WaitCommand::Pause {
                        wait_type: WaitType::PreWait,
                        instance_id: cue_id,
                    })
                    .await?;
                return Ok(());
            }
            match active_instance.engine_type {
                EngineType::Audio => {
                    self.audio_tx
                        .send(AudioCommand::Pause { id: cue_id })
                        .await?;
                }
                EngineType::Wait => {
                    self.wait_tx
                        .send(WaitCommand::Pause {
                            wait_type: WaitType::Wait,
                            instance_id: cue_id,
                        })
                        .await?;
                }
                EngineType::Fade => {
                    log::warn!("Pause command is not available for Fade cue. ignoring...");
                }
                EngineType::Playback => {
                    log::warn!("Pause command is not available for Transport cues. ignoring...");
                }
                EngineType::Group => {
                    if let Some(cue) = self.model_handle.get_cue_by_id(&cue_id).await
                        && let CueParam::Group { children, .. } = cue.params
                    {
                        for child_id in children.iter() {
                            if self.active_instances.contains_key(child_id)
                                && let Err(e) = self
                                    .process_command(ExecutorCommand::Pause(*child_id))
                                    .await
                            {
                                log::error!("Failed to pause group child. e={}", e);
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    async fn resume_cue(&mut self, cue_id: Uuid) -> Result<(), anyhow::Error> {
        if let Some(active_instance) = self.active_instances.get(&cue_id) {
            if active_instance.is_prewaiting {
                self.wait_tx
                    .send(WaitCommand::Resume {
                        wait_type: WaitType::PreWait,
                        instance_id: cue_id,
                    })
                    .await?;
                return Ok(());
            }
            match active_instance.engine_type {
                EngineType::Audio => {
                    self.audio_tx
                        .send(AudioCommand::Resume { id: cue_id })
                        .await?;
                }
                EngineType::Wait => {
                    self.wait_tx
                        .send(WaitCommand::Resume {
                            wait_type: WaitType::Wait,
                            instance_id: cue_id,
                        })
                        .await?;
                }
                EngineType::Fade => {
                    log::warn!("Resume command is not available for Fade cue. ignoring...");
                }
                EngineType::Playback => {
                    log::warn!("Resume command is not available for Transport cues. ignoring...");
                }
                EngineType::Group => {
                    if let Some(cue) = self.model_handle.get_cue_by_id(&cue_id).await
                        && let CueParam::Group { children, .. } = cue.params
                    {
                        for child_id in children.iter() {
                            if self.active_instances.contains_key(child_id)
                                && let Err(e) = self
                                    .process_command(ExecutorCommand::Resume(*child_id))
                                    .await
                            {
                                log::error!("Failed to resume group child. e={}", e);
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    async fn stop_cue(&mut self, cue_id: Uuid, stop_mode: StopMode) -> Result<(), anyhow::Error> {
        if let Some(active_instance) = self.active_instances.get(&cue_id) {
            let is_prewaiting = active_instance.is_prewaiting;
            if is_prewaiting {
                match active_instance.engine_type {
                    EngineType::Audio => {
                        self.audio_tx
                            .send(AudioCommand::HardStop { id: cue_id })
                            .await?;
                    }
                    EngineType::Wait => {
                        self.wait_tx
                            .send(WaitCommand::Stop {
                                wait_type: WaitType::Wait,
                                instance_id: cue_id,
                            })
                            .await?;
                    }
                    EngineType::Fade => {
                        self.wait_tx
                            .send(WaitCommand::Stop {
                                wait_type: WaitType::FadeWait,
                                instance_id: cue_id,
                            })
                            .await?;
                    }
                    EngineType::Playback => {}
                    EngineType::Group => {
                        if let Some(cue) = self.model_handle.get_cue_by_id(&cue_id).await
                            && let CueParam::Group { children, .. } = cue.params
                        {
                            for child_id in children.iter() {
                                if self.active_instances.contains_key(child_id) {
                                    self.process_command(ExecutorCommand::Stop(
                                        *child_id,
                                        StopMode::Hard,
                                    ))
                                    .await?;
                                }
                            }
                        }
                    }
                }
                self.wait_tx
                    .send(WaitCommand::Stop {
                        wait_type: WaitType::PreWait,
                        instance_id: cue_id,
                    })
                    .await?;
            } else {
                match active_instance.engine_type {
                    EngineType::Audio => {
                        let command = match stop_mode {
                            StopMode::Soft => AudioCommand::SoftStop { id: cue_id },
                            StopMode::Hard => AudioCommand::HardStop { id: cue_id },
                        };
                        self.audio_tx.send(command).await?;
                    }
                    EngineType::Wait => {
                        self.wait_tx
                            .send(WaitCommand::Stop {
                                wait_type: WaitType::Wait,
                                instance_id: cue_id,
                            })
                            .await?;
                    }
                    EngineType::Fade => {
                        log::warn!("Stop command is not available for Fade cue. ignoring...");
                    }
                    EngineType::Playback => {
                        self.active_instances.remove(&cue_id);
                        self.emit_stopped(cue_id).await?;
                    }
                    EngineType::Group => {
                        if let Some(cue) = self.model_handle.get_cue_by_id(&cue_id).await
                            && let CueParam::Group { children, .. } = cue.params
                        {
                            for child_id in children.iter() {
                                if self.active_instances.contains_key(child_id)
                                    && let Err(e) = self
                                        .process_command(ExecutorCommand::Stop(
                                            *child_id, stop_mode,
                                        ))
                                        .await
                                {
                                    log::error!("Failed to stop group child. e={}", e);
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    async fn seek_to_cue(&self, cue_id: Uuid, position: f64) -> Result<(), anyhow::Error> {
        if let Some(active_instance) = self.active_instances.get(&cue_id) {
            if active_instance.is_prewaiting {
                self.wait_tx
                    .send(WaitCommand::SeekTo {
                        wait_type: WaitType::PreWait,
                        instance_id: cue_id,
                        position,
                    })
                    .await?;
                return Ok(());
            }
            match active_instance.engine_type {
                EngineType::Audio => {
                    self.audio_tx
                        .send(AudioCommand::SeekTo {
                            id: cue_id,
                            position,
                        })
                        .await?;
                }
                EngineType::Wait => {
                    self.wait_tx
                        .send(WaitCommand::SeekTo {
                            wait_type: WaitType::Wait,
                            instance_id: cue_id,
                            position,
                        })
                        .await?;
                }
                EngineType::Fade => {
                    log::warn!("SeekTo command is not available for Fade cue. ignoring...");
                }
                EngineType::Playback => {
                    log::warn!("SeekTo command is not available for Transport cues. ignoring...");
                }
                EngineType::Group => {
                    log::warn!("SeekTo command is not available for Group cues. ignoring...");
                }
            }
        }
        Ok(())
    }

    async fn seek_by_cue(&self, cue_id: Uuid, amount: f64) -> Result<(), anyhow::Error> {
        if let Some(active_instance) = self.active_instances.get(&cue_id) {
            if active_instance.is_prewaiting {
                self.wait_tx
                    .send(WaitCommand::SeekBy {
                        wait_type: WaitType::PreWait,
                        instance_id: cue_id,
                        amount,
                    })
                    .await?;
                return Ok(());
            }
            match active_instance.engine_type {
                EngineType::Audio => {
                    self.audio_tx
                        .send(AudioCommand::SeekBy { id: cue_id, amount })
                        .await?;
                }
                EngineType::Wait => {
                    self.wait_tx
                        .send(WaitCommand::SeekBy {
                            wait_type: WaitType::Wait,
                            instance_id: cue_id,
                            amount,
                        })
                        .await?;
                }
                EngineType::Fade => {
                    log::warn!("SeekBy command is not available for Fade cue. ignoring...");
                }
                EngineType::Playback => {
                    log::warn!("SeekBy command is not available for Transport cues. ignoring...");
                }
                EngineType::Group => {
                    log::warn!("SeekTo command is not available for Group cues. ignoring...");
                }
            }
        }
        Ok(())
    }

    async fn handle_engine_event(&mut self, event: EngineEvent) -> Result<(), anyhow::Error> {
        match event {
            EngineEvent::Audio(audio_event) => {
                let cue_id = audio_event.id();

                let playback_event = match audio_event {
                    AudioEngineEvent::Loaded {
                        position, duration, ..
                    } => ExecutorEvent::Loaded {
                        cue_id,
                        position,
                        duration,
                    },
                    AudioEngineEvent::Started {
                        position,
                        duration,
                        initial_params,
                        ..
                    } => ExecutorEvent::Started {
                        cue_id,
                        position,
                        duration,
                        initial_params: StateParam::Audio(initial_params),
                    },
                    AudioEngineEvent::Progress {
                        position, duration, ..
                    } => {
                        let event = ExecutorEvent::Progress {
                            cue_id,
                            position,
                            duration,
                        };
                        if let Err(e) = self.executor_event_tx.try_send(event) {
                            log::warn!("EngineEvent dropped: {:?}", e);
                        }
                        return Ok(());
                    }
                    AudioEngineEvent::Paused {
                        position, duration, ..
                    } => {
                        self.active_instances
                            .entry(cue_id)
                            .and_modify(|instance| instance.is_paused = true);
                        ExecutorEvent::Paused {
                            cue_id,
                            position,
                            duration,
                        }
                    }
                    AudioEngineEvent::Resumed { .. } => {
                        self.active_instances
                            .entry(cue_id)
                            .and_modify(|instance| instance.is_paused = false);
                        ExecutorEvent::Resumed { cue_id }
                    }
                    AudioEngineEvent::Seeked { position, .. } => {
                        ExecutorEvent::Seeked { cue_id, position }
                    }
                    AudioEngineEvent::Stopping {
                        position, duration, ..
                    } => {
                        let event = ExecutorEvent::Stopping {
                            cue_id,
                            position,
                            duration,
                        };
                        if let Err(e) = self.executor_event_tx.try_send(event) {
                            log::warn!("EngineEvent dropped: {:?}", e);
                        }
                        return Ok(());
                    }
                    AudioEngineEvent::Stopped { .. } => {
                        self.active_instances.remove(&cue_id);
                        return self.emit_stopped(cue_id).await;
                    }
                    AudioEngineEvent::Completed { .. } => {
                        self.active_instances.remove(&cue_id);
                        return self.emit_completed(cue_id).await;
                    }
                    AudioEngineEvent::StateParamUpdated { params, .. } => {
                        ExecutorEvent::StateParamUpdated {
                            cue_id,
                            params: StateParam::Audio(params),
                        }
                    }
                    AudioEngineEvent::Error { error, .. } => {
                        self.active_instances.remove(&cue_id);
                        return self.emit_error(cue_id, error).await;
                    }
                };

                self.executor_event_tx.send(playback_event).await?;
            }
            EngineEvent::PreWait(wait_event) => {
                let cue_id = wait_event.id();

                let executor_event = match wait_event {
                    WaitEvent::Loaded { .. } => unreachable!(),
                    WaitEvent::Started { duration, .. } => {
                        ExecutorEvent::PreWaitStarted { cue_id, duration }
                    }
                    WaitEvent::Progress {
                        position, duration, ..
                    } => {
                        let event = ExecutorEvent::PreWaitProgress {
                            cue_id,
                            position,
                            duration,
                        };
                        if let Err(e) = self.executor_event_tx.try_send(event) {
                            log::warn!("EngineEvent dropped: {:?}", e);
                        }
                        return Ok(());
                    }
                    WaitEvent::Paused {
                        position, duration, ..
                    } => {
                        self.active_instances
                            .entry(cue_id)
                            .and_modify(|instance| instance.is_paused = true);
                        ExecutorEvent::PreWaitPaused {
                            cue_id,
                            position,
                            duration,
                        }
                    }
                    WaitEvent::Resumed { .. } => {
                        self.active_instances
                            .entry(cue_id)
                            .and_modify(|instance| instance.is_paused = false);
                        ExecutorEvent::PreWaitResumed { cue_id }
                    }
                    WaitEvent::Seeked { position, .. } => {
                        ExecutorEvent::Seeked { cue_id, position }
                    }
                    WaitEvent::Stopped { .. } => {
                        if self.active_instances.remove(&cue_id).is_some() {
                            log::info!("PreWaitStopped cue_id={}", cue_id);
                            self.emit_stopped(cue_id).await?;
                        }
                        return Ok(());
                    }
                    WaitEvent::Completed { .. } => {
                        if let Some(cue) = self.model_handle.get_cue_by_id(&cue_id).await {
                            log::info!("PreWaitCompleted cue_id={}", cue.id);
                            self.executor_event_tx
                                .send(ExecutorEvent::PreWaitCompleted { cue_id })
                                .await?;
                            self.execute_cue(&cue).await?;
                            return Ok(());
                        } else {
                            self.executor_event_tx
                                .send(ExecutorEvent::PreWaitCompleted { cue_id })
                                .await?;
                            self.active_instances.remove(&cue_id);
                            anyhow::bail!("PreWait: cue to execute not found. id={}", cue_id);
                        }
                    }
                };

                self.executor_event_tx.send(executor_event).await?;
            }
            EngineEvent::Wait(wait_event) | EngineEvent::Fade(wait_event) => {
                let cue_id = wait_event.id();

                let playback_event = match wait_event {
                    WaitEvent::Loaded {
                        position, duration, ..
                    } => ExecutorEvent::Loaded {
                        cue_id,
                        position,
                        duration,
                    },
                    WaitEvent::Started {
                        position, duration, ..
                    } => ExecutorEvent::Started {
                        cue_id,
                        position,
                        duration,
                        initial_params: StateParam::None,
                    },
                    WaitEvent::Progress {
                        position, duration, ..
                    } => {
                        let event = ExecutorEvent::Progress {
                            cue_id,
                            position,
                            duration,
                        };
                        if let Err(e) = self.executor_event_tx.try_send(event) {
                            log::warn!("EngineEvent dropped: {:?}", e);
                        }
                        return Ok(());
                    }
                    WaitEvent::Paused {
                        position, duration, ..
                    } => {
                        self.active_instances
                            .entry(cue_id)
                            .and_modify(|instance| instance.is_paused = true);
                        ExecutorEvent::Paused {
                            cue_id,
                            position,
                            duration,
                        }
                    }
                    WaitEvent::Resumed { .. } => {
                        self.active_instances
                            .entry(cue_id)
                            .and_modify(|instance| instance.is_paused = false);
                        ExecutorEvent::Resumed { cue_id }
                    }
                    WaitEvent::Seeked { position, .. } => {
                        ExecutorEvent::Seeked { cue_id, position }
                    }
                    WaitEvent::Stopped { .. } => {
                        self.active_instances.remove(&cue_id);
                        return self.emit_stopped(cue_id).await;
                    }
                    WaitEvent::Completed { .. } => {
                        self.active_instances.remove(&cue_id);
                        return self.emit_completed(cue_id).await;
                    }
                };

                self.executor_event_tx.send(playback_event).await?;
            }
        }
        Ok(())
    }

    async fn emit_stopped(&mut self, cue_id: Uuid) -> Result<(), anyhow::Error> {
        self.executor_event_tx
            .send(ExecutorEvent::Stopped { cue_id })
            .await?;
        self.check_and_stop_parents(cue_id, false).await?;
        Ok(())
    }

    async fn emit_error(&mut self, cue_id: Uuid, error: String) -> Result<(), anyhow::Error> {
        self.executor_event_tx
            .send(ExecutorEvent::Error { cue_id, error })
            .await?;
        self.check_and_stop_parents(cue_id, false).await?;
        Ok(())
    }

    async fn emit_completed(&mut self, cue_id: Uuid) -> Result<(), anyhow::Error> {
        self.executor_event_tx
            .send(ExecutorEvent::Completed { cue_id })
            .await?;
        self.resolve_after_complete_chain(cue_id).await?; // trigger cue chain before stop parents
        self.check_and_stop_parents(cue_id, true).await?;
        Ok(())
    }

    async fn resolve_after_start_chain(&mut self, cue_id: Uuid) -> Result<(), anyhow::Error> {
        if let Some(chain) = self.model_handle.get_cue_chain_by_id(&cue_id).await {
            if let CueChain::AfterStart { target_id } = &chain {
                if let Some(target) = target_id {
                    if let Err(e) = self
                        .process_command(ExecutorCommand::Execute(*target))
                        .await
                    {
                        log::error!("Failed to perform cue chain. ignoring. error={}", e);
                    }
                } else if let Some(next_id) = self.model_handle.get_next_cue_id_by_id(&cue_id).await
                    && let Err(e) = self
                        .process_command(ExecutorCommand::Execute(next_id))
                        .await
                {
                    log::error!("Failed to perform cue chain. ignoring. error={}", e);
                }
            }
        } else {
            log::warn!(
                "Unknown cue started. model may be broken. cue_id={}",
                cue_id
            );
        }
        Ok(())
    }

    async fn resolve_after_complete_chain(&mut self, cue_id: Uuid) -> Result<(), anyhow::Error> {
        if let Some(chain) = self.model_handle.get_cue_chain_by_id(&cue_id).await {
            if let CueChain::AfterComplete { target_id } = &chain {
                if let Some(target) = target_id {
                    if let Err(e) = self
                        .process_command(ExecutorCommand::Execute(*target))
                        .await
                    {
                        log::error!("Failed to perform cue chain. ignoring. error={}", e);
                    }
                } else if let Some(next_id) = self.model_handle.get_next_cue_id_by_id(&cue_id).await
                    && let Err(e) = self
                        .process_command(ExecutorCommand::Execute(next_id))
                        .await
                {
                    log::error!("Failed to perform cue chain. ignoring. error={}", e);
                }
            }
        } else {
            log::warn!(
                "Unknown cue completed. model may be broken. cue_id={}",
                cue_id
            );
        }
        Ok(())
    }

    async fn check_and_start_parents(&mut self, cue_id: Uuid) -> Result<(), anyhow::Error> {
        let mut stack = VecDeque::from([cue_id]);

        while let Some(target_id) = stack.pop_back() {
            if let Some(parent) = self.model_handle.get_parent_by_id(&target_id).await {
                let mut need_notify_event = false;
                self.active_instances
                    .entry(parent.id)
                    .and_modify(|instance| {
                        if instance.is_prewaiting || !instance.is_triggered {
                            need_notify_event = true;
                        }
                        instance.is_prewaiting = false;
                        instance.is_triggered = true;
                    })
                    .or_insert_with(|| {
                        need_notify_event = true;
                        ActiveInstance {
                            engine_type: EngineType::Group,
                            is_prewaiting: false,
                            is_triggered: true,
                            is_paused: false,
                        }
                    });
                if need_notify_event {
                    self.resolve_after_start_chain(parent.id).await?;
                    self.executor_event_tx
                        .send(ExecutorEvent::Triggered { cue_id: parent.id })
                        .await?;
                    self.executor_event_tx
                        .send(ExecutorEvent::Started {
                            cue_id: parent.id,
                            position: 0.0,
                            duration: 0.0,
                            initial_params: StateParam::None,
                        })
                        .await?;
                }
                stack.push_back(parent.id);
            }
        }
        Ok(())
    }

    async fn check_and_stop_parents(
        &mut self,
        cue_id: Uuid,
        is_completed: bool,
    ) -> Result<(), anyhow::Error> {
        let mut stack = VecDeque::from([cue_id]);

        while let Some(target_id) = stack.pop_back() {
            if let Some(parent) = self.model_handle.get_parent_by_id(&target_id).await
                && let CueParam::Group { children, .. } = parent.params
            {
                if children
                    .iter()
                    .any(|cue_id| self.active_instances.contains_key(cue_id))
                    || !self.active_instances.contains_key(&parent.id)
                {
                    continue;
                };

                self.active_instances.remove(&parent.id);
                if is_completed {
                    self.resolve_after_complete_chain(parent.id).await?;
                }

                let reactivated = self.active_instances.contains_key(&parent.id)
                    || children
                        .iter()
                        .any(|id| self.active_instances.contains_key(id));
                if reactivated {
                    continue;
                }
                if is_completed {
                    self.executor_event_tx
                        .send(ExecutorEvent::Completed { cue_id: parent.id })
                        .await?;
                } else {
                    self.executor_event_tx
                        .send(ExecutorEvent::Stopped { cue_id: parent.id })
                        .await?;
                }
                stack.push_back(parent.id);
            }
        }
        Ok(())
    }
}
