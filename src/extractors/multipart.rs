use crate::helper::TempFile;
use actix_web::web;
use futures::StreamExt;
use std::{collections::HashMap, io::Write, path::Path};
use uuid::Uuid;

#[derive(Debug)]
pub struct MultipartFormData {
    pub fields: HashMap<String, String>,
    pub file_paths: Vec<TempFile>,
}

impl MultipartFormData {
    pub async fn from_multipart(
        dest_dir_path: &Path,
        files_path_prefix: &str,
        mut payload: actix_multipart::Multipart,
        max_files_number: Option<usize>,
        allowed_file_extensions: Option<&[&str]>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let mut contor = 0_usize;
        let mut res_files: Vec<TempFile> = Vec::new();
        let mut res_fields: HashMap<String, String> = HashMap::new();

        while let Some(item) = payload.next().await {
            let mut field = item?;
            if let Some(src_file_name) = field.content_disposition().get_filename() {
                if let Some(max_no) = max_files_number {
                    if contor >= max_no {
                        continue;
                    }
                }

                let src_path = Path::new(src_file_name);
                let file_extension =
                    match src_path.extension().map(|v| v.to_str().unwrap_or_default()) {
                        Some(v) => v,
                        None => "",
                    };

                if let Some(allowed_exts) = allowed_file_extensions {
                    if !allowed_exts.contains(&file_extension) {
                        continue;
                    }
                }

                let file_path = TempFile {
                    path: dest_dir_path.join(format!(
                        "{}-{}{}{}",
                        files_path_prefix,
                        Uuid::new_v4(),
                        if file_extension.len() > 0 { "." } else { "" },
                        file_extension
                    )),
                };

                let dest_path_clone = file_path.path.clone();
                let mut file = web::block(move || std::fs::File::create(dest_path_clone)).await??;
                while let Some(part) = field.next().await {
                    let chunk = part?;
                    file = web::block(move || file.write_all(&chunk).map(|_| file)).await??;
                }

                res_files.push(file_path);
                contor += 1;
            } else {
                let field_name: String = field.name().into();
                let mut bytes: Vec<u8> = Vec::new();
                while let Some(part) = field.next().await {
                    let chunk = part?;
                    bytes.reserve_exact(chunk.len());
                    bytes.append(&mut chunk.to_vec());
                }

                let field_value = String::from_utf8(bytes)?;
                res_fields.insert(field_name, field_value);
            }
        }

        Ok(Self {
            fields: res_fields,
            file_paths: res_files,
        })
    }
}
