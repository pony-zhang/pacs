# 技术架构文档（最佳实践版）

使用 Rust 来构建一个 PACS 系统是在性能、内存安全和并发性方面非常突出。
Rust 生态中可能会用到的核心技术包。

---

### 通用与基础技术

这些是贯穿整个系统的基石，几乎所有模块都会用到。

*   **`tokio`**: 异步运行时。PACS 是一个 I/O 密集型系统（网络通信、磁盘读写），`tokio` 是构建高性能异步网络服务的标准，是整个项目的动力核心。
*   **`serde`**: 序列化与反序列化框架。用于处理 JSON、TOML、YAML、MessagePack 等各种数据格式，是配置管理、API 接口和数据交换的必备工具。
*   **`tracing`**: 结构化日志和 instrumentation。相比 `log`，`tracing` 提供了更强大的上下文感知能力，非常适合在复杂的异步系统中追踪请求链路和诊断问题。
*   **`anyhow` / `thiserror`**: 错误处理。
    *   `anyhow`: 用于应用层，提供 `anyhow::Context` 方便地为错误添加上下文信息。
    *   `thiserror`: 用于库层，方便地创建自定义的错误类型。
*   **`clap`**: 命令行参数解析器，用于构建功能丰富的管理工具。

---

### 1. DICOM 服务模块

这是最核心的专业模块。

*   **`dicom-rs`**: 这是 Rust 生态中处理 DICOM 的“全家桶”。
    *   `dicom-core`: 核心数据结构和抽象。
    *   `dicom-dictionary`: DICOM 数据字典。
    *   `dicom-encoding`: 处理 DICOM 的各种传输语法和编码。
    *   `dicom-ul`: **关键**。实现了 DICOM Upper Layer 协议，用于构建 C-STORE, C-FIND, C-MOVE 等服务端和客户端。
    *   `dicom-transfer-syntax-registry`: 管理和注册传输语法。
    *   `dicom-object`: 用于解析和构建 DICOM 文件对象。

---

### 2. 影像存储与归档模块

*   **文件系统操作**:
    *   `std::fs`: Rust 标准库，提供基础的文件操作。
    *   `walkdir`: 用于高效地遍历目录结构，在数据迁移或校验时很有用。
*   **对象存储**:
    *   **`object_store`**: **强烈推荐**。一个统一的、与厂商无关的对象存储抽象层，支持 AWS S3, Google Cloud Storage, Azure Blob Storage, 以及本地文件系统。这对于构建可扩展的归档系统至关重要。
    *   `aws-sdk-s3`: 如果深度绑定 AWS，可以使用官方 SDK。
*   **数据完整性**:
    *   `sha2`, `md5`: 用于计算文件哈希值，确保数据在传输和存储过程中未损坏。
*   **压缩**:
    *   `zstd`, `flate2`: 用于在归档时对数据进行压缩，节省存储空间。

---

### 3. 数据库与元数据管理模块

*   **数据库驱动**:
    *   **`sqlx`**: **首选**。一个现代的、异步的 SQL 工具包，支持 PostgreSQL, MySQL, SQLite, MariaDB。其最大的优势是**编译时检查查询**，能极大减少运行时错误。它内置了连接池。
*   **连接池**:
    *   `sqlx` 内置连接池。

---

### 4. 工作流引擎模块

这个模块更多是业务逻辑，但可以用一些库来辅助。

*   **状态机**:
    *   `sm`: 一个简单的、类型安全的状态机宏库，可以用来为检查的生命周期建模（如“已到达” -> “诊断中” -> “已签发”）。
*   **任务队列**:
    *   **`lapin`**: 用于与 RabbitMQ 通信，实现可靠的任务队列。例如，将影像自动路由、数据归档等耗时操作放入队列中异步处理。

---

### 5. 系统集成与接口模块

*   **Web 框架**:
    *   **`axum`**: **强烈推荐**。由 `tokio` 团队开发，与 `tokio` 和 `tower` 生态无缝集成，模型、类型安全、性能优异。
*   **gRPC**:
    *   **`tonic`**: Rust 生态中最成熟的 gRPC 实现，基于 `http-proto` 和 `tokio`，性能极高。非常适合内部微服务之间的高效通信。
*   **HL7 v2 处理**:
    *   `hl7-parser`: 用于解析 HL7 v2.x 消息的库。虽然生态不如 DICOM 成熟，但已有可用的选项。

---

### 6. Web 与应用服务模块

*   **DICOM Web 服务**:
    *   **`dicom-web`**: **关键**。`dicom-rs` 生态的一部分，专门用于实现 DICOMweb 标准 (QIDO-RS, WADO-RS, STOW-RS)。可以直接与 `axum` 或 `actix-web` 集成，快速构建符合标准的 Web 服务。
*   **认证与授权**:
    *   `jsonwebtoken`: 用于处理 JWT (JSON Web Tokens)。
    *   `argon2` / `bcrypt`: 用于密码哈希，`argon2` 是更现代、更安全的选择。
*   **静态文件服务**:
    *   `tower-http::ServeDir`: 如果使用 `axum`，这是提供静态文件（如 Web Viewer 的 JS/CSS 文件）的标准方式。

---

### 7. 系统管理与运维模块

*   **配置管理**:
    *   `config`: 支持多种格式（TOML, JSON, YAML, ENV）的配置文件加载库，非常灵活。
*   **监控与指标**:
    *   `prometheus`: 用于暴露 Prometheus 格式的指标，方便与监控系统（如 Prometheus + Grafana）集成，监控服务健康状况、请求延迟、队列长度等。
*   **健康检查**:
    *   可以通过 Web 框架轻松实现一个 `/health` 端点，返回数据库连接、磁盘空间等关键组件的状态。

### 总结

构建一个 PACS 系统，你的 Rust 技术栈可能如下所示：

*   **核心**: `tokio` + `dicom-rs`
*   **API/Web**: `axum` + `dicom-web` + `serde`
*   **数据库**: `sqlx` (PostgreSQL)
*   **存储**: `object_store` (S3/本地)
*   **工作流**: `lapin` (RabbitMQ)
*   **运维**: `tracing` + `prometheus` + `config`

这个组合充分利用了 Rust 在安全、并发和性能上的优势，并且所选的库大多是各自领域内现代化、维护良好的佼佼者，能够为构建一个企业级的 PACS 系统提供坚实的技术基础。
