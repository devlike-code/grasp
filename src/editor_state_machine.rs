#[allow(clippy::large_enum_variant)]
#[derive(Clone, Copy)]
pub enum EditorState {
    Idle,
    Pan,
    Move,
    Link,
    Rect,
}

#[allow(dead_code)]
#[derive(Clone, Copy)]
pub enum EditorStateTrigger {
    MouseDownOverNode,

    DblClickToCreate,
    ClickToSelect,
    ClickToDeselect,

    DragToPan,
    DragToSelect,
    DragToMove,
    DragToLink,

    EndDrag,
}

pub trait StateMachine {
    type Trigger: Copy;
    type State: Copy;

    fn on_transition(&mut self, from: Self::State, trigger: Self::Trigger, next: Self::State);
    fn trigger(&self, trigger: Self::Trigger) -> Option<Self::State>;

    fn get_current_state(&self) -> Self::State;
    fn move_to(&mut self, next: Self::State);

    fn transition(&mut self, trigger: Self::Trigger, next: Self::State) {
        self.on_transition(self.get_current_state(), trigger, next);
        self.move_to(next);
    }
}

pub struct EditorStateMachine {
    pub state: EditorState,
}
