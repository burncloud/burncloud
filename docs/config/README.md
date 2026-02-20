# Configuration Files

This directory contains configuration files for BurnCloud pricing and other settings.

## Pricing Configuration

### pricing.example.json

A comprehensive example of the pricing configuration format, demonstrating:

- **Multi-currency pricing**: USD and CNY prices for each model
- **Standard pricing**: Input/output prices per 1M tokens
- **Cache pricing**: Prompt Caching prices (typically 10% of standard for cache reads)
- **Batch pricing**: Batch API prices (typically 50% of standard)
- **Tiered pricing**: Usage-based pricing tiers (for models like Qwen)
- **Model metadata**: Context window, capabilities, provider info

To use this configuration:
```bash
# Copy and customize
cp config/pricing.example.json config/pricing.json

# Validate the configuration
burncloud pricing validate config/pricing.json

# Import the prices
burncloud pricing import config/pricing.json
```

### tiered_pricing_example.json

Example tiered pricing configuration for models with usage-based tiers:

- **Qwen Max**: 3 tiers (0-32K, 32K-128K, 128K-252K) for both cn and international regions
- **Qwen Plus**: 3 tiers for both cn and international regions
- **DeepSeek Chat**: Single tier (no limit)

To import tiered pricing:
```bash
burncloud tiered import-tiered config/tiered_pricing_example.json
```

## Pricing Types Explained

### Standard Pricing
The base price per 1M tokens for input and output.

### Cache Pricing (Prompt Caching)
- **cache_read_input_price**: Price for tokens served from cache (typically 10% of input price)
- **cache_creation_input_price**: Price for writing tokens to cache

Supported by: Claude 3.5 Sonnet, GPT-4o, etc.

### Batch Pricing
Prices for Batch API requests, typically 50% discount:
- **batch_input_price**: Input price per 1M tokens for batch requests
- **batch_output_price**: Output price per 1M tokens for batch requests

### Tiered Pricing
Usage-based pricing where the price increases with token usage:
- **tier_start**: Starting token count (inclusive)
- **tier_end**: Ending token count (exclusive, null = no limit)
- **input_price**: Input price per 1M tokens for this tier
- **output_price**: Output price per 1M tokens for this tier

Example: Qwen charges more for longer context lengths.

### Priority Pricing
Premium pricing for high-priority requests (typically 170% of standard):
- **priority_input_price**: Input price for priority requests
- **priority_output_price**: Output price for priority requests

### Audio Pricing
Pricing for audio input tokens (typically 7x text price):
- **audio_input_price**: Price per 1M audio input tokens

## Region Support

Some models have different pricing for different regions:
- **cn**: China domestic pricing (typically 30% of international)
- **international**: International pricing
- **null**: Universal pricing (applies to all regions)

Configure the region in the channel settings to use region-specific pricing.
