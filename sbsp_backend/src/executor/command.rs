use uuid::Uuid;

use crate::{action::CueAction, model::settings::ShowSettings};

#[derive(Debug)]
pub enum ExecutorCommand {
    Load(Uuid),
    Execute(Uuid),
    Pause(Uuid),
    Resume(Uuid),
    Stop(Uuid),
    SeekTo(Uuid, f64),
    SeekBy(Uuid, f64),
    PerformAction(Uuid, CueAction),
    ReconfigureEngines(Box<ShowSettings>),
}
