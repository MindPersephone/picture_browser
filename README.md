# Picture browser

A local picture viewer, runs a web browser that shows a page with all the images and video.

Images will be resized, client side, so they are not taller or wider than the viewing area.

Works well enough for me currently

## Optional dependencies

this will attempt to figure out most video file sizes without needing external programs. However some do not work, in which cases it will attempt to use ffmpeg to find the size. If it can't find it, a warning message will appear and the size will be set to 0. Which will cause some bouncing when scrolling through the page.

## Running

Currently we don't have pre-built releases. You will need a working [rust installation](https://rust-lang.org/tools/install/) and use cargo to build the project from a git clone.

To display all the images in a folder, pass the path to the folder. Note: this will only display the images in the given folder, not any sub folders.

```sh
cargo run -- path/to/image/folder
```

This will cause the system default web browser to open displaying a page with all the images in the folder. They will be ordered in the way that the file system has the images ordered. Known as Inode ordering. This may not be the same as they appear in your folder view.

If you want to specify the ordering you can by passing one of `--randomise` `--date` or `--alphabetical`

```sh
cargo run -- --randomise path/to/image/folder
```

If you want to filter to only have particular kinds of files displayed the `--filter` parameter takes `video` `images`
`gif` or `none` with none, being the default, no filtering. The others will display only the kind of files you selected.
Note gif files are included in `images`, we can't tell if a gif is animated or static currently.

```sh
cargo run -- --filter gif path/to/image/folder
```

Note: This will only bind to local host. This can not and should not be used to host images publicly. Yes you probably
could use a proxy or something, but you're on your own. That is not what this was designed for. Don't. Security issues
related to running this publicly will be ignored.

If something is already using the port 6700 you may need to change the default, that can be done by passing in the
`--port` or `-p` parameter with a valid port value. Note values under 1024 may need admin privileges, pick something
between 1025 and 65535. It is beyond the scope of this readme to explain why.

If your images are in multiple sub folders you can use the `--recursive` flag to display images in sub folders as well as the folder provided.

If you do not want the web browser to open automatically use `--no-browser` E.G you want to open the page in a browser that is not the system default.

```sh
cargo run -- -p 6969 --recursive --no-browser path/to/image/folder
```

## Docker version
This project is also avilable as a docker container in the event you want to look at photos on a network drive and can run a docker container on that server to avoid permissions issues and network lag.

Keep in mind the security notes above, this project is not intended to host images publicly. Any security issues relating to running this publicly will be ignored.  

The default port is `6700` and pictures shoud be mounted read-only to `/pictures` inside the container.  

```bash
docker pull ghcr.io/mindpersephone/picture_browser:main
docker run -p 6700:6700 -v /your/pictures/directory:/pictures:ro ghcr.io/mindpersephone/picture_browser:main
```

You can provide additional command line arguments when running the docker, note that the directory will need to be specified at the end of your replacement arguments.  
The default command line arguments for the docker container can be found in [the Dockerfile](Dockerfile#L16).  
For example, to add the `--randomise` flag run the container as follows:

```
docker run -p 6700:6700 -v /your/pictures/directory:/pictures:ro ghcr.io/mindpersephone/picture_browser:main --no-browser --recursive --randomise --recursive /pictures
```


## Short cut keys

There are a couple of short cut keys built into the webpage, they are not easy to find though.

|Key   | Action                                 |
|------|----------------------------------------|
|s     | Slowly auto scroll the page downwards  |
|j     | Jump the page up to the previous image |
|k     | Jump the page to the next image        |

## Allowed file types

This does some crude but usually effective file extension matching to decide what files to show in the result. They are
"mp4", "webm", "jpg", "jpeg", "png", "gif", "webp" this currently covers all the things I need, if you need other file
types please feel free to add a pull request. The constants containing these are called `ALLOWED_IMG_EXTENSIONS` and
`ALLOWED_VID_EXTENSIONS`. Due to the way html renders videos and images we need to be able to tell the two apart.

## Development

Mostly a normal rust and html project, raw javascript, cargo build etc, however be careful when editing the index.html
template. Sometimes editors like to reformat the template fields that use the `{{value}}` markers also being used as
blocks css and javascript. This can cause problems around the background colour. Note that in vs code pressing ctrl+k
then ctrl+shift+s will save without triggering the formatter (at least in windows).

Logging is controlled by [env_logger](https://docs.rs/env_logger/latest/env_logger/). See the docs for it for
configuring log levels and so on.

Also note that the index.html page is a single file page. Everything must be in the one file. It *MUST NOT* load any
files from outside of local host, and even then it should only be loading the images that it has been asked to load.

When working on the html for the page using the `--hot-reload` command line flag will cause the template to be reloaded from disk every time it is requested rather than caching the result of the first run. This will mean you don't need to rebuild the rust code for changes.  

### Design requirements

* Must not load external files at runtime. No loading external javascript, fonts, css, etc.
  * We don't care what pictures users are display, we don't need to know if its their porn collection or their holiday
  snaps.
* Must be fast enough
* Must work on most systems.
* Prefer not to commit code crimes.

## Maybe one day

* Install function to add a magic entry to the windows registry to enable on right click menu like vs code or other
tools
* Add task bar icons when running in windows like that.
