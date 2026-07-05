use super::layout::CoreRoute;
use crate::components::console_sidebar::{ConsoleNav, ConsoleSidebar, NavId};
use dioxus::prelude::*;

struct DevConsoleNav;

impl ConsoleNav for DevConsoleNav {
    type Route = CoreRoute;

    fn nav_route(id: NavId) -> CoreRoute {
        match id {
            NavId::Dashboard => CoreRoute::Dashboard {},
            NavId::Channel => CoreRoute::ChannelPage {},
            NavId::Connect => CoreRoute::ConnectPage {},
            NavId::Access => CoreRoute::ApiManagement {},
            NavId::Playground => CoreRoute::PlaygroundPage {},
            NavId::Monitor => CoreRoute::ServiceMonitor {},
            NavId::Logs => CoreRoute::LogPage {},
            NavId::Users => CoreRoute::UsersPage {},
            NavId::Finance => CoreRoute::FinancePage {},
            NavId::Settings => CoreRoute::SystemSettings {},
        }
    }
}

#[component]
pub fn Sidebar() -> Element {
    rsx! {
        ConsoleSidebar::<DevConsoleNav> {}
    }
}
