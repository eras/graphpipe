use actix_web::{web, Scope};

#[cfg(feature = "embed-assets")]
use actix_web_rust_embed_responder::{EmbedResponse, IntoResponse};

#[cfg(feature = "embed-assets")]
use rust_embed::{EmbeddedFile, RustEmbed};

#[cfg(feature = "embed-assets")]
#[derive(RustEmbed)]
#[folder = "./assets/"] // Path to your assets directory
struct EmbeddedAssets;

#[cfg(feature = "embed-assets")]
async fn embedded_assets_handler(
    path: web::Path<String>,
    index_file_name_data: web::Data<String>,
) -> EmbedResponse<EmbeddedFile> {
    let file_path = if path.is_empty() {
        index_file_name_data.as_str()
    } else {
        path.as_str()
    };
    EmbeddedAssets::get(file_path).into_response()
}

pub fn assets(path_prefix: &str, index_file_name: &str) -> Scope {
    let mut scope = web::scope(&path_prefix);

    #[cfg(not(feature = "embed-assets"))]
    {
        let fs_path = "./backend/assets/";
        scope = scope.service(actix_files::Files::new("/", fs_path).index_file(index_file_name));
    }

    #[cfg(feature = "embed-assets")]
    {
        let index_data = web::Data::new(index_file_name.to_string());
        scope = scope
            .app_data(index_data)
            .route("/{path:.*}", web::get().to(embedded_assets_handler));
    }

    scope
}
