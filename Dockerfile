# Build in the rust container
FROM rust AS builder

WORKDIR /build
COPY . .
RUN cargo install --path .

# Install dependencies and copy built executable into a smaller debian container for distribution
FROM debian:trixie-slim

WORKDIR /app
RUN apt-get update && apt-get install -y ffmpeg && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/picture_browser /app/picture_browser
ENV DOCKERIZED=true
ENTRYPOINT ["/app/picture_browser"]
CMD ["--port", "6700", "--no-browser", "--recursive", "/pictures"]