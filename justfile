build: clean render-typ render-adoc make-css cp-svg cp-assets tidy

clean:
  find . -name ".DS_Store" -type f -delete
  rm -r public 2> /dev/null
  mkdir -p public

render-typ:
  #!/usr/bin/env sh
  TYPST=typst
  for file in $(find resource/typst -type f -print0 | xargs -0); do
    NAME=${file#resource/typst/}
    NAME="${NAME%.typ}.svg"
    echo "Typst is compiling $file to public/resource/$NAME"
    mkdir -p $(dirname "public/resource/$NAME")
    $TYPST compile -f svg $file "public/resource/$NAME"
  done

cp-svg:
  #!/usr/bin/env sh
  for file in $(find resource/svg -type f -print0 | xargs -0); do
    NAME=${file#resource/svg/}
    echo "Copying $file to public/resource/$NAME"
    cp $file "public/resource/$NAME"
  done

render-adoc:
  #!/usr/bin/env sh
  for file in $(find content \( -name '*.adoc' \)  -print0 | xargs -0); do
    output=${file#content/}
    outfile="${output%.*}.html"
    echo "Rendering $file to public/$outfile"
    mkdir -p $(dirname "public/$outfile")
    asciidoctor -b w-html -r ./w-asciidoc/convert.rb -r ./w-asciidoc/hljs.rb $file -o "public/$outfile"
  done

make-css:
  #!/usr/bin/env sh
  GRASS=grass
  echo "Rendering style.scss"
  $GRASS sass/style.scss public/style.css

tidy:
  #!/usr/bin/env sh
  TIDY=tidy
  for file in $(find public \( -name '*.html' \) -print0 | xargs -0); do
    echo "Tidying $file"
    $TIDY -config tidy.cfg $file
  done

cp-assets:
  #!/usr/bin/env sh
  for file in $(find static -type f -print0 | xargs -0); do
    echo "Copying $file to public/${file#static/}"
    mkdir -p $(dirname "public/${file#static/}")
    cp $file "public/${file#static/}"
  done

live-server:
  live-server public/

watch-adoc:
  #!/usr/bin/env sh
  watchexec -e adoc -w content/ -- just render-adoc
  