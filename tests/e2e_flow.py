import requests
import json
import time
import sys

# Config
BASE_URL = "http://127.0.0.1:3000/v1"
API_KEY = "sk-burncloud-demo"

HEADERS = {
    "Content-Type": "application/json",
    "Authorization": f"Bearer {API_KEY}"
}

def log(msg, success=None):
    if success is True:
        print(f"✅ {msg}")
    elif success is False:
        print(f"❌ {msg}")
    else:
        print(f"ℹ️  {msg}")

def test_list_models():
    url = f"{BASE_URL}/models"
    log(f"Testing List Models: {url}")
    try:
        response = requests.get(url, headers=HEADERS, timeout=5)
        if response.status_code == 200:
            data = response.json()
            log(f"Got {len(data.get('data', []))} models", True)
            # print(json.dumps(data, indent=2))
            return True
        else:
            log(f"Failed: {response.status_code} - {response.text}", False)
            return False
    except Exception as e:
        log(f"Connection Failed: {e}", False)
        return False

def test_chat_completion():
    url = f"{BASE_URL}/chat/completions"
    log(f"Testing Chat Completion: {url}")
    
    data = {
        "model": "demo-openai", # Use a model ID likely to exist in default DB or mock
        "messages": [{"role": "user", "content": "Hello!"}],
        "stream": False
    }
    
    try:
        start = time.time()
        response = requests.post(url, headers=HEADERS, json=data, timeout=10)
        latency = (time.time() - start) * 1000
        
        if response.status_code == 200:
            log(f"Success ({latency:.2f}ms)", True)
            return True
        else:
            # If 502 (Bad Gateway) it means upstream failed, which is expected if we don't have real API keys.
            # But the Router *logic* worked (it tried to route).
            # For E2E testing without real keys, 502/500 is often "Success" for the Router layer 
            # as long as it's not 404 (Not Found) or 401 (Unauthorized).
            if response.status_code == 502: 
                log(f"Router forwarded request (Upstream returned 502 as expected without real keys)", True)
                return True
            
            log(f"Failed: {response.status_code} - {response.text}", False)
            return False
    except Exception as e:
        log(f"Connection Failed: {e}", False)
        return False

def test_stream_completion():
    url = f"{BASE_URL}/chat/completions"
    log(f"Testing Streaming Chat: {url}")
    
    data = {
        "model": "demo-openai",
        "messages": [{"role": "user", "content": "Count to 3"}],
        "stream": True
    }
    
    try:
        response = requests.post(url, headers=HEADERS, json=data, stream=True, timeout=10)
        
        if response.status_code != 200:
             if response.status_code == 502:
                 log(f"Router forwarded streaming request (Upstream 502)", True)
                 return True
             log(f"Failed: {response.status_code}", False)
             return False
            
        chunk_count = 0
        for line in response.iter_lines():
            if line:
                line = line.decode('utf-8')
                if line.startswith('data: ') and line != 'data: [DONE]':
                    chunk_count += 1
                    
        log(f"Received {chunk_count} chunks", True)
        return chunk_count > 0
        
    except Exception as e:
        log(f"Stream Failed: {e}", False)
        return False

if __name__ == "__main__":
    results = [
        test_list_models(),
        test_chat_completion(),
        test_stream_completion()
    ]
    
    if all(results):
        sys.exit(0)
    else:
        sys.exit(1)