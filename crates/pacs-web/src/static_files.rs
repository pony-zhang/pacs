//! 静态文件服务模块

use axum::{
    extract::Path,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};
use pacs_core::Result;
use std::path::PathBuf;
use tokio::fs;
use tower_http::services::ServeDir;
use tracing::{error, info};

/// 静态文件配置
pub struct StaticFileConfig {
    pub root_dir: PathBuf,
    pub index_file: String,
    pub enable_directory_listing: bool,
}

impl Default for StaticFileConfig {
    fn default() -> Self {
        Self {
            root_dir: PathBuf::from("static"),
            index_file: "index.html".to_string(),
            enable_directory_listing: false,
        }
    }
}

/// 创建静态文件服务
pub fn create_static_service() -> ServeDir {
    // 首先确保static目录存在
    if let Err(e) = std::fs::create_dir_all("static") {
        error!("Failed to create static directory: {}", e);
    }

    // 创建一些基础静态文件
    create_default_static_files();

    ServeDir::new("static").append_index_html_on_directories(true)
}

/// 创建默认的静态文件
fn create_default_static_files() {
    // 创建index.html
    let index_html = r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>PACS Web Interface</title>
    <style>
        * {
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }

        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            min-height: 100vh;
            color: #333;
        }

        .container {
            max-width: 1200px;
            margin: 0 auto;
            padding: 20px;
        }

        .header {
            text-align: center;
            margin-bottom: 40px;
            color: white;
        }

        .header h1 {
            font-size: 2.5rem;
            margin-bottom: 10px;
            text-shadow: 0 2px 4px rgba(0,0,0,0.3);
        }

        .header p {
            font-size: 1.2rem;
            opacity: 0.9;
        }

        .cards {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
            gap: 20px;
            margin-bottom: 40px;
        }

        .card {
            background: white;
            border-radius: 10px;
            padding: 30px;
            box-shadow: 0 10px 30px rgba(0,0,0,0.1);
            transition: transform 0.3s ease, box-shadow 0.3s ease;
        }

        .card:hover {
            transform: translateY(-5px);
            box-shadow: 0 15px 40px rgba(0,0,0,0.15);
        }

        .card h2 {
            color: #667eea;
            margin-bottom: 15px;
            font-size: 1.5rem;
        }

        .card p {
            line-height: 1.6;
            color: #666;
            margin-bottom: 20px;
        }

        .card .endpoint {
            background: #f8f9fa;
            padding: 10px 15px;
            border-radius: 5px;
            font-family: 'Courier New', monospace;
            font-size: 0.9rem;
            margin: 5px 0;
            border-left: 3px solid #667eea;
        }

        .api-section {
            background: white;
            border-radius: 10px;
            padding: 30px;
            box-shadow: 0 10px 30px rgba(0,0,0,0.1);
            margin-bottom: 20px;
        }

        .api-section h3 {
            color: #333;
            margin-bottom: 20px;
            font-size: 1.3rem;
        }

        .method-badge {
            display: inline-block;
            padding: 4px 8px;
            border-radius: 4px;
            font-size: 0.8rem;
            font-weight: bold;
            margin-right: 10px;
        }

        .method-get { background: #28a745; color: white; }
        .method-post { background: #007bff; color: white; }
        .method-put { background: #ffc107; color: #212529; }
        .method-delete { background: #dc3545; color: white; }

        .footer {
            text-align: center;
            margin-top: 40px;
            color: white;
            opacity: 0.8;
        }
    </style>
</head>
<body>
    <div class="container">
        <header class="header">
            <h1>🏥 PACS Web Interface</h1>
            <p>医学影像存档与通信系统 - Web API服务</p>
        </header>

        <div class="cards">
            <div class="card">
                <h2>🔐 认证服务</h2>
                <p>用户登录和身份验证服务</p>
                <div class="endpoint">POST /auth/login - 用户登录</div>
                <div class="endpoint">GET /auth/me - 获取当前用户信息</div>
                <div class="endpoint">GET /auth/users - 获取所有用户（管理员）</div>
            </div>

            <div class="card">
                <h2>📊 RESTful API</h2>
                <p>REST风格的医疗数据查询接口</p>
                <div class="endpoint">GET /api/v1/patients - 查询患者</div>
                <div class="endpoint">GET /api/v1/studies - 查询检查</div>
                <div class="endpoint">GET /api/v1/series - 查询序列</div>
                <div class="endpoint">GET /api/v1/instances - 查询实例</div>
            </div>

            <div class="card">
                <h2>🏥 DICOMweb</h2>
                <p>标准DICOMweb协议服务</p>
                <div class="endpoint">GET /dicom-web/search - QIDO-RS查询</div>
                <div class="endpoint">GET /dicom-web/retrieve/* - WADO-RS检索</div>
                <div class="endpoint">POST /dicom-web/store - STOW-RS存储</div>
            </div>

            <div class="card">
                <h2>🔧 系统服务</h2>
                <p>系统状态和健康检查服务</p>
                <div class="endpoint">GET /health - 健康检查</div>
                <div class="endpoint">GET / - API信息</div>
                <div class="endpoint">GET /static/* - 静态文件</div>
            </div>
        </div>

        <div class="api-section">
            <h3>📖 API使用说明</h3>
            <p><strong>1. 用户登录：</strong></p>
            <div class="endpoint">
                POST /auth/login<br>
                Content-Type: application/json<br>
                { "username": "admin", "password": "admin" }
            </div>

            <p style="margin-top: 20px;"><strong>2. 添加认证头：</strong></p>
            <div class="endpoint">
                Authorization: Bearer &lt;your_token_here&gt;
            </div>

            <p style="margin-top: 20px;"><strong>3. 访问API：</strong></p>
            <div class="endpoint">
                GET /api/v1/patients?limit=10&offset=0<br>
                GET /dicom-web/search?level=study&limit=20
            </div>
        </div>

        <div class="api-section">
            <h3>👥 默认用户账户</h3>
            <div class="card" style="margin: 10px 0;">
                <strong>管理员：</strong> admin / admin
            </div>
            <div class="card" style="margin: 10px 0;">
                <strong>放射科医生：</strong> radiologist / radiologist
            </div>
            <div class="card" style="margin: 10px 0;">
                <strong>技师：</strong> tech / tech
            </div>
        </div>

        <footer class="footer">
            <p>© 2025 PACS System - Built with Rust & Axum</p>
            <p>🚀 高性能医学影像管理系统</p>
        </footer>
    </div>

    <script>
        // 添加一些交互效果
        document.addEventListener('DOMContentLoaded', function() {
            // 为所有endpoint添加点击复制功能
            const endpoints = document.querySelectorAll('.endpoint');
            endpoints.forEach(endpoint => {
                endpoint.style.cursor = 'pointer';
                endpoint.title = '点击复制';
                endpoint.addEventListener('click', function() {
                    navigator.clipboard.writeText(this.textContent.trim());
                    this.style.background = '#d4edda';
                    setTimeout(() => {
                        this.style.background = '#f8f9fa';
                    }, 1000);
                });
            });

            // 测试API连接
            fetch('/health')
                .then(response => response.json())
                .then(data => {
                    console.log('✅ API服务正常运行:', data);
                })
                .catch(error => {
                    console.error('❌ API服务连接失败:', error);
                });
        });
    </script>
</body>
</html>"#;

    if let Err(e) = std::fs::write("static/index.html", index_html) {
        error!("Failed to create index.html: {}", e);
    }

    // 创建简单的CSS文件
    let css_content = r#"/* PACS Web Interface Styles */
body { font-family: system-ui, sans-serif; }
.container { max-width: 1200px; margin: 0 auto; padding: 20px; }
"#;

    if let Err(e) = std::fs::write("static/style.css", css_content) {
        error!("Failed to create style.css: {}", e);
    }

    info!("Default static files created successfully");
}

/// 动态处理静态文件请求
pub async fn serve_static_file(Path(file_path): Path<String>) -> Result<impl IntoResponse> {
    let full_path = PathBuf::from("static").join(&file_path);

    // 安全检查：确保路径不会跳出static目录
    if !full_path.starts_with("static") {
        return Err(pacs_core::error::PacsError::Validation(
            "Invalid file path".to_string(),
        ));
    }

    // 尝试读取文件
    match fs::read(&full_path).await {
        Ok(contents) => {
            let content_type = guess_content_type(&full_path);
            Ok(Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, content_type)
                .body(axum::body::Body::from(contents))
                .unwrap())
        }
        Err(_) => {
            // 文件不存在，返回404
            Err(pacs_core::error::PacsError::NotFound(
                "File not found".to_string(),
            ))
        }
    }
}

/// 根据文件扩展名猜测内容类型
fn guess_content_type(path: &PathBuf) -> &'static str {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("html") => "text/html",
        Some("css") => "text/css",
        Some("js") => "application/javascript",
        Some("json") => "application/json",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("svg") => "image/svg+xml",
        Some("ico") => "image/x-icon",
        Some("pdf") => "application/pdf",
        Some("txt") => "text/plain",
        Some("xml") => "application/xml",
        Some("zip") => "application/zip",
        _ => "application/octet-stream",
    }
}
