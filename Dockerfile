FROM ubuntu:16.04
MAINTAINER J. Dumont <j.dumont@coinamics.io>

RUN apt-get update && apt-get install -y libssl-dev pkg-config  ca-certificates
EXPOSE 3014

ENV appname server-websockets

RUN mkdir /coinfolio && mkdir /coinfolio/${appname}
ADD target/release/server-websockets /coinfolio/${appname}
ADD certs /coinfolio/${appname}
RUN chmod 777 /coinfolio/${appname}/certs/coinamics.crt
RUN chmod 777 /coinfolio/${appname}/certs/coinamics.crt
RUN ls
RUN ls /coinfolio/${appname}/
RUN ls /coinfolio/${appname}/certs
CMD exec /coinfolio/${appname}/server-websockets /coinfolio/${appname}/certs/coinamics.crt  /coinfolio/${appname}/certs/coinamics.key