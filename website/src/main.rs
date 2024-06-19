use actix_files as fs;
use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer, Result, Responder};
use actix_web::middleware::Logger;
use actix_web::http::header::ContentType;
use pulldown_cmark::{Parser, Options, html};
use serde::{Deserialize};
use serde_yaml::from_str;
use std::fs::{read_to_string, OpenOptions};
use std::io::Write;
use tera::{Context, Tera};
use dotenv::dotenv;
use std::env;
use reqwest::Client;
use serde_json::json;

#[derive(Debug, Deserialize)]
struct FrontMatter {
    title: Option<String>,
    description: Option<String>,
    tags: Option<String>,
    image: Option<String>,
    common_parts: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ContactForm {
    name: String,
    email: String,
    message: String,
}

async fn render_page(path: &str, url: &str) -> Result<String> {
    let file_path = format!("pages/{}.html", if path.ends_with('/') { format!("{}/index", path.trim_end_matches('/')) } else { path.to_string() });
    let content = read_to_string(&file_path).map_err(|_| actix_web::error::ErrorNotFound("File not found"))?;
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

async fn render_markdown(path: &str, url: &str) -> Result<String> {
    let file_path = format!("news/{}.md", path);
    let content = read_to_string(&file_path).map_err(|_| actix_web::error::ErrorNotFound("File not found"))?;
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

async fn handle_request(req: HttpRequest) -> Result<impl Responder> {
    let path = req.path().trim_start_matches('/');
    let url = req.uri().path();
    let user_agent = req.headers().get("User-Agent").and_then(|h| h.to_str().ok()).unwrap_or("Unknown");

    // Log the user agent to access.log
    let mut file = OpenOptions::new().create(true).append(true).open("log/access.log")
        .map_err(|_| actix_web::error::ErrorInternalServerError("Failed to open access.log"))?;
    writeln!(file, "Path: {}, Method: {}, Status: {}, User-Agent: {}", path, req.method(), req.connection_info().realip_remote_addr().unwrap_or("Unknown"), user_agent)
        .map_err(|_| actix_web::error::ErrorInternalServerError("Failed to write to access.log"))?;

    let result = if path.is_empty() {
        render_page("index", url).await
    } else if path.starts_with("news/") {
        let relative_path = &path["news/".len()..];
        render_markdown(relative_path, url).await
    } else {
        render_page(path, url).await
    };

    match result {
        Ok(content) => Ok(HttpResponse::Ok().content_type("text/html").body(content)),
        Err(_) => {
            // Render 404 page
            let content = read_to_string("modules/404.html")
                .map_err(|_| actix_web::error::ErrorInternalServerError("Failed to read 404 page"))?;
            let final_html = content.replace("{{url}}", url);
            Ok(HttpResponse::NotFound().content_type("text/html").body(final_html))
        }
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

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    std::env::set_var("RUST_LOG", "actix_web=info");
    env_logger::init();

    HttpServer::new(|| {
        App::new()
            .wrap(Logger::default())
            .service(fs::Files::new("/public", "./public").show_files_listing().use_last_modified(true))
            .service(
                web::resource("/submit_contact")
                    .route(web::post().to(handle_contact_form))
            )
            .service(
                web::resource("/{filename:.*}")
                    .route(web::get().to(handle_request))
            )
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
