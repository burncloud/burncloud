(() => {
  const STORAGE_KEY = "burncloud_selected_language";
  const SUPPORTED = ["en", "zh", "zh-TW", "ja"];
  const LANGUAGE_META = {
    en: { lang: "en", flag: "US", native: "English", short: "EN" },
    zh: { lang: "zh-CN", flag: "CN", native: "简体中文", short: "中" },
    "zh-TW": { lang: "zh-TW", flag: "HK", native: "繁體中文", short: "繁" },
    ja: { lang: "ja", flag: "JP", native: "日本語", short: "日" },
  };

  const entries = [
    ["选择语言", "Select language", "選擇語言", "言語を選択"],
    ["打开导航", "Open navigation", "開啟導覽", "ナビゲーションを開く"],
    ["关闭导航", "Close navigation", "關閉導覽", "ナビゲーションを閉じる"],
    ["API 采购方", "API Buyer", "API 採購方", "API バイヤー"],
    ["算力供应方", "Compute Supplier", "算力供應方", "コンピュートサプライヤー"],
    ["平台管理员", "Platform Admin", "平台管理員", "プラットフォーム管理者"],
    ["模型、调用与账单", "Models, requests, and billing", "模型、調用與帳單", "モデル、リクエスト、請求"],
    ["资源、部署与收益", "Resources, deployments, and earnings", "資源、部署與收益", "リソース、デプロイ、収益"],
    ["平台治理与运营", "Platform governance and operations", "平台治理與營運", "プラットフォーム管理と運用"],
    ["需要 admin 权限", "Admin permission required", "需要 admin 權限", "管理者権限が必要"],
    ["当前账户没有 admin 权限", "This account does not have admin permission", "目前帳戶沒有 admin 權限", "このアカウントには管理者権限がありません"],
    ["切换工作区角色", "Switch workspace role", "切換工作區角色", "ワークスペースの役割を切り替え"],
    ["切换角色", "Switch role", "切換角色", "役割を切り替え"],
    ["模型 → API → 用量 → 账单", "Models → API → Usage → Billing", "模型 → API → 用量 → 帳單", "モデル → API → 使用量 → 請求"],
    ["Buyer 导航", "Buyer navigation", "Buyer 導覽", "バイヤーナビゲーション"],
    ["概览", "Overview", "總覽", "概要"],
    ["操练场", "Playground", "操練場", "Playground"],
    ["模型市场", "Model Marketplace", "模型市場", "モデルマーケット"],
    ["API 密钥", "API Keys", "API 金鑰", "API キー"],
    ["用量分析", "Usage Analytics", "用量分析", "使用量分析"],
    ["账单与余额", "Billing & Balance", "帳單與餘額", "請求と残高"],
    ["调用日志", "Request Logs", "調用日誌", "リクエストログ"],
    ["账户余额", "Account Balance", "帳戶餘額", "アカウント残高"],
    ["充值", "Top Up", "儲值", "チャージ"],
    ["账户数据已连接", "Account data connected", "帳戶資料已連接", "アカウントデータ接続済み"],
    ["搜索页面、模型或功能…", "Search pages, models, or features...", "搜尋頁面、模型或功能…", "ページ、モデル、機能を検索..."],
    ["Autopilot 已启用", "Autopilot Active", "Autopilot 已啟用", "Autopilot 有効"],
    ["需要处理的通知", "Notifications requiring attention", "需要處理的通知", "対応が必要な通知"],
    ["通知", "Notifications", "通知", "通知"],
    ["退出登录", "Sign Out", "登出", "ログアウト"],
    ["平台管理员 · 无权限", "Platform Admin · No permission", "平台管理員 · 無權限", "プラットフォーム管理者 · 権限なし"],
    ["BurnCloud 账户", "BurnCloud Account", "BurnCloud 帳戶", "BurnCloud アカウント"],

    ["连接模型、用量与账单的统一工作区", "One workspace for models, usage, and billing", "連接模型、用量與帳單的統一工作區", "モデル、使用量、請求を一つのワークスペースで"],
    ["登录后将从 BurnCloud 数据库读取账户、模型目录、账单和 API 密钥状态。", "Sign in to load account, model catalog, billing, and API key status from the BurnCloud database.", "登入後將從 BurnCloud 資料庫讀取帳戶、模型目錄、帳單和 API 金鑰狀態。", "ログインすると、BurnCloud データベースからアカウント、モデルカタログ、請求、API キーの状態を読み込みます。"],
    ["管理面 JWT 与推理 API 密钥严格隔离", "Management JWTs are strictly isolated from inference API keys", "管理面 JWT 與推理 API 金鑰嚴格隔離", "管理 JWT と推論 API キーを厳密に分離"],
    ["真实路由操练场不会向浏览器暴露密钥", "The live routing playground never exposes keys to the browser", "真實路由操練場不會向瀏覽器暴露金鑰", "実ルーティング Playground はブラウザにキーを公開しません"],
    ["角色权限每次从后端账户记录确认", "Role permissions are verified against backend account records", "角色權限每次從後端帳戶記錄確認", "役割権限は毎回バックエンドのアカウント情報で確認"],
    ["登录 BurnCloud", "Sign in to BurnCloud", "登入 BurnCloud", "BurnCloud にログイン"],
    ["使用 BurnCloud 管理账户继续。", "Continue with your BurnCloud management account.", "使用 BurnCloud 管理帳戶繼續。", "BurnCloud 管理アカウントで続行します。"],
    ["用户名", "Username", "使用者名稱", "ユーザー名"],
    ["密码", "Password", "密碼", "パスワード"],
    ["登录工作区", "Sign in to Workspace", "登入工作區", "ワークスペースにログイン"],
    ["会话保存在 HttpOnly Cookie 中，浏览器脚本无法读取。", "Your session is stored in an HttpOnly cookie that browser scripts cannot read.", "工作階段儲存在 HttpOnly Cookie 中，瀏覽器腳本無法讀取。", "セッションはブラウザスクリプトから読み取れない HttpOnly Cookie に保存されます。"],
    ["无法连接 BurnCloud 服务，请确认后端已启动后重试。", "Unable to connect to BurnCloud. Confirm the backend is running and try again.", "無法連接 BurnCloud 服務，請確認後端已啟動後重試。", "BurnCloud に接続できません。バックエンドの起動を確認して再試行してください。"],
    ["用户名或密码不正确，请重新输入。", "Incorrect username or password. Please try again.", "使用者名稱或密碼不正確，請重新輸入。", "ユーザー名またはパスワードが正しくありません。"],

    ["查看今日用量、账户余额与 API 服务状态。", "Review today's usage, account balance, and API service status.", "查看今日用量、帳戶餘額與 API 服務狀態。", "本日の使用量、アカウント残高、API サービス状態を確認します。"],
    ["打开操练场", "Open Playground", "開啟操練場", "Playground を開く"],
    ["浏览模型市场", "Browse Model Marketplace", "瀏覽模型市場", "モデルマーケットを見る"],
    ["部分实时数据暂不可用", "Some live data is temporarily unavailable", "部分即時資料暫不可用", "一部のライブデータを現在利用できません"],
    ["账户当前不可用", "Account currently unavailable", "帳戶目前不可用", "アカウントは現在利用できません"],
    ["账户状态已被后端标记为停用，请联系管理员恢复后再发送请求。", "The backend has disabled this account. Contact an administrator before sending requests.", "帳戶狀態已被後端標記為停用，請聯絡管理員恢復後再發送請求。", "バックエンドでこのアカウントは無効化されています。管理者に連絡してからリクエストしてください。"],
    ["余额偏低，需要及时充值", "Low balance - top up soon", "餘額偏低，需要及時儲值", "残高が少なくなっています"],
    ["当前余额可能影响后续推理请求，请在服务暂停前补充余额。", "Your balance may affect future inference requests. Add funds before service is interrupted.", "目前餘額可能影響後續推理請求，請在服務暫停前補充餘額。", "現在の残高では今後の推論リクエストに影響する可能性があります。サービス停止前にチャージしてください。"],
    ["工作区还需要完成配置", "Workspace setup is incomplete", "工作區還需要完成設定", "ワークスペースの設定が未完了です"],
    ["API 服务与账户状态正常", "API service and account are healthy", "API 服務與帳戶狀態正常", "API サービスとアカウントは正常です"],
    ["今日核心指标", "Today's key metrics", "今日核心指標", "本日の主要指標"],
    ["今日费用", "Today Spend", "今日費用", "本日の費用"],
    ["从当前账户的结算日志汇总", "Summed from this account's settlement logs", "從目前帳戶的結算日誌彙總", "このアカウントの決済ログから集計"],
    ["数据库账户钱包实时余额", "Live database wallet balance", "資料庫帳戶錢包即時餘額", "データベースのリアルタイム残高"],
    ["API 可用性", "API Availability", "API 可用性", "API 可用性"],
    ["在线", "Online", "在線", "オンライン"],
    ["待配置", "Setup required", "待設定", "設定が必要"],
    ["今日 Token", "Tokens Today", "今日 Token", "本日のトークン"],
    ["余额不足可能导致服务中断", "Low balance may interrupt service", "餘額不足可能導致服務中斷", "残高不足によりサービスが中断する可能性があります"],
    ["立即充值", "Top Up Now", "立即儲值", "今すぐチャージ"],
    ["实时数据连接不完整", "Live data connection is incomplete", "即時資料連接不完整", "ライブデータ接続が不完全です"],
    ["重试", "Retry", "重試", "再試行"],
    ["开始使用", "Get Started", "開始使用", "はじめに"],
    ["尚无已结算的模型调用", "No settled model requests yet", "尚無已結算的模型調用", "決済済みのモデルリクエストはまだありません"],
    ["选择数据库中已配置的模型，并使用有效 API 密钥执行首个真实请求。", "Select a configured database model and use an active API key for your first live request.", "選擇資料庫中已設定的模型，並使用有效 API 金鑰執行首個真實請求。", "設定済みモデルと有効な API キーを選び、最初の実リクエストを実行します。"],
    ["确认 API 密钥", "Confirm API Key", "確認 API 金鑰", "API キーを確認"],
    ["选择模型", "Select Model", "選擇模型", "モデルを選択"],
    ["运行请求", "Run Request", "執行請求", "リクエストを実行"],
    ["操练场通过真实 BurnCloud 路由执行。", "The Playground runs through the live BurnCloud router.", "操練場透過真實 BurnCloud 路由執行。", "Playground は実際の BurnCloud ルーターを使用します。"],
    ["开始测试", "Start Testing", "開始測試", "テストを開始"],
    ["当前服务", "CURRENT SERVICES", "目前服務", "現在のサービス"],
    ["正在使用的模型", "Models in Use", "正在使用的模型", "使用中のモデル"],
    ["今日真实结算用量与当前模型目录可用状态。", "Today's settled usage and current model catalog availability.", "今日真實結算用量與目前模型目錄可用狀態。", "本日の決済済み使用量とモデルカタログの可用性。"],
    ["查看全部模型", "View All Models", "查看全部模型", "すべてのモデルを見る"],
    ["模型", "Model", "模型", "モデル"],
    ["请求数", "Requests", "請求數", "リクエスト数"],
    ["P95 延迟", "P95 Latency", "P95 延遲", "P95 遅延"],
    ["服务状态", "Service Status", "服務狀態", "サービス状態"],
    ["测试", "Test", "測試", "テスト"],
    ["可用", "Available", "可用", "利用可能"],
    ["当前未暴露", "Not currently exposed", "目前未暴露", "現在利用不可"],
    ["待采样", "Awaiting samples", "待取樣", "サンプル待ち"],
    ["账户事件", "ACCOUNT EVENTS", "帳戶事件", "アカウントイベント"],
    ["最近活动", "Recent Activity", "最近活動", "最近のアクティビティ"],
    ["来自充值与 API 密钥记录的最新账户变化。", "Latest account changes from top-ups and API key records.", "來自儲值與 API 金鑰記錄的最新帳戶變化。", "チャージと API キー記録に基づく最新のアカウント変更。"],
    ["查看调用日志", "View Request Logs", "查看調用日誌", "リクエストログを見る"],
    ["充值记录已写入账户", "Top-up recorded on the account", "儲值記錄已寫入帳戶", "チャージをアカウントに記録しました"],
    ["时间未知", "Time unavailable", "時間未知", "時刻不明"],
    ["已创建", "Created", "已建立", "作成済み"],
    ["数据面密钥未向页面暴露", "Data-plane key is not exposed to the page", "資料面金鑰未向頁面暴露", "データプレーンキーはページに公開されません"],
    ["尚无账户活动", "No account activity yet", "尚無帳戶活動", "アカウントアクティビティはまだありません"],
    ["充值或创建 API 密钥后，数据库记录会显示在这里。", "Database records appear here after a top-up or API key creation.", "儲值或建立 API 金鑰後，資料庫記錄會顯示在這裡。", "チャージまたは API キー作成後、データベース記録がここに表示されます。"],

    ["交互式推理操练场", "Interactive Inference Playground", "互動式推理操練場", "インタラクティブ推論 Playground"],
    ["使用当前账户的真实 API 密钥引用验证模型、路由与上游响应，密钥只保留在后端。", "Validate models, routing, and upstream responses using this account's live API key reference. Keys remain on the backend.", "使用目前帳戶的真實 API 金鑰引用驗證模型、路由與上游回應，金鑰只保留在後端。", "このアカウントの実 API キー参照でモデル、ルーティング、上流応答を検証します。キーはバックエンドだけに保持されます。"],
    ["操练场数据连接不完整", "Playground data connection is incomplete", "操練場資料連接不完整", "Playground のデータ接続が不完全です"],
    ["尚无可用模型", "No models available", "尚無可用模型", "利用可能なモデルがありません"],
    ["请先在 BurnCloud 后端启用至少一个包含模型的渠道。", "Enable at least one backend channel containing a model.", "請先在 BurnCloud 後端啟用至少一個包含模型的渠道。", "BurnCloud バックエンドでモデルを含むチャネルを1つ以上有効にしてください。"],
    ["需要有效的 API 密钥", "An active API key is required", "需要有效的 API 金鑰", "有効な API キーが必要です"],
    ["请先创建或启用当前账户的 API 密钥，推理密钥不会暴露给浏览器。", "Create or activate an API key for this account. The inference key will not be exposed to the browser.", "請先建立或啟用目前帳戶的 API 金鑰，推理金鑰不會暴露給瀏覽器。", "このアカウントの API キーを作成または有効化してください。推論キーはブラウザに公開されません。"],
    ["真实推理服务已就绪", "Live inference service is ready", "真實推理服務已就緒", "実推論サービスの準備ができました"],
    ["未选择模型", "No model selected", "未選擇模型", "モデル未選択"],
    ["数据库中没有已启用的模型", "No enabled models in the database", "資料庫中沒有已啟用的模型", "データベースに有効なモデルがありません"],
    ["模型和调用参数", "Model and request parameters", "模型和調用參數", "モデルとリクエストパラメータ"],
    ["模型与路由", "Model & Routing", "模型與路由", "モデルとルーティング"],
    ["选择数据库模型", "Select Database Model", "選擇資料庫模型", "データベースモデルを選択"],
    ["算力优化路由等级", "Optimization Routing Tier", "算力優化路由等級", "最適化ルーティングティア"],
    ["路由等级", "Routing tier", "路由等級", "ルーティングティア"],
    ["经济级", "Economy", "經濟級", "エコノミー"],
    ["标准级", "Standard", "標準級", "スタンダード"],
    ["性能级", "Performance", "效能級", "パフォーマンス"],
    ["路由等级会写入可复制代码示例；真实路由仍遵循后端当前策略。", "The tier is included in copyable code samples; live routing still follows the backend policy.", "路由等級會寫入可複製程式碼範例；真實路由仍遵循後端目前策略。", "ティアはコピー可能なコード例に反映されます。実ルーティングはバックエンドの現在のポリシーに従います。"],
    ["API 调用代码", "API Request Code", "API 調用程式碼", "API リクエストコード"],
    ["代码语言", "Code language", "程式碼語言", "コード言語"],
    ["复制代码", "Copy Code", "複製程式碼", "コードをコピー"],
    ["真实路由测试", "Live Routing Test", "真實路由測試", "実ルーティングテスト"],
    ["系统提示词", "System Prompt", "系統提示詞", "システムプロンプト"],
    ["可选", "Optional", "可選", "任意"],
    ["用户提示词", "User Prompt", "使用者提示詞", "ユーザープロンプト"],
    ["管理 JWT 与推理密钥隔离 · 密钥服务端托管", "Management JWT isolated from inference key · Key held server-side", "管理 JWT 與推理金鑰隔離 · 金鑰由伺服器端託管", "管理 JWT と推論キーを分離 · キーはサーバー側で管理"],
    ["清空", "Clear", "清空", "クリア"],
    ["运行真实推理", "Run Live Inference", "執行真實推理", "実推論を実行"],
    ["等待配置", "Awaiting Setup", "等待設定", "設定待ち"],
    ["正在通过真实路由请求", "Requesting through live routing", "正在透過真實路由請求", "実ルーティングでリクエスト中"],
    ["点击“运行真实推理”后执行数据库中配置的模型路由。", "Click “Run Live Inference” to execute the database-configured model route.", "點擊「執行真實推理」後執行資料庫中設定的模型路由。", "「実推論を実行」をクリックしてデータベース設定済みルートを実行します。"],
    ["首包耗时", "Time to First Byte", "首包耗時", "初回応答時間"],
    ["总耗时", "Total Time", "總耗時", "合計時間"],
    ["费用估算", "Estimated Cost", "費用估算", "推定コスト"],
    ["真实路由证据", "Live Routing Evidence", "真實路由證據", "実ルーティング証跡"],
    ["请选择模型、确认有效 API 密钥并输入提示词。", "Select a model, confirm an active API key, and enter a prompt.", "請選擇模型、確認有效 API 金鑰並輸入提示詞。", "モデルを選択し、有効な API キーを確認してプロンプトを入力してください。"],
    ["真实路由执行中…", "Running live route...", "真實路由執行中…", "実ルートを実行中..."],
    ["正在等待 BurnCloud 数据面返回上游响应…", "Waiting for the BurnCloud data plane to return the upstream response...", "正在等待 BurnCloud 資料面返回上游回應…", "BurnCloud データプレーンからの上流応答を待っています..."],
    ["非流式", "Non-streaming", "非串流", "非ストリーミング"],
    ["已复制", "Copied", "已複製", "コピー済み"],
    ["代码已复制到剪贴板", "Code copied to clipboard", "程式碼已複製到剪貼簿", "コードをクリップボードにコピーしました"],
    ["浏览器未允许访问剪贴板", "Clipboard access was not permitted", "瀏覽器未允許存取剪貼簿", "クリップボードへのアクセスが許可されていません"],

    ["模型市场与基准评测", "Model Marketplace & Benchmarks", "模型市場與基準評測", "モデルマーケットとベンチマーク"],
    ["发现、比较并接入 BurnCloud 数据库中实际启用的模型服务。", "Discover, compare, and connect to model services enabled in the BurnCloud database.", "發現、比較並接入 BurnCloud 資料庫中實際啟用的模型服務。", "BurnCloud データベースで有効なモデルサービスを検索、比較、接続します。"],
    ["模型目录连接不完整", "Model catalog connection is incomplete", "模型目錄連接不完整", "モデルカタログ接続が不完全です"],
    ["尚无可购买的模型服务", "No model services available", "尚無可購買的模型服務", "利用可能なモデルサービスがありません"],
    ["数据库中没有已启用且声明模型的渠道。", "No enabled database channels declare a model.", "資料庫中沒有已啟用且聲明模型的渠道。", "モデルを宣言した有効なデータベースチャネルがありません。"],
    ["模型服务目录已与数据库同步", "Model catalog synchronized with the database", "模型服務目錄已與資料庫同步", "モデルカタログをデータベースと同期しました"],
    ["筛选模型", "Filter models", "篩選模型", "モデルを絞り込み"],
    ["模型类别", "Model categories", "模型類別", "モデルカテゴリ"],
    ["全量模型", "All Models", "全部模型", "すべてのモデル"],
    ["通用大模型", "General LLM", "通用大模型", "汎用 LLM"],
    ["深度推理与数学", "Reasoning & Math", "深度推理與數學", "推論・数学"],
    ["代码与智能体", "Coding & Agents", "程式碼與智慧體", "コーディング・エージェント"],
    ["多模态", "Multimodal", "多模態", "マルチモーダル"],
    ["搜索模型", "Search models", "搜尋模型", "モデルを検索"],
    ["搜索模型、厂商或能力...", "Search models, providers, or capabilities...", "搜尋模型、廠商或能力...", "モデル、プロバイダー、機能を検索..."],
    ["价格按每 100 万 Token 计费", "Prices are per 1M tokens", "價格按每 100 萬 Token 計費", "価格は100万トークン単位"],
    ["模型目录", "Model catalog", "模型目錄", "モデルカタログ"],
    ["没有匹配的模型", "No matching models", "沒有匹配的模型", "一致するモデルがありません"],
    ["尝试更换类别、缩短搜索关键词，或在后端启用模型渠道。", "Try another category, shorten the search, or enable a backend model channel.", "嘗試更換類別、縮短搜尋關鍵詞，或在後端啟用模型渠道。", "カテゴリを変更するか検索語を短くするか、バックエンドのモデルチャネルを有効にしてください。"],
    ["清除筛选", "Clear Filters", "清除篩選", "フィルターをクリア"],
    ["关闭模型详情", "Close model details", "關閉模型詳情", "モデル詳細を閉じる"],
    ["关闭面板", "Close panel", "關閉面板", "パネルを閉じる"],
    ["输入 / 输出价格", "Input / Output Price", "輸入 / 輸出價格", "入力 / 出力価格"],
    ["每 100 万 Token", "Per 1M tokens", "每 100 萬 Token", "100万トークンあたり"],
    ["查看参数规格", "View Specifications", "查看參數規格", "仕様を確認"],
    ["在操练场体验", "Test in Playground", "在操練場體驗", "Playground で試す"],
    ["文本", "Text", "文字", "テキスト"],
    ["视觉", "Vision", "視覺", "画像"],
    ["工具调用", "Tool Calling", "工具調用", "ツール呼び出し"],
    ["数据库服务信息", "Database Service Information", "資料庫服務資訊", "データベースサービス情報"],
    ["BurnCloud 数据库费率", "BurnCloud Database Rates", "BurnCloud 資料庫費率", "BurnCloud データベース料金"],
    ["输入价格", "Input Price", "輸入價格", "入力価格"],
    ["输出价格", "Output Price", "輸出價格", "出力価格"],
    ["/ 100万 Token", "/ 1M tokens", "/ 100萬 Token", "/ 100万トークン"],
    ["模型能力", "Model Capabilities", "模型能力", "モデル機能"],
    ["视觉输入", "Vision Input", "視覺輸入", "画像入力"],
    ["函数调用", "Function Calling", "函數調用", "関数呼び出し"],
    ["类型", "Type", "類型", "タイプ"],
    ["推荐使用场景", "Recommended Use Cases", "推薦使用場景", "推奨ユースケース"],
    ["运行规格与路由数据", "Runtime Specifications & Routing Data", "運行規格與路由資料", "実行仕様とルーティングデータ"],
    ["渠道响应延迟", "Channel Response Latency", "渠道回應延遲", "チャネル応答遅延"],
    ["上下文窗口", "Context Window", "上下文視窗", "コンテキストウィンドウ"],
    ["最大输出", "Maximum Output", "最大輸出", "最大出力"],
    ["可用渠道", "Available Channels", "可用渠道", "利用可能なチャネル"],
    ["数据来源", "Data Source", "資料來源", "データソース"],
    ["支持", "Supported", "支援", "対応"],
    ["未声明", "Not Declared", "未聲明", "未定義"],
    ["数据库未声明", "Not declared in database", "資料庫未聲明", "データベース未定義"],
    ["待运行采样", "Awaiting runtime sample", "待運行取樣", "実行サンプル待ち"],
    ["待定价", "Pricing pending", "待定價", "価格未設定"],
    ["复杂数学、架构推理和多步决策工作负载。", "Complex math, architectural reasoning, and multi-step decision workloads.", "複雜數學、架構推理和多步決策工作負載。", "複雑な数学、アーキテクチャ推論、多段階意思決定のワークロード。"],
    ["代码生成、重构、智能体工具调用与工程分析。", "Code generation, refactoring, agent tool calls, and engineering analysis.", "程式碼生成、重構、智慧體工具調用與工程分析。", "コード生成、リファクタリング、エージェントツール呼び出し、エンジニアリング分析。"],
    ["图像理解、语音或其他多模态输入输出任务。", "Image understanding, speech, and other multimodal input/output tasks.", "圖像理解、語音或其他多模態輸入輸出任務。", "画像理解、音声、その他のマルチモーダル入出力タスク。"],
    ["通用对话、内容生成、分类与企业知识应用。", "General chat, content generation, classification, and enterprise knowledge applications.", "通用對話、內容生成、分類與企業知識應用。", "一般対話、コンテンツ生成、分類、企業ナレッジ用途。"],

    ["算力供应方工作区尚未迁移", "Compute Supplier workspace is not migrated yet", "算力供應方工作區尚未遷移", "コンピュートサプライヤー画面は未移行です"],
    ["角色切换已生效，但 Supplier 页面不在本次前三页迁移范围内。返回 Buyer 工作区可继续使用完整的前三页功能。", "Role switching is active, but Supplier pages are outside this three-page migration. Return to the Buyer workspace to use the migrated pages.", "角色切換已生效，但 Supplier 頁面不在本次前三頁遷移範圍內。返回 Buyer 工作區可繼續使用完整的前三頁功能。", "役割切り替えは有効ですが、Supplier ページは今回の3ページ移行対象外です。Buyer ワークスペースに戻って移行済みページを利用してください。"],
    ["没有管理员权限", "No administrator permission", "沒有管理員權限", "管理者権限がありません"],
    ["当前账户的数据库角色中不包含 admin，因此不能进入平台管理员工作区。", "This account does not have the admin database role and cannot enter the Platform Admin workspace.", "目前帳戶的資料庫角色中不包含 admin，因此不能進入平台管理員工作區。", "このアカウントには admin データベースロールがないため、管理者ワークスペースには入れません。"],
    ["平台管理员工作区尚未迁移", "Platform Admin workspace is not migrated yet", "平台管理員工作區尚未遷移", "プラットフォーム管理者画面は未移行です"],
    ["管理员权限已经验证，但 Admin 页面不在本次前三页迁移范围内。", "Admin permission is verified, but Admin pages are outside this three-page migration.", "管理員權限已經驗證，但 Admin 頁面不在本次前三頁遷移範圍內。", "管理者権限は確認済みですが、Admin ページは今回の3ページ移行対象外です。"],
    ["返回 Buyer 概览", "Return to Buyer Overview", "返回 Buyer 總覽", "Buyer 概要に戻る"],
    ["BurnCloud 后端暂不可用", "BurnCloud backend is unavailable", "BurnCloud 後端暫不可用", "BurnCloud バックエンドを利用できません"],
    ["重新连接", "Reconnect", "重新連接", "再接続"],
  ];

  const languageIndex = { en: 1, "zh-TW": 2, ja: 3 };
  const maps = Object.fromEntries(
    Object.entries(languageIndex).map(([language, index]) => [
      language,
      new Map(entries.map((entry) => [entry[0], entry[index]])),
    ]),
  );
  const textSources = new WeakMap();
  const attributeSources = new WeakMap();
  let currentLanguage = "zh";

  const detectLanguage = () => {
    try {
      const saved = localStorage.getItem(STORAGE_KEY);
      if (SUPPORTED.includes(saved)) return saved;
      const browser = navigator.language.toLowerCase();
      if (/zh-(tw|hk|mo)|zh-hant/.test(browser)) return "zh-TW";
      if (browser.startsWith("zh")) return "zh";
      if (browser.startsWith("ja")) return "ja";
    } catch (_) {}
    return "zh";
  };

  const dynamicTranslate = (source, language) => {
    if (language === "zh") return source;
    const pick = (en, tw, ja) => ({ en, "zh-TW": tw, ja })[language];
    const rules = [
      [/^已连接 (\d+) 个模型，当前有 (\d+) 个有效 API 密钥可用于真实路由。$/, (m) => pick(`Connected to ${m[1]} models with ${m[2]} active API keys available for live routing.`, `已連接 ${m[1]} 個模型，目前有 ${m[2]} 個有效 API 金鑰可用於真實路由。`, `${m[1]} モデルに接続し、${m[2]} 個の有効な API キーを実ルーティングに使用できます。`)],
      [/^数据库中检测到 (\d+) 个可用模型、(\d+) 个有效 API 密钥。完成配置后即可发送真实请求。$/, (m) => pick(`The database has ${m[1]} available models and ${m[2]} active API keys. Complete setup to send live requests.`, `資料庫中檢測到 ${m[1]} 個可用模型、${m[2]} 個有效 API 金鑰。完成設定後即可發送真實請求。`, `データベースに利用可能なモデルが ${m[1]} 件、有効な API キーが ${m[2]} 件あります。設定を完了すると実リクエストを送信できます。`)],
      [/^(\d+) 个模型可由启用渠道提供$/, (m) => pick(`${m[1]} models available from enabled channels`, `${m[1]} 個模型可由啟用渠道提供`, `有効なチャネルから ${m[1]} モデルを利用可能`)],
      [/^(\d+) 次已结算请求$/, (m) => pick(`${m[1]} settled requests`, `${m[1]} 次已結算請求`, `${m[1]} 件の決済済みリクエスト`)],
      [/^当前有 (\d+) 个有效密钥。$/, (m) => pick(`${m[1]} active keys available.`, `目前有 ${m[1]} 個有效金鑰。`, `有効なキーが ${m[1]} 件あります。`)],
      [/^模型目录提供 (\d+) 个可用模型。$/, (m) => pick(`The catalog provides ${m[1]} available models.`, `模型目錄提供 ${m[1]} 個可用模型。`, `カタログに利用可能なモデルが ${m[1]} 件あります。`)],
      [/^当前余额为 (.+)。请充值或检查预算策略，确保生产请求持续可用。$/, (m) => pick(`Current balance: ${m[1]}. Top up or review budget policies to keep production requests available.`, `目前餘額為 ${m[1]}。請儲值或檢查預算策略，確保生產請求持續可用。`, `現在の残高は ${m[1]} です。チャージまたは予算ポリシーを確認し、本番リクエストを継続してください。`)],
      [/^账户充值 (.+)$/, (m) => pick(`Account top-up ${m[1]}`, `帳戶儲值 ${m[1]}`, `アカウントチャージ ${m[1]}`)],
      [/^API 密钥 (.+)$/, (m) => pick(`API Key ${m[1]}`, `API 金鑰 ${m[1]}`, `API キー ${m[1]}`)],
      [/^(\d+) 个数据库模型 · (\d+) 个有效 API 密钥 · 请求通过 BurnCloud 数据面路由$/, (m) => pick(`${m[1]} database models · ${m[2]} active API keys · Requests route through the BurnCloud data plane`, `${m[1]} 個資料庫模型 · ${m[2]} 個有效 API 金鑰 · 請求透過 BurnCloud 資料面路由`, `データベースモデル ${m[1]} 件 · 有効な API キー ${m[2]} 件 · BurnCloud データプレーン経由`)],
      [/^(.+) · (经济级|标准级|性能级) · 真实数据库路由$/, (m) => pick(`${m[1]} · ${{ "经济级": "Economy", "标准级": "Standard", "性能级": "Performance" }[m[2]]} · Live database routing`, `${m[1]} · ${{ "经济级": "經濟級", "标准级": "標準級", "性能级": "效能級" }[m[2]]} · 真實資料庫路由`, `${m[1]} · ${{ "经济级": "エコノミー", "标准级": "スタンダード", "性能级": "パフォーマンス" }[m[2]]} · 実データベースルーティング`)],
      [/^(经济级|标准级|性能级)将写入外部 API 示例；控制台测试由后端当前路由策略执行。$/, (m) => pick(`${{ "经济级": "Economy", "标准级": "Standard", "性能级": "Performance" }[m[1]]} is included in external API samples; console tests use the current backend routing policy.`, `${{ "经济级": "經濟級", "标准级": "標準級", "性能级": "效能級" }[m[1]]}會寫入外部 API 範例；控制台測試由後端目前路由策略執行。`, `${{ "经济级": "エコノミー", "标准级": "スタンダード", "性能级": "パフォーマンス" }[m[1]]}を外部 API サンプルに反映し、コンソールテストは現在のバックエンドポリシーを使用します。`)],
      [/^请求失败：(.*)$/, (m) => pick(`Request failed: ${m[1]}`, `請求失敗：${m[1]}`, `リクエスト失敗: ${m[1]}`)],
      [/^请求失败 \((\d+)\)$/, (m) => pick(`Request failed (${m[1]})`, `請求失敗 (${m[1]})`, `リクエスト失敗 (${m[1]})`)],
      [/^找到 (\d+) 个可用模型$/, (m) => pick(`Found ${m[1]} available models`, `找到 ${m[1]} 個可用模型`, `利用可能なモデル ${m[1]} 件`)],
      [/^共 (\d+) 个可用模型$/, (m) => pick(`${m[1]} available models`, `共 ${m[1]} 個可用模型`, `利用可能なモデル ${m[1]} 件`)],
      [/^未找到匹配模型$/, () => pick("No matching models", "未找到匹配模型", "一致するモデルがありません")],
      [/^(\d+) 个模型由已启用渠道提供，价格与能力来自 BurnCloud 数据库。$/, (m) => pick(`${m[1]} models are provided by enabled channels. Pricing and capabilities come from the BurnCloud database.`, `${m[1]} 個模型由已啟用渠道提供，價格與能力來自 BurnCloud 資料庫。`, `${m[1]} モデルを有効なチャネルから提供しています。価格と機能は BurnCloud データベースに基づきます。`)],
      [/^(\d+) 个渠道$/, (m) => pick(`${m[1]} channels`, `${m[1]} 個渠道`, `${m[1]} チャネル`)],
      [/^可用 · (.+)$/, (m) => pick(`Available · ${m[1]}`, `可用 · ${m[1]}`, `利用可能 · ${m[1]}`)],
      [/^由 (.+) 提供，能力和价格从数据库实时汇总。$/, (m) => pick(`Provided by ${m[1]}; capabilities and prices are aggregated live from the database.`, `由 ${m[1]} 提供，能力和價格從資料庫即時彙總。`, `${m[1]} が提供し、機能と価格はデータベースからリアルタイム集計されます。`)],
      [/^当前模型由 (.+) 提供，共有 (\d+) 个已启用渠道。页面不返回渠道密钥或内部地址。$/, (m) => pick(`This model is provided by ${m[1]} through ${m[2]} enabled channels. Channel keys and internal addresses are never returned.`, `目前模型由 ${m[1]} 提供，共有 ${m[2]} 個已啟用渠道。頁面不返回渠道金鑰或內部位址。`, `このモデルは ${m[1]} が ${m[2]} 件の有効なチャネルで提供します。チャネルキーや内部アドレスは返しません。`)],
      [/^(\d+) 个$/, (m) => pick(`${m[1]}`, `${m[1]} 個`, `${m[1]} 件`)],
      [/^(.+) · 模型$/, (m) => pick(`${m[1]} · Model`, `${m[1]} · 模型`, `${m[1]} · モデル`)],
    ];
    for (const [pattern, render] of rules) {
      const match = source.match(pattern);
      if (match) return render(match);
    }
    return source;
  };

  const translate = (source, language = currentLanguage) => {
    if (!source || language === "zh") return source;
    const exact = maps[language]?.get(source);
    if (exact) return exact;
    const dynamic = dynamicTranslate(source, language);
    if (dynamic !== source) return dynamic;
    let result = source;
    const replacements = [...maps[language].entries()].sort((a, b) => b[0].length - a[0].length);
    for (const [from, to] of replacements) {
      if (result.includes(from)) result = result.split(from).join(to);
    }
    return result;
  };

  const isProtectedText = (node) => node.parentElement?.closest("script, style, code, pre, textarea");
  const translateTextNode = (node) => {
    if (!node.nodeValue?.trim() || isProtectedText(node)) return;
    if (!textSources.has(node)) textSources.set(node, node.nodeValue);
    const source = textSources.get(node);
    const leading = source.match(/^\s*/)?.[0] || "";
    const trailing = source.match(/\s*$/)?.[0] || "";
    node.nodeValue = `${leading}${translate(source.trim())}${trailing}`;
  };

  const translatableAttributes = ["aria-label", "placeholder", "title", "data-label", "data-search"];
  const translateElement = (element) => {
    if (!(element instanceof Element)) return;
    if (!attributeSources.has(element)) attributeSources.set(element, {});
    const sources = attributeSources.get(element);
    for (const name of translatableAttributes) {
      if (!element.hasAttribute(name)) continue;
      if (!(name in sources)) sources[name] = element.getAttribute(name);
      element.setAttribute(name, translate(sources[name]));
    }
  };

  const translateTree = (root) => {
    if (root instanceof Element) translateElement(root);
    const documentRoot = root instanceof Document ? root.documentElement : root;
    if (!documentRoot) return;
    const walker = document.createTreeWalker(documentRoot, NodeFilter.SHOW_TEXT);
    let node;
    while ((node = walker.nextNode())) translateTextNode(node);
    documentRoot.querySelectorAll?.("*").forEach(translateElement);
  };

  const updateSwitchers = () => {
    const meta = LANGUAGE_META[currentLanguage];
    const updateText = (selector, value) => document.querySelectorAll(selector).forEach((node) => {
      if (node.textContent !== value) node.textContent = value;
    });
    updateText("[data-language-current-flag]", meta.flag);
    updateText("[data-language-current-name]", meta.native);
    updateText("[data-language-current-short]", meta.short);
    document.querySelectorAll("[data-language-option]").forEach((option) => {
      const selected = option.dataset.languageOption === currentLanguage;
      option.classList.toggle("selected", selected);
      option.setAttribute("aria-checked", String(selected));
    });
  };

  const applyLanguage = (language, persist = true) => {
    currentLanguage = SUPPORTED.includes(language) ? language : "zh";
    if (persist) {
      try { localStorage.setItem(STORAGE_KEY, currentLanguage); } catch (_) {}
    }
    document.documentElement.lang = LANGUAGE_META[currentLanguage].lang;
    translateTree(document);
    updateSwitchers();
    document.dispatchEvent(new CustomEvent("burncloud:languagechange", { detail: { language: currentLanguage } }));
  };

  const closeLanguageMenus = () => {
    document.querySelectorAll("[data-language-panel]").forEach((panel) => panel.setAttribute("hidden", ""));
    document.querySelectorAll("[data-language-trigger]").forEach((trigger) => trigger.setAttribute("aria-expanded", "false"));
  };

  document.querySelectorAll("[data-language-trigger]").forEach((trigger) => {
    trigger.addEventListener("click", (event) => {
      event.stopPropagation();
      const panel = trigger.closest("[data-language-switcher]")?.querySelector("[data-language-panel]");
      if (!panel) return;
      const open = panel.hasAttribute("hidden");
      closeLanguageMenus();
      panel.toggleAttribute("hidden", !open);
      trigger.setAttribute("aria-expanded", String(open));
      if (open) panel.querySelector("[data-language-option]")?.focus();
    });
  });
  document.querySelectorAll("[data-language-option]").forEach((option) => {
    option.addEventListener("click", () => {
      applyLanguage(option.dataset.languageOption);
      closeLanguageMenus();
    });
  });
  document.addEventListener("click", (event) => {
    if (!event.target.closest("[data-language-switcher]")) closeLanguageMenus();
  });
  document.addEventListener("keydown", (event) => {
    if (event.key === "Escape") closeLanguageMenus();
  });

  const observer = new MutationObserver((mutations) => {
    for (const mutation of mutations) {
      mutation.addedNodes.forEach((node) => {
        if (node.nodeType === Node.TEXT_NODE) translateTextNode(node);
        else if (node.nodeType === Node.ELEMENT_NODE) translateTree(node);
      });
    }
    updateSwitchers();
  });
  observer.observe(document.documentElement, { childList: true, subtree: true });

  window.BurnCloudI18n = {
    get language() { return currentLanguage; },
    setLanguage: applyLanguage,
    t: (source) => translate(source),
  };
  applyLanguage(detectLanguage(), false);
})();
