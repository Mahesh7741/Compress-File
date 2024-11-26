use actix_files as fs;
use actix_multipart::Multipart;
use actix_web::{web, App, HttpResponse, HttpServer, Result};
use flate2::write::GzEncoder;
use flate2::Compression;
use futures_util::stream::StreamExt as _;
use std::fs::File;
use std::io::{Write, BufReader};
use std::time::Instant;

/// Handles file compression and returns downloadable file details
async fn compress_file(mut payload: Multipart) -> Result<HttpResponse> {
    let mut compression_level = Compression::default();
    let mut file_path = String::new();
    let mut output_file_name = "compressed_output.gz".to_string();

    // Process multipart form fields
    while let Some(field) = payload.next().await {
        let mut field = field?;
        let field_name = field.name();

        if field_name == "file" {
            let temp_file_path = "temp_uploaded_file";
            let mut file = File::create(temp_file_path)?;

            while let Some(chunk) = field.next().await {
                file.write_all(&chunk?)?;
            }
            file_path = temp_file_path.to_string();
        } else if field_name == "level" {
            let mut level_data = Vec::new();
            while let Some(chunk) = field.next().await {
                level_data.extend_from_slice(&chunk?);
            }
            if let Ok(level_str) = String::from_utf8(level_data) {
                if let Ok(level) = level_str.parse::<u32>() {
                    if (0..=9).contains(&level) {
                        compression_level = Compression::new(level);
                    }
                }
            }
        } else if field_name == "output_name" {
            let mut name_data = Vec::new();
            while let Some(chunk) = field.next().await {
                name_data.extend_from_slice(&chunk?);
            }
            if let Ok(name) = String::from_utf8(name_data) {
                output_file_name = name + ".gz";
            }
        }
    }

    if file_path.is_empty() {
        return Ok(HttpResponse::BadRequest().body("No file uploaded"));
    }

    let start = Instant::now();
    let mut input = BufReader::new(File::open(&file_path)?);
    let output_path = output_file_name.clone();
    let mut output = File::create(&output_path)?;
    let mut encoder = GzEncoder::new(&mut output, compression_level);

    std::io::copy(&mut input, &mut encoder)?;
    encoder.finish()?;
    let elapsed = start.elapsed();

    Ok(HttpResponse::Ok().json({
        serde_json::json!({
            "status": "success",
            "download_url": format!("/download/{}", output_file_name),
            "elapsed_time": format!("{:?}", elapsed),
        })
    }))
}

/// Serves compressed files for download
async fn download_file(file_name: web::Path<String>) -> Result<fs::NamedFile> {
    Ok(fs::NamedFile::open(file_name.into_inner())?)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .route("/compress", web::post().to(compress_file))
            .route("/download/{file_name}", web::get().to(download_file))
            .service(fs::Files::new("/", "./static").index_file("index.html"))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
