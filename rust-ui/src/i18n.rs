use crate::models::{NavKey, Role};

pub const STORAGE_KEY: &str = "burncloud_selected_language";

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Locale {
    En,
    #[default]
    Zh,
    ZhTw,
    Ja,
}

impl Locale {
    pub const ALL: [Self; 4] = [Self::En, Self::Zh, Self::ZhTw, Self::Ja];

    pub const fn code(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::Zh => "zh",
            Self::ZhTw => "zh-TW",
            Self::Ja => "ja",
        }
    }

    pub const fn flag(self) -> &'static str {
        match self {
            Self::En => "🇺🇸",
            Self::Zh => "🇨🇳",
            Self::ZhTw => "🇭🇰",
            Self::Ja => "🇯🇵",
        }
    }

    pub const fn native_name(self) -> &'static str {
        match self {
            Self::En => "English",
            Self::Zh => "简体中文",
            Self::ZhTw => "繁體中文",
            Self::Ja => "日本語",
        }
    }

    pub const fn english_name(self) -> &'static str {
        match self {
            Self::En => "English",
            Self::Zh => "Simplified Chinese",
            Self::ZhTw => "Traditional Chinese",
            Self::Ja => "Japanese",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "en" => Some(Self::En),
            "zh" => Some(Self::Zh),
            "zh-TW" => Some(Self::ZhTw),
            "ja" => Some(Self::Ja),
            _ => None,
        }
    }

    pub fn from_browser_language(value: &str) -> Self {
        let value = value.to_ascii_lowercase();
        if value.contains("zh-tw")
            || value.contains("zh-hk")
            || value.contains("zh-mo")
            || value.contains("zh-hant")
        {
            Self::ZhTw
        } else if value.starts_with("zh") {
            Self::Zh
        } else if value.starts_with("ja") {
            Self::Ja
        } else if value.starts_with("en") {
            Self::En
        } else {
            Self::Zh
        }
    }

    pub const fn common(self) -> CommonText {
        match self {
            Self::En => CommonText {
                public_portal: "Public Portal",
                balance: "Prepaid Escrow Balance",
                top_up: "Top Up",
                search_buyer: "Search models, endpoints, token logs (e.g. DeepSeek, latency, api-key)...",
                search_supplier: "Search GPU nodes, cluster deployments, hardware specs...",
                search_admin: "Search tenants, models, supply nodes, autopilot decisions...",
                switch_role: "Switch Workspace Role",
                active: "Active",
                live: "LIVE",
                autopilot: "Autopilot Active",
                attention: "Needs Attention",
                sla: "99.999% SLA",
                select_language: "Select Language",
                coming_soon: "This page has not been migrated yet",
                coming_soon_desc: "Its route is reserved for the next Rust migration stage.",
                back_overview: "Back to Overview",
                open_menu: "Open navigation",
                close_menu: "Close navigation",
            },
            Self::Zh => CommonText {
                public_portal: "门户主页",
                balance: "预付托管金余额",
                top_up: "充值",
                search_buyer: "搜索模型、API 接口、Token 审计日志 (如 DeepSeek, 延迟, key)...",
                search_supplier: "搜索 GPU 算力节点、部署集群、硬件规格...",
                search_admin: "搜索租户企业、模型目录、算力供应商、Autopilot 调度决策...",
                switch_role: "切换工作台角色视图",
                active: "当前活跃",
                live: "实时",
                autopilot: "Autopilot 调度运行中",
                attention: "待处理告警",
                sla: "99.999% SLA",
                select_language: "选择语言",
                coming_soon: "此页面尚未迁移",
                coming_soon_desc: "路由已保留，将在后续 Rust 迁移阶段完成。",
                back_overview: "返回工作台概览",
                open_menu: "打开导航",
                close_menu: "关闭导航",
            },
            Self::ZhTw => CommonText {
                public_portal: "門戶主頁",
                balance: "預付託管金餘額",
                top_up: "儲值",
                search_buyer: "搜尋模型、API 端點、Token 稽核日誌...",
                search_supplier: "搜尋 GPU 算力節點、部署叢集、硬體規格...",
                search_admin: "搜尋租戶企業、模型目錄、算力供應商、Autopilot 決策...",
                switch_role: "切換工作台角色視圖",
                active: "當前活躍",
                live: "即時",
                autopilot: "Autopilot 調度運行中",
                attention: "待處理警示",
                sla: "99.999% SLA",
                select_language: "選擇語言",
                coming_soon: "此頁面尚未遷移",
                coming_soon_desc: "路由已保留，將於後續 Rust 遷移階段完成。",
                back_overview: "返回工作台總覽",
                open_menu: "開啟導覽",
                close_menu: "關閉導覽",
            },
            Self::Ja => CommonText {
                public_portal: "ポータルホーム",
                balance: "前払いエスクロー残高",
                top_up: "チャージ",
                search_buyer: "モデル、API エンドポイント、トークンログを検索...",
                search_supplier: "GPU ノード、クラスタ、ハードウェア仕様を検索...",
                search_admin: "テナント、モデルカタログ、サプライヤー、Autopilot 決定を検索...",
                switch_role: "ワークスペースの役割を切替",
                active: "アクティブ",
                live: "リアルタイム",
                autopilot: "Autopilot 稼働中",
                attention: "要対応アラート",
                sla: "99.999% SLA",
                select_language: "言語を選択",
                coming_soon: "このページはまだ移行されていません",
                coming_soon_desc: "ルートは次の Rust 移行ステージ用に予約されています。",
                back_overview: "概要へ戻る",
                open_menu: "ナビゲーションを開く",
                close_menu: "ナビゲーションを閉じる",
            },
        }
    }

    pub const fn role(self, role: Role) -> RoleText {
        match (self, role) {
            (Self::En, Role::Buyer) => RoleText {
                title: "Developer / Model Buyer",
                subtext: "API routing, playgrounds & token spend",
                flow: "Buyer Workflows: Marketplace → Playground → API Keys → Usage & Escrow",
            },
            (Self::En, Role::Supplier) => RoleText {
                title: "Compute Supplier / Host",
                subtext: "Node provisioning & 80% revenue share",
                flow: "Supplier Workflows: Node Daemon → Hardware Attestation → Auto-Scaling → Settlement",
            },
            (Self::En, Role::Admin) => RoleText {
                title: "Platform Operator / Admin",
                subtext: "Supply orchestration, economics & safety",
                flow: "Admin Workflows: Global Headroom → Autopilot Scaler → Pricing Master → Circuit Breaker",
            },
            (Self::Zh, Role::Buyer) => RoleText {
                title: "开发者 / 算力买方",
                subtext: "API 路由调度、实时操练场与 Token 消耗",
                flow: "买方全流程：模型市场 → 操练场调试 → API Key 管理 → 用量与托管金",
            },
            (Self::Zh, Role::Supplier) => RoleText {
                title: "算力服务商 / 供应商",
                subtext: "裸金属节点纳管与 80% 算力结算分成",
                flow: "供应商全流程：节点守护进程 → 硬件密码学认证 → 弹性算力上架 → 结算打款",
            },
            (Self::Zh, Role::Admin) => RoleText {
                title: "平台全局运维 / 管理员",
                subtext: "全网算力调度、经济学模型与安全熔断",
                flow: "管理员全流程：全网算力冗余 → 智能弹性伸缩 → 定价与分润 → 紧急熔断控制",
            },
            (Self::ZhTw, Role::Buyer) => RoleText {
                title: "開發者 / 算力買方",
                subtext: "API 路由調度、即時操練場與 Token 消耗",
                flow: "買方全流程：模型市場 → 操練場調試 → API Key 管理 → 用量與託管金",
            },
            (Self::ZhTw, Role::Supplier) => RoleText {
                title: "算力服務商 / 供應商",
                subtext: "裸金屬節點納管與 80% 算力結算分成",
                flow: "供應商全流程：節點守護程序 → 硬體密碼學認證 → 彈性算力上架 → 結算打款",
            },
            (Self::ZhTw, Role::Admin) => RoleText {
                title: "平台全域運維 / 管理員",
                subtext: "全網算力調度、經濟學模型與安全熔斷",
                flow: "管理員全流程：全網算力冗餘 → 智慧彈性伸縮 → 定價與分潤 → 緊急熔斷控制",
            },
            (Self::Ja, Role::Buyer) => RoleText {
                title: "開発者 / バイヤー",
                subtext: "API ルーティング、Playground、トークン消費",
                flow: "バイヤーフロー：モデル一覧 → Playground → API キー → 利用量とエスクロー",
            },
            (Self::Ja, Role::Supplier) => RoleText {
                title: "コンピュート提供者 / ホスト",
                subtext: "ベアメタル GPU ノード提供と 80% レベニューシェア",
                flow: "サプライヤーフロー：デーモン接続 → 暗号ハードウェア認証 → スケーリング → 決済",
            },
            (Self::Ja, Role::Admin) => RoleText {
                title: "プラットフォーム運用 / 管理者",
                subtext: "グローバル供給調整、経済モデル、緊急サーキットブレーカー",
                flow: "管理者フロー：グローバル供給 → Autopilot スケーラー → 価格統括 → 緊急遮断",
            },
        }
    }

    pub const fn nav(self, key: NavKey) -> &'static str {
        match self {
            Self::En => match key {
                NavKey::Overview => "Overview",
                NavKey::Playground => "Playground",
                NavKey::Marketplace => "Model Marketplace",
                NavKey::ApiKeys => "API Keys",
                NavKey::Usage => "Usage Analytics",
                NavKey::Billing => "Billing & Escrow",
                NavKey::Logs => "Request Logs",
                NavKey::Resources => "GPU Resources",
                NavKey::Deployments => "Autopilot Deployments",
                NavKey::Earnings => "Revenue & Payouts",
                NavKey::Settlements => "Settlement Batches",
                NavKey::Reliability => "SLA & Reliability",
                NavKey::Settings => "Settings",
                NavKey::Supply => "Supply Fleet",
                NavKey::Capacity => "Capacity & Autoscale",
                NavKey::Demand => "Token Demand",
                NavKey::Models => "Model Catalog",
                NavKey::Revenue => "Platform Revenue",
                NavKey::Suppliers => "Supplier Accounts",
                NavKey::Customers => "Enterprise Customers",
                NavKey::Operations => "Emergency Controls",
            },
            Self::Zh => match key {
                NavKey::Overview => "工作台概览",
                NavKey::Playground => "实时操练场",
                NavKey::Marketplace => "模型市场",
                NavKey::ApiKeys => "API 密钥管理",
                NavKey::Usage => "用量与消耗分析",
                NavKey::Billing => "财务与充值托管",
                NavKey::Logs => "请求调用日志",
                NavKey::Resources => "GPU 算力资源",
                NavKey::Deployments => "Autopilot 部署",
                NavKey::Earnings => "算力收益明细",
                NavKey::Settlements => "结算打款批次",
                NavKey::Reliability => "SLA 可用性审计",
                NavKey::Settings => "节点与环境配置",
                NavKey::Supply => "全局算力池",
                NavKey::Capacity => "容量与弹性伸缩",
                NavKey::Demand => "全网 Token 需求",
                NavKey::Models => "模型定价目录",
                NavKey::Revenue => "平台利润与分成",
                NavKey::Suppliers => "供应商名录档案",
                NavKey::Customers => "企业客户账户",
                NavKey::Operations => "应急安全熔断",
            },
            Self::ZhTw => match key {
                NavKey::Overview => "工作台總覽",
                NavKey::Playground => "即時操練場",
                NavKey::Marketplace => "模型市場",
                NavKey::ApiKeys => "API 金鑰管理",
                NavKey::Usage => "用量與消耗分析",
                NavKey::Billing => "財務與儲值託管",
                NavKey::Logs => "請求調用日誌",
                NavKey::Resources => "GPU 算力資源",
                NavKey::Deployments => "Autopilot 部署",
                NavKey::Earnings => "算力收益明細",
                NavKey::Settlements => "結算打款批次",
                NavKey::Reliability => "SLA 可用性稽核",
                NavKey::Settings => "節點與環境配置",
                NavKey::Supply => "全域算力池",
                NavKey::Capacity => "容量與彈性伸縮",
                NavKey::Demand => "全網 Token 需求",
                NavKey::Models => "模型定價目錄",
                NavKey::Revenue => "平台利潤與分成",
                NavKey::Suppliers => "供應商名錄檔案",
                NavKey::Customers => "企業客戶帳戶",
                NavKey::Operations => "緊急安全熔斷",
            },
            Self::Ja => match key {
                NavKey::Overview => "概要",
                NavKey::Playground => "Playground",
                NavKey::Marketplace => "モデルマーケット",
                NavKey::ApiKeys => "API キー管理",
                NavKey::Usage => "利用量分析",
                NavKey::Billing => "請求とエスクロー",
                NavKey::Logs => "リクエストログ",
                NavKey::Resources => "GPU リソース",
                NavKey::Deployments => "Autopilot デプロイ",
                NavKey::Earnings => "収益と支払い",
                NavKey::Settlements => "決済バッチ",
                NavKey::Reliability => "SLA と信頼性",
                NavKey::Settings => "設定",
                NavKey::Supply => "供給フリート",
                NavKey::Capacity => "キャパシティと拡張",
                NavKey::Demand => "トークン需要",
                NavKey::Models => "モデルカタログ",
                NavKey::Revenue => "プラットフォーム収益",
                NavKey::Suppliers => "サプライヤー一覧",
                NavKey::Customers => "エンタープライズ顧客",
                NavKey::Operations => "緊急制御",
            },
        }
    }

    pub const fn overview(self) -> OverviewText {
        match self {
            Self::En => OverviewText {
                title: "Developer Overview",
                subtitle: "Monitor token expenditure, balance escrow health, active model routes, and real-time P95 latencies.",
                conclusion_healthy: "All active model routes operating normally with cryptographic hardware verification.",
                conclusion_warning: "Prepaid escrow balance is low. Auto-recharge recommended to prevent throttling.",
                open_playground: "Open Live Playground",
                browse_marketplace: "Explore Marketplace",
                today_spend: "Today Spend",
                today_spend_sub: "Prepaid escrow burn",
                balance: "Prepaid Balance",
                balance_sub: "Est. 14 days remaining",
                availability: "API Availability",
                availability_sub: "Across all active routes",
                tokens: "Tokens Today",
                tokens_sub: "620K prompt • 1.22M completion",
                attention_title: "Prepaid balance below $20.00",
                attention_desc: "At your current burn rate (~$14.28/day), your API requests may experience rate limits within 24 hours.",
                attention_top_up: "Top Up Balance",
                models_title: "Models in Use",
                models_desc: "Active model API routes serving your application requests.",
                explore_models: "Explore all models",
                col_model: "Model",
                col_tier: "Selected Tier",
                col_tokens: "Today Tokens",
                col_latency: "p95 Latency",
                col_cost: "Today Cost",
                col_status: "Service Status",
                col_action: "Action",
                test: "Test in Playground →",
                activity_title: "Recent Activity",
                activity_desc: "Key account events, recharge confirmations, and routing notices.",
                view_logs: "View Full Request Logs",
            },
            Self::Zh => OverviewText {
                title: "开发者工作台概览",
                subtitle: "实时监控 Token 支出、预付托管金余额、在线模型路由及全球 P95 响应延迟。",
                conclusion_healthy: "所有活跃模型路由均正常运行，已通过硬件密码学真实性校验。",
                conclusion_warning: "预付托管金余额偏低，建议配置自动充值以避免 API 被限流。",
                open_playground: "体验实时操练场",
                browse_marketplace: "探索模型市场",
                today_spend: "今日 Token 消耗",
                today_spend_sub: "从预付托管金中扣除",
                balance: "预付托管金余额",
                balance_sub: "预计可支撑 14 天用量",
                availability: "API 服务可用率",
                availability_sub: "覆盖全量活跃路由",
                tokens: "今日生成 Token 数",
                tokens_sub: "620K 输入 • 1.22M 输出",
                attention_title: "预付托管金余额低于 $20.00",
                attention_desc: "按您当前的消耗速率 (约 $14.28/天)，API 请求可能在 24 小时内受到频次限制。",
                attention_top_up: "立即充值托管金",
                models_title: "正在调用的模型路由",
                models_desc: "当前为您生产应用提供实时流量调度的主力模型。",
                explore_models: "查看全量模型",
                col_model: "模型名称",
                col_tier: "优化等级",
                col_tokens: "今日 Token 数",
                col_latency: "p95 延迟",
                col_cost: "今日费用",
                col_status: "服务状态",
                col_action: "操作",
                test: "在操练场调试 →",
                activity_title: "最近动态",
                activity_desc: "账户核心事件、充值到账记录与自动容灾通知。",
                view_logs: "查看完整调用日志",
            },
            Self::ZhTw => OverviewText {
                title: "開發者工作台總覽",
                subtitle: "即時監控 Token 支出、預付託管金餘額、在線模型路由及全球 P95 回應延遲。",
                conclusion_healthy: "所有活躍模型路由均正常運行，已通過硬體密碼學真實性校驗。",
                conclusion_warning: "預付託管金餘額偏低，建議配置自動儲值以避免 API 被限流。",
                open_playground: "體驗即時操練場",
                browse_marketplace: "探索模型市場",
                today_spend: "今日 Token 消耗",
                today_spend_sub: "從預付託管金中扣除",
                balance: "預付託管金餘額",
                balance_sub: "預計可支撐 14 天用量",
                availability: "API 服務可用率",
                availability_sub: "覆蓋全量活躍路由",
                tokens: "今日生成 Token 數",
                tokens_sub: "620K 輸入 • 1.22M 輸出",
                attention_title: "預付託管金餘額低於 $20.00",
                attention_desc: "按您當前的消耗速率 (約 $14.28/天)，API 請求可能在 24 小時內受到頻次限制。",
                attention_top_up: "立即儲值託管金",
                models_title: "正在調用的模型路由",
                models_desc: "當前為您生產應用提供即時流量調度的主力模型。",
                explore_models: "查看全量模型",
                col_model: "模型名稱",
                col_tier: "優化等級",
                col_tokens: "今日 Token 數",
                col_latency: "p95 延遲",
                col_cost: "今日費用",
                col_status: "服務狀態",
                col_action: "操作",
                test: "在操練場調試 →",
                activity_title: "最近動態",
                activity_desc: "帳戶核心事件、儲值到帳記錄與自動容災通知。",
                view_logs: "查看完整調用日誌",
            },
            Self::Ja => OverviewText {
                title: "開発者ダッシュボード概要",
                subtitle: "トークン支出、前払いエスクロー残高、稼働中のモデルルート、P95 遅延を監視。",
                conclusion_healthy: "すべてのモデルルートは暗号ハードウェア検証付きで正常に稼働しています。",
                conclusion_warning: "前払いエスクロー残高が少なくなっています。制限を防ぐため自動チャージを推奨します。",
                open_playground: "Playground を開く",
                browse_marketplace: "マーケットプレイスを探索",
                today_spend: "本日のトークン支出",
                today_spend_sub: "前払いエスクローから差し引き",
                balance: "前払いエスクロー残高",
                balance_sub: "約 14 日間利用可能",
                availability: "API サービス稼働率",
                availability_sub: "全稼働ルート対象",
                tokens: "本日生成トークン数",
                tokens_sub: "620K 入力 • 1.22M 出力",
                attention_title: "エスクロー残高が $20.00 未満です",
                attention_desc: "現在の消費ペース (約 $14.28/日) では、24時間以内に API レート制限が発生する可能性があります。",
                attention_top_up: "残高をチャージ",
                models_title: "利用中のモデルルート",
                models_desc: "現在アプリケーションのリクエストを処理しているアクティブなモデル。",
                explore_models: "全モデルを見る",
                col_model: "モデル",
                col_tier: "選択ティア",
                col_tokens: "本日トークン数",
                col_latency: "p95 遅延",
                col_cost: "本日費用",
                col_status: "ステータス",
                col_action: "操作",
                test: "Playground でテスト →",
                activity_title: "最近のアクティビティ",
                activity_desc: "アカウントイベント、チャージ完了、自動フェイルオーバー通知。",
                view_logs: "リクエストログをすべて確認",
            },
        }
    }
}

#[derive(Clone, Copy)]
pub struct CommonText {
    pub public_portal: &'static str,
    pub balance: &'static str,
    pub top_up: &'static str,
    pub search_buyer: &'static str,
    pub search_supplier: &'static str,
    pub search_admin: &'static str,
    pub switch_role: &'static str,
    pub active: &'static str,
    pub live: &'static str,
    pub autopilot: &'static str,
    pub attention: &'static str,
    pub sla: &'static str,
    pub select_language: &'static str,
    pub coming_soon: &'static str,
    pub coming_soon_desc: &'static str,
    pub back_overview: &'static str,
    pub open_menu: &'static str,
    pub close_menu: &'static str,
}

#[derive(Clone, Copy)]
pub struct RoleText {
    pub title: &'static str,
    pub subtext: &'static str,
    pub flow: &'static str,
}

#[derive(Clone, Copy)]
pub struct OverviewText {
    pub title: &'static str,
    pub subtitle: &'static str,
    pub conclusion_healthy: &'static str,
    pub conclusion_warning: &'static str,
    pub open_playground: &'static str,
    pub browse_marketplace: &'static str,
    pub today_spend: &'static str,
    pub today_spend_sub: &'static str,
    pub balance: &'static str,
    pub balance_sub: &'static str,
    pub availability: &'static str,
    pub availability_sub: &'static str,
    pub tokens: &'static str,
    pub tokens_sub: &'static str,
    pub attention_title: &'static str,
    pub attention_desc: &'static str,
    pub attention_top_up: &'static str,
    pub models_title: &'static str,
    pub models_desc: &'static str,
    pub explore_models: &'static str,
    pub col_model: &'static str,
    pub col_tier: &'static str,
    pub col_tokens: &'static str,
    pub col_latency: &'static str,
    pub col_cost: &'static str,
    pub col_status: &'static str,
    pub col_action: &'static str,
    pub test: &'static str,
    pub activity_title: &'static str,
    pub activity_desc: &'static str,
    pub view_logs: &'static str,
}

pub fn initial_locale() -> Locale {
    let Some(window) = web_sys::window() else {
        return Locale::Zh;
    };
    if let Ok(Some(storage)) = window.local_storage()
        && let Ok(Some(saved)) = storage.get_item(STORAGE_KEY)
        && let Some(locale) = Locale::parse(&saved)
    {
        return locale;
    }
    Locale::from_browser_language(&window.navigator().language().unwrap_or_default())
}

pub fn persist_locale(locale: Locale) {
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            let _ = storage.set_item(STORAGE_KEY, locale.code());
        }
        if let Some(element) = window
            .document()
            .and_then(|document| document.document_element())
        {
            let _ = element.set_attribute("lang", locale.code());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn browser_languages_map_to_supported_locales() {
        assert_eq!(Locale::from_browser_language("zh-CN"), Locale::Zh);
        assert_eq!(Locale::from_browser_language("zh-Hant-HK"), Locale::ZhTw);
        assert_eq!(Locale::from_browser_language("ja-JP"), Locale::Ja);
        assert_eq!(Locale::from_browser_language("en-US"), Locale::En);
        assert_eq!(Locale::from_browser_language("fr-FR"), Locale::Zh);
    }

    #[test]
    fn storage_codes_round_trip() {
        for locale in Locale::ALL {
            assert_eq!(Locale::parse(locale.code()), Some(locale));
        }
        assert_eq!(Locale::parse("zh-CN"), None);
    }
}
