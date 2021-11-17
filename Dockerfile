FROM lukemathwalker/cargo-chef:latest-rust-1.56.1 as chef
WORKDIR /app

FROM chef as planner
COPY . .
# Compute a lock-like file for our project
RUN cargo chef prepare  --recipe-path recipe.json

FROM chef as builder
COPY --from=planner /app/recipe.json recipe.json

# Build project dependencies
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .

# Build project
RUN cargo build --release --bin saitocli

#FROM debian:bullseye-slim AS runtime
#WORKDIR /app


#RUN apt-get update -y \
#    && apt-get install -y --no-install-recommends openssl \
#    # Clean up
#    && apt-get autoremove -y \
#    && apt-get clean -y \
#    && rm -rf /var/lib/apt/lists/*

#COPY --from=builder /app/target/release/saitocli saitocli

ENV APP_ENVIRONMENT development

#ENTRYPOINT ["./saito_rust"]
#CMD ["/bin/bash", "-c", "cargo run --bin saitocli tx --amount 1 --to gYsu1fVHjP6Z8CHCzti9K9xb5JPqpEL7zi7arvLiVANm \
#--filename data/test/tx.out --keyfile test/testwallet --password asdf --log-app-path saito-node.log --log-level info"]
