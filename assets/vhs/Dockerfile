FROM ghcr.io/charmbracelet/vhs@sha256:9d5fc3dc0c160b0fb1d2212baff07e6bdf3fa9438c504a3237484567302fcf93
USER root
# Allow apt to accept a release codename change (eg. bookworm -> trixie)
# and install non-interactively with minimal extras.
RUN apt-get update -o Acquire::AllowReleaseInfoChange=true \
 && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
	 build-essential fish curl ca-certificates \
 && rm -rf /var/lib/apt/lists/*

# Install rust via rustup and pin toolchain to 1.91.1
ENV RUSTUP_HOME=/usr/local/rustup CARGO_HOME=/root/.cargo
ENV PATH="/root/.cargo/bin:${PATH}"
RUN curl https://sh.rustup.rs -sSf | sh -s -- -y --default-toolchain stable
