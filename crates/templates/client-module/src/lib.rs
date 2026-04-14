use burncloud_client_shared::components::StandardCrudPage;
use dioxus::prelude::*;

mod schema;

#[component]
pub fn {{entity_label}}Page() -> Element {
    let schema = schema::get_schema();
    let api_path = schema["api_path"].as_str().unwrap_or("{{api_path}}");

    rsx! {
        // StandardCrudPage 是 client-shared 中的高阶组件
        // 它会自动处理：获取列表、弹出表单、提交数据、删除数据
        StandardCrudPage {
            schema: schema,
            api_endpoint: api_path.to_string(),
        }
    }
}
