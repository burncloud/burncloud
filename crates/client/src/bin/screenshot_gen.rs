use burncloud_client::pages::login::LoginPage;
use burncloud_client_shared::liveview_style_tags;
use dioxus::prelude::*;

#[derive(Clone, Routable, Debug, PartialEq)]
enum MockRoute {
    #[route("/")]
    LoginPage {},
}

#[component]
fn MockApp() -> Element {
    burncloud_client_shared::i18n::use_init_i18n();
    burncloud_client_shared::use_init_toast();
    burncloud_client_shared::use_init_auth();

    rsx! {
        Router::<MockRoute> {}
    }
}

fn main() {
    let mut vdom = VirtualDom::new(MockApp);
    vdom.rebuild_in_place();
    let html_content = dioxus_ssr::render(&vdom);

    let full_html = format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    {styles}
    <style>
        .animate-in {{ animation: none !important; opacity: 1 !important; transform: none !important; }}
    </style>
</head>
<body style="margin:0; padding:0;">
    {body}
</body>
</html>"#,
        styles = liveview_style_tags(),
        body = html_content
    );

    println!("{}", full_html);
}
