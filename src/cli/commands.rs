use anyhow::Result;
use burncloud_auto_update::AutoUpdater;
use burncloud_database::Database;
use clap::{Arg, Command};
use tracing::{error, info};

use super::bundle::handle_bundle_command;
use super::channel::handle_channel_command;
use super::currency::handle_currency_command;
use super::group::handle_group_command;
use super::install::handle_install_command;
use super::log::handle_log_command;
use super::monitor::handle_monitor_command;
use super::price::{handle_price_command, handle_tiered_command};
use super::protocol::handle_protocol_command;
use super::token::handle_token_command;
use super::user::handle_user_command;

pub async fn handle_command(args: &[String]) -> Result<()> {
    let app = Command::new("burncloud")
        .version("0.1.0")
        .about("AI model deployment and management platform")
        .subcommand_required(false)
        .subcommand(
            Command::new("update").about("Check and update the application").arg(
                Arg::new("check-only")
                    .long("check-only")
                    .help("Check for updates only, do not update")
                    .action(clap::ArgAction::SetTrue),
            ),
        )
        .subcommand(
            Command::new("install")
                .about("Install third-party AI software")
                .arg(
                    Arg::new("software")
                        .help("Software ID to install (e.g., openclaw, cherry-studio)"),
                )
                .arg(
                    Arg::new("list")
                        .long("list")
                        .help("List available software")
                        .action(clap::ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("status")
                        .long("status")
                        .help("Check installation status")
                        .action(clap::ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("auto-deps")
                        .long("auto-deps")
                        .help("Automatically install dependencies")
                        .action(clap::ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("local")
                        .long("local")
                        .value_name("PATH")
                        .help("Install from local file/directory instead of downloading")
                        .value_parser(clap::value_parser!(String)),
                )
                .arg(
                    Arg::new("bundle")
                        .long("bundle")
                        .value_name("DIR")
                        .help("Use local bundle directory for dependencies (offline mode)")
                        .value_parser(clap::value_parser!(String)),
                ),
        )
        .subcommand(
            Command::new("bundle")
                .about("Manage offline installation bundles")
                .subcommand_required(true)
                .subcommand(
                    Command::new("create")
                        .about("Create an offline installation bundle")
                        .arg(
                            Arg::new("software")
                                .required(true)
                                .help("Software ID to bundle (e.g., 'openclaw')"),
                        )
                        .arg(
                            Arg::new("output")
                                .short('o')
                                .long("output")
                                .value_name("DIR")
                                .help("Output directory for the bundle (default: ./bundles)")
                                .value_parser(clap::value_parser!(String)),
                        ),
                )
                .subcommand(
                    Command::new("verify")
                        .about("Verify bundle integrity")
                        .arg(
                            Arg::new("bundle")
                                .required(true)
                                .help("Path to the bundle directory to verify"),
                        ),
                ),
        )
        .subcommand(
            Command::new("channel")
                .about("Manage API channels")
                .subcommand_required(true)
                .subcommand(
                    Command::new("add")
                        .about("Add a new channel")
                        .arg(
                            Arg::new("type")
                                .short('t')
                                .long("type")
                                .required(true)
                                .help("Channel type (openai, azure, anthropic, gemini, aws, vertexai, deepseek)"),
                        )
                        .arg(
                            Arg::new("key")
                                .short('k')
                                .long("key")
                                .required(true)
                                .help("API key for the channel"),
                        )
                        .arg(
                            Arg::new("models")
                                .short('m')
                                .long("models")
                                .help("Comma-separated list of supported models (uses defaults if not specified)"),
                        )
                        .arg(
                            Arg::new("url")
                                .short('u')
                                .long("url")
                                .help("Custom base URL for the channel"),
                        )
                        .arg(
                            Arg::new("name")
                                .short('n')
                                .long("name")
                                .help("Channel name (uses default if not specified)"),
                        )
                        .arg(
                            Arg::new("pricing-region")
                                .long("pricing-region")
                                .help("Pricing region for this channel (cn, international, or omit for universal)"),
                        )
                        .arg(
                            Arg::new("rpm-cap")
                                .long("rpm-cap")
                                .help("L2 Shaper RPM cap (requests per minute, omit for fail-open)"),
                        )
                        .arg(
                            Arg::new("tpm-cap")
                                .long("tpm-cap")
                                .help("L2 Shaper TPM cap (tokens per minute, omit for fail-open)"),
                        )
                        .arg(
                            Arg::new("reservation-green")
                                .long("reservation-green")
                                .help("L2 Shaper green reservation share (0.0-1.0)"),
                        )
                        .arg(
                            Arg::new("reservation-yellow")
                                .long("reservation-yellow")
                                .help("L2 Shaper yellow reservation share (0.0-1.0)"),
                        )
                        .arg(
                            Arg::new("reservation-red")
                                .long("reservation-red")
                                .help("L2 Shaper red reservation share (0.0-1.0)"),
                        ),
                )
                .subcommand(
                    Command::new("list")
                        .about("List all channels")
                        .arg(
                            Arg::new("format")
                                .long("format")
                                .default_value("table")
                                .value_parser(["table", "json"])
                                .help("Output format (table or json)"),
                        ),
                )
                .subcommand(
                    Command::new("delete")
                        .about("Delete a channel")
                        .arg(
                            Arg::new("id")
                                .required(true)
                                .help("Channel ID to delete"),
                        )
                        .arg(
                            Arg::new("yes")
                                .short('y')
                                .long("yes")
                                .action(clap::ArgAction::SetTrue)
                                .help("Skip confirmation prompt"),
                        ),
                )
                .subcommand(
                    Command::new("show")
                        .about("Show channel details")
                        .arg(
                            Arg::new("id")
                                .required(true)
                                .help("Channel ID to show"),
                        ),
                )
                .subcommand(
                    Command::new("update")
                        .about("Update a channel")
                        .arg(
                            Arg::new("id")
                                .required(true)
                                .help("Channel ID to update"),
                        )
                        .arg(
                            Arg::new("name")
                                .long("name")
                                .help("Channel name"),
                        )
                        .arg(
                            Arg::new("key")
                                .long("key")
                                .help("API key for the channel"),
                        )
                        .arg(
                            Arg::new("status")
                                .long("status")
                                .help("Channel status (1=enabled, 2=disabled, 3=auto-disabled)"),
                        )
                        .arg(
                            Arg::new("models")
                                .long("models")
                                .help("Comma-separated list of supported models"),
                        )
                        .arg(
                            Arg::new("priority")
                                .long("priority")
                                .help("Channel priority"),
                        )
                        .arg(
                            Arg::new("weight")
                                .long("weight")
                                .help("Channel weight"),
                        )
                        .arg(
                            Arg::new("base-url")
                                .long("base-url")
                                .help("Custom base URL for the channel"),
                        )
                        .arg(
                            Arg::new("pricing-region")
                                .long("pricing-region")
                                .help("Pricing region for this channel (cn, international, or omit for universal)"),
                        )
                        .arg(
                            Arg::new("rpm-cap")
                                .long("rpm-cap")
                                .help("L2 Shaper RPM cap (requests per minute)"),
                        )
                        .arg(
                            Arg::new("tpm-cap")
                                .long("tpm-cap")
                                .help("L2 Shaper TPM cap (tokens per minute)"),
                        )
                        .arg(
                            Arg::new("reservation-green")
                                .long("reservation-green")
                                .help("L2 Shaper green reservation share (0.0-1.0)"),
                        )
                        .arg(
                            Arg::new("reservation-yellow")
                                .long("reservation-yellow")
                                .help("L2 Shaper yellow reservation share (0.0-1.0)"),
                        )
                        .arg(
                            Arg::new("reservation-red")
                                .long("reservation-red")
                                .help("L2 Shaper red reservation share (0.0-1.0)"),
                        ),
                ),
        )
        .subcommand(
            Command::new("price")
                .about("Manage model pricing")
                .subcommand_required(true)
                .subcommand(
                    Command::new("list")
                        .about("List all prices")
                        .arg(
                            Arg::new("limit")
                                .long("limit")
                                .default_value("100")
                                .help("Maximum number of results"),
                        )
                        .arg(
                            Arg::new("offset")
                                .long("offset")
                                .default_value("0")
                                .help("Offset for pagination"),
                        )
                        .arg(
                            Arg::new("currency")
                                .long("currency")
                                .help("Filter by currency (USD, CNY, EUR)"),
                        )
                        .arg(
                            Arg::new("region")
                                .long("region")
                                .help("Filter by region (cn, international)"),
                        ),
                )
                .subcommand(
                    Command::new("set")
                        .about("Set price for a model")
                        .arg(
                            Arg::new("model")
                                .required(true)
                                .help("Model name"),
                        )
                        .arg(
                            Arg::new("input")
                                .long("input")
                                .required(true)
                                .help("Input price per 1M tokens"),
                        )
                        .arg(
                            Arg::new("output")
                                .long("output")
                                .required(true)
                                .help("Output price per 1M tokens"),
                        )
                        .arg(
                            Arg::new("currency")
                                .long("currency")
                                .default_value("USD")
                                .help("Currency for this price (USD, CNY, EUR)"),
                        )
                        .arg(
                            Arg::new("region")
                                .long("region")
                                .help("Region for this price (cn, international)"),
                        )
                        .arg(
                            Arg::new("cache-read")
                                .long("cache-read")
                                .help("Cache read input price per 1M tokens"),
                        )
                        .arg(
                            Arg::new("cache-creation")
                                .long("cache-creation")
                                .help("Cache creation input price per 1M tokens"),
                        )
                        .arg(
                            Arg::new("batch-input")
                                .long("batch-input")
                                .help("Batch input price per 1M tokens"),
                        )
                        .arg(
                            Arg::new("batch-output")
                                .long("batch-output")
                                .help("Batch output price per 1M tokens"),
                        )
                        .arg(
                            Arg::new("alias")
                                .long("alias")
                                .help("Alias to another model's pricing"),
                        )
                        .arg(
                            Arg::new("priority-input")
                                .long("priority-input")
                                .help("Priority input price per 1M tokens"),
                        )
                        .arg(
                            Arg::new("priority-output")
                                .long("priority-output")
                                .help("Priority output price per 1M tokens"),
                        )
                        .arg(
                            Arg::new("audio-input")
                                .long("audio-input")
                                .help("Audio input price per 1M tokens"),
                        )
                        .arg(
                            Arg::new("audio-output")
                                .long("audio-output")
                                .help("Audio output price per 1M tokens"),
                        )
                        .arg(
                            Arg::new("reasoning")
                                .long("reasoning")
                                .help("Reasoning token price per 1M tokens (o1/DeepSeek-R1)"),
                        )
                        .arg(
                            Arg::new("embedding")
                                .long("embedding")
                                .help("Embedding token price per 1M tokens"),
                        )
                        .arg(
                            Arg::new("image")
                                .long("image")
                                .help("Image token price per 1M tokens"),
                        )
                        .arg(
                            Arg::new("video")
                                .long("video")
                                .help("Video token price per 1M tokens"),
                        ),
                )
                .subcommand(
                    Command::new("get")
                        .about("Get price for a model")
                        .arg(
                            Arg::new("model")
                                .required(true)
                                .help("Model name"),
                        )
                        .arg(
                            Arg::new("currency")
                                .long("currency")
                                .help("Filter by currency (USD, CNY, EUR)"),
                        )
                        .arg(
                            Arg::new("region")
                                .long("region")
                                .help("Filter by region (cn, international)"),
                        )
                        .arg(
                            Arg::new("verbose")
                                .short('v')
                                .long("verbose")
                                .action(clap::ArgAction::SetTrue)
                                .help("Show tiered pricing configuration"),
                        ),
                )
                .subcommand(
                    Command::new("show")
                        .about("Show detailed pricing information for a model including all currencies")
                        .arg(
                            Arg::new("model")
                                .required(true)
                                .help("Model name"),
                        )
                        .arg(
                            Arg::new("currency")
                                .long("currency")
                                .help("Filter by currency (USD, CNY, EUR)"),
                        )
                        .arg(
                            Arg::new("region")
                                .long("region")
                                .help("Filter by region (cn, international)"),
                        ),
                )
                .subcommand(
                    Command::new("delete")
                        .about("Delete price for a model")
                        .arg(
                            Arg::new("model")
                                .required(true)
                                .help("Model name"),
                        )
                        .arg(
                            Arg::new("region")
                                .long("region")
                                .help("Delete only for a specific region"),
                        ),
                )
                .subcommand(
                    Command::new("sync-status")
                        .about("Show pricing sync status and advanced pricing statistics"),
                )
                .subcommand(
                    Command::new("import")
                        .about("Import pricing configuration from a JSON file")
                        .arg(
                            Arg::new("file")
                                .required(true)
                                .help("JSON file with pricing configuration"),
                        )
                        .arg(
                            Arg::new("override")
                                .long("override")
                                .action(clap::ArgAction::SetTrue)
                                .help("Override existing prices without confirmation"),
                        ),
                )
                .subcommand(
                    Command::new("export")
                        .about("Export pricing configuration to a JSON file")
                        .arg(
                            Arg::new("file")
                                .required(true)
                                .help("Output JSON file path"),
                        )
                        .arg(
                            Arg::new("format")
                                .long("format")
                                .default_value("json")
                                .value_parser(["json", "csv"])
                                .help("Output format (json or csv)"),
                        ),
                )
                .subcommand(
                    Command::new("validate")
                        .about("Validate a pricing configuration JSON file")
                        .arg(
                            Arg::new("file")
                                .required(true)
                                .help("JSON file to validate"),
                        ),
                )
                .subcommand(
                    Command::new("sync")
                        .about("Sync prices from the remote catalog (default: burncloud official)")
                        .arg(
                            Arg::new("url")
                                .long("url")
                                .help("Custom catalog URL (default: burncloud official)"),
                        )
                        .arg(
                            Arg::new("no-litellm")
                                .long("no-litellm")
                                .action(clap::ArgAction::SetTrue)
                                .help("Skip LiteLLM fallback sync"),
                        ),
                ),
        )
        .subcommand(
            Command::new("tiered")
                .about("Manage tiered pricing for models")
                .subcommand_required(true)
                .subcommand(
                    Command::new("list-tiers")
                        .about("List tiered pricing for a model")
                        .arg(
                            Arg::new("model")
                                .required(true)
                                .help("Model name"),
                        )
                        .arg(
                            Arg::new("region")
                                .long("region")
                                .help("Filter by region (cn, international)"),
                        ),
                )
                .subcommand(
                    Command::new("add-tier")
                        .about("Add a tiered pricing entry")
                        .arg(
                            Arg::new("model")
                                .required(true)
                                .help("Model name"),
                        )
                        .arg(
                            Arg::new("tier-start")
                                .long("tier-start")
                                .required(true)
                                .help("Starting token count for this tier"),
                        )
                        .arg(
                            Arg::new("tier-end")
                                .long("tier-end")
                                .help("Ending token count for this tier (omit for no limit)"),
                        )
                        .arg(
                            Arg::new("input-price")
                                .long("input-price")
                                .required(true)
                                .help("Input price per 1M tokens for this tier"),
                        )
                        .arg(
                            Arg::new("output-price")
                                .long("output-price")
                                .required(true)
                                .help("Output price per 1M tokens for this tier"),
                        )
                        .arg(
                            Arg::new("region")
                                .long("region")
                                .help("Region for this tier (cn, international, omit for universal)"),
                        ),
                )
                .subcommand(
                    Command::new("import-tiered")
                        .about("Import tiered pricing from a JSON file")
                        .arg(
                            Arg::new("file")
                                .required(true)
                                .help("JSON file with tiered pricing data"),
                        ),
                )
                .subcommand(
                    Command::new("delete-tiers")
                        .about("Delete tiered pricing for a model")
                        .arg(
                            Arg::new("model")
                                .required(true)
                                .help("Model name"),
                        )
                        .arg(
                            Arg::new("region")
                                .long("region")
                                .help("Delete only for a specific region"),
                        ),
                )
                .subcommand(
                    Command::new("check-tiered")
                        .about("Check if a model has tiered pricing configured")
                        .arg(
                            Arg::new("model")
                                .required(true)
                                .help("Model name to check"),
                        ),
                ),
        )
        .subcommand(
            Command::new("token")
                .about("Manage API tokens")
                .subcommand_required(true)
                .subcommand(
                    Command::new("list")
                        .about("List all tokens")
                        .arg(
                            Arg::new("limit")
                                .long("limit")
                                .default_value("100")
                                .help("Maximum number of results"),
                        )
                        .arg(
                            Arg::new("offset")
                                .long("offset")
                                .default_value("0")
                                .help("Offset for pagination"),
                        )
                        .arg(
                            Arg::new("user-id")
                                .long("user-id")
                                .help("Filter by user ID"),
                        ),
                )
                .subcommand(
                    Command::new("create")
                        .about("Create a new token")
                        .arg(
                            Arg::new("user-id")
                                .long("user-id")
                                .required(true)
                                .help("User ID for the token"),
                        )
                        .arg(
                            Arg::new("name")
                                .long("name")
                                .help("Token name"),
                        )
                        .arg(
                            Arg::new("quota")
                                .long("quota")
                                .help("Remaining quota for the token"),
                        )
                        .arg(
                            Arg::new("unlimited")
                                .long("unlimited")
                                .action(clap::ArgAction::SetTrue)
                                .help("Set unlimited quota"),
                        )
                        .arg(
                            Arg::new("expired")
                                .long("expired")
                                .help("Expiration timestamp (-1 for never)"),
                        ),
                )
                .subcommand(
                    Command::new("update")
                        .about("Update a token")
                        .arg(
                            Arg::new("key")
                                .required(true)
                                .help("Token key to update"),
                        )
                        .arg(
                            Arg::new("name")
                                .long("name")
                                .help("New token name"),
                        )
                        .arg(
                            Arg::new("quota")
                                .long("quota")
                                .help("New remaining quota"),
                        )
                        .arg(
                            Arg::new("status")
                                .long("status")
                                .help("New status (1=active, 0=disabled)"),
                        ),
                )
                .subcommand(
                    Command::new("delete")
                        .about("Delete a token")
                        .arg(
                            Arg::new("key")
                                .required(true)
                                .help("Token key to delete"),
                        )
                        .arg(
                            Arg::new("yes")
                                .short('y')
                                .long("yes")
                                .action(clap::ArgAction::SetTrue)
                                .help("Skip confirmation prompt"),
                        ),
                ),
        )
        .subcommand(
            Command::new("protocol")
                .about("Manage protocol configurations")
                .subcommand_required(true)
                .subcommand(
                    Command::new("list")
                        .about("List all protocol configs")
                        .arg(
                            Arg::new("limit")
                                .long("limit")
                                .default_value("100")
                                .help("Maximum number of results"),
                        )
                        .arg(
                            Arg::new("offset")
                                .long("offset")
                                .default_value("0")
                                .help("Offset for pagination"),
                        ),
                )
                .subcommand(
                    Command::new("add")
                        .about("Add a new protocol config")
                        .arg(
                            Arg::new("channel-type")
                                .long("channel-type")
                                .required(true)
                                .help("Channel type ID (0=OpenAI, 1=Anthropic, 2=Azure, 3=AWS, 4=Gemini, 5=VertexAI, 6=DeepSeek, 7=Moonshot)"),
                        )
                        .arg(
                            Arg::new("api-version")
                                .long("api-version")
                                .required(true)
                                .help("API version string"),
                        )
                        .arg(
                            Arg::new("default")
                                .long("default")
                                .action(clap::ArgAction::SetTrue)
                                .help("Set as default for this channel type"),
                        )
                        .arg(
                            Arg::new("chat-endpoint")
                                .long("chat-endpoint")
                                .help("Chat endpoint template (e.g., /v1/chat/completions)"),
                        )
                        .arg(
                            Arg::new("embed-endpoint")
                                .long("embed-endpoint")
                                .help("Embedding endpoint template"),
                        )
                        .arg(
                            Arg::new("models-endpoint")
                                .long("models-endpoint")
                                .help("Models listing endpoint"),
                        )
                        .arg(
                            Arg::new("request-mapping")
                                .long("request-mapping")
                                .help("JSON request mapping configuration"),
                        )
                        .arg(
                            Arg::new("response-mapping")
                                .long("response-mapping")
                                .help("JSON response mapping configuration"),
                        )
                        .arg(
                            Arg::new("detection-rules")
                                .long("detection-rules")
                                .help("JSON detection rules for API version"),
                        ),
                )
                .subcommand(
                    Command::new("delete")
                        .about("Delete a protocol config")
                        .arg(
                            Arg::new("id")
                                .required(true)
                                .help("Protocol config ID to delete"),
                        ),
                )
                .subcommand(
                    Command::new("show")
                        .about("Show protocol config details")
                        .arg(
                            Arg::new("id")
                                .required(true)
                                .help("Protocol config ID to show"),
                        ),
                )
                .subcommand(
                    Command::new("test")
                        .about("Test a protocol configuration")
                        .arg(
                            Arg::new("channel-id")
                                .long("channel-id")
                                .required(true)
                                .help("Channel ID to test"),
                        )
                        .arg(
                            Arg::new("model")
                                .long("model")
                                .help("Model name to test with"),
                        ),
                ),
        )
        .subcommand(
            Command::new("currency")
                .about("Manage exchange rates and currency conversion")
                .subcommand_required(true)
                .subcommand(
                    Command::new("list-rates")
                        .about("List all exchange rates"),
                )
                .subcommand(
                    Command::new("set-rate")
                        .about("Set an exchange rate")
                        .arg(
                            Arg::new("from")
                                .long("from")
                                .required(true)
                                .help("Source currency (USD, CNY, EUR)"),
                        )
                        .arg(
                            Arg::new("to")
                                .long("to")
                                .required(true)
                                .help("Target currency (USD, CNY, EUR)"),
                        )
                        .arg(
                            Arg::new("rate")
                                .long("rate")
                                .required(true)
                                .help("Exchange rate (e.g., 7.2 for USD→CNY)"),
                        ),
                )
                .subcommand(
                    Command::new("refresh")
                        .about("Refresh exchange rates from external API"),
                )
                .subcommand(
                    Command::new("convert")
                        .about("Convert amount between currencies")
                        .arg(
                            Arg::new("amount")
                                .required(true)
                                .help("Amount to convert"),
                        )
                        .arg(
                            Arg::new("from")
                                .long("from")
                                .required(true)
                                .help("Source currency (USD, CNY, EUR)"),
                        )
                        .arg(
                            Arg::new("to")
                                .long("to")
                                .required(true)
                                .help("Target currency (USD, CNY, EUR)"),
                        ),
                ),
        )
        .subcommand(
            Command::new("user")
                .about("Manage users")
                .subcommand_required(true)
                .subcommand(
                    Command::new("register")
                        .about("Register a new user")
                        .arg(
                            Arg::new("username")
                                .long("username")
                                .required(true)
                                .help("Username for the new user"),
                        )
                        .arg(
                            Arg::new("password")
                                .long("password")
                                .required(true)
                                .help("Password for the new user"),
                        )
                        .arg(
                            Arg::new("email")
                                .long("email")
                                .help("Email address (optional)"),
                        ),
                )
                .subcommand(
                    Command::new("login")
                        .about("Login as a user")
                        .arg(
                            Arg::new("username")
                                .long("username")
                                .required(true)
                                .help("Username to login"),
                        )
                        .arg(
                            Arg::new("password")
                                .long("password")
                                .required(true)
                                .help("Password for the user"),
                        ),
                )
                .subcommand(
                    Command::new("list")
                        .about("List all users")
                        .arg(
                            Arg::new("limit")
                                .long("limit")
                                .default_value("100")
                                .help("Maximum number of results"),
                        )
                        .arg(
                            Arg::new("offset")
                                .long("offset")
                                .default_value("0")
                                .help("Offset for pagination"),
                        )
                        .arg(
                            Arg::new("format")
                                .long("format")
                                .default_value("table")
                                .value_parser(["table", "json"])
                                .help("Output format (table or json)"),
                        ),
                )
                .subcommand(
                    Command::new("topup")
                        .about("Topup user balance")
                        .arg(
                            Arg::new("user-id")
                                .long("user-id")
                                .required(true)
                                .help("User ID to topup"),
                        )
                        .arg(
                            Arg::new("amount")
                                .long("amount")
                                .required(true)
                                .help("Amount to topup (in dollars, e.g., 100.00)"),
                        )
                        .arg(
                            Arg::new("currency")
                                .long("currency")
                                .required(true)
                                .value_parser(["USD", "CNY", "usd", "cny"])
                                .help("Currency for topup (USD or CNY)"),
                        ),
                )
                .subcommand(
                    Command::new("recharges")
                        .about("List user recharge history")
                        .arg(
                            Arg::new("user-id")
                                .long("user-id")
                                .required(true)
                                .help("User ID to query"),
                        )
                        .arg(
                            Arg::new("limit")
                                .long("limit")
                                .default_value("100")
                                .help("Maximum number of results"),
                        ),
                )
                .subcommand(
                    Command::new("check-username")
                        .about("Check if a username is available")
                        .arg(
                            Arg::new("username")
                                .long("username")
                                .required(true)
                                .help("Username to check"),
                        ),
                ),
        )
        .subcommand(
            Command::new("group")
                .about("Manage router groups")
                .subcommand_required(true)
                .subcommand(
                    Command::new("create")
                        .about("Create a new group")
                        .arg(
                            Arg::new("name")
                                .long("name")
                                .required(true)
                                .help("Group name"),
                        )
                        .arg(
                            Arg::new("members")
                                .long("members")
                                .help("Comma-separated list of upstream IDs to add as members"),
                        ),
                )
                .subcommand(
                    Command::new("list")
                        .about("List all groups")
                        .arg(
                            Arg::new("format")
                                .long("format")
                                .default_value("table")
                                .value_parser(["table", "json"])
                                .help("Output format (table or json)"),
                        ),
                )
                .subcommand(
                    Command::new("show")
                        .about("Show group details")
                        .arg(
                            Arg::new("id")
                                .required(true)
                                .help("Group ID"),
                        ),
                )
                .subcommand(
                    Command::new("delete")
                        .about("Delete a group")
                        .arg(
                            Arg::new("id")
                                .required(true)
                                .help("Group ID"),
                        )
                        .arg(
                            Arg::new("yes")
                                .short('y')
                                .long("yes")
                                .action(clap::ArgAction::SetTrue)
                                .help("Skip confirmation prompt"),
                        ),
                )
                .subcommand(
                    Command::new("members")
                        .about("Manage group members")
                        .arg(
                            Arg::new("id")
                                .required(true)
                                .help("Group ID"),
                        )
                        .arg(
                            Arg::new("set")
                                .long("set")
                                .help("Set members (comma-separated, format: upstream_id:weight or upstream_id)"),
                        ),
                ),
        )
        .subcommand(
            Command::new("log")
                .about("Manage request logs")
                .subcommand_required(true)
                .subcommand(
                    Command::new("list")
                        .about("List request logs")
                        .arg(
                            Arg::new("user-id")
                                .long("user-id")
                                .help("Filter by user ID"),
                        )
                        .arg(
                            Arg::new("channel-id")
                                .long("channel-id")
                                .help("Filter by channel ID (upstream ID)"),
                        )
                        .arg(
                            Arg::new("model")
                                .long("model")
                                .help("Filter by model name"),
                        )
                        .arg(
                            Arg::new("limit")
                                .long("limit")
                                .default_value("100")
                                .help("Maximum number of results"),
                        )
                        .arg(
                            Arg::new("offset")
                                .long("offset")
                                .default_value("0")
                                .help("Offset for pagination"),
                        )
                        .arg(
                            Arg::new("format")
                                .long("format")
                                .default_value("table")
                                .value_parser(["table", "json"])
                                .help("Output format (table or json)"),
                        ),
                )
                .subcommand(
                    Command::new("usage")
                        .about("Show usage statistics for a user or token")
                        .arg(
                            Arg::new("user-id")
                                .long("user-id")
                                .help("User ID to show usage for"),
                        )
                        .arg(
                            Arg::new("token")
                                .long("token")
                                .help("API token key to show usage for (e.g. sk-xxx)"),
                        )
                        .arg(
                            Arg::new("period")
                                .long("period")
                                .default_value("month")
                                .value_parser(["day", "week", "month"])
                                .help("Time period (day, week, month)"),
                        )
                        .arg(
                            Arg::new("format")
                                .long("format")
                                .default_value("table")
                                .value_parser(["table", "json"])
                                .help("Output format (table or json)"),
                        ),
                ),
        )
        .subcommand(
            Command::new("monitor")
                .about("Monitor system status")
                .subcommand_required(true)
                .subcommand(
                    Command::new("status")
                        .about("Show system status and metrics")
                        .arg(
                            Arg::new("format")
                                .long("format")
                                .default_value("table")
                                .value_parser(["table", "json"])
                                .help("Output format (table or json)"),
                        ),
                ),
        );

    let matches = app.try_get_matches_from(
        std::iter::once("burncloud".to_string()).chain(args.iter().cloned()),
    )?;

    match matches.subcommand() {
        Some(("update", sub_m)) => {
            let check_only = sub_m.get_flag("check-only");
            let res = tokio::task::spawn_blocking(move || handle_update_command(check_only)).await;
            match res {
                Ok(Ok(())) => {}
                Ok(Err(e)) => return Err(e),
                Err(e) => return Err(anyhow::anyhow!(format!("Update thread failed: {:?}", e))),
            }
        }
        Some(("install", sub_m)) => {
            handle_install_command(sub_m).await?;
        }
        Some(("bundle", sub_m)) => {
            handle_bundle_command(sub_m).await?;
        }
        Some(("channel", sub_m)) => {
            let db = Database::new().await?;
            handle_channel_command(&db, sub_m).await?;
            db.close().await?;
        }
        Some(("price", sub_m)) => {
            let db = Database::new().await?;
            handle_price_command(&db, sub_m).await?;
            db.close().await?;
        }
        Some(("tiered", sub_m)) => {
            let db = Database::new().await?;
            handle_tiered_command(&db, sub_m).await?;
            db.close().await?;
        }
        Some(("token", sub_m)) => {
            let db = Database::new().await?;
            handle_token_command(&db, sub_m).await?;
            db.close().await?;
        }
        Some(("protocol", sub_m)) => {
            let db = Database::new().await?;
            handle_protocol_command(&db, sub_m).await?;
            db.close().await?;
        }
        Some(("currency", sub_m)) => {
            let db = Database::new().await?;
            handle_currency_command(&db, sub_m).await?;
            db.close().await?;
        }
        Some(("user", sub_m)) => {
            let db = Database::new().await?;
            handle_user_command(&db, sub_m).await?;
            db.close().await?;
        }
        Some(("group", sub_m)) => {
            let db = Database::new().await?;
            handle_group_command(&db, sub_m).await?;
            db.close().await?;
        }
        Some(("log", sub_m)) => {
            let db = Database::new().await?;
            handle_log_command(&db, sub_m).await?;
            db.close().await?;
        }
        Some(("monitor", sub_m)) => {
            let db = Database::new().await?;
            handle_monitor_command(&db, sub_m).await?;
            db.close().await?;
        }
        _ => {
            show_help();
        }
    }

    Ok(())
}

/// Handle update command (uses sync version to avoid runtime conflicts)
fn handle_update_command(check_only: bool) -> Result<()> {
    info!("Initializing auto-updater...");

    let updater = AutoUpdater::with_default_config();

    if check_only {
        println!("Checking for updates...");
        match updater.sync_check_for_updates() {
            Ok(true) => {
                println!("✅ New version available!");
                println!("Run 'burncloud update' to update to the latest version");
            }
            Ok(false) => {
                println!("✅ Already up to date");
            }
            Err(e) => {
                error!("Update check failed: {}", e);
                println!("❌ Update check failed: {}", e);
                let (github_url, gitee_url) = updater.get_download_links();
                println!("You can manually download the latest version from:");
                println!("  GitHub: {}", github_url);
                println!("  Gitee:  {}", gitee_url);
                return Err(anyhow::anyhow!("Update check failed: {}", e));
            }
        }
    } else {
        println!("Updating BurnCloud...");
        match updater.sync_update() {
            Ok(_) => {
                println!("✅ Update successful!");
                println!("Please restart the application to use the new version");
            }
            Err(e) => {
                error!("Update failed: {}", e);
                println!("❌ Update failed: {}", e);
                let (github_url, gitee_url) = updater.get_download_links();
                println!("You can manually download the latest version from:");
                println!("  GitHub: {}", github_url);
                println!("  Gitee:  {}", gitee_url);
                return Err(anyhow::anyhow!("Update failed: {}", e));
            }
        }
    }

    Ok(())
}

pub fn show_help() {
    println!("BurnCloud - AI model deployment and management platform");
    println!();
    println!("Usage:");
    println!("  burncloud                     - Start GUI (Windows) / Show help (Linux)");
    println!("  burncloud client              - Start GUI client");
    println!("  burncloud server              - Start server");
    println!("  burncloud update              - Update application");
    println!("  burncloud update --check-only - Check for updates only");
    println!();
    println!("Software Installation:");
    println!("  burncloud install --list              - List available software");
    println!("  burncloud install <software>          - Install software");
    println!("  burncloud install <software> --status - View installation status");
    println!("  burncloud install <software> --auto-deps - Auto-install dependencies");
    println!();
    println!("Offline Bundles:");
    println!("  burncloud bundle create <software> -o <dir> - Create offline bundle");
    println!("  burncloud bundle verify <bundle-dir>         - Verify bundle");
    println!("  burncloud install <software> --bundle <dir>  - Install from bundle");
    println!();
    println!("Pricing Management:");
    println!("  burncloud price list          - List model prices");
    println!("  burncloud price set           - Set model price");
    println!("  burncloud tiered list-tiers   - List tiered pricing");
    println!("  burncloud tiered add-tier     - Add tiered pricing");
    println!("  burncloud tiered import-tiered - Import tiered pricing JSON");
    println!();
    println!("Examples:");
    println!("  burncloud client");
    println!("  burncloud update --check-only");
}
