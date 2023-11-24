use std::sync::{Arc, Mutex};

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

    ClickToSelect,
    ClickToDeselect,

    DragToPan,
    DragToSelect,
    DragToMove,
    DragToLink,
    
    EndDrag,
}

pub trait StateMachine {
    type Trigger : Copy;
    type State;

    fn trigger(&self, trigger: Self::Trigger) -> Option<Self::State>;
    fn transition(&mut self, trigger: Self::Trigger, next: Self::State);

    fn subscribe(&mut self, sub: Arc<Mutex<dyn StateMachineSubscriber<Self::State, Self::Trigger>>>);
}

pub trait StateMachineSubscriber<S, T> {
    fn on_transition(&mut self, from: S, trigger: T, to: S);
}

pub struct EditorStateMachine {
    pub state: EditorState,
    pub(crate) subscribers: Vec<Arc<Mutex<dyn StateMachineSubscriber<EditorState, EditorStateTrigger>>>>,
}

impl StateMachine for EditorStateMachine {
    type Trigger = EditorStateTrigger;
    type State = EditorState;
    
    fn trigger(&self, trigger: EditorStateTrigger) -> Option<EditorState> {
        match (self.state, trigger) {
            (EditorState::Idle, EditorStateTrigger::MouseDownOverNode) => Some(EditorState::Idle),
            (EditorState::Idle, EditorStateTrigger::ClickToSelect) => Some(EditorState::Idle),
            (EditorState::Idle, EditorStateTrigger::ClickToDeselect) => Some(EditorState::Idle),
            (EditorState::Idle, EditorStateTrigger::DragToPan) => Some(EditorState::Pan),
            (EditorState::Idle, EditorStateTrigger::DragToLink) => Some(EditorState::Link),
            (EditorState::Idle, EditorStateTrigger::DragToMove) => Some(EditorState::Move),
            (EditorState::Idle, EditorStateTrigger::DragToSelect) => Some(EditorState::Rect),
            (EditorState::Pan, EditorStateTrigger::EndDrag) => Some(EditorState::Idle),
            (EditorState::Link, EditorStateTrigger::EndDrag) => Some(EditorState::Idle),
            (EditorState::Move, EditorStateTrigger::EndDrag) => Some(EditorState::Idle),
            (EditorState::Rect, EditorStateTrigger::EndDrag) => Some(EditorState::Idle),
            
            _ => None
        }
    }

    fn transition(&mut self, trigger: Self::Trigger, next: Self::State) {
        for sub in &self.subscribers {
            sub.lock().unwrap().on_transition(self.state, trigger, next);
        }
        self.state = next;
    }

    fn subscribe(&mut self, sub: Arc<Mutex<dyn StateMachineSubscriber<Self::State, Self::Trigger>>>) {
        self.subscribers.push(Arc::clone(&sub));
    }

}

pub struct EditorStateSubscriber {}
impl StateMachineSubscriber<EditorState, EditorStateTrigger> for EditorStateSubscriber {
    fn on_transition(&mut self, from: EditorState, trigger: EditorStateTrigger, to: EditorState) {
        match (from, trigger, to) {
            (EditorState::Idle, EditorStateTrigger::MouseDownOverNode, EditorState::Idle) => {}
            (EditorState::Idle, EditorStateTrigger::ClickToSelect, EditorState::Idle) => {}
            (EditorState::Idle, EditorStateTrigger::ClickToDeselect, EditorState::Idle) => {}
            (EditorState::Idle, EditorStateTrigger::DragToPan, EditorState::Pan) => {}
            (EditorState::Idle, EditorStateTrigger::DragToLink, EditorState::Link) => {}
            (EditorState::Idle, EditorStateTrigger::DragToMove, EditorState::Move) => {}
            (EditorState::Idle, EditorStateTrigger::DragToSelect, EditorState::Rect) => {}
            (EditorState::Pan, EditorStateTrigger::EndDrag, EditorState::Idle) => {}
            (EditorState::Link, EditorStateTrigger::EndDrag, EditorState::Idle) => {}
            (EditorState::Move, EditorStateTrigger::EndDrag, EditorState::Idle) => {}
            (EditorState::Rect, EditorStateTrigger::EndDrag, EditorState::Idle) => {}
            _ => {}
        }
    }
}