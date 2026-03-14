#!/usr/bin/env python3
"""
在阿里云 ECS 上执行 BurnCloud Bundle 安装
测试离线安装功能

依赖:
- paramiko: pip install paramiko

用法:
    python 04_install_openclaw.py --ip 47.115.88.59
"""

import paramiko
import sys
import time
import argparse

# 默认配置
DEFAULT_IP = "47.115.143.233"
USERNAME = "Administrator"
PASSWORD = "Burncloud@Test123"
PORT = 22

REMOTE_DIR = r"C:\burncloud-test"


def execute_command(ssh: paramiko.SSHClient, cmd: str, desc: str = "", timeout: int = 60) -> tuple:
    """执行 SSH 命令并返回 (stdout, stderr, exit_code)"""
    if desc:
        print(f"\n{desc}")
        print(f"Command: {cmd}")

    stdin, stdout, stderr = ssh.exec_command(cmd, timeout=timeout)
    out = stdout.read().decode("utf-8", errors="replace")
    err = stderr.read().decode("utf-8", errors="replace")
    exit_code = stdout.channel.recv_exit_status()

    return out, err, exit_code


def check_environment(ssh: paramiko.SSHClient):
    """检查安装前环境"""
    print("\n" + "=" * 50)
    print("  Pre-Install Environment Check")
    print("=" * 50)

    commands = [
        ("Node.js", "node --version 2>nul || echo Not installed"),
        ("npm", "npm --version 2>nul || echo Not installed"),
        ("fnm", "fnm --version 2>nul || echo Not installed"),
        ("Git", "git --version 2>nul || echo Not installed"),
    ]

    for name, cmd in commands:
        out, _, _ = execute_command(ssh, cmd, timeout=30)
        print(f"  {name}: {out.strip()}")


def run_installation(ssh: paramiko.SSHClient, debug: bool = False):
    """执行 Bundle 安装"""
    print("\n" + "=" * 50)
    print("  Running Bundle Installation")
    print("=" * 50)

    log_level = "debug" if debug else "info"
    cmd = f'set RUST_LOG={log_level} && {REMOTE_DIR}\\burncloud.exe install openclaw --bundle {REMOTE_DIR}\\openclaw-bundle --auto-deps'

    print(f"\nCommand: {cmd}")
    print("Please wait (this may take several minutes)...\n")

    out, err, exit_code = execute_command(ssh, cmd, timeout=600)

    # 打印输出
    if out:
        print(out)
    if err.strip():
        print(f"\n[stderr] {err}")

    print(f"\nExit code: {exit_code}")
    return exit_code


def verify_installation(ssh: paramiko.SSHClient):
    """验证安装结果"""
    print("\n" + "=" * 50)
    print("  Post-Install Verification")
    print("=" * 50)

    # 检查工具版本
    commands = [
        ("Node.js", "node --version 2>nul || echo Not installed"),
        ("npm", "npm --version 2>nul || echo Not installed"),
        ("fnm", "fnm --version 2>nul || echo Not installed"),
        ("Git", "git --version 2>nul || echo Not installed"),
    ]

    for name, cmd in commands:
        out, _, _ = execute_command(ssh, cmd, timeout=30)
        print(f"  {name}: {out.strip()}")

    # 检查安装状态
    print("\n--- BurnCloud Installation Status ---")
    cmd = f'{REMOTE_DIR}\\burncloud.exe install openclaw --status'
    out, err, _ = execute_command(ssh, cmd, timeout=30)
    print(out if out.strip() else "No status output")
    if err.strip():
        print(f"Error: {err.strip()}")

    # 检查安装目录
    print("\n--- OpenClaw Installation Directory ---")
    cmd = 'dir C:\\Users\\Administrator\\.burncloud\\software\\openclaw 2>nul || echo Directory not found'
    out, _, _ = execute_command(ssh, cmd, timeout=30)
    print(out)


def test_openclaw(ssh: paramiko.SSHClient):
    """测试 OpenClaw 是否可用"""
    print("\n" + "=" * 50)
    print("  Testing OpenClaw")
    print("=" * 50)

    # 设置 Node.js PATH
    node_path = r'C:\Users\Administrator\AppData\Roaming\fnm\node-versions\v24.14.0\installation'
    cmd = f'set "PATH={node_path};%PATH%" && openclaw --version'

    out, err, _ = execute_command(ssh, cmd, "\n--- openclaw --version ---", timeout=30)
    print(f"Output: {out.strip()}")
    if err.strip():
        print(f"Error: {err.strip()}")


def main():
    parser = argparse.ArgumentParser(description="Install OpenClaw on Aliyun ECS via Bundle")
    parser.add_argument("--ip", default=DEFAULT_IP, help="Server IP address")
    parser.add_argument("--username", default=USERNAME, help="SSH username")
    parser.add_argument("--password", default=PASSWORD, help="SSH password")
    parser.add_argument("--debug", action="store_true", help="Enable debug logging")
    args = parser.parse_args()

    print("=" * 50)
    print("  BurnCloud Bundle Installation Test")
    print("=" * 50)
    print(f"  Server:   {args.ip}")
    print(f"  Username: {args.username}")
    print()

    try:
        # 连接 SSH
        print("Connecting to server...")
        ssh = paramiko.SSHClient()
        ssh.set_missing_host_key_policy(paramiko.AutoAddPolicy())
        ssh.connect(args.ip, PORT, args.username, args.password, timeout=30)
        print("Connected!")

        # 检查安装前环境
        check_environment(ssh)

        # 执行安装
        exit_code = run_installation(ssh, debug=args.debug)

        # 等待一下
        time.sleep(2)

        # 验证安装
        verify_installation(ssh)

        # 测试 OpenClaw
        test_openclaw(ssh)

        ssh.close()

        print("\n" + "=" * 50)
        if exit_code == 0:
            print("  ALL TESTS PASSED!")
        else:
            print(f"  Installation completed with exit code: {exit_code}")
        print("=" * 50)

    except Exception as e:
        import traceback
        print(f"\nError: {e}")
        traceback.print_exc()
        sys.exit(1)


if __name__ == "__main__":
    main()
