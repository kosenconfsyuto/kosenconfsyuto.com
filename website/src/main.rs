use actix_files as fs;
use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer, Result, Responder};
use actix_web::middleware::Logger;
use actix_web::http::header::ContentType;
use pulldown_cmark::{Parser, Options, html};
use serde::{Deserialize};
use serde_yaml::from_str;
use std::fs::{read_to_string, OpenOptions, File};
use std::io::{Write, BufReader, BufWriter};
use tera::{Context, Tera};
use dotenv::dotenv;
use std::env;
use reqwest::Client;
use serde_json::json;
use chrono::{Local, NaiveDate};
use urlencoding::encode;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::path::Path;
use std::io::prelude::*;

#[derive(Debug, Deserialize)]
struct FrontMatter {
    title: Option<String>,
    description: Option<String>,
    topics: Option<String>,
    tags: Option<String>,
    image: Option<String>,
    common_parts: Option<String>,
    displayslider: Option<bool>,
    date: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ContactForm {
    name: String,
    email: String,
    message: String,
}

// 404ページのレンダリング
async fn custom_404() -> Result<HttpResponse, actix_web::Error> {
    let content = read_to_string("modules/404.html")
        .map_err(|_| actix_web::error::ErrorNotFound("Custom 404 page not found"))?;
    Ok(HttpResponse::NotFound().content_type(ContentType::html()).body(content))
}


async fn render_page(path: &str, url: &str, access_count: usize, news_list: &str, all_news_list: &str) -> Result<String> {
    let file_path = format!("pages/{}.html", if path.ends_with('/') { format!("{}/index", path.trim_end_matches('/')) } else { path.to_string() });
    let content = match read_to_string(&file_path) {
        Ok(content) => content,
        Err(_) => {
            let custom_404_response = custom_404().await?;
            let body_bytes = actix_web::body::to_bytes(custom_404_response.into_body()).await?;
            let custom_404_body = String::from_utf8(body_bytes.to_vec()).map_err(|_| actix_web::error::ErrorInternalServerError("Failed to convert 404 body to string"))?;
            return Ok(custom_404_body);
        }        
    };
    let parts: Vec<&str> = content.splitn(3, "---").collect();
    if parts.len() < 3 {
        return Err(actix_web::error::ErrorInternalServerError("Invalid front matter format"));
    }
    let yaml_str = parts[1];
    let yaml_data: FrontMatter = from_str(yaml_str).map_err(|_| actix_web::error::ErrorInternalServerError("Failed to parse YAML"))?;
    let mut html_content = parts[2].to_string();
    let mut tera = Tera::new("includes/**/*").unwrap();
    let mut context = Context::new();
    context.insert("url", url);
    context.insert("access_count", &access_count.to_string());
    context.insert("newslist", news_list);
    context.insert("allnewslist", all_news_list);
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
    if let Some(common_parts) = yaml_data.common_parts {
        let parts: Vec<&str> = common_parts.split('|').map(|s| s.trim()).collect();
        for part in parts.iter().rev() {
            let common_part_content = read_to_string(format!("includes/{}.html", part))
                .map_err(|_| actix_web::error::ErrorInternalServerError("Failed to read common part file"))?;
            html_content = common_part_content.replace("{{contents}}", &html_content);
        }
    }
    let final_html = tera.render_str(&html_content, &context)
        .map_err(|e| {
            eprintln!("Template rendering error: {:?}", e);
            actix_web::error::ErrorInternalServerError("Failed to render template")
        })?;
    Ok(final_html)
}

async fn render_markdown(path: &str, url: &str, access_count: usize) -> Result<String> {
    let file_path = format!("news/{}.md", path);
    let content = match read_to_string(&file_path) {
        Ok(content) => content,
        Err(_) => {
            let custom_404_response = custom_404().await?;
            let body_bytes = actix_web::body::to_bytes(custom_404_response.into_body()).await?;
            let custom_404_body = String::from_utf8(body_bytes.to_vec()).map_err(|_| actix_web::error::ErrorInternalServerError("Failed to convert 404 body to string"))?;
            return Ok(custom_404_body);
        }        
    };
    let parts: Vec<&str> = content.splitn(3, "---").collect();
    if parts.len() < 3 {
        return Err(actix_web::error::ErrorInternalServerError("Invalid front matter format"));
    }
    let yaml_str = parts[1];
    let yaml_data: FrontMatter = from_str(yaml_str).map_err(|_| actix_web::error::ErrorInternalServerError("Failed to parse YAML"))?;
    let markdown_content = parts[2];
    let mut html_output = String::new();
    let parser = Parser::new_ext(&markdown_content, Options::all());
    html::push_html(&mut html_output, parser);
    let mut tera = Tera::new("includes/**/*").unwrap();
    let mut context = Context::new();
    context.insert("url", url);
    context.insert("access_count", &access_count.to_string());
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
    let mut final_html = html_output;
    if let Some(common_parts) = yaml_data.common_parts {
        let parts: Vec<&str> = common_parts.split('|').map(|s| s.trim()).collect();
        for part in parts.iter().rev() {
            let common_part_content = read_to_string(format!("includes/{}.html", part))
                .map_err(|_| actix_web::error::ErrorInternalServerError("Failed to read common part file"))?;
            final_html = common_part_content.replace("{{contents}}", &final_html);
        }
    }
    let final_html = tera.render_str(&final_html, &context)
        .map_err(|e| {
            eprintln!("Template rendering error: {:?}", e);
            actix_web::error::ErrorInternalServerError("Failed to render template")
        })?;
    Ok(final_html)
}

async fn generate_news_list() -> Result<String> {
    let news_dir = std::fs::read_dir("news").map_err(|_| actix_web::error::ErrorInternalServerError("Failed to read news directory"))?;
    let mut news_files: Vec<_> = news_dir
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let path = entry.path();
            if path.extension()? == "md" {
                Some(path)
            } else {
                None
            }
        })
        .collect();

    // Sort news files by date in descending order
    news_files.sort_by_key(|path| {
        let content = std::fs::read_to_string(path).ok()?;
        let parts: Vec<&str> = content.splitn(3, "---").collect();
        if parts.len() < 3 {
            return None;
        }
        let yaml_str = parts[1];
        let yaml_data: FrontMatter = from_str(yaml_str).ok()?;
        yaml_data.date.and_then(|d| NaiveDate::parse_from_str(&d, "%Y.%m.%d").ok())
    });
    news_files.reverse();

    // Select the latest 5 news
    let latest_news_files = news_files.into_iter().take(5);

    let mut tera = Tera::new("includes/**/*").unwrap();
    let mut news_list = String::new();
    for path in latest_news_files {
        let content = std::fs::read_to_string(&path).map_err(|_| actix_web::error::ErrorInternalServerError("Failed to read news file"))?;
        let parts: Vec<&str> = content.splitn(3, "---").collect();
        if parts.len() < 3 {
            continue;
        }
        let yaml_str = parts[1];
        let yaml_data: FrontMatter = from_str(yaml_str).map_err(|_| actix_web::error::ErrorInternalServerError("Failed to parse YAML"))?;
        let mut context = Context::new();
        let path_str = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        context.insert("url", &format!("/news/{}", path_str));
        if let Some(title) = yaml_data.title {
            context.insert("title", &title);
        }
        if let Some(date) = yaml_data.date {
            context.insert("date", &date);
        }
        if let Some(ref topics) = yaml_data.topics {
            context.insert("topics", topics);
        }
        let important_news = if yaml_data.topics.as_deref() == Some("重要") {
            "important-news"
        } else {
            ""
        };
        context.insert("important_news", important_news);

        let news_item = tera.render("parts/news.html", &context).map_err(|e| {
            eprintln!("Template rendering error: {:?}", e);
            actix_web::error::ErrorInternalServerError("Failed to render news item template")
        })?;
        news_list.push_str(&news_item);
    }
    Ok(news_list)
}

async fn generate_all_news_list() -> Result<String> {
    let news_dir = std::fs::read_dir("news").map_err(|_| actix_web::error::ErrorInternalServerError("Failed to read news directory"))?;
    let mut news_files: Vec<_> = news_dir
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let path = entry.path();
            if path.extension()? == "md" {
                Some(path)
            } else {
                None
            }
        })
        .collect();

    // Sort news files by date in descending order
    news_files.sort_by_key(|path| {
        let content = std::fs::read_to_string(path).ok()?;
        let parts: Vec<&str> = content.splitn(3, "---").collect();
        if parts.len() < 3 {
            return None;
        }
        let yaml_str = parts[1];
        let yaml_data: FrontMatter = from_str(yaml_str).ok()?;
        yaml_data.date.and_then(|d| NaiveDate::parse_from_str(&d, "%Y.%m.%d").ok())
    });
    news_files.reverse();

    // Select the latest all news
    let latest_news_files = news_files.into_iter();

    let mut tera = Tera::new("includes/**/*").unwrap();
    let mut all_news_list = String::new();
    for path in latest_news_files {
        let content = std::fs::read_to_string(&path).map_err(|_| actix_web::error::ErrorInternalServerError("Failed to read news file"))?;
        let parts: Vec<&str> = content.splitn(3, "---").collect();
        if parts.len() < 3 {
            continue;
        }
        let yaml_str = parts[1];
        let yaml_data: FrontMatter = from_str(yaml_str).map_err(|_| actix_web::error::ErrorInternalServerError("Failed to parse YAML"))?;
        let mut context = Context::new();
        let path_str = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        context.insert("url", &format!("/news/{}", path_str));
        if let Some(title) = yaml_data.title {
            context.insert("title", &title);
        }
        if let Some(date) = yaml_data.date {
            context.insert("date", &date);
        }
        if let Some(ref topics) = yaml_data.topics {
            context.insert("topics", topics);
        }
        let important_news = if yaml_data.topics.as_deref() == Some("重要") {
            "important-news"
        } else {
            ""
        };
        context.insert("important_news", important_news);

        let news_item = tera.render("parts/news.html", &context).map_err(|e| {
            eprintln!("Template rendering error: {:?}", e);
            actix_web::error::ErrorInternalServerError("Failed to render news item template")
        })?;
        all_news_list.push_str(&news_item);
    }
    Ok(all_news_list)
}

async fn handle_request(req: HttpRequest, counter: web::Data<Arc<AtomicUsize>>) -> Result<impl Responder> {
    let mut path = req.path().trim_start_matches('/').to_string();
    if path.ends_with('/') {
        path.pop();
    }

    let url = req.uri().path();
    // &str を String に変換する
    let mut url_string = url.to_string();
    
    // 文字列 "index" を空文字列 "" に置き換える
    if let Some(pos) = url_string.find("index") {
        url_string.replace_range(pos..pos + 5, "");
    }
    
    // 最後の文字が "/" であれば削除する
    if url_string.ends_with('/') {
        url_string.pop();
    }

    let url_str: &str = &url_string;

    let user_agent = req.headers().get("User-Agent").and_then(|h| h.to_str().ok()).unwrap_or("Unknown");

    // Generate news list
    let news_list = generate_news_list().await?;

    let all_news_list = generate_all_news_list().await?;

    // Check if the file exists in the public directory
    let public_path = if path.is_empty() { "public/index.html".to_string() } else { format!("public/{}", path) };
    if Path::new(&public_path).exists() {
        return Ok(fs::NamedFile::open(public_path)?.into_response(&req));
    }

    // Check if the request is for the main page or a resource
    let is_resource_request = path.ends_with(".css") || path.ends_with(".js") || path.ends_with(".png") || path.ends_with(".jpg") || path.ends_with(".jpeg") || path.ends_with(".gif") || path.ends_with(".svg");

    // Increment the access counter only for the main page requests
    let access_count = if !is_resource_request {
        let count = counter.fetch_add(1, Ordering::SeqCst) + 1;

        // Save the updated access count to a file
        let mut file = BufWriter::new(File::create("access_count.txt").map_err(|_| actix_web::error::ErrorInternalServerError("Failed to open access_count.txt"))?);
        writeln!(file, "{}", count).map_err(|_| actix_web::error::ErrorInternalServerError("Failed to write to access_count.txt"))?;

        count
    } else {
        counter.load(Ordering::SeqCst)
    };

    // Log the user agent to access.log
    let mut file = OpenOptions::new().create(true).append(true).open("log/access.log")
        .map_err(|_| actix_web::error::ErrorInternalServerError("Failed to open access.log"))?;
    writeln!(file, "Time: {}, Path: {}, Method: {}, Status: {}, User-Agent: {}, Access Count: {}", Local::now().to_rfc3339(), path, req.method(), req.connection_info().realip_remote_addr().unwrap_or("Unknown"), user_agent, access_count)
        .map_err(|_| actix_web::error::ErrorInternalServerError("Failed to write to access.log"))?;

    let result = if path.is_empty() {
        render_page("index", url_str, access_count, &news_list, &all_news_list).await
    } else if path.starts_with("news/") {
        let relative_path = &path["news/".len()..];
        render_markdown(relative_path, url_str, access_count).await
    } else {
        render_page(&path, url_str, access_count, &news_list, &all_news_list).await
    };

    match result {
        Ok(content) => Ok(HttpResponse::Ok().content_type(ContentType::html()).body(content)),
        Err(e) => Err(e),
    }
}

async fn handle_contact_form(form: web::Form<ContactForm>) -> impl Responder {
    println!("Received contact form: {:?}", form);
    println!("Name: {}", form.name);
    println!("Email: {}", form.email);
    println!("Message: {}", form.message);

    let webhook_url = env::var("WEBHOOK_URL").expect("WEBHOOK_URL must be set in .env");

    let client = Client::new();
    let payload = json!({
        "embeds": [{
            "title": "New Contact Form Submission",
            "color": 16777215, // White color
            "fields": [
                { "name": "Name", "value": form.name, "inline": true },
                { "name": "Email", "value": form.email, "inline": true },
                { "name": "Message", "value": form.message }
            ]
        }]
    });

    match client.post(&webhook_url).json(&payload).send().await {
        Ok(response) => {
            if response.status().is_success() {
                HttpResponse::Ok().content_type(ContentType::html()).body("Thank you for your message!")
            } else {
                eprintln!("Failed to send message to Discord: {:?}", response.text().await);
                HttpResponse::InternalServerError().content_type(ContentType::html()).body("Failed to send message.")
            }
        }
        Err(e) => {
            eprintln!("Failed to send message to Discord: {:?}", e);
            HttpResponse::InternalServerError().content_type(ContentType::html()).body("Failed to send message.")
        }
    }
}

async fn generate_sitemap() -> Result<impl Responder> {
    let base_url = "http://127.0.0.1:8000";

    // Collect static pages from the pages directory
    let static_pages = std::fs::read_dir("pages")
        .map_err(|_| actix_web::error::ErrorInternalServerError("Failed to read pages directory"))?
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let path = entry.path();
            if path.extension()? == "html" {
                Some(path)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    // Collect dynamic news pages from the news directory
    let news_files = std::fs::read_dir("news")
        .map_err(|_| actix_web::error::ErrorInternalServerError("Failed to read news directory"))?
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let path = entry.path();
            if path.extension()? == "md" {
                Some(path)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    // Generate URLs for static pages
    let mut urls = static_pages.into_iter().map(|path| {
        let relative_path = path.strip_prefix("pages").unwrap().to_str().unwrap().trim_end_matches(".html");
        format!("{}/{}", base_url, encode(relative_path))
    }).collect::<Vec<_>>();

    // Generate URLs for news files
    for path in news_files {
        let relative_path = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        urls.push(format!("{}/news/{}", base_url, encode(relative_path)));
    }

    let mut sitemap = String::new();
    sitemap.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    sitemap.push_str("<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n");

    for url in urls {
        sitemap.push_str("  <url>\n");
        sitemap.push_str(&format!("    <loc>{}</loc>\n", url));
        sitemap.push_str("    <changefreq>weekly</changefreq>\n");
        sitemap.push_str("  </url>\n");
    }

    sitemap.push_str("</urlset>");

    Ok(HttpResponse::Ok()
        .content_type("application/xml")
        .body(sitemap))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    std::env::set_var("RUST_LOG", "actix_web=info");
    env_logger::init();

    // Load the access count from the file
    let access_count = if Path::new("access_count.txt").exists() {
        let mut file = BufReader::new(File::open("access_count.txt")?);
        let mut count_str = String::new();
        file.read_to_string(&mut count_str)?;
        count_str.trim().parse().unwrap_or(0)
    } else {
        0
    };

    let counter = Arc::new(AtomicUsize::new(access_count));

    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .app_data(web::Data::new(counter.clone()))
            .service(fs::Files::new("/public", "./public").show_files_listing().use_last_modified(true))
            .service(
                web::resource("/submit_contact")
                    .route(web::post().to(handle_contact_form))
            )
            .service(
                web::resource("/sitemap.xml")
                    .route(web::get().to(generate_sitemap))
            )
            .service(
                web::resource("/{filename:.*}")
                    .route(web::get().to(handle_request))
            )
    })
    .bind("127.0.0.1:8000")?
    .run()
    .await
}
