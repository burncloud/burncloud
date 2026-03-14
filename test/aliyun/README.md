# 阿里云 ECS Bundle 安装测试

本目录包含用于在阿里云 Windows ECS 上测试 BurnCloud Bundle 离线安装的脚本。

## 测试环境

- **区域**: cn-shenzhen
- **实例类型**: ecs.g7.large (2 vCPU, 8 GB RAM)
- **操作系统**: Windows Server 2025
- **当前测试服务器 IP**: 47.115.143.233

## 前置条件

1. **阿里云凭证** - 配置好 `~/.aliyun/config.json`
   ```bash
   aliyun configure
   ```

2. **Python 依赖**
   ```bash
   py -3 -m pip install paramiko
   ```

3. **编译 BurnCloud**
   ```bash
   cargo build --release --bin burncloud
   ```

4. **创建 Bundle**
   ```bash
   cargo run --release --bin burncloud -- bundle create openclaw -o target/release
   ```

## 快速开始

```cmd
cd test\aliyun

REM 步骤 1: 创建服务器 (或使用现有服务器)
01_create_server.bat

REM 步骤 2: 上传文件
03_upload_files.bat

REM 步骤 3: 执行安装
04_install_openclaw.bat

REM 步骤 4: 验证安装
05_verify.bat
```

## 脚本说明

| 脚本 | 功能 |
|------|------|
| `aliyun_api.py` | 阿里云 ECS API 封装 (核心库) |
| `01_create_server.bat` | 创建 ECS Windows 实例 |
| `03_upload_files.bat` | 上传 burncloud.exe 和 bundle |
| `04_install_openclaw.bat` | 执行 bundle 离线安装 |
| `05_verify.bat` | 验证安装结果 |

## 指定不同的服务器 IP

```cmd
03_upload_files.bat --ip 1.2.3.4
04_install_openclaw.bat --ip 1.2.3.4
05_verify.bat --ip 1.2.3.4
```

## 预期结果

安装成功后，05_verify 应显示:

```
=== Component Verification ===
  ✓ Node.js: v24.14.0
  ✓ npm: 11.9.0
  ✓ OpenClaw: OpenClaw 2026.3.12
  ✓ fnm: fnm 1.38.1
  ✓ Git: git version 2.53.0.windows.2

=== Installation Status ===
Software: OpenClaw (openclaw)
Status: Installed

=== ALL TESTS PASSED ===
```

## 故障排除

### SSH 连接失败

1. 确认安全组已开放 22 端口
2. 等待 Windows 完全启动 (3-5 分钟)
3. 检查 SSH 服务是否已安装:
   ```powershell
   Get-Service sshd
   ```

### Bundle 上传失败

检查 SFTP 路径格式。Windows SFTP 使用 POSIX 路径:
- 正确: `/C:/burncloud-test`
- 错误: `C:\burncloud-test`

### 安装失败

1. 检查 burncloud.exe 是否能运行
2. 检查 bundle 目录结构是否完整
3. 查看安装日志: `C:\burncloud-test\install.log`
