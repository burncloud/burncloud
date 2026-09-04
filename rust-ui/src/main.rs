mod app;
mod components;
mod i18n;
mod models;
mod pages;

use app::App;
use leptos::prelude::*;

fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(App);
}
