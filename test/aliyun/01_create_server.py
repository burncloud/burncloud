#!/usr/bin/env python3
"""
创建阿里云 Windows ECS 服务器
直接使用阿里云 API，无需安装 aliyun CLI

用法:
    py -3 01_create_server.py
    py -3 01_create_server.py --delete-existing  # 删除现有实例后创建新的
"""

import sys
import os
import json
import argparse

sys.stdout.reconfigure(encoding='utf-8', errors='replace')

# 添加当前目录到路径
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from aliyun_api import AliyunECS, create_test_server


def load_aliyun_config():
    """从配置文件加载阿里云凭证"""
    config_path = os.path.expanduser('~/.aliyun/config.json')

    if not os.path.exists(config_path):
        print(f'错误: 配置文件不存在: {config_path}')
        print('请先运行: aliyun configure')
        sys.exit(1)

    with open(config_path, 'r', encoding='utf-8') as f:
        config = json.load(f)

    current_profile = config.get('current', 'default')
    for profile in config.get('profiles', []):
        if profile.get('name') == current_profile:
            return {
                'access_key_id': profile.get('access_key_id'),
                'access_key_secret': profile.get('access_key_secret'),
                'region_id': profile.get('region_id', 'cn-shenzhen'),
            }

    print(f'错误: 找不到配置文件中的 profile: {current_profile}')
    sys.exit(1)


def main():
    parser = argparse.ArgumentParser(description='创建阿里云 ECS Windows 测试服务器')
    parser.add_argument('--delete-existing', action='store_true', help='删除现有的 burncloud-test 实例')
    parser.add_argument('--password', default='Burncloud@Test123', help='Windows 登录密码')
    parser.add_argument('--region', default='cn-shenzhen', help='阿里云区域')
    args = parser.parse_args()

    print('=' * 50)
    print('  阿里云 ECS 服务器创建工具')
    print('=' * 50)

    # 加载配置
    config = load_aliyun_config()
    print(f'\n区域: {config["region_id"]}')

    ecs = AliyunECS(
        access_key_id=config['access_key_id'],
        access_key_secret=config['access_key_secret'],
        region_id=config['region_id'],
    )

    # 检查现有实例
    instances = ecs.list_instances()
    existing = [i for i in instances if 'burncloud' in i.get('InstanceName', '').lower()]

    if existing:
        print(f'\n发现 {len(existing)} 个现有 burncloud 实例:')
        for inst in existing:
            ips = inst.get('PublicIpAddress', {}).get('IpAddress', [])
            print(f"  - {inst['InstanceId']}: {inst.get('Status')}, IP: {ips[0] if ips else 'N/A'}")

        if args.delete_existing:
            print('\n删除现有实例...')
            for inst in existing:
                print(f"  删除 {inst['InstanceId']}...")
                ecs.delete_instance(inst['InstanceId'], force=True)
            import time
            time.sleep(5)
        else:
            print('\n提示: 使用 --delete-existing 参数删除现有实例后创建新的')
            print('或者直接使用现有实例进行测试')

            # 返回现有实例信息
            inst = existing[0]
            ips = inst.get('PublicIpAddress', {}).get('IpAddress', [])
            if ips and inst.get('Status') == 'Running':
                print(f'\n=== 现有服务器信息 ===')
                print(f'Instance ID: {inst["InstanceId"]}')
                print(f'Public IP:   {ips[0]}')
                print(f'Username:    Administrator')
                print(f'Password:    {args.password}')
            return

    # 创建新服务器
    print('\n' + '=' * 50)
    print('  创建新服务器')
    print('=' * 50)

    result = create_test_server(
        access_key_id=config['access_key_id'],
        access_key_secret=config['access_key_secret'],
        region_id=config['region_id'],
        password=args.password,
        instance_name='burncloud-test',
    )

    print('\n' + '=' * 50)
    print('  服务器创建成功!')
    print('=' * 50)
    print(f'Instance ID: {result["instance_id"]}')
    print(f'Public IP:   {result["public_ip"]}')
    print(f'Username:    {result["username"]}')
    print(f'Password:    {result["password"]}')
    print('=' * 50)
    print('\n下一步:')
    print('  1. 等待 3-5 分钟让 Windows 完全启动')
    print('  2. 运行: 03_upload_files.bat')
    print(f'     (或手动设置 IP: {result["public_ip"]})')


if __name__ == '__main__':
    main()
