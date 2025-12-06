import requests
import json
import time
import sys

# Config
ROUTER_URL = "http://127.0.0.1:3000/v1/chat/completions"
API_KEY = "sk-burncloud-demo"

def test_chat_completion():
    print(f">>> Testing Chat Completion against {ROUTER_URL}...")
    
    headers = {
        "Content-Type": "application/json",
        "Authorization": f"Bearer {API_KEY}"
    }
    
    data = {
        "model": "gpt-3.5-turbo", # Should route to demo-openai or any configured upstream
        "messages": [
            {"role": "user", "content": "Hello, are you working?"}
        ],
        "stream": False
    }
    
    try:
        start = time.time()
        response = requests.post(ROUTER_URL, headers=headers, json=data, timeout=10)
        latency = (time.time() - start) * 1000
        
        print(f"Status Code: {response.status_code}")
        print(f"Latency: {latency:.2f}ms")
        
        if response.status_code == 200:
            print("Response:", json.dumps(response.json(), indent=2))
            print("✅ Test Passed")
            return True
        else:
            print("Error Response:", response.text)
            print("❌ Test Failed")
            return False
            
    except Exception as e:
        print(f"❌ Connection Failed: {e}")
        return False

if __name__ == "__main__":
    if test_chat_completion():
        sys.exit(0)
    else:
        sys.exit(1)
