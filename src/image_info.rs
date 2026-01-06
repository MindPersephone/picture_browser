use std::fs;
use std::fs::DirEntry;
use std::fs::File;
use std::fs::Metadata;
use std::io::BufReader;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::time::SystemTime;

use imagesize::size;
use log::info;
use log::warn;
use mp4::Mp4Reader;
use serde::Serialize;
use serde_json::Value;
use which::which;

use crate::error::Error;
use crate::FilterParameter;

const ALLOWED_IMG_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "gif", "webp"];

const ALLOWED_VID_EXTENSIONS: &[&str] = &["mp4", "webm"];

pub fn find_files(
    target_path: &str,
    filter_value: FilterParameter,
    recurse: bool,
) -> Vec<ImageInfo> {
    let allow_list: Vec<&str> = match filter_value {
        FilterParameter::None => ALLOWED_IMG_EXTENSIONS
            .iter()
            .copied()
            .chain(ALLOWED_VID_EXTENSIONS.iter().copied())
            .collect(),
        FilterParameter::Video => ALLOWED_VID_EXTENSIONS.into(),
        FilterParameter::Images => ALLOWED_IMG_EXTENSIONS.into(),
        FilterParameter::Gif => vec!["gif"],
    };

    inner_find_files(&target_path.into(), &allow_list, recurse)
}

fn inner_find_files(target_dir: &PathBuf, allow_list: &Vec<&str>, recurse: bool) -> Vec<ImageInfo> {
    let mut result = Vec::new();
    if target_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(target_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() && recurse {
                    let mut new_files = inner_find_files(&path, allow_list, recurse);
                    result.append(&mut new_files);
                } else if path.is_file() {
                    let extension = path
                        .extension()
                        .map(|p| p.to_str().unwrap())
                        .unwrap_or("")
                        .to_lowercase();
                    if allow_list.contains(&extension.as_str()) {
                        result.push(file_to_image(&entry).unwrap())
                    } else {
                        warn!("disallowed file type {:?}", entry.file_name());
                    }
                }
            }
        }
    }

    result
}

#[derive(Debug, Clone, Serialize)]
pub struct ImageInfo {
    pub url: String,
    pub source: String,
    pub date: SystemTime,
    pub is_video: bool,
    pub width: usize,
    pub height: usize,
}

fn file_to_image(entry: &DirEntry) -> Result<ImageInfo, Error> {
    let filepath = entry.path().to_str().unwrap().to_string();
    let p = Path::new(&filepath);
    let url = p.file_name().unwrap().to_str().unwrap().to_string();
    let extension = p.extension().map(|p| p.to_str().unwrap()).unwrap_or("");

    let metadata = entry.metadata()?;
    let date = date(&metadata)?;

    let is_video = ALLOWED_VID_EXTENSIONS.contains(&extension);

    let (width, height) = if is_video {
        // we have a way of handling some mp4 files. Others need working on.
        if extension == "mp4" {
            mp4_size(&filepath)?
        } else {
            (0, 0)
        }
    } else {
        image_size(&filepath)?
    };

    Ok(ImageInfo {
        url,
        source: p.to_str().unwrap().to_string(),
        date,
        is_video,
        width,
        height,
    })
}

fn date(metadata: &Metadata) -> Result<SystemTime, Error> {
    metadata
        .modified()
        .or(metadata.created())
        .or(Ok(metadata.accessed()?))
}

fn image_size(filepath: &str) -> Result<(usize, usize), Error> {
    let result = size(filepath)?;
    Ok((result.width, result.height))
}

fn mp4_size(filepath: &str) -> Result<(usize, usize), Error> {
    let f = File::open(filepath)?;
    let size = f.metadata()?.len();
    let reader = BufReader::new(f);

    let mp4_result = Mp4Reader::read_header(reader, size);
    if let Err(mp4_error) = mp4_result {
        warn!("Could not get metadata from {filepath}, attempting ffmpeg. {mp4_error:?}");
        let ffmpeg_result = try_ffmpeg(filepath);
        if let Err(ffmpeg_err) = ffmpeg_result {
            warn!("Could not get metadata with ffmpeg either {filepath}. Returning zeros. {ffmpeg_err}");
            return Ok((0, 0));
        }

        return ffmpeg_result;
    }

    let mut result_width = 0;
    let mut result_height = 0;

    for track in mp4_result.unwrap().tracks().values() {
        result_width = result_width.max(track.width());
        result_height = result_height.max(track.height());
    }

    Ok((result_width as usize, result_height as usize))
}

fn try_ffmpeg(filepath: &str) -> Result<(usize, usize), Error> {
    let which_result = which("ffprobe");
    if let Err(err) = which_result {
        warn!(
            "Could not find ffprobe, you may need to install ffmpeg: {}",
            err
        );
        return Err(Error::MissingFFProbe);
    }

    let output = Command::new("ffprobe")
        .args([
            "-i",
            filepath,
            "-print_format",
            "json",
            "-find_stream_info",
            "-show_streams",
            "-v",
            "quiet",
        ])
        .output()?;

    let json_result: Value = serde_json::from_slice(&output.stdout)?;

    let mut width: usize = 0;
    let mut height: usize = 0;

    for stream in json_result["streams"].as_array().unwrap() {
        let stream_object = stream.as_object().unwrap();
        if stream_object.contains_key("width") && stream_object.contains_key("height") {
            width = width.max(stream_object.get("width").unwrap().as_u64().unwrap() as usize);
            height = height.max(stream_object.get("height").unwrap().as_u64().unwrap() as usize);
        }
    }

    info!("success using ffmpeg: {},{}", width, height);
    return Ok((width, height));
}
