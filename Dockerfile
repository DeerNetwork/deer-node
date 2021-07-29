FROM debian:stretch-slim

# show backtraces
ENV RUST_BACKTRACE 1

# install tools and dependencies
RUN apt-get update && \
	DEBIAN_FRONTEND=noninteractive apt-get upgrade -y && \
	DEBIAN_FRONTEND=noninteractive apt-get install -y \
		libssl1.1 \
		ca-certificates \
		curl && \
# apt cleanup
	apt-get autoremove -y && \
	apt-get clean && \
	find /var/lib/apt/lists/ -type f -not -name lock -delete

# add node to docker image
COPY ./target/release/nft360 /usr/local/bin

EXPOSE 30333 9933 9944

ENTRYPOINT ["/usr/local/bin/nft360"]