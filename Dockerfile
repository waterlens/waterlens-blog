FROM rust:alpine3.21 as setup

RUN apk add --no-cache build-base musl-dev cmake clang wget

RUN cargo install --locked grass@0.13.4

WORKDIR /tmp

RUN wget -O tidy-html5.tar.gz https://github.com/htacg/tidy-html5/archive/refs/tags/5.8.0.tar.gz && \
    tar -xzf tidy-html5.tar.gz && \
    cd tidy-html5-5.8.0/build/cmake && \
    cmake ../.. && \
    make && \
    make install
RUN wget -O typst-0.13.1.tar.xz https://github.com/typst/typst/releases/download/v0.13.1/typst-x86_64-unknown-linux-musl.tar.xz && \
    tar -xvf typst-0.13.1.tar.xz && \
    cp ./typst-x86_64-unknown-linux-musl/typst /usr/local/bin/
RUN wget -O just-1.40.0.tar.gz https://github.com/casey/just/releases/download/1.40.0/just-1.40.0-x86_64-unknown-linux-musl.tar.gz && \
    tar -xvf just-1.40.0.tar.gz && \
    cp ./just /usr/local/bin/

FROM alpine:3.21 as build

COPY --from=setup /usr/local/cargo/bin/grass /usr/local/bin/typst /usr/local/bin/just /usr/local/bin/tidy /usr/local/bin/

RUN apk add --no-cache asciidoctor

COPY . /blog
WORKDIR /blog

RUN just build && cp -r public/ /public/

CMD ["sh"] 