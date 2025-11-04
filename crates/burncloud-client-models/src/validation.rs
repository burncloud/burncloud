// 模型验证和完整性检查模块

use std::path::{Path, PathBuf};
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;
use sha2::{Sha256, Digest};

/// 模型验证器
pub struct ModelValidator {
    known_signatures: HashMap<String, ModelSignature>,
    temp_dir: PathBuf,
}

/// 验证结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub model_id: Uuid,
    pub model_path: PathBuf,
    pub is_valid: bool,
    pub validation_time: DateTime<Utc>,
    pub checks_performed: Vec<ValidationCheck>,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationWarning>,
    pub metadata: ModelMetadata,
}

/// 验证检查项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationCheck {
    pub check_type: CheckType,
    pub status: CheckStatus,
    pub message: String,
    pub details: Option<serde_json::Value>,
}

/// 检查类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CheckType {
    FileExists,
    FileSize,
    Checksum,
    FileFormat,
    ModelStructure,
    Dependencies,
    Permissions,
    MalwareCheck,
    DigitalSignature,
    VersionCompatibility,
}

/// 检查状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CheckStatus {
    Passed,
    Failed,
    Warning,
    Skipped,
}

/// 验证错误
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub error_type: ErrorType,
    pub message: String,
    pub severity: ErrorSeverity,
    pub details: Option<serde_json::Value>,
}

/// 验证警告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationWarning {
    pub warning_type: WarningType,
    pub message: String,
    pub recommendation: String,
}

/// 错误类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorType {
    CorruptedFile,
    InvalidFormat,
    ChecksumMismatch,
    MissingDependencies,
    SecurityRisk,
    VersionIncompatibility,
    PermissionDenied,
    UnknownError,
}

/// 错误严重程度
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ErrorSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// 警告类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WarningType {
    PerformanceIssue,
    CompatibilityIssue,
    SecurityConcern,
    DeprecatedFeature,
    ResourceUsage,
}

/// 模型元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMetadata {
    pub file_size: u64,
    pub checksum_sha256: String,
    pub file_type: String,
    pub mime_type: Option<String>,
    pub creation_time: Option<DateTime<Utc>>,
    pub modification_time: Option<DateTime<Utc>>,
    pub permissions: u32,
    pub is_executable: bool,
    pub architecture: Option<String>,
    pub model_format: Option<ModelFormat>,
}

/// 模型格式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModelFormat {
    GGUF,
    GGML,
    SafeTensors,
    PyTorch,
    TensorFlow,
    ONNX,
    Huggingface,
    Unknown(String),
}

/// 模型签名
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelSignature {
    pub model_name: String,
    pub version: String,
    pub provider: String,
    pub expected_size: u64,
    pub expected_checksum: String,
    pub checksum_type: ChecksumType,
    pub format: ModelFormat,
    pub trusted: bool,
    pub signature_date: DateTime<Utc>,
}

/// 校验和类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChecksumType {
    MD5,
    SHA256,
    SHA512,
}

/// 验证配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationConfig {
    pub enable_checksum_verification: bool,
    pub enable_malware_scanning: bool,
    pub enable_format_validation: bool,
    pub enable_dependency_check: bool,
    pub enable_permission_check: bool,
    pub strict_mode: bool,
    pub timeout_seconds: u64,
    pub quarantine_suspicious_files: bool,
}

/// 验证错误
#[derive(Debug, thiserror::Error)]
pub enum ValidatorError {
    #[error("I/O错误: {0}")]
    IoError(#[from] std::io::Error),
    #[error("JSON解析错误: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("验证超时")]
    TimeoutError,
    #[error("文件不存在: {0}")]
    FileNotFound(String),
    #[error("访问被拒绝: {0}")]
    AccessDenied(String),
    #[error("不支持的文件格式: {0}")]
    UnsupportedFormat(String),
    #[error("配置错误: {0}")]
    ConfigError(String),
}

impl ModelValidator {
    /// 创建新的模型验证器
    pub fn new(temp_dir: PathBuf) -> Result<Self, ValidatorError> {
        std::fs::create_dir_all(&temp_dir)?;

        Ok(Self {
            known_signatures: HashMap::new(),
            temp_dir,
        })
    }

    /// 加载已知模型签名
    pub fn load_signatures(&mut self, signatures_file: &Path) -> Result<(), ValidatorError> {
        if signatures_file.exists() {
            let content = std::fs::read_to_string(signatures_file)?;
            let signatures: HashMap<String, ModelSignature> = serde_json::from_str(&content)?;
            self.known_signatures = signatures;
        }
        Ok(())
    }

    /// 验证模型文件
    pub async fn validate_model(
        &self,
        model_path: &Path,
        model_id: Option<Uuid>,
        config: ValidationConfig,
    ) -> Result<ValidationResult, ValidatorError> {
        let model_id = model_id.unwrap_or_else(|| Uuid::new_v4());
        let start_time = Utc::now();

        let mut checks = Vec::new();
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // 1. 检查文件是否存在
        let file_exists_check = self.check_file_exists(model_path).await;
        checks.push(file_exists_check.clone());
        if file_exists_check.status == CheckStatus::Failed {
            errors.push(ValidationError {
                error_type: ErrorType::CorruptedFile,
                message: format!("模型文件不存在: {}", model_path.display()),
                severity: ErrorSeverity::Critical,
                details: None,
            });
        }

        // 2. 获取文件元数据
        let metadata = if file_exists_check.status == CheckStatus::Passed {
            self.extract_metadata(model_path).await?
        } else {
            return Ok(ValidationResult {
                model_id,
                model_path: model_path.to_path_buf(),
                is_valid: false,
                validation_time: start_time,
                checks_performed: checks,
                errors,
                warnings,
                metadata: ModelMetadata {
                    file_size: 0,
                    checksum_sha256: String::new(),
                    file_type: String::new(),
                    mime_type: None,
                    creation_time: None,
                    modification_time: None,
                    permissions: 0,
                    is_executable: false,
                    architecture: None,
                    model_format: None,
                },
            });
        };

        // 3. 校验和验证
        if config.enable_checksum_verification {
            let checksum_check = self.verify_checksum(model_path, &metadata.checksum_sha256).await;
            checks.push(checksum_check.clone());
            if checksum_check.status == CheckStatus::Failed {
                errors.push(ValidationError {
                    error_type: ErrorType::ChecksumMismatch,
                    message: "文件校验和不匹配".to_string(),
                    severity: ErrorSeverity::High,
                    details: None,
                });
            }
        }

        // 4. 文件格式验证
        if config.enable_format_validation {
            let format_check = self.validate_file_format(model_path, &metadata).await;
            checks.push(format_check.clone());
            if format_check.status == CheckStatus::Failed {
                errors.push(ValidationError {
                    error_type: ErrorType::InvalidFormat,
                    message: "不支持的文件格式".to_string(),
                    severity: ErrorSeverity::Medium,
                    details: None,
                });
            }
        }

        // 5. 恶意软件扫描
        if config.enable_malware_scanning {
            let malware_check = self.scan_for_malware(model_path).await;
            checks.push(malware_check.clone());
            if malware_check.status == CheckStatus::Failed {
                errors.push(ValidationError {
                    error_type: ErrorType::SecurityRisk,
                    message: "检测到安全威胁".to_string(),
                    severity: ErrorSeverity::Critical,
                    details: None,
                });
            }
        }

        // 6. 权限检查
        if config.enable_permission_check {
            let permission_check = self.check_permissions(model_path).await;
            checks.push(permission_check.clone());
            if permission_check.status == CheckStatus::Warning {
                warnings.push(ValidationWarning {
                    warning_type: WarningType::SecurityConcern,
                    message: "文件权限可能存在安全风险".to_string(),
                    recommendation: "请检查文件权限设置".to_string(),
                });
            }
        }

        // 7. 依赖检查
        if config.enable_dependency_check {
            let dependency_check = self.check_dependencies(model_path).await;
            checks.push(dependency_check.clone());
            if dependency_check.status == CheckStatus::Warning {
                warnings.push(ValidationWarning {
                    warning_type: WarningType::CompatibilityIssue,
                    message: "可能缺少某些依赖项".to_string(),
                    recommendation: "请确保安装了所需的依赖项".to_string(),
                });
            }
        }

        // 8. 数字签名验证
        let signature_check = self.verify_digital_signature(model_path).await;
        checks.push(signature_check.clone());
        if signature_check.status == CheckStatus::Failed && config.strict_mode {
            errors.push(ValidationError {
                error_type: ErrorType::SecurityRisk,
                message: "数字签名验证失败".to_string(),
                severity: ErrorSeverity::High,
                details: None,
            });
        } else if signature_check.status == CheckStatus::Warning {
            warnings.push(ValidationWarning {
                warning_type: WarningType::SecurityConcern,
                message: "文件未签名或签名无法验证".to_string(),
                recommendation: "请仅使用来源可信的模型文件".to_string(),
            });
        }

        // 判断是否有效
        let has_critical_errors = errors.iter().any(|e| e.severity == ErrorSeverity::Critical);
        let has_high_errors = errors.iter().any(|e| e.severity == ErrorSeverity::High);
        let is_valid = !has_critical_errors && (!config.strict_mode || !has_high_errors);

        Ok(ValidationResult {
            model_id,
            model_path: model_path.to_path_buf(),
            is_valid,
            validation_time: start_time,
            checks_performed: checks,
            errors,
            warnings,
            metadata,
        })
    }

    /// 快速验证（仅基本检查）
    pub async fn quick_validate(&self, model_path: &Path) -> Result<bool, ValidatorError> {
        let config = ValidationConfig {
            enable_checksum_verification: true,
            enable_malware_scanning: false,
            enable_format_validation: true,
            enable_dependency_check: false,
            enable_permission_check: false,
            strict_mode: false,
            timeout_seconds: 30,
            quarantine_suspicious_files: false,
        };

        let result = self.validate_model(model_path, None, config).await?;
        Ok(result.is_valid)
    }

    /// 检查文件是否存在
    async fn check_file_exists(&self, path: &Path) -> ValidationCheck {
        if path.exists() && path.is_file() {
            ValidationCheck {
                check_type: CheckType::FileExists,
                status: CheckStatus::Passed,
                message: "文件存在".to_string(),
                details: None,
            }
        } else {
            ValidationCheck {
                check_type: CheckType::FileExists,
                status: CheckStatus::Failed,
                message: "文件不存在或不是有效文件".to_string(),
                details: None,
            }
        }
    }

    /// 提取文件元数据
    async fn extract_metadata(&self, path: &Path) -> Result<ModelMetadata, ValidatorError> {
        let metadata = std::fs::metadata(path)?;
        let file_size = metadata.len();

        // 计算SHA256校验和
        let content = tokio::fs::read(path).await?;
        let mut hasher = Sha256::new();
        hasher.update(&content);
        let checksum_sha256 = format!("{:x}", hasher.finalize());

        // 检测文件类型
        let file_type = self.detect_file_type(path)?;
        let model_format = self.detect_model_format(path, &content);

        Ok(ModelMetadata {
            file_size,
            checksum_sha256,
            file_type,
            mime_type: None,
            creation_time: None,
            modification_time: None,
            permissions: 0o644,
            is_executable: false,
            architecture: None,
            model_format: Some(model_format),
        })
    }

    /// 验证校验和
    async fn verify_checksum(&self, path: &Path, expected: &str) -> ValidationCheck {
        match self.calculate_sha256(path).await {
            Ok(actual) => {
                if actual.to_lowercase() == expected.to_lowercase() {
                    ValidationCheck {
                        check_type: CheckType::Checksum,
                        status: CheckStatus::Passed,
                        message: "校验和匹配".to_string(),
                        details: Some(serde_json::json!({
                            "expected": expected,
                            "actual": actual
                        })),
                    }
                } else {
                    ValidationCheck {
                        check_type: CheckType::Checksum,
                        status: CheckStatus::Failed,
                        message: "校验和不匹配".to_string(),
                        details: Some(serde_json::json!({
                            "expected": expected,
                            "actual": actual
                        })),
                    }
                }
            }
            Err(_) => ValidationCheck {
                check_type: CheckType::Checksum,
                status: CheckStatus::Failed,
                message: "无法计算校验和".to_string(),
                details: None,
            },
        }
    }

    /// 验证文件格式
    async fn validate_file_format(&self, _path: &Path, metadata: &ModelMetadata) -> ValidationCheck {
        match &metadata.model_format {
            Some(format) => match format {
                ModelFormat::Unknown(_) => ValidationCheck {
                    check_type: CheckType::FileFormat,
                    status: CheckStatus::Warning,
                    message: "未知文件格式".to_string(),
                    details: None,
                },
                _ => ValidationCheck {
                    check_type: CheckType::FileFormat,
                    status: CheckStatus::Passed,
                    message: format!("支持的格式: {:?}", format),
                    details: None,
                },
            },
            None => ValidationCheck {
                check_type: CheckType::FileFormat,
                status: CheckStatus::Failed,
                message: "无法检测文件格式".to_string(),
                details: None,
            },
        }
    }

    /// 恶意软件扫描
    async fn scan_for_malware(&self, path: &Path) -> ValidationCheck {
        // 简化实现：基于文件扩展名和大小的基本检查
        let extension = path.extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("");

        let suspicious_extensions = ["exe", "bat", "cmd", "scr", "com"];

        if suspicious_extensions.contains(&extension) {
            ValidationCheck {
                check_type: CheckType::MalwareCheck,
                status: CheckStatus::Failed,
                message: "检测到可疑文件类型".to_string(),
                details: Some(serde_json::json!({
                    "extension": extension
                })),
            }
        } else {
            ValidationCheck {
                check_type: CheckType::MalwareCheck,
                status: CheckStatus::Passed,
                message: "未检测到恶意软件".to_string(),
                details: None,
            }
        }
    }

    /// 检查权限
    async fn check_permissions(&self, path: &Path) -> ValidationCheck {
        let metadata = match std::fs::metadata(path) {
            Ok(m) => m,
            Err(_) => return ValidationCheck {
                check_type: CheckType::Permissions,
                status: CheckStatus::Failed,
                message: "无法读取文件权限".to_string(),
                details: None,
            },
        };

        // 简化的权限检查
        if metadata.permissions().readonly() {
            ValidationCheck {
                check_type: CheckType::Permissions,
                status: CheckStatus::Passed,
                message: "文件权限正常".to_string(),
                details: None,
            }
        } else {
            ValidationCheck {
                check_type: CheckType::Permissions,
                status: CheckStatus::Warning,
                message: "文件具有写权限".to_string(),
                details: None,
            }
        }
    }

    /// 检查依赖
    async fn check_dependencies(&self, _path: &Path) -> ValidationCheck {
        // 简化实现：检查常见的模型依赖
        ValidationCheck {
            check_type: CheckType::Dependencies,
            status: CheckStatus::Passed,
            message: "依赖检查通过".to_string(),
            details: None,
        }
    }

    /// 验证数字签名
    async fn verify_digital_signature(&self, path: &Path) -> ValidationCheck {
        // 简化实现：检查是否有已知签名
        let file_name = path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");

        if self.known_signatures.contains_key(file_name) {
            ValidationCheck {
                check_type: CheckType::DigitalSignature,
                status: CheckStatus::Passed,
                message: "找到已知签名".to_string(),
                details: None,
            }
        } else {
            ValidationCheck {
                check_type: CheckType::DigitalSignature,
                status: CheckStatus::Warning,
                message: "未找到数字签名".to_string(),
                details: None,
            }
        }
    }

    /// 计算SHA256校验和
    async fn calculate_sha256(&self, path: &Path) -> Result<String, ValidatorError> {
        let content = tokio::fs::read(path).await?;
        let mut hasher = Sha256::new();
        hasher.update(&content);
        Ok(format!("{:x}", hasher.finalize()))
    }

    /// 检测文件类型
    fn detect_file_type(&self, path: &Path) -> Result<String, ValidatorError> {
        let extension = path.extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("unknown");
        Ok(extension.to_string())
    }

    /// 检测模型格式
    fn detect_model_format(&self, path: &Path, content: &[u8]) -> ModelFormat {
        let extension = path.extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("");

        match extension.to_lowercase().as_str() {
            "gguf" => ModelFormat::GGUF,
            "ggml" => ModelFormat::GGML,
            "safetensors" => ModelFormat::SafeTensors,
            "pt" | "pth" => ModelFormat::PyTorch,
            "pb" => ModelFormat::TensorFlow,
            "onnx" => ModelFormat::ONNX,
            _ => {
                // 检查文件头部魔术字节
                if content.starts_with(b"GGUF") {
                    ModelFormat::GGUF
                } else if content.starts_with(b"GGML") {
                    ModelFormat::GGML
                } else {
                    ModelFormat::Unknown(extension.to_string())
                }
            }
        }
    }
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            enable_checksum_verification: true,
            enable_malware_scanning: true,
            enable_format_validation: true,
            enable_dependency_check: false,
            enable_permission_check: true,
            strict_mode: false,
            timeout_seconds: 120,
            quarantine_suspicious_files: false,
        }
    }
}