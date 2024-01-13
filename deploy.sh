#!/usr/bin/env sh

if command -v tera >/dev/null 2>&1 ; then
  TERA=tera
else
  TERA=./tera
fi
if command -v grass >/dev/null 2>&1 ; then
  GRASS=grass
else
  GRASS=./grass
fi
if command -v tidy >/dev/null 2>&1 ; then
  TIDY=tidy
else
  TIDY=./tidy
fi

rm -r public 2> /dev/null
mkdir -p public

echo "Rendering style.scss"
$GRASS sass/style.scss public/style.css
for file in content/*; do
  echo "Rendering $file"
  $TERA --template content/$(basename $file) --env-only --include-path templates/ -o public/$(basename $file)
done
for file in static/*; do
  echo "Copying $file"
  cp static/$(basename $file) public/$(basename $file)
done
for file in public/*.html; do
  echo "Tidying $file"
  $TIDY -m -utf8 -i -w 0 -q $file
done