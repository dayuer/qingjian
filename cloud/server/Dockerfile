# 青简 Cloud 服务端镜像。构建上下文是 cloud/ 目录（deploy/docker-compose.yml 已经这样配好）：
#   docker build -f server/Dockerfile -t qingjian-cloud .
FROM rust:1-bookworm AS build
WORKDIR /src
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo build --release --locked -p qingjian-cloud-server \
 && cp target/release/qingjian-cloud /usr/local/bin/qingjian-cloud \
 && mkdir /empty

# distroless：只有 glibc、CA 证书与非 root 用户（uid 65532），没有 shell 与包管理器
FROM gcr.io/distroless/cc-debian12:nonroot
COPY --from=build /usr/local/bin/qingjian-cloud /usr/local/bin/qingjian-cloud
# 命名卷第一次挂载时继承这个目录的属主，非 root 也能写
COPY --from=build --chown=65532:65532 /empty /data
VOLUME /data
EXPOSE 8080
ENV QINGJIAN_CLOUD_DATA=/data RUST_LOG=info
ENTRYPOINT ["qingjian-cloud"]
CMD ["serve"]
