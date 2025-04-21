
use log::{debug, info, warn};
use actix_files::NamedFile;

use actix_web::http::header::ContentType;
use actix_web::middleware::Logger;
use actix_web::{
    web, App, HttpRequest, HttpResponse, HttpServer, Responder, Result,
};
use clap::{Parser, ValueEnum};
use env_logger::Env;
use rand::prelude::*;
use tera::{Context, Tera};
use tokio::task::JoinSet;

use crate::image_info::{find_files, ImageInfo};

pub mod error;
pub mod image_info;

struct AppData {
    target_path: String,
    images: Vec<ImageInfo>,
    templates: Tera,
    background: String,
    hot_reload: bool,
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

    #[arg(
        long, default_value_t = false,
        help = "recurse down directories"
    )]
    pub recursive: bool,

    #[arg(long, default_value_t = 2, help="The number of worker threads to start.")]
    pub workers: usize,

    #[arg(long, default_value_t = false, help="Reload the index template on every page load (useful when developing)")]
    pub hot_reload: bool,
}


#[derive(ValueEnum, Copy, Clone, Debug, PartialEq, Eq)]
pub enum FilterParameter {
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

    let images: Vec<ImageInfo> = find_files(&args.path, args.filter, args.recursive);
    info!("Found {} files", images.len());

    let sorted_images = sort(&args, images);

    let templates = create_templates("./");

    let data = AppData {
        target_path: args.path.clone(),
        images: sorted_images,
        templates,
        background: args.background.clone(),
        hot_reload: args.hot_reload.clone(),
    };

    let web_data = web::Data::new(data);

    let server_builder = HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .app_data(web_data.clone()) 
            .route("/index.html", web::get().to(index))
            .route("/", web::get().to(index))
            .route("/img/{image_name}", web::get().to(image_request))
    })
    .workers(args.workers)
    .bind(("127.0.0.1", args.port))  // only bind local so this can't be accessed outside the current machine
    .unwrap(); 

    let server = server_builder.run();

    // launch web browser
    if webbrowser::open(format!("http://127.0.0.1:{}/", args.port).as_str()).is_err() {
        warn!("Could not open web browser aborting");
        return;
    }

    // the server must be awaited otherwise it will not actually do anything.
    // we also want a time out so here we need to await both of them and return
    // when the first one exits.
    let mut set = JoinSet::new();
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
            data.hot_reload,
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

fn sort(args: &Parameters, input: Vec<ImageInfo>) -> Vec<ImageInfo> {
    let mut result = input.clone();
    if args.alphabetical {
        result.sort_by(|a, b| a.url.cmp(&b.url));
    } else if args.date {
        result.sort_by(|a, b| a.date.cmp(&b.date));
    } else if args.randomise {
        let mut rng = rand::rng();
        result.shuffle(&mut rng);
    }
    result
}

fn create_templates(path: &str) -> Tera {
    let mut tera = match Tera::new(format!("{}*.html", path).as_str()) {
        Ok(t) => t,
        Err(e) => {
            warn!("Parsing error(s): {}", e);
            panic!("Could not parse custom template. Aborting");
        }
    };
    tera.autoescape_on(vec![]);

    // look and see if we have a custom index.html file loaded from init.
    // if we don't we can load in a default later.
    let mut index = false;
    debug!("found templates: ");
    for name in tera.get_template_names() {
        debug!("{}", name);
        if name == "index.html" {
            index = true;
        }
    }

    if !index {
        info!("No custom index.html found in the current directory so using default template");
        tera.add_raw_template("index.html", DEFAULT_INDEX).unwrap();
    }

    tera
}

fn generate_index(
    templates: &Tera,
    target_path: &str,
    images: &Vec<ImageInfo>,
    background: &str,
    hot_reload: bool,
) -> String {
    let mut context = Context::new();
    context.insert("images", &images);
    context.insert("path", target_path);
    context.insert("background", background);

    if !hot_reload {
        templates.render("index.html", &context).unwrap()
    } else {
        let templates = create_templates("./src/");
        templates.render("index.html", &context).unwrap()
    }
}

const DEFAULT_INDEX: &str = include_str!("./index.html");
