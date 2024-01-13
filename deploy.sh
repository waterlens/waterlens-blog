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

for file in $(find content -type f -print); do
  echo "Rendering $file to public/${file#content/}"
  mkdir -p $(dirname "public/${file#content/}")
  $TERA --template $file --env-only --include-path templates/ -o "public/${file#content/}"
done

echo "Rendering style.scss"
$GRASS sass/style.scss public/style.css

for file in $(find static -type f -print); do
  echo "Copying $file to public/${file#static/}"
  mkdir -p $(dirname "public/${file#static/}")
  cp $file "public/${file#static/}"
done

for file in public/*.html; do
  echo "Tidying $file"
  $TIDY -m -utf8 -i -w 0 -q $file
done