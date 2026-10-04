# syntax=docker/dockerfile:1.7
FROM rust:1.98.0-slim-bookworm@sha256:1469a27c125cb5a3aebfa4f4e4665d935b02fb72cc093b2c974b3d740e43f157

# The downloaded browser and build tools target Linux x86_64.
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates curl git pkg-config libssl-dev python3 unzip \
    libnss3 libatk-bridge2.0-0 libx11-xcb1 libxcomposite1 libxdamage1 \
    libxrandr2 libgbm1 libasound2 libpango-1.0-0 libcairo2 libcups2 libxkbcommon0 libxfixes3 \
    fonts-liberation \
    && rm -rf /var/lib/apt/lists/*

RUN rustup component add rustfmt clippy \
    && rustup target add wasm32-unknown-unknown

ADD --checksum=sha256:f2b4680cd239693a646a2795e4633c625328d7b2a044fbe749fa3a2fe9e7036b \
    https://github.com/trunk-rs/trunk/releases/download/v0.21.14/trunk-x86_64-unknown-linux-gnu.tar.gz /tmp/trunk.tar.gz
ADD --checksum=sha256:82d12bb940e2d4e72e0d5605387fc1b8ca179044e012b620f0ce4e7440e8320e \
    https://github.com/wasm-bindgen/wasm-bindgen/releases/download/0.2.129/wasm-bindgen-0.2.129-x86_64-unknown-linux-musl.tar.gz /tmp/wasm-bindgen.tar.gz
ADD --checksum=sha256:ff43322f335e436b2f4dcdfeeec5db032299e335a7e8c1c618b326e100ce8732 \
    https://storage.googleapis.com/chrome-for-testing-public/154.0.8037.92/linux64/chrome-linux64.zip /tmp/chrome.zip
ADD --checksum=sha256:cc99c87cfd10e1da1b7d42ea9800a211df279f5a47c19a55b02c927cb9f606c2 \
    https://storage.googleapis.com/chrome-for-testing-public/154.0.8037.92/linux64/chromedriver-linux64.zip /tmp/chromedriver.zip

RUN tar -xzf /tmp/trunk.tar.gz -C /usr/local/bin trunk \
    && tar -xzf /tmp/wasm-bindgen.tar.gz -C /usr/local/bin --strip-components=1 \
    && unzip -q /tmp/chrome.zip -d /opt \
    && unzip -q /tmp/chromedriver.zip -d /opt \
    && ln -s /opt/chrome-linux64/chrome /usr/local/bin/google-chrome \
    && ln -s /opt/chromedriver-linux64/chromedriver /usr/local/bin/chromedriver \
    && rm /tmp/trunk.tar.gz /tmp/wasm-bindgen.tar.gz /tmp/chrome.zip /tmp/chromedriver.zip \
    && google-chrome --version && chromedriver --version

WORKDIR /workspace

ADD --checksum=sha256:8aca8db96f1b94770f1b0d72b6dddcb1ebb8123cb3712530b08cc387b349a3d8 \
    https://github.com/rhysd/actionlint/releases/download/v1.7.12/actionlint_1.7.12_linux_amd64.tar.gz /tmp/actionlint.tar.gz
RUN tar -xzf /tmp/actionlint.tar.gz -C /usr/local/bin actionlint \
    && rm /tmp/actionlint.tar.gz

# Use the published Tailwind theme as build input, without copying its palette into application code.
ADD --checksum=sha256:f1493c5bcb29a80310da63d19282d233ba259479f36561a27146cd4747d1faef \
    https://registry.npmjs.org/tailwindcss/-/tailwindcss-4.3.3.tgz /tmp/tailwindcss.tgz
RUN mkdir -p /opt/tailwindcss \
    && tar -xzf /tmp/tailwindcss.tgz -C /opt/tailwindcss --strip-components=1 package/theme.css package/LICENSE \
    && rm /tmp/tailwindcss.tgz

CMD ["bash"]
