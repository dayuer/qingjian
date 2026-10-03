# 青简 Cloud 纠错闭环镜像：上游 qingjian-cli + 产品数据 + qingjian-cloud-tuner。
# 构建上下文是仓库根目录（要编上游的 CLI）；同目录的 Dockerfile.dockerignore 把 target/ 等挡在外面：
#   docker build -f cloud/tuner/Dockerfile -t qingjian-cloud-tuner .
FROM rust:1-bookworm AS build
WORKDIR /src
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    --mount=type=cache,target=/src/cloud/target \
    cargo build --release --locked -p qingjian-cli \
 && cp target/release/qingjian-cli /usr/local/bin/ \
 && (cd cloud && cargo build --release --locked -p qingjian-cloud-tuner) \
 && cp cloud/target/release/qingjian-cloud-tuner /usr/local/bin/
# 产品数据（词库、语言模型、领域词库）按 tools/release/data.lock 从上游的 data Release 下载并校验
RUN bash tools/release/data-fetch.sh \
 && mkdir -p /opt/qingjian/data \
 && cp -r data/generated /opt/qingjian/data/generated \
 && mkdir /empty

FROM gcr.io/distroless/cc-debian12:nonroot
COPY --from=build /usr/local/bin/qingjian-cli /usr/local/bin/qingjian-cli
COPY --from=build /usr/local/bin/qingjian-cloud-tuner /usr/local/bin/qingjian-cloud-tuner
COPY --from=build /opt/qingjian/data/generated /opt/qingjian/data/generated
COPY --from=build --chown=65532:65532 /empty /data
VOLUME /data
ENV QINGJIAN_CLI=/usr/local/bin/qingjian-cli \
    QINGJIAN_DATA=/opt/qingjian/data/generated \
    QINGJIAN_TUNER_STATE=/data \
    RUST_LOG=info
ENTRYPOINT ["qingjian-cloud-tuner"]
