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
if command -v typst >/dev/null 2>&1 ; then
  TYPST=typst
else
  TYPST=./typst
fi

find . -name ".DS_Store" -type f -delete
rm -r public 2> /dev/null
mkdir -p public

for file in $(find resource/typst -type f -print0 | xargs -0); do
  NAME=${file#resource/typst/}
  NAME="${NAME%.typ}.svg"
  echo "Typst is compiling $file to public/resource/$NAME"
  mkdir -p $(dirname "public/resource/$NAME")
  $TYPST compile -f svg $file "public/resource/$NAME"
done

for file in $(find content -type f -print0 | xargs -0); do
  echo "Rendering $file to public/${file#content/}"
  mkdir -p $(dirname "public/${file#content/}")
  $TERA --template $file --env-only --include-path templates/ -o "public/${file#content/}"
done

echo "Rendering style.scss"
$GRASS sass/style.scss public/style.css

for file in $(find static -type f -print0 | xargs -0); do
  echo "Copying $file to public/${file#static/}"
  mkdir -p $(dirname "public/${file#static/}")
  cp $file "public/${file#static/}"
done

for file in $(find public \( -name '*.html' \) -print0 | xargs -0); do
  echo "Tidying $file"
  $TIDY -config tidy.cfg $file
done