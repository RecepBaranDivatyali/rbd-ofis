use crate::error::CoreError;
use crate::session::DocumentSession;

/// An undoable/redoable document manipulation command.
pub trait Command: Send + Sync {
    /// Human-readable description of this command action.
    fn name(&self) -> &str;

    /// Execute the command forward.
    fn execute(&mut self, session: &mut DocumentSession) -> Result<(), CoreError>;

    /// Undo the effects of the command.
    fn undo(&mut self, session: &mut DocumentSession) -> Result<(), CoreError>;
}

/// A standard undo/redo history stack for document editing operations.
pub struct CommandStack {
    undo_stack: Vec<Box<dyn Command>>,
    redo_stack: Vec<Box<dyn Command>>,
    max_history: usize,
}

impl CommandStack {
    /// Create a new command stack with the specified max history capacity.
    pub fn new(max_history: usize) -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_history: max_history.max(1),
        }
    }

    /// Push and immediately execute a new command. Clears the redo stack.
    pub fn execute(
        &mut self,
        mut command: Box<dyn Command>,
        session: &mut DocumentSession,
    ) -> Result<(), CoreError> {
        command.execute(session)?;
        self.redo_stack.clear();
        self.undo_stack.push(command);

        if self.undo_stack.len() > self.max_history {
            self.undo_stack.remove(0);
        }

        Ok(())
    }

    /// Undo the top command on the undo stack.
    pub fn undo(&mut self, session: &mut DocumentSession) -> Result<bool, CoreError> {
        if let Some(mut command) = self.undo_stack.pop() {
            command.undo(session)?;
            self.redo_stack.push(command);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Redo the top command on the redo stack.
    pub fn redo(&mut self, session: &mut DocumentSession) -> Result<bool, CoreError> {
        if let Some(mut command) = self.redo_stack.pop() {
            command.execute(session)?;
            self.undo_stack.push(command);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Check if undo is available.
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Check if redo is available.
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Clear all undo and redo history.
    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }
}

impl Default for CommandStack {
    fn default() -> Self {
        Self::new(100)
    }
}
