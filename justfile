# Install the `wblog` binary into PATH for use from `just`.
bootstrap:
  cargo install --path wblog

# Build the whole site incrementally.
build:
  wblog build

# Force a full rebuild of the whole site.
rebuild:
  wblog build --full

# Remove generated site output and incremental build cache.
clean:
  wblog clean

# Serve the generated site from `public/`.
serve:
  wblog serve

# Watch all supported inputs, rebuild affected targets, and serve the site.
watch:
  wblog watch
