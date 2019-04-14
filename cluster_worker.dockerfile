FROM debian:stretch as builder
ENV APT_KEY_DONT_WARN_ON_DANGEROUS_USAGE=DontWarn
RUN apt-get update && \
    apt-get install -y build-essential cmake libsqlite3-dev curl software-properties-common && \
    curl https://apt.llvm.org/llvm-snapshot.gpg.key -sSf | apt-key add - && \
    add-apt-repository "deb http://apt.llvm.org/stretch/ llvm-toolchain-stretch-6.0 main" && \
    apt-get update && \
    apt-get install -y build-essential cmake libsqlite3-dev libclang-6.0-dev curl && \
    curl https://sh.rustup.rs -sSf | sh -s -- --default-toolchain stable-2019-04-11 -y

ENV LIBCLANG_PATH=/usr/lib/llvm-6.0/lib
ENV PATH=/root/.cargo/bin:$PATH
COPY . /app
WORKDIR /app
RUN cargo build --bin cluster_worker


FROM debian:stretch
RUN apt-get update
RUN apt-get install -y libsqlite3-0
COPY --from=builder /app/target/debug/cluster_worker /root

ENV RUST_BACKTRACE=1
ENV QT_RITUAL_WORKER_QUEUE_ADDRESS=amqp://localhost//

CMD /root/cluster_worker
