// ----------------------------------------------------------------------------
use bevy::utils::HashSet;

use crate::cmds::{TrackedProgress, TrackedTaskname};
// ----------------------------------------------------------------------------
#[derive(Default)]
pub struct ProgressTracking {
    task: Option<MultiTaskProgress>,
}
// ----------------------------------------------------------------------------
#[derive(Default)]
pub struct MultiTaskProgress {
    last_msg: String,
    progress: f32,
    tasks: HashSet<TrackedProgress>,
}
// ----------------------------------------------------------------------------
impl MultiTaskProgress {
    // ------------------------------------------------------------------------
    pub fn update(&mut self, update: &TrackedProgress) {
        // make sure updates for events that are not tracked are ignored
        if self.tasks.contains(update) && self.tasks.replace(*update).is_some() {
            self.last_msg = if update.is_finished() {
                update.finished_msg().to_string()
            } else {
                update.progress_msg()
            };
            self.refresh_progress()
        }
    }
    // ------------------------------------------------------------------------
    fn refresh_progress(&mut self) {
        self.progress =
            self.tasks.iter().map(|t| t.progress()).sum::<f32>() / self.tasks.len() as f32;
    }
    // ------------------------------------------------------------------------
    pub fn progress(&self) -> f32 {
        self.progress
    }
    // ------------------------------------------------------------------------
    pub fn is_finished(&self) -> bool {
        self.tasks.iter().all(|p| p.is_finished())
    }
    // ------------------------------------------------------------------------
    pub fn last_msg(&self) -> &str {
        &self.last_msg
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl ProgressTracking {
    // ------------------------------------------------------------------------
    pub fn task(&self) -> Option<&MultiTaskProgress> {
        self.task.as_ref()
    }
    // ------------------------------------------------------------------------
    pub fn start_task_tracking(&mut self, name: &TrackedTaskname, sub_tasks: &[TrackedProgress]) {
        let mut tasks = MultiTaskProgress {
            last_msg: name.as_str().map(|s| s.to_string()).unwrap_or_default(),
            progress: 0.0,
            tasks: sub_tasks.iter().cloned().collect(),
        };
        tasks.refresh_progress();
        self.task = Some(tasks);
    }
    // ------------------------------------------------------------------------
    pub fn update(&mut self, update: &TrackedProgress) {
        if let Some(multi_task) = self.task.as_mut() {
            multi_task.update(update);

            if multi_task.is_finished() {
                self.task = None;
            }
        }
    }
    // ------------------------------------------------------------------------
    #[allow(dead_code)]
    pub fn cancel_tracking(&mut self) {
        self.task = None;
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
