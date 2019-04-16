FROM debian:stretch as common
ENV APT_KEY_DONT_WARN_ON_DANGEROUS_USAGE=DontWarn
RUN apt-get update && \
    apt-get install -y build-essential cmake curl software-properties-common && \
    curl https://apt.llvm.org/llvm-snapshot.gpg.key -sSf | apt-key add - && \
    add-apt-repository "deb http://apt.llvm.org/stretch/ llvm-toolchain-stretch-6.0 main" && \
    apt-get update

FROM common as builder
RUN apt-get install -y libsqlite3-dev libclang-6.0-dev && \
    curl https://sh.rustup.rs -sSf | sh -s -- --default-toolchain stable-2019-04-11 -y

ENV LIBCLANG_PATH=/usr/lib/llvm-6.0/lib
ENV PATH=/root/.cargo/bin:$PATH
COPY . /app
WORKDIR /app
RUN cargo build --bin cluster_worker


FROM common
RUN apt-get update
RUN apt-get install -y libsqlite3-0 libclang1-6.0 p7zip-full mesa-common-dev \
    libgl1-mesa-glx
RUN apt-get install -y python3-bs4
COPY --from=builder /app/target/debug/cluster_worker /root

COPY . /app

RUN mkdir -p /opt/qt
WORKDIR /opt/qt

RUN /app/scripts/install_qt.py

RUN /opt/qt/5.9.7/gcc_64/bin/qmake -query
ENV QT_RITUAL_QMAKE_5_9_7=/opt/qt/5.9.7/gcc_64/bin/qmake
ENV QT_RITUAL_QMAKE_5_11_3=/opt/qt/5.11.3/gcc_64/bin/qmake
ENV QT_RITUAL_QMAKE_5_12_2=/opt/qt/5.12.2/gcc_64/bin/qmake
ENV QT_RITUAL_QMAKE_5_13_0=/opt/qt/5.13.0/gcc_64/bin/qmake

ENV RUST_BACKTRACE=1
ENV QT_RITUAL_WORKER_QUEUE_ADDRESS=amqp://localhost//
ENV QT_RITUAL_WORKER_RUN_TESTS=0
CMD /root/cluster_worker
