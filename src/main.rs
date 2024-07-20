use std::fs::{self, DirEntry, Metadata};
use std::io;
use std::path::Path;
use std::time::{Duration, SystemTime};

use actix_files::NamedFile;

use actix_web::http::header::ContentType;
use actix_web::middleware::Logger;
use actix_web::{
    web, App, HttpRequest, HttpResponse, HttpServer, Responder, Result,
};
use clap::{Parser, ValueEnum};
use env_logger::Env;
use rand::prelude::*;
use serde::Serialize;
use tera::{Context, Tera};
use tokio::task::JoinSet;

struct AppData {
    target_path: String,
    images: Vec<ImageInfo>,
    templates: Tera,
    background: String,
}


#[derive(Parser, Debug)]
#[command(version, about)]
struct Parameters {
    #[arg(
        short, 
        long, 
        default_value_t = 6700, 
        help="port to use for the web server",
    )]
    pub port : u16,

    #[arg(
        short, 
        long, 
        default_value_t = 10, 
        help="number of seconds to wait after opening the browser before exiting. If set to 0 it will not exit. Press ctrl+c to exit",
    )]
    pub delay: u64,

    #[arg(
        long,
        default_value_t = false,
        conflicts_with_all=["date", "alphabetical"],
        help="randomise the order of the images",
    )]
    pub randomise: bool,

    #[arg(
        long,
        default_value_t = false,
        conflicts_with_all=["randomise", "alphabetical"],
        help="sort by the order of the images by date",
    )]
    pub date: bool,

    #[arg(
        long, 
        default_value_t = false, 
        conflicts_with_all=["date", "randomise"],
        help="sort by the order of the images by name",
    )]
    pub alphabetical:bool,

    #[arg(
        short, 
        long, 
        default_value = "hotpink",
        help="The background colour to use for the page. Accepts any css compatible colour string",
    )]
    pub background: String,

    #[arg(
        short,
        long,
        default_value_t = FilterParameter::None,
        help="filter for different file types",
    )]
    pub filter: FilterParameter,

    #[arg(
        index=1,
        required=true,
        help="file system path to the images to host",
    )]
    pub path: String,
}


#[derive(ValueEnum, Copy, Clone, Debug, PartialEq, Eq)]
enum FilterParameter {
    None,
    Video,
    Images,
    Gif,
}

impl std::fmt::Display for FilterParameter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.to_possible_value()
            .expect("no values are skipped")
            .get_name()
            .fmt(f)
    }
}


#[actix_web::main]
async fn main() {
    env_logger::init_from_env(Env::default().default_filter_or("info"));
    let args = Parameters::parse();

    let images: Vec<ImageInfo> = find_files(&args.path, args.filter);
    let sorted_images = sort(&args, images);

    let templates = create_templates("./");

    let data = AppData {
        target_path: args.path.clone(),
        images: sorted_images,
        templates,
        background: args.background.clone(),
    };

    let web_data = web::Data::new(data);

    let mut server_builder = HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .app_data(web_data.clone()) 
            .route("/index.html", web::get().to(index))
            .route("/", web::get().to(index))
            .route("/img/{image_name}", web::get().to(image_request))
    })
    .workers(2)
    .bind(("127.0.0.1", args.port))  // only bind local so this can't be accessed outside the current machine
    .unwrap(); 

    // if we have a delay, set the keep alive to it so that we don't try and wait longer than we will exist.
    if args.delay > 0 {
        server_builder = server_builder.keep_alive(Duration::from_secs(args.delay));
    }

    let server = server_builder.run();

    // launch web browser
    if !webbrowser::open(format!("http://127.0.0.1:{}/", args.port).as_str()).is_ok() {
        println!("Could not open web browser aborting");
        return;
    }
    // the server must be awaited otherwise it will not actually do anything.
    // we also want a time out so here we need to await both of them and return
    // when the first one exits.
    let mut set = JoinSet::new();
    if args.delay > 0 {
        // if delay is zero then we don't timeout
        set.spawn(async move {
            let duration = Duration::from_secs(args.delay);
            Ok(tokio::time::sleep(duration).await)
        });
    }
    set.spawn(server);
    set.join_next().await;
}

async fn index(data: web::Data<AppData>) -> impl Responder {
    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(generate_index(
            &data.templates,
            &data.target_path,
            &data.images,
            &data.background,
        ))
}

async fn image_request(data: web::Data<AppData>, req: HttpRequest) -> Result<NamedFile> {
    let path = req.match_info().query("image_name");

    for img in data.images.iter() {
        if img.url.as_str() == path {
            return Ok(NamedFile::open(&img.source)?);
        }
    }
    panic!("not found {}", path);
}

const ALLOWED_IMG_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "gif", "webp"];

const ALLOWED_VID_EXTENSIONS: &[&str] = &["mp4", "webm"];

fn find_files(target_path: &str, filter_value: FilterParameter) -> Vec<ImageInfo> {

    let allow_list: Vec<&str> = match filter_value {
        FilterParameter::None => ALLOWED_IMG_EXTENSIONS.to_vec().into_iter()
            .chain(ALLOWED_VID_EXTENSIONS.to_vec().into_iter())
            .collect(),
        FilterParameter::Video => ALLOWED_VID_EXTENSIONS.into(),
        FilterParameter::Images => ALLOWED_IMG_EXTENSIONS.into(),
        FilterParameter::Gif => vec!["gif"],
    };

    let mut result = Vec::new();

    if let Ok(entries) = fs::read_dir(target_path) {
        for entry in entries {
            if let Ok(entry) = entry {
                // Here, `entry` is a `DirEntry`.
                let path = entry.path();
                let extension = path.extension().map(|p| p.to_str().unwrap()).unwrap_or("");
                if allow_list.contains(&extension)
                {
                    result.push(file_to_image(&entry).unwrap())
                }
            }
        }
    }

    result
}

#[derive(Debug, Clone, Serialize)]
struct ImageInfo {
    pub url: String,
    pub source: String,
    pub date: SystemTime,
    pub is_video: bool,
}

fn file_to_image(entry: &DirEntry) -> Result<ImageInfo, io::Error> {
    let filepath = entry.path().to_str().unwrap().to_string();
    let p = Path::new(&filepath);
    let url = p.file_name().unwrap().to_str().unwrap().to_string();
    let extension = p.extension().map(|p| p.to_str().unwrap()).unwrap_or("");

    let metadata = entry.metadata()?;
    let date = date(&metadata)?;

    Ok(ImageInfo {
        url,
        source: p.to_str().unwrap().to_string(),
        date,
        is_video: ALLOWED_VID_EXTENSIONS.contains(&extension),
    })
}

fn date(metadata: &Metadata) -> Result<SystemTime, io::Error> {
    metadata
        .accessed()
        .or(metadata.created())
        .or(metadata.modified())
}

fn sort(args: &Parameters, input: Vec<ImageInfo>) -> Vec<ImageInfo> {
    let mut result = input.clone();
    if args.alphabetical {
        result.sort_by(|a, b| a.url.cmp(&b.url));
    } else if args.date {
        result.sort_by(|a, b| a.date.cmp(&b.date));
    } else if args.randomise {
        let mut rng = rand::thread_rng();
        result.shuffle(&mut rng);
    }
    result
}

fn create_templates(path: &str) -> Tera {
    let mut tera = match Tera::new(format!("{}*.html", path).as_str()) {
        Ok(t) => t,
        Err(e) => {
            println!("Parsing error(s): {}", e);
            panic!("Could not parse custom template. Aborting");
        }
    };
    tera.autoescape_on(vec![]);

    // look and see if we have a custom index.html file loaded from init.
    // if we don't we can load in a default later.
    let mut index = false;
    println!("found templates: ");
    for name in tera.get_template_names() {
        println!("{}", name);
        if name == "index.html" {
            index = true;
        }
    }

    if !index {
        println!("No custom index.html found in the current directory so using default template");
        tera.add_raw_template("index.html", DEFAULT_INDEX).unwrap();
    }

    tera
}

fn generate_index(
    templates: &Tera,
    target_path: &str,
    images: &Vec<ImageInfo>,
    background: &str,
) -> String {
    let mut context = Context::new();
    context.insert("images", &images);
    context.insert("path", target_path);
    context.insert("background", background);

    templates.render("index.html", &context).unwrap()
}

const DEFAULT_INDEX: &str = include_str!("./index.html");
