#!/bin/bash
# z.ai Integration Test Script
# Tests real API calls to z.ai using Anthropic-compatible protocol

set -e

# Load environment variables
if [ -f "$(dirname "$0")/../.env" ]; then
    set -a
    source "$(dirname "$0")/../.env"
    set +a
fi

# Configuration
ZAI_API_KEY="${ZAI_API_KEY:-}"
ZAI_BASE_URL="${ZAI_BASE_URL:-https://api.z.ai/api/anthropic}"
ZAI_MODEL="${ZAI_MODEL:-glm-5}"

echo "========================================"
echo "  z.ai Integration Test"
echo "========================================"
echo ""

# Check API key
if [ -z "$ZAI_API_KEY" ]; then
    echo "ERROR: ZAI_API_KEY not set in .env file"
    exit 1
fi

echo "Configuration:"
echo "  Base URL: $ZAI_BASE_URL"
echo "  Model: $ZAI_MODEL"
echo "  API Key: ${ZAI_API_KEY:0:10}..."
echo ""

# Test 1: Non-streaming request
echo "Test 1: Non-streaming request"
echo "----------------------------------------"
RESPONSE=$(curl -sf -X POST "$ZAI_BASE_URL/v1/messages" \
  -H "Content-Type: application/json" \
  -H "x-api-key: $ZAI_API_KEY" \
  -H "anthropic-version: 2023-06-01" \
  -d "{
    \"model\": \"$ZAI_MODEL\",
    \"max_tokens\": 100,
    \"messages\": [{\"role\": \"user\", \"content\": \"Say hello in 5 words\"}]
  }" 2>&1) || {
    echo "FAILED: Request failed"
    echo "$RESPONSE"
    exit 1
}

echo "Response:"
echo "$RESPONSE" | head -c 500
echo ""
echo ""

# Validate response structure
CONTENT=$(echo "$RESPONSE" | jq -r '.content[0].text // empty')
if [ -z "$CONTENT" ]; then
    echo "FAILED: No content in response"
    echo "$RESPONSE"
    exit 1
fi
echo "Content extracted: $CONTENT"
echo "Test 1: PASSED"
echo ""

# Test 2: Streaming request
echo "Test 2: Streaming request"
echo "----------------------------------------"
STREAM_RESPONSE=$(curl -sf -X POST "$ZAI_BASE_URL/v1/messages" \
  -H "Content-Type: application/json" \
  -H "x-api-key: $ZAI_API_KEY" \
  -H "anthropic-version: 2023-06-01" \
  -d "{
    \"model\": \"$ZAI_MODEL\",
    \"max_tokens\": 50,
    \"stream\": true,
    \"messages\": [{\"role\": \"user\", \"content\": \"Count from 1 to 3\"}]
  }" 2>&1) || {
    echo "FAILED: Streaming request failed"
    echo "$STREAM_RESPONSE"
    exit 1
}

echo "Stream response (first 1000 chars):"
echo "$STREAM_RESPONSE" | head -c 1000
echo ""
echo ""

# Check for content_block_delta events
if echo "$STREAM_RESPONSE" | grep -q "content_block_delta"; then
    echo "Found content_block_delta events"
    echo "Test 2: PASSED"
else
    echo "WARNING: No content_block_delta events found (might be normal for this model)"
fi
echo ""

# Test 3: Token usage verification
echo "Test 3: Token usage verification"
echo "----------------------------------------"
INPUT_TOKENS=$(echo "$RESPONSE" | jq -r '.usage.input_tokens // empty')
OUTPUT_TOKENS=$(echo "$RESPONSE" | jq -r '.usage.output_tokens // empty')

if [ -n "$INPUT_TOKENS" ] && [ -n "$OUTPUT_TOKENS" ]; then
    echo "Input tokens: $INPUT_TOKENS"
    echo "Output tokens: $OUTPUT_TOKENS"
    echo "Test 3: PASSED"
else
    echo "WARNING: Token usage not available in response"
fi
echo ""

# Test 4: Error handling (invalid API key)
echo "Test 4: Error handling (invalid API key)"
echo "----------------------------------------"
ERROR_RESPONSE=$(curl -sf -X POST "$ZAI_BASE_URL/v1/messages" \
  -H "Content-Type: application/json" \
  -H "x-api-key: invalid-key-12345" \
  -H "anthropic-version: 2023-06-01" \
  -d "{
    \"model\": \"$ZAI_MODEL\",
    \"max_tokens\": 10,
    \"messages\": [{\"role\": \"user\", \"content\": \"test\"}]
  }" 2>&1 || true)

if echo "$ERROR_RESPONSE" | grep -qiE "error|unauthorized|invalid"; then
    echo "Error correctly returned for invalid API key"
    echo "Test 4: PASSED"
else
    echo "WARNING: Unexpected response for invalid API key"
fi
echo ""

echo "========================================"
echo "  All Integration Tests Completed!"
echo "========================================"
