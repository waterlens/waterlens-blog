FROM rust:alpine3.21 as setup

RUN apk add --no-cache build-base musl-dev cmake clang wget

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

FROM rust:alpine3.21 as build

COPY --from=setup /usr/local/bin/typst /usr/local/bin/tidy /usr/local/bin/

RUN apk add --no-cache asciidoctor

COPY . /blog
WORKDIR /blog

RUN cargo install --path wblog && \
    wblog clean && \
    wblog build --full && \
    cp -r public/ /public/

FROM alpine:3.21 as final

COPY --from=build /public /public

CMD ["sh"] 
