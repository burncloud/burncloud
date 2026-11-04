//! 通过 VirtualDom 监听器验证页面；旧工件流程通过根上下文注入起始会话。
use super::*;
use dioxus::core::{
    AttributeValue, ElementId, Template, TemplateAttribute, TemplateNode, WriteMutations,
};
use dioxus::html::{
    self,
    geometry::{ClientPoint, ElementPoint, PagePoint, ScreenPoint},
    input_data::{MouseButton, MouseButtonSet},
    HasFileData, HasFormData, HasMouseData, InteractionElementOffset, InteractionLocation,
    ModifiersInteraction, PointerInteraction,
};
use dioxus::prelude::VirtualDom;
use std::collections::BTreeMap;

macro_rules! converters {
    (enum Event { $(#[convert = $convert:ident] $(#[$($extra:tt)*])* $variant:ident($data:ident),)* }) => {
        impl html::HtmlEventConverter for TestConverter {
            $(fn $convert(&self, event: &html::PlatformEventData) -> html::$data {
                event.downcast::<RefCell<Option<html::$data>>>().expect("事件类型").borrow_mut().take().expect("事件数据")
            })*
        }
    };
}
struct TestConverter;
html::with_html_event_groups!(converters);

struct Input(String);
impl HasFileData for Input {
    fn files(&self) -> Vec<html::FileData> {
        Vec::new()
    }
}
impl HasFormData for Input {
    fn value(&self) -> String {
        self.0.clone()
    }
    fn valid(&self) -> bool {
        true
    }
    fn values(&self) -> Vec<(String, html::FormValue)> {
        Vec::new()
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
struct Click;
impl InteractionLocation for Click {
    fn client_coordinates(&self) -> ClientPoint {
        ClientPoint::zero()
    }
    fn screen_coordinates(&self) -> ScreenPoint {
        ScreenPoint::zero()
    }
    fn page_coordinates(&self) -> PagePoint {
        PagePoint::zero()
    }
}
impl InteractionElementOffset for Click {
    fn element_coordinates(&self) -> ElementPoint {
        ElementPoint::zero()
    }
}
impl ModifiersInteraction for Click {
    fn modifiers(&self) -> html::keyboard_types::Modifiers {
        html::keyboard_types::Modifiers::empty()
    }
}
impl PointerInteraction for Click {
    fn trigger_button(&self) -> Option<MouseButton> {
        Some(MouseButton::Primary)
    }
    fn held_buttons(&self) -> MouseButtonSet {
        MouseButtonSet::empty()
    }
}
impl HasMouseData for Click {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

// 只采集控件标识和文本以定位真实监听器，不引入第三方 DOM 或测试依赖。
#[derive(Default)]
struct Probe {
    template: Option<TemplateNode>,
    attributes: BTreeMap<usize, BTreeMap<String, String>>,
    listeners: BTreeMap<usize, Vec<String>>,
    text: String,
    static_text: String,
}
impl Probe {
    fn collect_static_text(&mut self, node: TemplateNode) {
        match node {
            TemplateNode::Text { text } => self.static_text.push_str(text),
            TemplateNode::Element { children, .. } => {
                for child in children {
                    self.collect_static_text(*child);
                }
            }
            _ => {}
        }
    }
    fn register(&mut self, id: ElementId, node: TemplateNode) {
        let values = self.attributes.entry(id.0).or_default();
        values.clear();
        if let TemplateNode::Element { attrs, .. } = node {
            for attr in attrs {
                if let TemplateAttribute::Static { name, value, .. } = attr {
                    values.insert((*name).into(), (*value).into());
                }
            }
        }
    }
    fn find(&self, attribute: &str, value: &str, event: &str) -> ElementId {
        self.attributes
            .iter()
            .rev()
            .find(|(id, attrs)| {
                attrs.get(attribute).is_some_and(|v| v == value)
                    && self
                        .listeners
                        .get(id)
                        .is_some_and(|events| events.iter().any(|e| e == event))
            })
            .map(|(id, _)| ElementId(*id))
            .unwrap_or_else(|| {
                panic!(
                    "找不到 {attribute}={value} 的 {event} 监听器：{:?}",
                    self.attributes
                )
            })
    }
}
impl WriteMutations for Probe {
    fn append_children(&mut self, _: ElementId, _: usize) {}
    fn assign_node_id(&mut self, path: &'static [u8], id: ElementId) {
        let mut node = self.template.expect("已加载模板");
        for index in path {
            if let TemplateNode::Element { children, .. } = node {
                node = children[usize::from(*index)];
            }
        }
        self.register(id, node);
    }
    fn create_placeholder(&mut self, _: ElementId) {}
    fn create_text_node(&mut self, value: &str, _: ElementId) {
        self.text.push_str(value);
    }
    fn load_template(&mut self, template: Template, index: usize, id: ElementId) {
        // 静态说明不会触发文本变更，单独采集以免干扰流式内容顺序断言。
        self.collect_static_text(template.roots[index]);
        self.template = Some(template.roots[index]);
        self.register(id, template.roots[index]);
    }
    fn replace_node_with(&mut self, id: ElementId, _: usize) {
        self.attributes.remove(&id.0);
        self.listeners.remove(&id.0);
    }
    fn replace_placeholder_with_nodes(&mut self, _: &'static [u8], _: usize) {}
    fn insert_nodes_after(&mut self, _: ElementId, _: usize) {}
    fn insert_nodes_before(&mut self, _: ElementId, _: usize) {}
    fn set_attribute(
        &mut self,
        name: &'static str,
        _: Option<&'static str>,
        value: &AttributeValue,
        id: ElementId,
    ) {
        if let AttributeValue::Text(value) = value {
            self.attributes
                .entry(id.0)
                .or_default()
                .insert(name.into(), value.clone());
        }
    }
    fn set_node_text(&mut self, value: &str, _: ElementId) {
        self.text.push_str(value);
    }
    fn create_event_listener(&mut self, name: &'static str, id: ElementId) {
        self.listeners.entry(id.0).or_default().push(name.into());
    }
    fn remove_event_listener(&mut self, name: &'static str, id: ElementId) {
        if let Some(events) = self.listeners.get_mut(&id.0) {
            events.retain(|event| event != name);
        }
    }
    fn remove_node(&mut self, id: ElementId) {
        self.attributes.remove(&id.0);
        self.listeners.remove(&id.0);
    }
    fn push_root(&mut self, _: ElementId) {}
}
struct Ui {
    dom: VirtualDom,
    probe: Probe,
}
impl Ui {
    fn new(backend: Rc<ControlledBackend>) -> Self {
        let mut initial = chat();
        let request = initial.begin_selection(MODEL).expect("初始工件");
        initial.finish_selection(
            request,
            Err(backend_error(ErrorKind::NotConnected, Operation::Status)),
        );
        let dom = VirtualDom::new(super::super::super::ChatHome)
            .with_root_context(initial)
            .with_root_context(BackendHandle(backend));
        Self::mount(dom)
    }
    fn mount(mut dom: VirtualDom) -> Self {
        static CONVERTER: std::sync::Once = std::sync::Once::new();
        CONVERTER.call_once(|| html::set_event_converter(Box::new(TestConverter)));
        let mut probe = Probe::default();
        dom.rebuild(&mut probe);
        let mut result = Self { dom, probe };
        result.flush();
        result
    }
    fn flush(&mut self) {
        for _ in 0..8 {
            self.dom.render_immediate(&mut self.probe);
        }
    }
    fn event<T: 'static>(&mut self, attribute: &str, value: &str, event: &str, data: T) {
        let id = self.probe.find(attribute, value, event);
        self.dom.runtime().handle_event(
            event,
            dioxus::core::Event::new(
                Rc::new(html::PlatformEventData::new(Box::new(RefCell::new(Some(
                    data,
                ))))) as Rc<dyn std::any::Any>,
                true,
            ),
            id,
        );
        self.flush();
    }
    fn click(&mut self, class: &str) {
        self.event("class", class, "click", html::MouseData::new(Click));
    }
    fn input(&mut self, label: &str, text: &str) {
        self.event(
            "aria-label",
            label,
            "input",
            html::FormData::new(Input(text.into())),
        );
    }
    fn select(&mut self) {
        self.event(
            "aria-label",
            "重试状态查询",
            "click",
            html::MouseData::new(Click),
        );
    }
    fn send(&mut self) {
        self.input("输入消息或指令", "真实用户问题");
        self.click("chat-send-button");
    }
}

#[tokio::test]
async fn injected_backend_preserves_existing_artifact_flow_and_history() {
    let backend = Rc::new(ControlledBackend::default());
    let mut ui = Ui::new(backend.clone());
    ui.select();
    ui.send();
    assert!(!backend
        .calls
        .borrow()
        .iter()
        .any(|call| call.starts_with("initialize")));
    ui.click("chat-variant");
    let calls = backend.calls.borrow();
    assert!(calls.contains(&format!("status:{MODEL}")));
    assert!(calls.contains(&"hardware".into()));
    assert_eq!(
        calls
            .iter()
            .filter(|call| call.starts_with("initialize:"))
            .count(),
        1
    );
    assert_eq!(calls.iter().filter(|call| *call == "generate").count(), 1);
    let requests = backend.requests.borrow();
    assert_eq!(requests[0].messages.len(), 1);
    assert_eq!(requests[0].messages[0].content, "真实用户问题");
    assert_eq!(requests[0].reasoning_effort, ReasoningEffort::High);
    assert!(ui.probe.text.contains("接口分段一接口分段二"));
}

#[tokio::test]
async fn status_future_survives_model_option_unmount_and_blocks_send() {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let backend = Rc::new(ControlledBackend {
        status_gate: RefCell::new(Some(receiver)),
        ..Default::default()
    });
    let mut ui = Ui::new(backend.clone());
    ui.select();
    ui.send();
    assert!(!backend.calls.borrow().iter().any(|call| call == "hardware"));
    sender.send(()).expect("关闭搜索弹层不得取消状态请求");
    ui.flush();
    ui.click("chat-send-button");
    assert!(backend.calls.borrow().iter().any(|call| call == "hardware"));
}

#[tokio::test]
async fn stopping_ui_generation_drops_future_and_ignores_late_callback() {
    let backend = Rc::new(ControlledBackend {
        hold_generation: true,
        ..Default::default()
    });
    let mut ui = Ui::new(backend.clone());
    ui.select();
    ui.send();
    ui.click("chat-variant");
    assert!(!backend.generation_dropped.get());
    ui.click("chat-send-button chat-stop-button");
    assert!(backend.generation_dropped.get());
    ui.dom.in_runtime(|| {
        backend
            .generation_callback
            .borrow_mut()
            .as_mut()
            .expect("回调")(GenerationEvent::Content("不得追加的迟到内容".into()))
    });
    ui.flush();
    assert!(!ui.probe.text.contains("不得追加的迟到内容"));
    assert!(ui.probe.text.contains("接口分段一接口分段二"));
}

#[tokio::test]
async fn ui_initialization_failure_never_calls_generation_and_restores_draft() {
    let backend = Rc::new(ControlledBackend {
        fail_initialization: Cell::new(true),
        ..Default::default()
    });
    let mut ui = Ui::new(backend.clone());
    ui.select();
    ui.send();
    ui.click("chat-variant");
    assert!(!backend.calls.borrow().iter().any(|call| call == "generate"));
    assert!(ui.probe.text.contains("受控后端失败"));
    let id = ui.probe.find("aria-label", "输入消息或指令", "input");
    assert_eq!(ui.probe.attributes[&id.0]["value"], "真实用户问题");
    backend.fail_initialization.set(false);
    ui.click("chat-flow-button primary");
    ui.click("chat-variant");
    assert!(!backend.calls.borrow().iter().any(|call| call == "generate"));
    ui.click("chat-send-button");
    assert_eq!(backend.requests.borrow()[0].messages.len(), 1);
}

#[tokio::test]
async fn generation_error_is_shown_in_the_existing_assistant_message() {
    let backend = Rc::new(ControlledBackend {
        fail_generation: true,
        ..Default::default()
    });
    let mut ui = Ui::new(backend.clone());
    ui.select();
    ui.send();
    ui.click("chat-variant");
    assert!(ui.probe.text.contains("受控后端失败"));
    assert!(ui.probe.text.contains("接口分段一接口分段二"));
    assert!(!ui.probe.text.contains("tok/s"));
    assert_eq!(backend.requests.borrow().len(), 1);
    ui.probe.find("class", "chat-message-copy", "click");
}

#[tokio::test]
async fn invalid_initialization_event_cancels_pending_backend_task() {
    let backend = Rc::new(ControlledBackend {
        invalid_pending_initialization: true,
        ..Default::default()
    });
    let mut ui = Ui::new(backend.clone());
    ui.select();
    ui.send();
    ui.click("chat-variant");
    assert!(backend.initialization_dropped.get());
    assert!(!backend.calls.borrow().iter().any(|call| call == "generate"));
    let id = ui.probe.find("aria-label", "输入消息或指令", "input");
    assert_eq!(ui.probe.attributes[&id.0]["value"], "真实用户问题");
}

use super::super::super::catalog::CatalogClient;
use serde_json::json;
use std::{
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

struct CatalogServer {
    client: CatalogClient,
    requests: Arc<Mutex<Vec<String>>>,
    task: tokio::task::JoinHandle<()>,
}
impl Drop for CatalogServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}
impl CatalogServer {
    async fn start(status: &'static str, fail_first: bool, fail_status: bool) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("测试服务");
        let client = CatalogClient::new(
            &format!("http://{}", listener.local_addr().expect("地址")),
            Duration::from_secs(2),
        )
        .expect("客户端");
        let requests = Arc::new(Mutex::new(Vec::new()));
        let seen = requests.clone();
        let fail = AtomicBool::new(fail_first);
        let task = tokio::spawn(async move {
            loop {
                let (mut stream, _) = listener.accept().await.expect("连接");
                let mut request = Vec::new();
                loop {
                    let mut buf = [0; 1024];
                    let count = stream.read(&mut buf).await.expect("读取");
                    request.extend_from_slice(&buf[..count]);
                    if count == 0 || request.windows(4).any(|bytes| bytes == b"\r\n\r\n") {
                        break;
                    }
                }
                let request = String::from_utf8(request).expect("请求");
                seen.lock().expect("请求记录").push(request.clone());
                let (code, body) = if request.contains("/api/models/presets") {
                    if fail.swap(false, Ordering::SeqCst) {
                        (503, json!({"message":"失败"}))
                    } else {
                        (
                            200,
                            json!([{"id":"test-1","name":"测试型号","preset":true,"model":"owner/Test-1","revision":"master","source":"modelscope"}]),
                        )
                    }
                } else if request.contains("/api/models/status?") {
                    if fail_status {
                        (503, json!({"message":"状态失败"}))
                    } else {
                        (
                            200,
                            json!({"status":status,"model":"owner/Test-1","revision":"master","source":"modelscope","resource_url":"https://modelscope.cn/models/owner/Test-1/files?Revision=master"}),
                        )
                    }
                } else {
                    (404, json!({"message":"不支持旧接口"}))
                };
                let body = body.to_string();
                let response=format!("HTTP/1.1 {code} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len());
                if let Err(error) = stream.write_all(response.as_bytes()).await {
                    // 关闭弹层会取消目录请求；测试服务允许客户端先断开。
                    eprintln!("目录测试连接关闭：{error}");
                }
            }
        });
        Self {
            client,
            requests,
            task,
        }
    }
}
impl Ui {
    fn catalog(backend: Rc<ControlledBackend>, client: CatalogClient) -> Self {
        Self::mount(
            VirtualDom::new(super::super::super::ChatHome)
                .with_root_context(BackendHandle(backend))
                .with_root_context(client),
        )
    }
    fn label(&mut self, label: &str) {
        self.event("aria-label", label, "click", html::MouseData::new(Click));
    }
    async fn wait_for(&mut self, check: impl Fn(&Probe) -> bool) {
        tokio::time::timeout(Duration::from_secs(3), async {
            loop {
                self.flush();
                if check(&self.probe) {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .expect("目录界面及时更新");
    }
    async fn preview_catalog(&mut self) {
        self.click("chat-model-trigger");
        self.wait_for(|probe| probe.text.contains("测试型号")).await;
        self.click("chat-model-option");
        self.wait_for(|probe| {
            probe.attributes.values().any(|attrs| {
                attrs
                    .get("aria-label")
                    .is_some_and(|label| label == "下载" || label == "使用此版本")
            })
        })
        .await;
    }
}
#[tokio::test]
async fn catalog_handoff_detects_once_and_stops_without_artifact_or_deployment() {
    let service = CatalogServer::start("downloadable", false, false).await;
    let backend = Rc::new(ControlledBackend::default());
    let mut ui = Ui::catalog(backend.clone(), service.client.clone());
    ui.input("输入消息或指令", "目录草稿");
    ui.preview_catalog().await;
    assert!(backend.calls.borrow().is_empty());
    ui.label("下载");
    ui.wait_for(|probe| probe.text.contains("尚未接入该仓库版本的工件解析与部署"))
        .await;
    assert!(ui.probe.static_text.contains("当前入口未提供可下载工件"));
    assert!(!ui.probe.static_text.contains("工件下载并校验后停止"));
    assert_eq!(*backend.calls.borrow(), vec!["hardware"]);
    let input = ui.probe.find("aria-label", "输入消息或指令", "input");
    assert_eq!(ui.probe.attributes[&input.0]["value"], "目录草稿");
    ui.click("chat-send-button");
    assert_eq!(*backend.calls.borrow(), vec!["hardware"]);
    let calls = service.requests.lock().expect("请求记录");
    assert_eq!(calls.len(), 2);
    assert!(calls[0].contains("/api/models/presets"));
    assert!(calls[1].contains("/api/models/status?preset=test-1"));
}
#[tokio::test]
async fn catalog_status_failure_keeps_chat_model_and_allows_retry() {
    let service = CatalogServer::start("downloadable", false, true).await;
    let backend = Rc::new(ControlledBackend::default());
    let mut ui = Ui::catalog(backend.clone(), service.client.clone());
    ui.click("chat-model-trigger");
    ui.wait_for(|probe| probe.text.contains("测试型号")).await;
    ui.click("chat-model-option");
    ui.wait_for(|probe| {
        probe.attributes.values().any(|attrs| {
            attrs
                .get("aria-label")
                .is_some_and(|label| label == "重试版本状态")
        })
    })
    .await;
    assert!(backend.calls.borrow().is_empty());
    ui.label("重试版本状态");
    ui.wait_for(|probe| {
        service.requests.lock().expect("请求").len() == 3
            && probe.attributes.values().any(|attrs| {
                attrs
                    .get("aria-label")
                    .is_some_and(|label| label == "重试版本状态")
            })
    })
    .await;
    ui.label("关闭模型菜单");
    let input = ui.probe.find("aria-label", "输入消息或指令", "input");
    assert!(!ui.probe.attributes[&input.0]["placeholder"].contains("测试型号"));
    assert_eq!(service.requests.lock().expect("请求").len(), 3);
}
#[tokio::test]
async fn catalog_search_failure_retry_and_close_do_not_change_chat_model() {
    for has_selected_model in [false, true] {
        let service = CatalogServer::start("available", true, false).await;
        let backend = Rc::new(ControlledBackend::default());
        let mut state = if has_selected_model {
            selected_chat()
        } else {
            chat()
        };
        state.set_draft("未提交选择".into());
        let mut ui = Ui::mount(
            VirtualDom::new(super::super::super::ChatHome)
                .with_root_context(BackendHandle(backend.clone()))
                .with_root_context(service.client.clone())
                .with_root_context(state),
        );
        ui.click("chat-model-trigger");
        ui.wait_for(|probe| probe.text.contains("模型服务请求失败"))
            .await;
        ui.label("重试模型查询");
        ui.wait_for(|probe| probe.text.contains("测试型号")).await;
        ui.click("chat-model-option");
        // 行内出现操作即表示状态已返回，预览仍不能提交聊天模型。
        ui.wait_for(|probe| {
            probe.attributes.values().any(|attrs| {
                attrs
                    .get("aria-label")
                    .is_some_and(|label| label == "使用此版本")
            })
        })
        .await;
        ui.label("关闭模型菜单");
        let input = ui.probe.find("aria-label", "输入消息或指令", "input");
        let attrs = &ui.probe.attributes[&input.0];
        let expected = if has_selected_model {
            MODEL
        } else {
            "尚未选择"
        };
        assert!(attrs["placeholder"].contains(expected));
        assert!(!attrs["placeholder"].contains("测试型号"));
        assert_eq!(attrs["value"], "未提交选择");
        assert!(backend.calls.borrow().is_empty());
    }
}
#[tokio::test]
async fn catalog_available_uses_version_but_does_not_fake_ready_credentials() {
    let service = CatalogServer::start("available", false, false).await;
    let backend = Rc::new(ControlledBackend::default());
    let mut ui = Ui::catalog(backend.clone(), service.client.clone());
    ui.preview_catalog().await;
    ui.label("使用此版本");
    assert!(ui.probe.text.contains("尚未接入该版本的运行后端"));
    ui.send();
    assert!(backend.calls.borrow().is_empty());
    assert!(!service
        .requests
        .lock()
        .expect("请求记录")
        .iter()
        .any(|call| call.contains("resource-status")));
}

struct PendingCatalog {
    client: CatalogClient,
    started: Arc<AtomicUsize>,
    closed: Arc<AtomicUsize>,
    task: tokio::task::JoinHandle<()>,
}
impl Drop for PendingCatalog {
    fn drop(&mut self) {
        self.task.abort();
    }
}
impl PendingCatalog {
    async fn start() -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("取消测试监听");
        let client = CatalogClient::new(
            &format!("http://{}", listener.local_addr().expect("地址")),
            Duration::from_secs(10),
        )
        .expect("客户端");
        let started = Arc::new(AtomicUsize::new(0));
        let closed = Arc::new(AtomicUsize::new(0));
        let counts = (started.clone(), closed.clone());
        let task = tokio::spawn(async move {
            // 子连接归监听任务所有，测试结束时一起销毁，避免泄漏等待中的连接。
            let mut connections = tokio::task::JoinSet::new();
            loop {
                let (mut stream, _) = listener.accept().await.expect("取消测试连接");
                let (started, closed) = counts.clone();
                connections.spawn(async move {
                    let mut request = Vec::new();
                    loop {
                        let mut bytes = [0;1024];
                        let count = stream.read(&mut bytes).await.expect("读取请求");
                        request.extend_from_slice(&bytes[..count]);
                        if count == 0 || request.windows(4).any(|bytes| bytes == b"\r\n\r\n") { break; }
                    }
                    if String::from_utf8_lossy(&request).contains("/api/models/presets") {
                        let body = json!([{"id":"test-1","name":"测试型号","preset":true,"model":"owner/Test-1","source":"modelscope","revision":"master"}]).to_string();
                        stream.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).as_bytes()).await.expect("返回预置");
                    } else {
                        stream.write_all(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n").await.expect("悬挂响应头");
                        started.fetch_add(1,Ordering::SeqCst);
                        let mut bytes = [0;1024];
                        match stream.read(&mut bytes).await {
                            Ok(0) => {},
                            Err(error) => { eprintln!("客户端取消连接：{error}"); },
                            Ok(_) => panic!("未完成响应不应复用连接"),
                        }
                        closed.fetch_add(1,Ordering::SeqCst);
                    }
                });
            }
        });
        Self {
            client,
            started,
            closed,
            task,
        }
    }
}

#[tokio::test]
async fn switching_queries_status_and_closing_picker_cancel_http_requests() {
    let service = PendingCatalog::start().await;
    let backend = Rc::new(ControlledBackend::default());
    let mut ui = Ui::catalog(backend.clone(), service.client.clone());
    ui.click("chat-model-trigger");
    ui.wait_for(|probe| probe.text.contains("测试型号")).await;
    ui.input("请输入模型名称", "q");
    ui.wait_for(|_| service.started.load(Ordering::SeqCst) == 1)
        .await;
    ui.input("请输入模型名称", "qw");
    ui.wait_for(|_| {
        service.started.load(Ordering::SeqCst) == 2 && service.closed.load(Ordering::SeqCst) == 1
    })
    .await;
    ui.label("关闭模型菜单");
    ui.wait_for(|_| service.closed.load(Ordering::SeqCst) == 2)
        .await;
    // 探针只记录变更而不维护完整 DOM 子树，使用新视图避免命中过期监听器。
    drop(ui);
    let mut ui = Ui::catalog(backend.clone(), service.client.clone());
    ui.click("chat-model-trigger");
    ui.wait_for(|probe| probe.text.contains("测试型号")).await;
    ui.click("chat-model-option");
    ui.wait_for(|_| service.started.load(Ordering::SeqCst) == 3)
        .await;
    ui.click("chat-model-option selected");
    ui.wait_for(|_| {
        service.started.load(Ordering::SeqCst) == 4 && service.closed.load(Ordering::SeqCst) == 3
    })
    .await;
    ui.label("关闭模型菜单");
    ui.wait_for(|_| service.closed.load(Ordering::SeqCst) == 4)
        .await;
    assert!(backend.calls.borrow().is_empty());
}
