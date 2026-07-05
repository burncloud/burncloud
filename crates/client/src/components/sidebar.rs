use crate::app::Route;
use burncloud_client_shared::components::console_sidebar::{ConsoleNav, ConsoleSidebar, NavId};
use dioxus::prelude::*;

struct AppConsoleNav;

impl ConsoleNav for AppConsoleNav {
    type Route = Route;

    fn nav_route(id: NavId) -> Route {
        match id {
            NavId::Dashboard => Route::Dashboard {},
            NavId::Channel => Route::ChannelPage {},
            NavId::Connect => Route::ConnectPage {},
            NavId::Access => Route::AccessPage {},
            NavId::Playground => Route::PlaygroundPage {},
            NavId::Monitor => Route::ServiceMonitor {},
            NavId::Logs => Route::LogPage {},
            NavId::Users => Route::UsersPage {},
            NavId::Finance => Route::FinancePage {},
            NavId::Settings => Route::SystemSettings {},
        }
    }
}

#[component]
pub fn Sidebar() -> Element {
    rsx! {
        ConsoleSidebar::<AppConsoleNav> {}
    }
}
