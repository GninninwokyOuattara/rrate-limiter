<img src="https://github.com/GninninwokyOuattara/rrate-limiter/raw/main/docs/logo.png" width="250" align="right"/>

# RRATE LIMITER
A simple and efficient rate limiter that supports five well-known algorithms: Fixed Window, Sliding Window Log, Sliding Window Counter, Leaky Bucket, and Token Bucket. 

It easy to setup, configure and is design to be easily scallable.


## Why ?

**This is hobby project**. The goal is to improve my knowledge and skills of the Rust programming language while also providing practical experience with some system design concepts. 

It addresses the common problem in system design aka 'design a rate limiter' but goes beyong the conceptual design by providing a concrete implementation.    


## How to build

## Prerequisites

- [A redis instance](https://redis.io/)
- [Rust](https://www.rust-lang.org/)
- [Docker](https://www.docker.com/) if you want to run the rate limiter in a container. In which case the two previous points are not required


## Manual Installation

1. Clone the repository
```git clone https://github.com/eddycharly/rate_limiter.git```

2. Build the rate limiter server
```cd rate_limiter && cargo build --release```

3. Build the configurations loader
```cd config_loader && cargo build --release```



# Usage

## Environment Variables
- `RL_REDIS_HOST`: The host of the redis instance. Default is `localhost`
- `RL_REDIS_PORT`: The port of the redis instance. Default is `6379`
- `RL_REDIS_PASSWORD`: The password of the redis instance.``
- `RL_CONFIG_FILE_PATH`: The path to the configuration file. Default is `./config.yaml`. **Only for config_loader**


## Configuration File

The configuration file is a list of rules that should be applied to each route or endpoint. It should look like this:
```yaml
- route: "/"
    limit: 1
    expiration: 30
    algorithm: "fw"
    tracking_type: "ip"
    custom_tracking_key: ""
    active: true
```

- `route`: The route or endpoint that should be rate limited. it support dynamic route segments. eg `/api/v1/{id}`
- `limit`: The maximum number of requests that can be made within the specified time window. `Required``
- `expiration`: The time window in seconds. `Required`
- `algorithm`: The algorithm to use. `fw`, `swl`, `swc`, `lb` or `tb`. `Required`
- `tracking_type`: The type of tracking to use. `ip` or `header`. `Required`
- `custom_tracking_key`: The custom tracking key to use. `Required when tracking type is header`
- `active`: Whether the rule is active or not. Default to true. `Optional`

## Manual

1. Start an instance of the rate limiter server
2. Run the config loader after making some changes.


## Docker Compose

Edit the compose file to suit your needs and run ```docker-compose up```