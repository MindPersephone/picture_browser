# Picture browser

A local picture viewer, runs a web browser that shows a page with all the images and video.

Images will be resized, client side, so they are not taller or wider than the viewing area.

Works well enough for me currently

## Running

To display all the images in a folder, pass the path to the folder.

```
cargo run -- path/to/image/folder
```

This will cause the system default web browser to open displaying a page with all the images in the folder. They will be
ordered in the way that the file system has the images ordered. Known as Inode ordering. This may not be the same as 
they appear in your folder view.

If you want to specify the ordering you can by passing one of `--randomise` `--date` or `--alphabetical`

```
cargo run -- --randomise path/to/image/folder
```

If you want to filter to only have particular kinds of files displayed the `--filter` parameter takes `video` `images` 
`gif` or `none` with none, being the default, no filtering. The others will display only the kind of files you selected. 
Note gif files are included in `images`

```
cargo run -- --filter gif path/to/image/folder
```

Note: This will only bind to local host. This can not and should not be used to host images publicly. Yes you probably
could use a proxy or something, but you're on your own. That is not what this was designed for. Don't. Security issues 
related to running this publicly will be ignored.

The background colour of the page defaults to the [css colour "hotpink"](https://htmlcolorcodes.com/color-names/) This
can be changed with the `--background` or `-b` parameter. Any css compatible colour can be used here. `black`, `#FFFFFF`
, etc

If something is already using the port 6700 you may need to change the default, that can be done by passing in the 
`--port` or `-p` parameter with a valid port value. Note values under 1024 may need admin privileges, pick something 
between 1025 and 65535. It is beyond the scope of this readme to explain why.

```
cargo run -- --background #66FF77 -p 6969 path/to/image/folder
```

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

### Design requirements

* Must not load external files at runtime. No loading external javascript, fonts, css, etc.
    * We don't care what pictures users are display, we don't need to know if its their porn collection or their holiday
    snaps.
* Must be fast enough
* Must work on most systems. 
* Prefer not to commit code crimes.

## TODO:
re-implement auto video start

## Maybe one day:
* Install function to add a magic entry to the windows registry to enable on right click menu like vs code or other 
tools
* Add task bar icons when running in windows like that.
