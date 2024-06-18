use actix_files as fs;
use actix_web::{web, App, HttpRequest, HttpServer, Result, Responder};
use serde::Deserialize;
use serde_yaml::from_str;
use std::fs::read_to_string;
use tera::{Context, Tera};

#[derive(Debug, Deserialize)]
struct FrontMatter {
    title: Option<String>,
    description: Option<String>,
    tags: Option<String>,
    image: Option<String>,
    common_parts: Option<String>,
}

async fn render_page(path: &str) -> Result<String> {
    let file_path = format!("pages/{}", path);

    // ファイルを読み込む
    let content = read_to_string(&file_path).map_err(|_| actix_web::error::ErrorNotFound("File not found"))?;

    // YAMLフロントマターと本文を分割
    let parts: Vec<&str> = content.splitn(3, "---").collect();
    if parts.len() < 3 {
        return Err(actix_web::error::ErrorInternalServerError("Invalid front matter format"));
    }

    // YAMLフロントマターの解析
    let yaml_str = parts[1];
    let yaml_data: FrontMatter = from_str(yaml_str).map_err(|_| actix_web::error::ErrorInternalServerError("Failed to parse YAML"))?;

    // HTML本文
    let mut html_content = parts[2].to_string();

    // テンプレートエンジンのセットアップ
    let mut tera = Tera::new("includes/**/*").unwrap();
    let mut context = Context::new();

    // フロントマターのデータをコンテキストに追加
    if let Some(title) = yaml_data.title {
        context.insert("title", &title);
    }
    if let Some(description) = yaml_data.description {
        context.insert("description", &description);
    }
    if let Some(tags) = yaml_data.tags {
        context.insert("tags", &tags);
    }
    if let Some(image) = yaml_data.image {
        context.insert("image", &image);
    }

    // common_partsの処理
    if let Some(common_parts) = yaml_data.common_parts {
        let parts: Vec<&str> = common_parts.split('|').map(|s| s.trim()).collect();
        for part in parts.iter().rev() {
            let common_part_content = read_to_string(format!("includes/{}.html", part))
                .map_err(|_| actix_web::error::ErrorInternalServerError("Failed to read common part file"))?;
            html_content = common_part_content.replace("{{contents}}", &html_content);
        }
    }

    // Teraを使ってテンプレートをレンダリング
    let final_html = tera.render_str(&html_content, &context)
        .map_err(|e| {
            eprintln!("Template rendering error: {:?}", e);
            actix_web::error::ErrorInternalServerError("Failed to render template")
        })?;

    Ok(final_html)
}

async fn index(req: HttpRequest) -> Result<impl Responder> {
    let path = req.match_info().get("filename").unwrap_or("index.html");
    let content = render_page(path).await?;
    Ok(actix_web::HttpResponse::Ok().content_type("text/html").body(content))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .route("/", web::get().to(index))
            .route("/{filename:.*}", web::get().to(index))
            .service(fs::Files::new("/public", "./public").show_files_listing())
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
