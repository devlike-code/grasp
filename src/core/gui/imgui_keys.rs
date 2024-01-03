use imgui::{Key, Ui};

pub trait ExtraKeyEvents {
    fn is_fkey_down(&self, key: Key) -> bool;
    fn is_fkey_up(&self, key: Key) -> bool;

    fn is_modkey_down(&self, key: Key) -> bool;
    fn is_modkey_up(&self, key: Key) -> bool;
}

impl ExtraKeyEvents for Ui {
    fn is_fkey_down(&self, key: Key) -> bool {
        *self.io().keys_down.get(key as usize).unwrap()
    }

    fn is_fkey_up(&self, key: Key) -> bool {
        !*self.io().keys_down.get(key as usize).unwrap()
    }

    fn is_modkey_down(&self, key: Key) -> bool {
        self.is_key_index_down(key as u32 - 303)
    }

    fn is_modkey_up(&self, key: Key) -> bool {
        !self.is_modkey_down(key)
    }
}
