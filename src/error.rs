use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("an io error {0:?}")]
    Io(#[from] std::io::Error),
    #[error("an serde encoding error {0:?}")]
    Serde(#[from] serde_json::Error),
    #[error("an image size reading error {0:?}")]
    ImageSize(#[from] imagesize::ImageError),

    #[error("a mp4 processing error {0:?}")]
    Mp4(#[from] mp4::Error),

    #[error("an error calling ffprobe")]
    MissingFFProbe,
}
