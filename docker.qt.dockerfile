FROM debian:buster as qt_downloader
RUN apt-get update
RUN apt-get install -y python3-bs4 p7zip-full
RUN mkdir -p /opt/qt
WORKDIR /opt/qt
COPY scripts/install_qt.py /
RUN /install_qt.py 5.9.7 linux_x64 gcc_64 && \
    /install_qt.py 5.9.7 --docs && \
    /install_qt.py 5.11.3 linux_x64 gcc_64 && \
    /install_qt.py 5.11.3 --docs && \
    /install_qt.py 5.12.2 linux_x64 gcc_64 && \
    /install_qt.py 5.12.2 --docs && \
    /install_qt.py 5.13.0 linux_x64 gcc_64 && \
    /install_qt.py 5.13.0 --docs


FROM ritual_builder
COPY --from=qt_downloader /opt/qt /opt/qt
COPY scripts/qt_env.sh /bin/qt_env

RUN apt-get install -y libxrender1 libfontconfig libxkbcommon-x11-0 mesa-common-dev xvfb
RUN mkdir /tmp/run && chmod 0700 /tmp/run
ENV XDG_RUNTIME_DIR=/tmp/run
