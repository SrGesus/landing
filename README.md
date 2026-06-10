# Landing (W.I.P)

A simple server powered by [`axum`](https://github.com/tokio-rs/axum/) that serves files,
renders [`minijinja`](https://github.com/mitsuhiko/minijinja) templates,
has cgi-like capabilities and uses [`tailwind_css`](https://github.com/oovm/tailwind-rs)
to have an always just-in-time up to date stylesheet. Configured with toml.

Made for my homelab landing page.

## What does (will) it do?

For example, you can have a folder structure like this.
- When you request `/` you will get `index.html` like a static file
- When you request `/foo`, `foo.sh` will run,
its stdout will be used to render `foo.html.j2`
- When you request `_layout.html.j`, you get 404, templates starting with `_` are hidden

```tree
  public
 ├── index.html
 ├── _layout.html.j2
 ├── foo.sh
 └── foo.html.j2
```

## Configuration

```toml
# config.toml

include = "./config.d/*.toml"

index_word = "index"

path = "./public"
endpoint = "/"

# Jinja templates
[templates]
suffixes = [ "html.j2" ]

# CGI Scripts
[scripts]
path = "./public"
endpoint = "/"
suffixes = [ ".sh" ]

# Static files
[files]
# Path and endpoint can be changed individually per templates/scripts/files
path = "./assets"
endpoint = "/assets"
suffixes = [ ".html", "" ] # Thus includes every file 

# Tailwind settings
[tailwind]
enable = true
# Check rendered output for classes, not just source template
# If all classes are in templates, this should be set to false.
check_rendered = true
```

## Templates

If you're using tailwind, your layout template should have the following:
```html
    <link rel="stylesheet" href="/assets/tailwind.css">
```


## Tailwind

