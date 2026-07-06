use crate::utils::storage::{ClientState, Theme};
use dioxus::prelude::*;

#[derive(Clone, Copy)]
pub struct ThemeContext {
    pub theme: Signal<Theme>,
}

impl ThemeContext {
    pub fn new() -> Self {
        let state = ClientState::load();
        Self {
            theme: Signal::new(state.theme.unwrap_or_default()),
        }
    }

    pub fn set_theme(mut self, theme: Theme) {
        *self.theme.write() = theme.clone();
        let mut state = ClientState::load();
        state.theme = Some(theme);
        state.save();
    }

    pub fn data_theme_attr(&self) -> &'static str {
        self.theme.read().as_str()
    }
}

impl Default for ThemeContext {
    fn default() -> Self {
        Self::new()
    }
}

pub fn use_theme() -> ThemeContext {
    use_context::<ThemeContext>()
}

pub fn use_init_theme() -> ThemeContext {
    use_context_provider(ThemeContext::new)
}
