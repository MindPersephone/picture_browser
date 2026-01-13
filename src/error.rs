use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("an io error {0:?}")]
    Io(#[from] std::io::Error),
    #[error("an serde encoding error {0:?}")]
    Serde(#[from] serde_json::Error),

    #[error("Lock error")]
    Lock(),

    #[error("an image size reading error {0:?}")]
    ImageSize(#[from] imagesize::ImageError),

    #[error("a mp4 processing error {0:?}")]
    Mp4(#[from] mp4::Error),

    #[error("an error calling ffprobe")]
    MissingFFProbe,
}

impl actix_web::ResponseError for Error {
    fn status_code(&self) -> actix_web::http::StatusCode {
        actix_web::http::StatusCode::INTERNAL_SERVER_ERROR
    }

    fn error_response(&self) -> actix_web::HttpResponse<actix_web::body::BoxBody> {
        let message = self.to_string();
        actix_web::HttpResponse::build(self.status_code()).body(message)
    }
}
