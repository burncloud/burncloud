#!/usr/bin/env python3
"""
上传 BurnCloud CLI 和 Bundle 到阿里云 ECS
使用 SFTP (paramiko) 进行文件传输

依赖:
- paramiko: pip install paramiko

用法:
    python 03_upload_files.py --ip 47.115.88.59
    python 03_upload_files.py --ip 47.115.88.59 --cli-only
"""

import paramiko
import os
import sys
import argparse
import time

# 默认配置
DEFAULT_IP = "47.115.143.233"
USERNAME = "Administrator"
PASSWORD = "Burncloud@Test123"
PORT = 22

# 文件路径
LOCAL_CLI = r"target\release\burncloud.exe"
LOCAL_BUNDLE = r"target\release\openclaw-bundle"
# SFTP 在 Windows 上使用 POSIX 路径格式
REMOTE_DIR = "/C:/burncloud-test"


def progress_callback(transferred: int, total: int):
    """显示上传进度"""
    percent = (transferred / total) * 100
    bar_len = 40
    filled = int(bar_len * transferred / total)
    bar = "=" * filled + " " * (bar_len - filled)
    mb_transferred = transferred / 1024 / 1024
    mb_total = total / 1024 / 1024
    sys.stdout.write(f"\r[{bar}] {percent:.1f}% ({mb_transferred:.1f}/{mb_total:.1f} MB)")
    sys.stdout.flush()


def upload_file(sftp: paramiko.SFTPClient, local_path: str, remote_path: str, desc: str = ""):
    """上传单个文件"""
    file_size = os.path.getsize(local_path)
    print(f"\n{desc}")
    print(f"  Local:  {local_path}")
    print(f"  Remote: {remote_path}")
    print(f"  Size:   {file_size / 1024 / 1024:.2f} MB")

    sftp.put(local_path, remote_path, callback=progress_callback)
    print("\n  Done!")


def upload_directory(sftp: paramiko.SFTPClient, local_dir: str, remote_dir: str, desc: str = ""):
    """上传整个目录"""
    print(f"\n{desc}")
    print(f"  Local:  {local_dir}")
    print(f"  Remote: {remote_dir}")

    # 创建远程目录
    try:
        sftp.mkdir(remote_dir)
    except IOError:
        pass  # 目录已存在

    total_files = 0
    total_size = 0

    for root, dirs, files in os.walk(local_dir):
        # 计算相对路径
        rel_path = os.path.relpath(root, local_dir)
        if rel_path == ".":
            remote_root = remote_dir
        else:
            remote_root = os.path.join(remote_dir, rel_path).replace("\\", "/")

        # 创建子目录
        for d in dirs:
            remote_path = os.path.join(remote_root, d).replace("\\", "/")
            try:
                sftp.mkdir(remote_path)
            except IOError:
                pass

        # 上传文件
        for f in files:
            local_path = os.path.join(root, f)
            remote_path = os.path.join(remote_root, f).replace("\\", "/")
            file_size = os.path.getsize(local_path)
            total_files += 1
            total_size += file_size

            rel_file = os.path.relpath(local_path, local_dir)
            sys.stdout.write(f"\r  [{total_files}] {rel_file} ({file_size / 1024:.1f} KB)")
            sys.stdout.flush()

            sftp.put(local_path, remote_path)

    print(f"\n  Uploaded {total_files} files ({total_size / 1024 / 1024:.2f} MB)")


def execute_command(ssh: paramiko.SSHClient, cmd: str, desc: str = "") -> str:
    """执行 SSH 命令"""
    if desc:
        print(f"\n{desc}")
    stdin, stdout, stderr = ssh.exec_command(cmd, timeout=60)
    out = stdout.read().decode("utf-8", errors="replace")
    err = stderr.read().decode("utf-8", errors="replace")
    exit_code = stdout.channel.recv_exit_status()

    if out.strip():
        print(out)
    if err.strip() and exit_code != 0:
        print(f"[Error] {err}")

    return out


def main():
    parser = argparse.ArgumentParser(description="Upload BurnCloud files to Aliyun ECS")
    parser.add_argument("--ip", default=DEFAULT_IP, help="Server IP address")
    parser.add_argument("--username", default=USERNAME, help="SSH username")
    parser.add_argument("--password", default=PASSWORD, help="SSH password")
    parser.add_argument("--cli-only", action="store_true", help="Only upload CLI")
    parser.add_argument("--bundle", default=LOCAL_BUNDLE, help="Local bundle directory")
    args = parser.parse_args()

    # 检查本地文件
    cli_path = os.path.join(os.getcwd(), LOCAL_CLI)
    if not os.path.exists(cli_path):
        cli_path = LOCAL_CLI  # 尝试绝对路径

    if not os.path.exists(cli_path):
        print(f"Error: burncloud.exe not found at {cli_path}")
        print("Please run 'cargo build --release --bin burncloud' first")
        sys.exit(1)

    bundle_path = args.bundle
    if not args.cli_only and not os.path.exists(bundle_path):
        print(f"Warning: Bundle not found at {bundle_path}")
        print("Will only upload CLI...")
        args.cli_only = True

    print("=" * 50)
    print("  BurnCloud File Uploader")
    print("=" * 50)
    print(f"  Server:   {args.ip}")
    print(f"  Username: {args.username}")
    print(f"  CLI:      {cli_path}")
    if not args.cli_only:
        print(f"  Bundle:   {bundle_path}")
    print()

    try:
        # 连接 SSH
        print("Connecting to server...")
        ssh = paramiko.SSHClient()
        ssh.set_missing_host_key_policy(paramiko.AutoAddPolicy())
        ssh.connect(args.ip, PORT, args.username, args.password, timeout=30)
        print("Connected!")

        sftp = ssh.open_sftp()

        # 创建远程目录
        print(f"\nCreating remote directory: {REMOTE_DIR}")
        try:
            sftp.mkdir(REMOTE_DIR)
        except IOError:
            pass  # 目录已存在

        # 上传 CLI
        remote_cli = f"{REMOTE_DIR}\\burncloud.exe"
        upload_file(sftp, cli_path, remote_cli, "[1/2] Uploading burncloud.exe...")

        # 验证上传
        print("\nVerifying CLI...")
        execute_command(ssh, f"{REMOTE_DIR}\\burncloud.exe --version")

        if not args.cli_only:
            # 上传 Bundle
            remote_bundle = f"{REMOTE_DIR}\\openclaw-bundle"
            upload_directory(sftp, bundle_path, remote_bundle, "\n[2/2] Uploading bundle...")

        sftp.close()
        ssh.close()

        print()
        print("=" * 50)
        print("  Upload Complete!")
        print("=" * 50)
        print()
        print("Next: python 04_install_openclaw.py")

    except Exception as e:
        import traceback
        print(f"\nError: {e}")
        traceback.print_exc()
        sys.exit(1)


if __name__ == "__main__":
    main()
