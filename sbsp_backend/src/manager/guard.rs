use crate::model::cue::CueList;

pub struct RollbackGuard<'a> {
    pub cue_list: &'a mut CueList,
    backup_cue_list: CueList,
    pub success: bool,
}

impl<'a> RollbackGuard<'a> {
    pub fn from(list: &'a mut CueList) -> Self {
        let backup_cue_list = list.clone();
        Self {
            cue_list: list,
            backup_cue_list,
            success: false,
        }
    }
}

impl<'a> Drop for RollbackGuard<'a> {
    fn drop(&mut self) {
        if !self.success {
            *self.cue_list = self.backup_cue_list.clone();
        }
    }
}
