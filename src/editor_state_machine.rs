#[allow(clippy::large_enum_variant)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EditorState {
    Idle,
    Pan,
    Move,
    Link,
    Rect,
    WindowResizing,
    PropertyChanging,
    ContextMenu,
}

impl EditorState {
    pub fn uses_dragging(&self) -> bool {
        self == &EditorState::Pan
            || self == &EditorState::Move
            || self == &EditorState::Rect
            || self == &EditorState::Link
    }
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub enum EditorStateTrigger {
    MouseDownOverNode,

    DblClickToCreate,
    DblClickToRename,
    ClickToSelect,
    ClickToDeselect,
    ClickToReposition,
    ClickToRename,

    DragToPan,
    DragToSelect,
    DragToMove,
    DragToLink,
    DragToWindowResize,

    EndDrag,

    ClickToContextMenu,
    ExitContextMenu,
}

pub trait StateMachine {
    type Trigger: Copy;
    type State: Copy;

    fn on_transition(&mut self, from: Self::State, trigger: Self::Trigger) -> Option<Self::State>;

    fn get_current_state(&self) -> Self::State;
    fn move_to(&mut self, next: Self::State);

    fn trigger(&mut self, trigger: Self::Trigger) {
        if let Some(next) = self.on_transition(self.get_current_state(), trigger) {
            self.move_to(next);
        }
    }
}
