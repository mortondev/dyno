# Dedicated Dyno server. Runs the latest release binary.
# Build: docker build -t dyno-server .
# Run:   docker run -p 7400:7400 -v dyno-data:/data dyno-server
FROM debian:bookworm-slim
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates curl libasound2 \
 && rm -rf /var/lib/apt/lists/*
RUN curl -fsSL -o /usr/local/bin/dyno https://github.com/mortondev/dyno/releases/latest/download/dyno-x86_64-unknown-linux-gnu \
 && chmod +x /usr/local/bin/dyno
VOLUME /data
ENV PORT=7400
EXPOSE 7400
# --auto-update installs new releases at start and every 6 hours.
CMD ["sh", "-c", "exec dyno server --port ${PORT} --world /data/world.dynw --auto-update"]
