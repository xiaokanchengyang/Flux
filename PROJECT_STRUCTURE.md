# Flux 项目结构

## 概览

Flux 项目采用了模块化的架构设计，将核心功能、应用程序和测试工具清晰地分离。

```
/workspace/
├── flux/                    # 核心库目录
│   ├── flux-core/          # 核心同步 I/O 库（坚实、稳定、不变）
│   ├── flux-cloud/         # 云适配层（异步、网络、依赖 Tokio）
│   └── flux-testing/       # 测试工具库（共享 fixtures 和 helpers）
│
├── apps/                   # 应用程序目录
│   ├── flux-cli/          # CLI 工具（主要依赖 flux-core，可选依赖 flux-cloud）
│   └── flux-gui/          # GUI 应用（主要依赖 flux-core，可选依赖 flux-cloud）
│
├── benches/               # 性能测试
├── docs/                  # 文档（位于 flux/ 下）
├── examples/              # 示例代码
├── scripts/               # 开发和构建脚本
├── test_data/             # 测试数据
└── Cargo.toml            # 根工作空间配置
```

## 模块说明

### flux-core
核心压缩和归档功能库，提供：
- 多种压缩算法支持（Zstd, Gzip, Brotli, XZ）
- 归档格式支持（TAR, ZIP, 7Z）
- 智能压缩策略
- 元数据保留
- 安全性检查

### flux-cloud
云存储适配层，提供：
- 异步 I/O 支持
- S3、Azure Blob、GCS 等云存储支持
- 流式读写能力
- 基于 Tokio 的高性能网络操作

### flux-testing
测试工具库，提供：
- 通用测试 fixtures
- 断言助手函数
- 测试目录管理
- 平台特定的测试工具

### flux-cli
命令行界面应用，特性：
- 交互式和非交互式模式
- TUI 界面支持
- 云存储集成（可选）
- 批处理能力

### flux-gui
图形用户界面应用，特性：
- 基于 egui 的现代 UI
- 实时进度跟踪
- 拖放支持
- 跨平台原生体验

## 依赖关系

```mermaid
graph TD
    flux-core[flux-core]
    flux-cloud[flux-cloud]
    flux-testing[flux-testing]
    flux-cli[flux-cli]
    flux-gui[flux-gui]
    
    flux-testing --> flux-core
    flux-cli --> flux-core
    flux-cli -.-> flux-cloud
    flux-gui --> flux-core
    flux-gui -.-> flux-cloud
    
    style flux-core fill:#f9f,stroke:#333,stroke-width:4px
```

- 实线箭头：必需依赖
- 虚线箭头：可选依赖

## 开发指南

### 构建整个项目
```bash
cargo build --workspace
```

### 运行特定应用
```bash
# CLI
cargo run --bin flux

# GUI
cargo run --bin flux-gui
```

### 运行测试
```bash
# 所有测试
cargo test --workspace

# 特定包的测试
cargo test -p flux-core
```

### 运行性能测试
```bash
cargo bench
```

## 设计原则

1. **模块化**：每个包都有明确的职责边界
2. **同步核心**：flux-core 保持同步 API，确保稳定性和易用性
3. **异步扩展**：flux-cloud 提供异步能力，但作为可选依赖
4. **共享测试**：flux-testing 避免测试代码重复
5. **应用分离**：CLI 和 GUI 独立开发，共享核心功能