#!/usr/bin/env python3
"""
阿里云 ECS API 封装
直接使用 HTTP 请求调用阿里云 API，无需安装 aliyun CLI

使用方法:
    from aliyun_api import AliyunECS

    ecs = AliyunECS(
        access_key_id='your-access-key-id',
        access_key_secret='your-access-key-secret',
        region_id='cn-shenzhen'
    )

    # 创建实例
    instance_id = ecs.create_windows_instance(
        password='YourPassword123!',
        instance_name='burncloud-test'
    )

    # 启动实例
    ecs.start_instance(instance_id)

    # 等待实例就绪
    ip = ecs.wait_for_instance_ready(instance_id)
"""

import hashlib
import hmac
import urllib.request
import urllib.parse
import json
import base64
import datetime
import sys
import time

sys.stdout.reconfigure(encoding='utf-8', errors='replace')


class AliyunECS:
    """阿里云 ECS API 客户端"""

    def __init__(self, access_key_id: str, access_key_secret: str, region_id: str = 'cn-shenzhen'):
        self.access_key_id = access_key_id
        self.access_key_secret = access_key_secret
        self.region_id = region_id
        self.endpoint = 'ecs.aliyuncs.com'

    def _sign(self, params: dict) -> str:
        """生成 API 签名"""
        # 排序参数
        sorted_params = sorted(params.items())
        query_string = '&'.join([
            f'{urllib.parse.quote(str(k), safe="")}={urllib.parse.quote(str(v), safe="")}'
            for k, v in sorted_params
        ])

        # 构造签名字符串
        string_to_sign = 'GET&%2F&' + urllib.parse.quote(query_string, safe='')

        # HMAC-SHA1 签名
        key = (self.access_key_secret + '&').encode('utf-8')
        signature = base64.b64encode(
            hmac.new(key, string_to_sign.encode('utf-8'), hashlib.sha1).digest()
        ).decode('utf-8')

        return signature

    def _call_api(self, action: str, params: dict) -> dict:
        """调用阿里云 API"""
        timestamp = datetime.datetime.now(datetime.UTC).strftime('%Y-%m-%dT%H:%M:%SZ')
        nonce = str(time.time()) + str(hash(timestamp) % 10000)

        public_params = {
            'Format': 'JSON',
            'Version': '2014-05-26',
            'AccessKeyId': self.access_key_id,
            'SignatureMethod': 'HMAC-SHA1',
            'Timestamp': timestamp,
            'SignatureVersion': '1.0',
            'SignatureNonce': nonce,
            'Action': action,
            'RegionId': self.region_id,
        }

        all_params = {**public_params, **params}
        signature = self._sign(all_params)
        all_params['Signature'] = signature

        url = f'https://{self.endpoint}/?{urllib.parse.urlencode(all_params)}'

        try:
            req = urllib.request.Request(url)
            with urllib.request.urlopen(req, timeout=120) as response:
                return json.loads(response.read().decode('utf-8'))
        except urllib.error.HTTPError as e:
            error_body = e.read().decode('utf-8')
            return json.loads(error_body)

    def list_instances(self) -> list:
        """列出所有实例"""
        result = self._call_api('DescribeInstances', {'PageSize': '50'})
        return result.get('Instances', {}).get('Instance', [])

    def get_instance(self, instance_id: str) -> dict:
        """获取实例详情"""
        result = self._call_api('DescribeInstances', {
            'InstanceIds': json.dumps([instance_id])
        })
        instances = result.get('Instances', {}).get('Instance', [])
        return instances[0] if instances else {}

    def create_windows_instance(
        self,
        password: str,
        instance_name: str = 'burncloud-test',
        instance_type: str = 'ecs.g7.large',
        zone_id: str = None,
        vswitch_id: str = None,
        security_group_id: str = None,
        disk_size: int = 40,
        bandwidth: int = 5,
    ) -> str:
        """
        创建 Windows ECS 实例

        Args:
            password: Windows 登录密码
            instance_name: 实例名称
            instance_type: 实例规格
            zone_id: 可用区 ID (如不指定则自动选择)
            vswitch_id: 交换机 ID (如不指定则自动选择)
            security_group_id: 安全组 ID (如不指定则自动选择)
            disk_size: 系统盘大小 (GB)
            bandwidth: 公网带宽 (Mbps)

        Returns:
            实例 ID
        """
        # 获取默认资源配置
        if not zone_id or not vswitch_id or not security_group_id:
            resources = self._get_available_resources()
            zone_id = zone_id or resources.get('zone_id')
            vswitch_id = vswitch_id or resources.get('vswitch_id')
            security_group_id = security_group_id or resources.get('security_group_id')

        if not all([zone_id, vswitch_id, security_group_id]):
            raise Exception('无法获取可用的网络资源，请手动指定 zone_id, vswitch_id, security_group_id')

        # Windows Server 2025 镜像
        image_id = 'win2025_24H2_x64_dtc_zh-cn_40G_alibase_20260211.vhd'

        print(f'Creating instance...')
        print(f'  Zone: {zone_id}')
        print(f'  Type: {instance_type}')
        print(f'  Image: Windows Server 2025')
        print(f'  Bandwidth: {bandwidth} Mbps')

        # 使用 RunInstances 创建并启动
        result = self._call_api('RunInstances', {
            'ZoneId': zone_id,
            'InstanceType': instance_type,
            'ImageId': image_id,
            'VSwitchId': vswitch_id,
            'SecurityGroupId': security_group_id,
            'InstanceName': instance_name,
            'Password': password,
            'InternetMaxBandwidthOut': str(bandwidth),
            'InternetChargeType': 'PayByBandwidth',
            'SystemDisk.Category': 'cloud_essd',
            'SystemDisk.Size': str(disk_size),
            'Amount': '1',
        })

        if 'InstanceIdSets' in result:
            instance_ids = result['InstanceIdSets'].get('InstanceIdSet', [])
            if instance_ids:
                return instance_ids[0]

        raise Exception(f"创建实例失败: {result.get('Message', result)}")

    def _get_available_resources(self) -> dict:
        """获取可用的网络资源"""
        resources = {}

        # 获取 VSwitch
        result = self._call_api('DescribeVSwitches', {})
        vswitches = result.get('VSwitches', {}).get('VSwitch', [])
        if vswitches:
            vs = vswitches[0]
            resources['vswitch_id'] = vs['VSwitchId']
            resources['zone_id'] = vs['ZoneId']

        # 获取安全组
        result = self._call_api('DescribeSecurityGroups', {})
        sgs = result.get('SecurityGroups', {}).get('SecurityGroup', [])
        if sgs:
            resources['security_group_id'] = sgs[0]['SecurityGroupId']

        return resources

    def start_instance(self, instance_id: str) -> bool:
        """启动实例"""
        result = self._call_api('StartInstance', {'InstanceId': instance_id})
        return 'RequestId' in result and 'Code' not in result

    def stop_instance(self, instance_id: str, force: bool = False) -> bool:
        """停止实例"""
        result = self._call_api('StopInstance', {
            'InstanceId': instance_id,
            'ForceStop': 'true' if force else 'false'
        })
        return 'RequestId' in result and 'Code' not in result

    def delete_instance(self, instance_id: str, force: bool = False) -> bool:
        """删除实例"""
        result = self._call_api('DeleteInstance', {
            'InstanceId': instance_id,
            'Force': 'true' if force else 'false'
        })
        return 'RequestId' in result and 'Code' not in result

    def wait_for_instance_ready(self, instance_id: str, timeout: int = 300) -> str:
        """
        等待实例就绪并返回公网 IP

        Args:
            instance_id: 实例 ID
            timeout: 超时时间 (秒)

        Returns:
            公网 IP 地址
        """
        print(f'Waiting for instance {instance_id}...')
        start_time = time.time()

        while time.time() - start_time < timeout:
            inst = self.get_instance(instance_id)
            state = inst.get('Status', 'Unknown')
            ips = inst.get('PublicIpAddress', {}).get('IpAddress', [])

            elapsed = int(time.time() - start_time)
            print(f'  [{elapsed}s] Status: {state}, IP: {ips[0] if ips else "waiting..."}')

            if state == 'Running' and ips:
                return ips[0]

            time.sleep(5)

        raise Exception(f'等待实例就绪超时 ({timeout}s)')

    def install_ssh_via_cloud_assistant(self, instance_id: str) -> bool:
        """通过云助手安装 OpenSSH Server"""
        install_script = '''
Add-WindowsCapability -Online -Name OpenSSH.Server~~~~0.0.1.0 -ErrorAction SilentlyContinue
Start-Service sshd -ErrorAction SilentlyContinue
Set-Service -Name sshd -StartupType 'Automatic' -ErrorAction SilentlyContinue
if (!(Get-NetFirewallRule -Name 'OpenSSH-Server-In-TCP' -ErrorAction SilentlyContinue)) {
    New-NetFirewallRule -Name 'OpenSSH-Server-In-TCP' -DisplayName 'OpenSSH Server' -Enabled True -Direction Inbound -Protocol TCP -Action Allow -LocalPort 22 -ErrorAction SilentlyContinue
}
Write-Host 'SSH Server installed successfully!'
'''

        result = self._call_api('RunCommand', {
            'Type': 'RunPowerShellScript',
            'InstanceId.1': instance_id,
            'CommandContent': install_script,
            'Timeout': '300',
        })

        if 'InvokeId' in result:
            print(f"SSH installation command sent (InvokeId: {result['InvokeId']})")
            return True

        print(f"Failed to send SSH installation command: {result.get('Message', result)}")
        return False


# 便捷函数
def create_test_server(
    access_key_id: str,
    access_key_secret: str,
    region_id: str = 'cn-shenzhen',
    password: str = 'Burncloud@Test123',
    instance_name: str = 'burncloud-test',
) -> dict:
    """
    创建测试服务器的便捷函数

    Returns:
        dict with keys: instance_id, public_ip, username, password
    """
    ecs = AliyunECS(access_key_id, access_key_secret, region_id)

    # 创建实例
    instance_id = ecs.create_windows_instance(
        password=password,
        instance_name=instance_name,
    )
    print(f'Instance created: {instance_id}')

    # 等待就绪
    public_ip = ecs.wait_for_instance_ready(instance_id)
    print(f'Instance ready: {public_ip}')

    # 安装 SSH
    print('Installing SSH via Cloud Assistant...')
    ecs.install_ssh_via_cloud_assistant(instance_id)

    # 等待 SSH 安装完成
    print('Waiting for SSH installation (60 seconds)...')
    time.sleep(60)

    return {
        'instance_id': instance_id,
        'public_ip': public_ip,
        'username': 'Administrator',
        'password': password,
    }


if __name__ == '__main__':
    # 测试代码
    print('=' * 50)
    print('  Aliyun ECS API Test')
    print('=' * 50)

    # 从配置文件读取
    import os
    config_path = os.path.expanduser('~/.aliyun/config.json')

    if os.path.exists(config_path):
        with open(config_path, 'r') as f:
            config = json.load(f)

        current_profile = config.get('current', 'default')
        for profile in config.get('profiles', []):
            if profile.get('name') == current_profile:
                access_key_id = profile.get('access_key_id')
                access_key_secret = profile.get('access_key_secret')
                region_id = profile.get('region_id', 'cn-shenzhen')
                break
        else:
            print('Profile not found!')
            sys.exit(1)

        # 列出实例
        ecs = AliyunECS(access_key_id, access_key_secret, region_id)
        instances = ecs.list_instances()

        print(f'\n=== Instances in {region_id} ===')
        for inst in instances:
            ips = inst.get('PublicIpAddress', {}).get('IpAddress', [])
            print(f"  {inst['InstanceId']} - {inst.get('Status')} - {ips[0] if ips else 'No IP'}")

    else:
        print(f'Config file not found: {config_path}')
        print('Please configure aliyun CLI first: aliyun configure')
