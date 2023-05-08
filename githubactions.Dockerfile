# syntax=docker/dockerfile:1
FROM debian:bullseye-slim as production

ARG binpath=./release/main

#
RUN apt-get update \
  && apt-get install -y ca-certificates \
  && update-ca-certificates \
  && rm -rf /var/lib/apt/lists/*

# RUN ldconfig

# RUN echo $LD_LIBRARY_PATH

# create and enter app directory
WORKDIR /app

COPY $binpath ./main

# specify default port
EXPOSE 16662

RUN chmod +x ./main

RUN groupadd -r user && useradd -r -g user user

RUN mkdir -p /home/user/.lodestone
RUN chown user /app
RUN chown user /home/user/.lodestone

USER user

# specify persistent volume
VOLUME ["/home/user/.lodestone"]

# start lodestone_core
CMD ["./main"]
