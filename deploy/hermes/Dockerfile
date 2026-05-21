# BurnCloud 开发环境镜像
# 包含 Rust 工具链和常用开发工具

FROM rust:1.88-slim

# 安装系统依赖
RUN apt-get update && apt-get install -y \
    git \
    curl \
    jq \
    ripgrep \
    pkg-config \
    libssl-dev \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

# 安装 Claude Code CLI（使用 npm）
RUN curl -fsSL https://deb.nodesource.com/setup_20.x | bash - && \
    apt-get install -y nodejs && \
    npm install -g @anthropic-ai/claude-code && \
    rm -rf /var/lib/apt/lists/*

# 安装 GitHub CLI
RUN curl -fsSL https://cli.github.com/packages/githubcli-archive-keyring.gpg | dd of=/usr/share/keyrings/githubcli-archive-keyring.gpg && \
    chmod go+r /usr/share/keyrings/githubcli-archive-keyring.gpg && \
    echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/githubcli-archive-keyring.gpg] https://cli.github.com/packages stable main" | tee /etc/apt/sources.list.d/github-cli.list > /dev/null && \
    apt-get update && \
    apt-get install -y gh && \
    rm -rf /var/lib/apt/lists/*

# 设置工作目录
WORKDIR /workspace

# 默认命令
CMD ["/bin/bash"]
