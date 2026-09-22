FROM rust:1.98.1-alpine3.24 AS setup

RUN apk add --no-cache build-base musl-dev cmake clang wget

WORKDIR /tmp

RUN wget -O tidy-html5.tar.gz https://github.com/htacg/tidy-html5/archive/refs/tags/5.8.0.tar.gz && \
    tar -xzf tidy-html5.tar.gz && \
    cd tidy-html5-5.8.0/build/cmake && \
    cmake -DCMAKE_POLICY_VERSION_MINIMUM=3.5 ../.. && \
    make && \
    make install
RUN wget -O typst-0.15.1.tar.xz https://github.com/typst/typst/releases/download/v0.15.1/typst-x86_64-unknown-linux-musl.tar.xz && \
    tar -xvf typst-0.15.1.tar.xz && \
    cp ./typst-x86_64-unknown-linux-musl/typst /usr/local/bin/

FROM rust:1.98.1-alpine3.24 AS build

COPY --from=setup /usr/local/bin/typst /usr/local/bin/tidy /usr/local/bin/

COPY . /blog
WORKDIR /blog

RUN cargo install --locked --path wblog/wblog && \
    wblog clean && \
    wblog build --full && \
    cp -r public/ /public/

FROM alpine:3.24.2 AS final

COPY --from=build /public /public

CMD ["sh"] 
