use std::env;
use std::sync::RwLock;

use actix_files::NamedFile;
use actix_web::body::BoxBody;
use log::{debug, info, warn};

use actix_web::http::header::ContentType;
use actix_web::middleware::Logger;
use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer, Responder, Result};
use clap::{Parser, ValueEnum};
use env_logger::Env;
use rand::prelude::*;
use tera::{Context, Tera};
use tokio::task::JoinSet;

use crate::error::Error;
use crate::image_info::{find_files, ImageInfo};
use crate::tree::{TreeNode, TreeNodeLayer};

pub mod error;
pub mod image_info;
pub mod tree;

struct AppData {
    target_path: String,
    images: Vec<ImageInfo>,
    tree: TreeNode,
    sort: SortBy,
    filter: FilterParameter,
    recursive: bool,
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
        help = "port to use for the web server"
    )]
    pub port: u16,

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
    pub alphabetical: bool,

    #[arg(
        short,
        long,
        default_value = "hotpink",
        help = "The background colour to use for the page. Accepts any css compatible colour string"
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
        index = 1,
        required = true,
        help = "file system path to the images to host"
    )]
    pub path: String,

    #[arg(long, default_value_t = false, help = "recurse down directories")]
    pub recursive: bool,

    #[arg(
        long,
        default_value_t = 8,
        help = "The number of worker threads to start."
    )]
    pub workers: usize,

    #[arg(
        long,
        default_value_t = false,
        help = "Reload the index template on every page load (useful when developing)"
    )]
    pub hot_reload: bool,

    #[arg(
        long,
        default_value_t = false,
        help = "Don't open the web browser automatically"
    )]
    pub no_browser: bool,
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
    if images.is_empty() {
        info!("Nothing found to display");
        return;
    }
    let sort_by = SortBy::from_parameters(&args);
    let sorted_images = sort(&sort_by, &images);

    // Should only be able to return none if the list is empty, and we've already short circuited if images is empty.
    let tree_root = tree::TreeNode::tree_from_images(&sorted_images).unwrap();

    let templates = create_templates("./");

    let data = AppData {
        target_path: args.path.clone(),
        images: sorted_images,
        tree: tree_root,
        sort: sort_by,
        filter: args.filter,
        recursive: args.recursive,
        templates,
        background: args.background.clone(),
        hot_reload: args.hot_reload.clone(),
    };

    let web_data = web::Data::new(RwLock::new(data));

    // Bind local so this can't be accessed outside the current machine if not dockerized
    let bind = if env::var("DOCKERIZED").is_ok() {
        warn!(
            "This app is running in dockerized mode and will listen on all interfaces.
            This can be insecure and this app should not be run publicly in this mode!"
        );
        "0.0.0.0"
    } else {
        "127.0.0.1"
    };

    let server_builder = HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .app_data(web_data.clone())
            .route("/index.html", web::get().to(index))
            .route("/", web::get().to(index))
            .route("/favicon.ico", web::get().to(favicon))
            .route("/refresh", web::get().to(refresh))
            .route("/tree/{tree_path}", web::get().to(tree_path))
            .route("/img/{image_name}", web::get().to(image_request))
    })
    .workers(args.workers)
    .bind((bind, args.port))
    .unwrap();

    let server = server_builder.run();

    if !args.no_browser {
        // launch web browser
        if webbrowser::open(&format!("http://127.0.0.1:{}/", args.port)).is_err() {
            warn!("Could not open web browser aborting");
            return;
        }
    }

    // the server must be awaited otherwise it will not actually do anything.
    // we also want a time out so here we need to await both of them and return
    // when the first one exits.
    let mut set = JoinSet::new();
    set.spawn(server);
    set.join_next().await;
}

async fn index(data: web::Data<RwLock<AppData>>) -> Result<impl Responder> {
    let data = data.read().map_err(|_e| Error::Lock())?;
    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(generate_index(
            &data.templates,
            &data.target_path,
            &data.images,
            &data.background,
            data.hot_reload,
        )))
}

async fn image_request(data: web::Data<RwLock<AppData>>, req: HttpRequest) -> Result<NamedFile> {
    let path = req.match_info().query("image_name");
    let data = data.read().map_err(|_e| Error::Lock())?;
    for img in data.images.iter() {
        if img.url.as_str() == path {
            return Ok(NamedFile::open(&img.source)?);
        }
    }
    panic!("not found {}", path);
}

async fn refresh(data: web::Data<RwLock<AppData>>) -> Result<()> {
    let mut data = data.write().map_err(|_e| Error::Lock())?;

    info!("Refreshing images from disk");
    let images: Vec<ImageInfo> = find_files(&data.target_path, data.filter, data.recursive);
    info!("Found {} files", images.len());
    if images.is_empty() {
        info!("Nothing found to display");
    }

    data.images = sort(&data.sort, &images);

    Ok(())
}

async fn tree_path(data: web::Data<RwLock<AppData>>, req: HttpRequest) -> Result<HttpResponse> {
    let path = req.match_info().query("image_name");
    let data = data.read().map_err(|_e| Error::Lock())?;

    Ok(HttpResponse::Ok().json(TreeNodeLayer::from(data.tree.path(path)?)))
}

async fn favicon() -> Result<impl Responder> {
    let icon_bytes = include_bytes!("../icon.png");
    Ok(HttpResponse::Ok()
        .content_type(ContentType::png())
        .body(BoxBody::new(icon_bytes.as_slice())))
}

fn sort(by: &SortBy, input: &Vec<ImageInfo>) -> Vec<ImageInfo> {
    let mut result = input.clone();
    match by {
        SortBy::Alphabetical => result.sort_by(|a, b| a.source.cmp(&b.source)),
        SortBy::Date => result.sort_by(|a, b| b.date.cmp(&a.date)),
        SortBy::Randomise => {
            let mut rng = rand::rng();
            result.shuffle(&mut rng);
        }
        SortBy::None => {}
    }

    // Re-calculate the height before and after fields.
    let mut running_total: u64 = 0;
    for e in result.iter_mut() {
        e.height_before = running_total;
        running_total += e.height + IMAGE_OFFSET;
    }

    let total = running_total;
    running_total = 0;
    for e in result.iter_mut() {
        running_total += e.height + IMAGE_OFFSET;
        e.height_after = total - running_total;
    }

    result
}

#[derive(PartialEq)]
enum SortBy {
    Alphabetical,
    Date,
    Randomise,
    None,
}

impl SortBy {
    pub fn from_parameters(args: &Parameters) -> Self {
        if args.alphabetical {
            SortBy::Alphabetical
        } else if args.date {
            SortBy::Date
        } else if args.randomise {
            SortBy::Randomise
        } else {
            SortBy::None
        }
    }
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
    context.insert("images", images);
    context.insert("path", target_path);
    context.insert("background", background);
    context.insert("image_offset", &IMAGE_OFFSET);

    if !hot_reload {
        templates.render("index.html", &context).unwrap()
    } else {
        let templates = create_templates("./src/");
        templates.render("index.html", &context).unwrap()
    }
}

const DEFAULT_INDEX: &str = include_str!("./index.html");
const IMAGE_OFFSET: u64 = 15;
