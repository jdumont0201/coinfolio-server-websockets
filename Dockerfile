FROM ubuntu:16.04
MAINTAINER J. Dumont <j.dumont@coinamics.io>

RUN apt-get update && apt-get install -y libssl-dev pkg-config  ca-certificates
EXPOSE 3014

ENV appname server-websockets

RUN mkdir /coinfolio && mkdir /coinfolio/${appname}
ADD target/release/server-websockets /coinfolio/${appname}
RUN chmod 777 /coinfolio/${appname}/server-websockets
CMD exec /coinfolio/${appname}/server-websockets