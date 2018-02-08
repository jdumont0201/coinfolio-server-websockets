FROM ubuntu:16.04
MAINTAINER J. Dumont <j.dumont@coinamics.io>

RUN apt-get update && apt-get install -y libssl-dev pkg-config  ca-certificates
EXPOSE 3014

ENV appname server-websockets

RUN mkdir /coinfolio && mkdir /coinfolio/${appname}
ADD target/release/server-websockets /coinfolio/${appname}
ADD certs/coinamics.crt /coinfolio/${appname}
ADD certs/coinamics.key /coinfolio/${appname}
RUN chmod 777 /coinfolio/${appname}/server-websockets
RUN chmod 777 /coinfolio/${appname}/coinamics.crt
RUN chmod 777 /coinfolio/${appname}/coinamics.key

RUN ls
RUN ls /coinfolio/${appname}/

CMD exec /coinfolio/${appname}/server-websockets /coinfolio/${appname}/coinamics.crt  /coinfolio/${appname}/coinamics.key