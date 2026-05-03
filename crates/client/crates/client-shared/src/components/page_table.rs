use dioxus::prelude::*;

#[derive(PartialEq, Clone)]
pub struct ColumnDef {
    pub key: String,
    pub label: String,
    pub width: Option<String>,
}

#[component]
pub fn PageTable(
    columns: Vec<ColumnDef>,
    children: Element,
) -> Element {
    rsx! {
        table { class: "table",
            thead {
                tr {
                    for col in &columns {
                        th {
                            style: col.width.as_ref().map(|w| format!("--bc-dynamic-width:{w}")).unwrap_or_default(),
                            class: if col.width.is_some() { "bc-dynamic-width" } else { "" },
                            "{col.label}"
                        }
                    }
                }
            }
            tbody {
                {children}
            }
        }
    }
}
