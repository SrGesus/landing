# Landing (W.I.P)

A simple server powered by [`axum`](https://github.com/tokio-rs/axum/) that serves files,
renders [`minijinja`](https://github.com/mitsuhiko/minijinja) templates,
has cgi-like capabilities and uses [`tailwind_css`](https://github.com/oovm/tailwind-rs)
to have an always just-in-time up to date stylesheet. Configured with toml.

Considering removing axum and just using tower directly.

## What does it do?

For example, you can have a folder structure like this.
- When you request `/` you will get `index.html` like a static file
- When you request `/foo`, `foo.sh` will run,
its stdout will be used to render `foo.html.j2`
- When you request `_layout`, you get 404 - templates starting with `_` are hidden

```tree
  routes/
 ├── index.html
 ├── _layout.html.j2
 ├── foo.sh
 └── foo.html.j2
  assets/
 ├── styles.css
 └── background.webp
```

You can choose to separate or join these three different type of files into different
directories as you wish.

## Why does it do it?

For fun! I just wanted something simple for my homelab landing page.

Something that:
- Renders simple `minijinja` templates.
- Can use the contents of some format (like `toml`) as template input.
- I can use to trigger simple scripts.
- Supports tailwind without needing a javascript runtime.

## Who's there?

Vee vill ask ze questions!

## How does one do it?

Other than static files, files will be interpreted as scripts or templates based on their suffix.

As shown before, routes will take the name without the suffix, `/foo.html.j2` will be
rendered when `/foo` or `/foo/` is requested.
Alternatively, since `index` is interpreted as a special word,
`/foo/index.html.j2` could also be served at this endpoint.

### Templates

If you're using tailwind, your layout template should have the following:
```html
    <link rel="stylesheet" href="{{ tailwind_href }}">
```

TOML files in the included directory will be sent as input for templates under their file
name, without their suffix.

### Configuration

```toml
# config.toml

# Default path and endpoint, can be overriden individually
# per templates/scripts/files section
path = "./routes"
endpoint = "/"

index_word = "index"

# Include toml files, will be used by templates
include = "./config.d/"

# Jinja templates
[templates]
suffixes = [ ".html.j2", ".html" ]

# CGI Scripts
[scripts]
suffixes = [ ".sh" ]

# Static files
[files]
path = "./assets"
endpoint = "/assets"

# Tailwind settings
[tailwind]
enable = true
# Check rendered output for classes, not just source template
# If all classes are in templates, this should be set to false.
check_rendered = true
```

### Tailwind

