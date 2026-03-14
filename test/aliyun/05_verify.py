#!/usr/bin/env python3
"""
验证阿里云 ECS 上的 BurnCloud 安装状态
快速检查所有组件是否正常工作

依赖:
- paramiko: pip install paramiko

用法:
    python 05_verify.py --ip 47.115.88.59
"""

import paramiko
import sys
import argparse

# 默认配置
DEFAULT_IP = "47.115.143.233"
USERNAME = "Administrator"
PASSWORD = "Burncloud@Test123"
PORT = 22


def execute_command(ssh: paramiko.SSHClient, cmd: str, timeout: int = 30) -> str:
    """执行 SSH 命令"""
    stdin, stdout, stderr = ssh.exec_command(cmd, timeout=timeout)
    out = stdout.read().decode("utf-8", errors="replace")
    err = stderr.read().decode("utf-8", errors="replace")
    return out.strip(), err.strip()


def main():
    parser = argparse.ArgumentParser(description="Verify BurnCloud installation on Aliyun ECS")
    parser.add_argument("--ip", default=DEFAULT_IP, help="Server IP address")
    parser.add_argument("--username", default=USERNAME, help="SSH username")
    parser.add_argument("--password", default=PASSWORD, help="SSH password")
    args = parser.parse_args()

    print("=" * 50)
    print("  BurnCloud Installation Verification")
    print("=" * 50)
    print(f"  Server: {args.ip}")
    print()

    try:
        # 连接 SSH
        print("Connecting...")
        ssh = paramiko.SSHClient()
        ssh.set_missing_host_key_policy(paramiko.AutoAddPolicy())
        ssh.connect(args.ip, PORT, args.username, args.password, timeout=30)
        print("Connected!\n")

        # 设置 Node.js PATH
        node_path = r'C:\Users\Administrator\AppData\Roaming\fnm\node-versions\v24.14.0\installation'
        path_prefix = f'set "PATH={node_path};%PATH%" && '

        # 检查各项
        all_passed = True
        checks = [
            ("node --version", "Node.js"),
            ("npm --version", "npm"),
            ("openclaw --version", "OpenClaw"),
            ("fnm --version", "fnm"),
            ("git --version", "Git"),
        ]

        print("=== Component Verification ===")
        for cmd, name in checks:
            full_cmd = path_prefix + cmd
            out, err = execute_command(ssh, full_cmd)
            if out and not out.startswith("'") and "not found" not in out.lower():
                print(f"  ✓ {name}: {out}")
            else:
                print(f"  ✗ {name}: {out or err or 'Not found'}")
                all_passed = False

        # 检查安装状态
        print("\n=== Installation Status ===")
        cmd = r'C:\burncloud-test\burncloud.exe install openclaw --status'
        out, err = execute_command(ssh, cmd)
        print(out if out else "No status output")
        if err:
            print(f"Error: {err}")

        ssh.close()

        print()
        if all_passed:
            print("=== ALL TESTS PASSED ===")
        else:
            print("=== SOME TESTS FAILED ===")

    except Exception as e:
        import traceback
        print(f"Error: {e}")
        traceback.print_exc()
        sys.exit(1)


if __name__ == "__main__":
    main()
