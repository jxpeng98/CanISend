use std::{fs, sync::Arc};

use eframe::egui::{self, FontData, FontDefinitions, FontFamily};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language {
    #[default]
    #[serde(rename = "en")]
    English,
    #[serde(rename = "zh-CN")]
    SimplifiedChinese,
}

impl Language {
    pub const ALL: [Self; 2] = [Self::English, Self::SimplifiedChinese];

    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::English => "en",
            Self::SimplifiedChinese => "zh-CN",
        }
    }

    #[must_use]
    pub const fn native_name(self) -> &'static str {
        match self {
            Self::English => "English",
            Self::SimplifiedChinese => "简体中文",
        }
    }

    #[must_use]
    pub const fn select<'a>(self, english: &'a str, simplified_chinese: &'a str) -> &'a str {
        match self {
            Self::English => english,
            Self::SimplifiedChinese => simplified_chinese,
        }
    }

    #[must_use]
    pub fn text(self, english: &'static str) -> &'static str {
        if self == Self::English {
            return english;
        }
        match english {
            "Workspace" => "工作区",
            "Choose a workspace" => "选择工作区",
            "No workspace" => "未选择工作区",
            "Not checked" => "尚未检查",
            "Healthy" => "正常",
            "Needs attention" => "需要处理",
            "Health" => "健康状态",
            "CanISend workspace header" => "CanISend 工作区标题栏",
            "Overview" => "概览",
            "Jobs" => "职位",
            "Discovery" => "职位发现",
            "Profile" => "个人资料",
            "Agent integration" => "Agent 集成",
            "Workspaces" => "工作区",
            "Command line" => "命令行",
            "Diagnostics" => "诊断",
            "Accessibility & appearance" => "无障碍与外观",
            "Language" => "语言",
            "Locale" => "区域代码",
            "CJK font" => "中日韩字体",
            "Dark appearance" => "深色外观",
            "Compact density" => "紧凑布局",
            "Reduce motion" => "减少动态效果",
            "Text size" => "文字大小",
            "Primary navigation" => "主导航",
            "Local workspace state" => "本地工作区状态",
            "Application status" => "应用状态",
            "Finish the current operation before starting another one." => {
                "请等待当前操作完成后再开始其他操作。"
            }
            "The background operation ended unexpectedly. No completion was recorded; review the current workspace state and try again." => {
                "后台操作意外结束，未记录完成结果。请检查当前工作区状态后重试。"
            }
            "Opening workspace" => "正在打开工作区",
            "Loading jobs" => "正在加载职位",
            "Loading job" => "正在加载职位",
            "Loading discovery catalog" => "正在加载发现适配器目录",
            "Loading discovery workspace" => "正在加载职位发现数据",
            "Promoting discovery lead" => "正在提升发现线索",
            "Loading profile sources" => "正在加载个人资料来源",
            "Loading profile sources…" => "正在加载个人资料来源…",
            "Agent integration content" => "Agent 集成内容",
            "Completed" => "已完成",
            "Dismiss" => "关闭",
            "Current local workspace and next actions" => "当前本地工作区和后续操作",
            "Leads" => "线索",
            "Discovery sources" => "发现来源",
            "Import batch" => "导入批次",
            "Refresh public source" => "刷新公开来源",
            "Active jobs" => "活跃职位",
            "Stored in this workspace" => "保存在此工作区中",
            "Artifacts" => "工件",
            "Revisioned local records" => "带修订记录的本地数据",
            "Workspace health" => "工作区健康状态",
            "Issues" => "存在问题",
            "Run an integrity check regularly" => "请定期运行完整性检查",
            "Add job" => "添加职位",
            "Check workspace" => "检查工作区",
            "View all jobs" => "查看全部职位",
            "Recently updated jobs" => "最近更新的职位",
            "No jobs yet. Add a job from a URL, PDF, Markdown, text, or JSON file." => {
                "还没有职位。可通过 URL、PDF、Markdown、文本或 JSON 文件添加职位。"
            }
            "Reusable applicant sources and confirmed evidence" => "可复用的申请人来源和已确认证据",
            "Import profile source" => "导入个人资料来源",
            "Load profile sources" => "加载个人资料来源",
            "Profile revision" => "个人资料修订版本",
            "Revision-bound applicant context" => "与修订版本绑定的申请人信息",
            "Profile sources" => "个人资料来源",
            "Sources available to evidence workflow" => "可供证据工作流使用的来源",
            "Source catalog" => "来源目录",
            "No profile sources yet." => "尚未导入个人资料来源。",
            "Import Markdown, text, or JSON to create reusable applicant context." => {
                "导入 Markdown、文本或 JSON，以创建可复用的申请人信息。"
            }
            "Markdown" => "Markdown",
            "Plain text" => "纯文本",
            "Public" => "公开",
            "Private local" => "本地私有",
            "Provider bound" => "仅限提供方",
            "Secret" => "机密",
            "Source ID" => "来源 ID",
            "Content type" => "内容类型",
            "Revision" => "修订版本",
            "Imported at" => "导入时间",
            "Original digest" => "原始文件摘要",
            "Normalized digest" => "规范化摘要",
            "Evidence review" => "证据审阅",
            "Choose a job" => "选择职位",
            "Job" => "职位",
            "Allow this user-invoked private evidence review" => "允许本次由用户发起的私有证据审阅",
            "Load evidence candidate" => "加载候选证据",
            "Evidence candidate" => "候选证据",
            "Qualification" => "资格",
            "Teaching" => "教学",
            "Research" => "研究",
            "Communication" => "沟通",
            "Leadership" => "领导力",
            "Service" => "服务",
            "Employment" => "工作经历",
            "Other" => "其他",
            "Summary" => "摘要",
            "Source quote" => "来源引文",
            "Exclude from application evidence" => "从申请证据中排除",
            "I reviewed this evidence item and its classification" => "我已审阅此证据条目及其分类",
            "bytes" => "字节",
            "I understand the downstream revision effects" => "我了解下游修订影响",
            "Confirm evidence" => "确认证据",
            "No evidence items are available to confirm" => "没有可确认的证据条目",
            "Confirm the downstream revision effects before saving evidence" => {
                "保存证据前请确认下游修订影响"
            }
            "Choose a job first" => "请先选择职位",
            "Confirm private evidence access before loading" => "加载前请确认允许访问私有证据",
            "Loading evidence candidate" => "正在加载候选证据",
            "No active workspace or job is selected" => "尚未选择当前工作区或职位",
            "Confirming profile evidence" => "正在确认个人资料证据",
            "Criteria review" => "职位条件审阅",
            "Allow this user-invoked private criteria review" => {
                "允许本次由用户发起的私有职位条件审阅"
            }
            "Load criteria candidate" => "加载候选条件",
            "Loading criteria candidate" => "正在加载候选条件",
            "Criteria candidate" => "候选条件",
            "Criterion" => "条件",
            "Evidence kind" => "证据类型",
            "Importance" => "重要性",
            "Essential" => "必要",
            "Desirable" => "优先",
            "Informational" => "参考",
            "Requirement" => "要求",
            "I reviewed this criterion and its importance" => "我已审阅此条件及其重要性",
            "No criteria are available to confirm" => "没有可确认的职位条件",
            "Confirm the downstream revision effects before saving criteria" => {
                "保存职位条件前请确认下游修订影响"
            }
            "Confirm criteria" => "确认职位条件",
            "Confirming criteria" => "正在确认职位条件",
            "Confirm private criteria access before loading" => "加载前请确认允许访问私有职位条件",
            "Current evidence matches" => "当前证据匹配",
            "Allow this user-invoked private match review" => "允许本次由用户发起的私有匹配审阅",
            "Load current matches" => "加载当前匹配",
            "Loading current matches" => "正在加载当前匹配",
            "Confirm private match access before loading" => "加载前请确认允许访问私有匹配",
            "Match results" => "匹配结果",
            "No current matches are recorded." => "没有记录当前匹配结果。",
            "Strong" => "强",
            "Partial" => "部分",
            "Gap" => "差距",
            "Unknown" => "未知",
            "Criterion ID" => "条件 ID",
            "Rationale" => "匹配依据",
            "Evidence references" => "证据引用",
            "No evidence reference" => "没有证据引用",
            "Prohibited claims" => "禁止使用的陈述",
            "Application plan" => "申请计划",
            "Allow this user-invoked private plan review" => "允许本次由用户发起的私有申请计划审阅",
            "Load editable plan" => "加载可编辑计划",
            "Load confirmed plan" => "加载已确认计划",
            "Loading application plan candidate" => "正在加载候选申请计划",
            "Loading current application plan" => "正在加载当前申请计划",
            "Application plan candidate" => "候选申请计划",
            "Matches artifact" => "匹配工件",
            "Decision" => "申请决定",
            "Apply" => "申请",
            "Hold" => "暂缓",
            "Skip" => "跳过",
            "Application strategy" => "申请策略",
            "Positioning" => "定位",
            "Priorities" => "优先事项",
            "Priority" => "优先事项",
            "Add priority" => "添加优先事项",
            "Risks" => "风险",
            "Risk" => "风险",
            "Add risk" => "添加风险",
            "Remove" => "移除",
            "Document plan" => "文档计划",
            "Cover letter" => "求职信",
            "Research statement" => "研究陈述",
            "Teaching statement" => "教学陈述",
            "CV" => "简历",
            "Required" => "必需",
            "Optional" => "可选",
            "Omitted" => "省略",
            "Executor" => "执行方式",
            "Omitted documents do not have an executor." => "省略的文档不设置执行方式。",
            "Document rationale" => "文档理由",
            "Constraints" => "约束",
            "Constraint" => "约束",
            "Add constraint" => "添加约束",
            "Derived blockers" => "派生阻塞项",
            "No derived blockers." => "没有派生阻塞项。",
            "Blocking" => "阻塞",
            "Warning" => "警告",
            "I confirm this application decision and its downstream effects" => {
                "我确认此申请决定及其下游影响"
            }
            "Confirm application plan" => "确认申请计划",
            "Confirming application plan" => "正在确认申请计划",
            "Confirm private plan access before loading" => "加载前请确认允许访问私有申请计划",
            "Application positioning is required" => "必须填写申请定位",
            "Add at least one non-empty application priority" => "请至少添加一项非空的申请优先事项",
            "Remove empty application risks" => "请移除空的申请风险",
            "The plan must contain each supported document exactly once" => {
                "计划必须且只能包含每种受支持的文档"
            }
            "An omitted document cannot have an executor" => "省略的文档不能设置执行方式",
            "Each included document needs a supported executor" => {
                "每个包含的文档都需要设置受支持的执行方式"
            }
            "A skipped application must omit every document" => "跳过申请时必须省略所有文档",
            "Resolve blocking evidence gaps before choosing Apply" => {
                "选择申请前必须解决阻塞性的证据差距"
            }
            "Explicitly confirm the application decision before saving" => {
                "保存前请明确确认申请决定"
            }
            "Application records, supplied sources, and workflow state" => {
                "申请记录、用户提供的来源和工作流状态"
            }
            "Search" => "搜索",
            "Title or institution" => "职位名称或机构",
            "Include archived" => "包含已归档",
            "No jobs match the current filter." => "没有符合当前筛选条件的职位。",
            "Archived" => "已归档",
            "Open job" => "打开职位",
            "Back to jobs" => "返回职位列表",
            "Import source" => "导入来源",
            "Start workflow" => "启动工作流",
            "Import at least one source first" => "请先导入至少一个来源",
            "Archive" => "归档",
            "Sources" => "来源",
            "No source has been imported." => "尚未导入来源。",
            "Workflow" => "工作流",
            "Workflow has not started." => "工作流尚未启动。",
            "Import a source, then start the durable stage graph." => {
                "请先导入来源，然后启动持久化阶段流程。"
            }
            "Loading workflow controls" => "正在加载工作流控制",
            "Loading workflow controls…" => "正在加载工作流控制…",
            "Execution mode" => "执行模式",
            "Current output" => "当前输出",
            "Expected output" => "预期输出",
            "Next actions" => "后续操作",
            "Begin stage" => "开始阶段",
            "Complete stage" => "完成阶段",
            "Rerun stage" => "重新运行阶段",
            "Deterministic" => "确定性执行",
            "Host agent" => "宿主 Agent",
            "Configured provider" => "已配置的提供方",
            "User decision" => "用户决策",
            "Manual import" => "手动导入",
            "Plan confirmation remains available through the CLI or Agent v2." => {
                "计划确认仍可通过 CLI 或 Agent v2 完成。"
            }
            "Workflow scope" => "工作流范围",
            "Plan confirmation, document creation, review, render, and export remain available through the CLI or Agent v2." => {
                "计划确认、文档创建、审阅、渲染和导出仍可通过 CLI 或 Agent v2 完成。"
            }
            "Document creation, review, render, and export remain available through the CLI or Agent v2." => {
                "文档创建、审阅、渲染和导出仍可通过 CLI 或 Agent v2 完成。"
            }
            "Stage-specific artifact creation, plan confirmation, criteria, evidence, documents, review, render, and export remain available through the CLI or Agent v2." => {
                "阶段专用工件创建、计划确认、条件、证据、文档、审阅、渲染和导出仍可通过 CLI 或 Agent v2 完成。"
            }
            "Alpha GUI scope" => "Alpha GUI 范围",
            "Stage begin/complete/rerun, criteria, evidence, documents, review, render, and export remain available through the CLI or Agent v2 until the Beta GUI." => {
                "在 Beta GUI 完成前，阶段开始/完成/重跑、条件、证据、文档、审阅、渲染和导出仍可通过 CLI 或 Agent v2 使用。"
            }
            "Local workspace registry, integrity, and backups" => "本地工作区注册、完整性和备份",
            "Create workspace" => "创建工作区",
            "Register existing" => "注册现有工作区",
            "Restore backup" => "恢复备份",
            "Repair active" => "修复当前工作区",
            "Rebuild missing or changed managed projections from verified records" => {
                "根据已验证的记录重建缺失或发生变化的托管文件"
            }
            "Check active" => "检查当前工作区",
            "Back up active" => "备份当前工作区",
            "Remove from list" => "从列表移除",
            "This does not delete workspace data" => "此操作不会删除工作区数据",
            "Open" => "打开",
            "Active" => "当前",
            "Latest integrity check" => "最近一次完整性检查",
            "Database and referenced blobs passed verification." => {
                "数据库及其引用的文件已通过验证。"
            }
            "The workspace needs attention before further mutation." => {
                "继续修改前需要先处理此工作区的问题。"
            }
            "Body-free product and runtime information" => "不包含用户内容的产品与运行时信息",
            "Product" => "产品",
            "Version" => "版本",
            "Protocol" => "协议",
            "Workspace format" => "工作区格式",
            "Target" => "目标平台",
            "Display scale" => "显示缩放",
            "Not reported by the window system" => "窗口系统未报告",
            "physical pixels per point" => "物理像素/点",
            "Reduced motion" => "减少动态效果",
            "Enabled" => "已启用",
            "Disabled" => "已停用",
            "Run native self-check" => "运行原生自检",
            "Running native self-check" => "正在运行原生自检",
            "Native foundation healthy" => "原生基础组件正常",
            "Native foundation needs attention" => "原生基础组件需要处理",
            "Python runtime: not required" => "Python 运行时：不需要",
            "Diagnostics intentionally omit job adverts, profile evidence, drafts, and provider payloads." => {
                "诊断信息不会包含职位广告、个人资料证据、草稿或服务提供方载荷。"
            }
            "Keep the terminal CLI aligned with this CanISend desktop release" => {
                "使终端 CLI 与当前 CanISend 桌面版本保持一致"
            }
            "Check CLI installation" => "检查 CLI 安装",
            "Checking CLI installation" => "正在检查 CLI 安装",
            "Checking CLI installation…" => "正在检查 CLI 安装…",
            "Terminal installation" => "终端安装",
            "Bundled version" => "内置版本",
            "Installed version" => "已安装版本",
            "Unknown (older version interface)" => "未知（旧版接口）",
            "Not installed" => "未安装",
            "Bundled CLI" => "内置 CLI",
            "Not found" => "未找到",
            "Install destination" => "安装位置",
            "Current PATH resolves" => "当前 PATH 解析结果",
            "No canisend command found on PATH" => "PATH 中未找到 canisend 命令",
            "Destination on current PATH" => "安装位置是否在当前 PATH 中",
            "Yes" => "是",
            "No" => "否",
            "No GUI-managed Rust CLI is installed at the destination." => {
                "目标位置尚未安装由 GUI 管理的 Rust CLI。"
            }
            "The GUI-managed native CLI is the command currently resolved by PATH." => {
                "PATH 当前解析到由 GUI 管理的原生 CLI。"
            }
            "The CLI installed by this GUI differs from the bundled release." => {
                "此 GUI 安装的 CLI 与当前内置版本不同。"
            }
            "Update CLI" => "更新 CLI",
            "Upgrade installed CLI" => "升级已安装的 CLI",
            "Migrate installed CLI" => "迁移已安装的 CLI",
            "Reinstall CLI" => "重新安装 CLI",
            "Install CLI" => "安装 CLI",
            "Uninstall managed CLI" => "卸载受管理的 CLI",
            "Refresh" => "刷新",
            "CanISend updates" => "CanISend 更新",
            "Check for updates" => "检查更新",
            "Copy release link" => "复制发布链接",
            "Use from a terminal or agent host" => "在终端或 Agent 宿主中使用",
            "Copy" => "复制",
            "Installing or upgrading CanISend CLI" => "正在安装或升级 CanISend CLI",
            "Checking for CanISend updates" => "正在检查 CanISend 更新",
            "No bundled CanISend CLI is available" => "没有可用的内置 CanISend CLI",
            "Choose a local workspace" => "选择本地工作区",
            "Create a new workspace or register an existing Rust v2 workspace." => {
                "创建新工作区，或注册现有的 Rust v2 工作区。"
            }
            "Open workspace manager" => "打开工作区管理器",
            "Checking workspace integrity" => "正在检查工作区完整性",
            "Creating verified backup" => "正在创建经过验证的备份",
            "Starting workflow" => "正在启动工作流",
            "Archiving job" => "正在归档职位",
            "Title" => "职位名称",
            "Institution" => "机构",
            "Create job" => "创建职位",
            "Cancel" => "取消",
            "Job title is required" => "必须填写职位名称",
            "Institution is required" => "必须填写机构",
            "Title and institution must each be at most 512 bytes" => {
                "职位名称和机构分别不能超过 512 字节"
            }
            "Creating job" => "正在创建职位",
            "Import job source" => "导入职位来源",
            "Local file" => "本地文件",
            "Public URL" => "公开 URL",
            "Choose a source file" => "选择来源文件",
            "No file selected" => "未选择文件",
            "Choose file" => "选择文件",
            "Supported: Markdown, text, JSON, and text-based PDF." => {
                "支持 Markdown、文本、JSON 和文本型 PDF。"
            }
            "Supported: Markdown, text, and JSON." => "支持 Markdown、文本和 JSON。",
            "Sensitivity" => "敏感级别",
            "Unsupported profile source sensitivity" => "不支持的个人资料来源敏感级别",
            "Allow CanISend to read and store this local profile source" => {
                "允许 CanISend 读取并保存此本地个人资料来源"
            }
            "Job source URL" => "职位来源 URL",
            "CanISend will fetch this user-supplied public HTTP(S) URL." => {
                "CanISend 将读取用户提供的公开 HTTP(S) URL。"
            }
            "Allow this user-invoked network fetch" => "允许本次由用户发起的网络读取",
            "Allow CanISend to read and store this private local source" => {
                "允许 CanISend 读取并保存此私有本地来源"
            }
            "Import" => "导入",
            "No active job is selected" => "尚未选择活跃职位",
            "Choose a file" => "请选择文件",
            "Confirm private local source access before importing" => {
                "导入前请确认允许访问私有本地来源"
            }
            "Choose a profile source file" => "请选择个人资料来源文件",
            "Confirm local profile source access before importing" => {
                "导入前请确认允许访问本地个人资料来源"
            }
            "Importing profile source" => "正在导入个人资料来源",
            "Enter a public HTTP(S) URL" => "请输入公开的 HTTP(S) URL",
            "Confirm the user-invoked network fetch before importing" => {
                "导入前请确认本次由用户发起的网络读取"
            }
            "Importing local source" => "正在导入本地来源",
            "Fetching and importing URL" => "正在读取并导入 URL",
            "Register existing workspace" => "注册现有工作区",
            "Workspace name" => "工作区名称",
            "Choose a new or empty directory." => "请选择新目录或空目录。",
            "Choose a directory containing canisend.toml." => "请选择包含 canisend.toml 的目录。",
            "No directory selected" => "未选择目录",
            "Choose directory" => "选择目录",
            "Create" => "创建",
            "Register" => "注册",
            "Choose a directory" => "请选择目录",
            "Creating workspace" => "正在创建工作区",
            "This stage has no supported execution mode." => "此阶段没有受支持的执行模式。",
            "Preparing rerun preview" => "正在准备重新运行预览",
            "Begin workflow stage" => "开始工作流阶段",
            "Complete workflow stage" => "完成工作流阶段",
            "Stage" => "阶段",
            "Choose one execution mode supported by the compiled stage descriptor." => {
                "请选择编译阶段描述符支持的一种执行模式。"
            }
            "Enter the current artifact UUIDv7. CanISend resolves and validates its kind, revision, and digest from the workspace." => {
                "请输入当前工件的 UUIDv7。CanISend 将从工作区解析并验证其类型、修订版本和摘要。"
            }
            "Artifact ID" => "工件 ID",
            "Continue" => "继续",
            "Rerun this workflow stage?" => "重新运行此工作流阶段？",
            "The target stage and its descendants will be reset. Current affected outputs become stale and are no longer selected as workflow outputs." => {
                "目标阶段及其后代阶段将被重置。当前受影响的输出会变为过期状态，不再作为工作流输出。"
            }
            "Target stage" => "目标阶段",
            "Affected stages" => "受影响的阶段",
            "Affected outputs" => "受影响的输出",
            "Confirm rerun" => "确认重新运行",
            "No active workspace is selected" => "尚未选择当前工作区",
            "Rerunning workflow stage" => "正在重新运行工作流阶段",
            "The selected job ID is invalid" => "所选职位 ID 无效",
            "Beginning workflow stage" => "正在开始工作流阶段",
            "Enter a canonical artifact UUIDv7" => "请输入规范的工件 UUIDv7",
            "Completing workflow stage" => "正在完成工作流阶段",
            "Restore workspace backup" => "恢复工作区备份",
            "Restore verifies the backup before creating a separate workspace directory." => {
                "恢复操作会先验证备份，再创建一个独立的工作区目录。"
            }
            "Verified backup directory" => "已验证的备份目录",
            "Choose backup" => "选择备份",
            "Choose a verified backup" => "选择经过验证的备份",
            "New workspace destination" => "新工作区目标目录",
            "Choose destination" => "选择目标目录",
            "Choose a new or empty destination" => "选择新建或空的目标目录",
            "The destination must be new or empty and is never overwritten." => {
                "目标目录必须是新建或空目录，已有内容绝不会被覆盖。"
            }
            "Review restore" => "检查恢复设置",
            "Restore this workspace backup?" => "恢复此工作区备份？",
            "CanISend will verify the backup and create a separate workspace. The source backup is not changed." => {
                "CanISend 将验证备份并创建独立工作区，不会修改源备份。"
            }
            "Confirm restore" => "确认恢复",
            "Repair the active workspace?" => "修复当前工作区？",
            "CanISend will rebuild managed projections from verified workspace records, then run an integrity check. User-edited files are protected by the workspace repair policy." => {
                "CanISend 将根据已验证的工作区记录重建托管文件，然后运行完整性检查。用户编辑过的文件受工作区修复策略保护。"
            }
            "Confirm repair" => "确认修复",
            "Restoring verified workspace backup" => "正在恢复经过验证的工作区备份",
            "Repairing managed workspace files" => "正在修复工作区托管文件",
            "Choose a backup directory" => "请选择备份目录",
            "Choose a destination directory" => "请选择目标目录",
            "Backup and destination directories must be different" => "备份目录与目标目录必须不同",
            "Archive this job?" => "归档此职位？",
            "Confirm archive" => "确认归档",
            "Promote this discovery lead?" => "提升这条发现线索？",
            "Confirm promotion" => "确认提升",
            "Uninstall the managed CLI?" => "卸载受管理的 CLI？",
            "Uninstall CLI" => "卸载 CLI",
            "Uninstalling managed CLI" => "正在卸载受管理的 CLI",
            "Blockers" => "阻塞项",
            "Complete" => "已完成",
            "Ready" => "可开始",
            "Running" => "运行中",
            "Awaiting user" => "等待用户",
            "Blocked" => "已阻塞",
            "Stale" => "已过期",
            "Intake" => "信息收集",
            "Parse" => "解析",
            "Criteria" => "条件提取",
            "Evidence" => "证据",
            "Match" => "匹配",
            "Plan" => "计划",
            "Draft" => "起草",
            "Review" => "审阅",
            "Package" => "打包",
            "Render" => "渲染",
            "Overview content" => "概览内容",
            "Jobs content" => "职位内容",
            "Discovery content" => "职位发现内容",
            "Profile content" => "个人资料内容",
            "Workspaces content" => "工作区内容",
            "Command line content" => "命令行内容",
            "Diagnostics content" => "诊断内容",
            "Installed; not active" => "已安装但未生效",
            "Update available" => "有可用更新",
            "Migration available" => "可迁移",
            "Newer version installed" => "已安装较新版本",
            "CLI missing from package" => "安装包中缺少 CLI",
            _ => english,
        }
    }
}

/// Install a system CJK font as a fallback. The font remains outside the app
/// bundle, keeping the binary small while allowing both supported locales to
/// render from the first frame.
pub fn install_cjk_fallback(ctx: &egui::Context) -> Option<&'static str> {
    const CANDIDATES: &[&str] = &[
        "/System/Library/Fonts/Hiragino Sans GB.ttc",
        "/System/Library/Fonts/STHeiti Light.ttc",
        r"C:\Windows\Fonts\msyh.ttc",
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc",
    ];
    let (path, bytes) = CANDIDATES
        .iter()
        .find_map(|path| fs::read(path).ok().map(|bytes| (*path, bytes)))?;

    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        "canisend-cjk".to_owned(),
        Arc::new(FontData::from_owned(bytes)),
    );
    for family in [FontFamily::Proportional, FontFamily::Monospace] {
        fonts
            .families
            .get_mut(&family)
            .expect("default egui font family")
            .push("canisend-cjk".to_owned());
    }
    ctx.set_fonts(fonts);
    Some(path)
}

#[cfg(test)]
mod tests {
    use super::Language;

    #[test]
    fn locale_codes_and_catalog_are_stable() {
        assert_eq!(Language::English.code(), "en");
        assert_eq!(Language::SimplifiedChinese.code(), "zh-CN");
        assert_eq!(Language::SimplifiedChinese.text("Workspaces"), "工作区");
        assert_eq!(Language::SimplifiedChinese.text("Profile"), "个人资料");
        assert_eq!(Language::SimplifiedChinese.text("Discovery"), "职位发现");
        assert_eq!(
            Language::SimplifiedChinese.text("Restore backup"),
            "恢复备份"
        );
        assert_eq!(
            Language::SimplifiedChinese.text("Confirm repair"),
            "确认修复"
        );
        assert_eq!(
            Language::SimplifiedChinese.text("Begin workflow stage"),
            "开始工作流阶段"
        );
        assert_eq!(
            Language::SimplifiedChinese.text("Affected outputs"),
            "受影响的输出"
        );
        assert_eq!(
            Language::SimplifiedChinese.text("Criteria review"),
            "职位条件审阅"
        );
        assert_eq!(
            Language::SimplifiedChinese.text("Current evidence matches"),
            "当前证据匹配"
        );
        assert_eq!(
            Language::SimplifiedChinese.text("Application plan"),
            "申请计划"
        );
        assert_eq!(
            Language::SimplifiedChinese.text("Confirm application plan"),
            "确认申请计划"
        );
        assert_eq!(
            serde_json::to_string(&Language::SimplifiedChinese).unwrap(),
            r#""zh-CN""#
        );
    }

    #[test]
    fn unknown_catalog_entries_fall_back_to_english() {
        assert_eq!(Language::SimplifiedChinese.text("CanISend"), "CanISend");
    }
}
