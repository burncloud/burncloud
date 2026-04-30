use dioxus::prelude::*;

#[derive(PartialEq, Clone, Copy)]
pub enum SkeletonVariant {
    Kpi,
    Row,
    Bar,
}

#[component]
pub fn SkeletonCard(variant: Option<SkeletonVariant>) -> Element {
    let class = match variant.unwrap_or(SkeletonVariant::Row) {
        SkeletonVariant::Kpi => "skeleton skeleton-kpi",
        SkeletonVariant::Row => "skeleton skeleton-row",
        SkeletonVariant::Bar => "skeleton skeleton-bar",
    };

    rsx! {
        div { class: "{class}" }
    }
}
