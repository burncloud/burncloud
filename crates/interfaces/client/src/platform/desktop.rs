//! Desktop platform adapter.

#[cfg(feature = "desktop")]
pub fn drag_window() {
    dioxus::desktop::window().drag();
}

#[cfg(not(feature = "desktop"))]
pub fn drag_window() {}
