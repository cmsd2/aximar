FROM ubuntu:24.04
RUN apt-get update && \
    apt-get install -y --no-install-recommends maxima gnuplot-nox && \
    rm -rf /var/lib/apt/lists/*
RUN mkdir -p /tmp/aximar && useradd -m maxima
USER maxima
ENTRYPOINT ["maxima"]
