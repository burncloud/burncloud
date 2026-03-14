#!/usr/bin/env python3
"""
快速检查服务器连接状态
"""

import socket
import sys

sys.stdout.reconfigure(encoding='utf-8', errors='replace')

SERVERS = [
    ("47.115.88.59", 22, "Test Server"),
]


def check_port(host: str, port: int, timeout: int = 10) -> bool:
    """检查端口是否开放"""
    try:
        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        sock.settimeout(timeout)
        result = sock.connect_ex((host, port))
        sock.close()
        return result == 0
    except Exception:
        return False


def main():
    print("=" * 50)
    print("  Server Connection Check")
    print("=" * 50)
    print()

    all_down = True
    for host, port, name in SERVERS:
        print(f"Checking {name} ({host}:{port})...", end=" ")
        if check_port(host, port):
            print("ONLINE")
            all_down = False
        else:
            print("OFFLINE")

    print()
    if all_down:
        print("All servers are offline!")
        print()
        print("To start the server:")
        print("  1. Login to Aliyun Console")
        print("  2. Find instance i-wz95gnff6t4gs4fz6cj4")
        print("  3. Click 'Start'")
        print()
        print("Or create a new server:")
        print("  test\\aliyun\\01_create_server.bat")


if __name__ == "__main__":
    main()
