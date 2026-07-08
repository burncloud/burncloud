//! Preview page shell: seeds admin auth + activates mock fixtures for one route.

use crate::auth_context::{AuthContext, CurrentUser, use_auth};
use dioxus::prelude::*;

use super::{activate, fixtures, E2eMockPage};

const PREVIEW_TOKEN: &str = "e2e-preview-token";

fn seed_preview_session(auth: AuthContext, page: E2eMockPage) {
    auth.set_auth(
        PREVIEW_TOKEN.into(),
        CurrentUser {
            id: "preview-admin".into(),
            username: "preview-admin".into(),
            roles: vec!["admin".into()],
        },
    );
    activate(fixtures::registry_for_page(page));
}

/// Wraps a console page for `/preview/*` routes.
/// **Never used on production routes** — only composed from preview route handlers.
#[component]
pub fn E2eMockPageShell(page: E2eMockPage, children: Element) -> Element {
    let auth = use_auth();

    // Run before child `use_resource` hooks: `use_effect` is too late (fires after paint).
    use_memo(move || {
        seed_preview_session(auth, page);
        page
    });

    rsx! {
        {children}
    }
}
