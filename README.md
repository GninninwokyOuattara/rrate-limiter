<img src="https://github.com/GninninwokyOuattara/rrate-limiter/raw/main/docs/logo.png" width="250" align="right"/>

# RRATE LIMITER
A simple and efficient rate limiter that supports five well-known algorithms: Fixed Window, Sliding Window Log, Sliding Window Counter, Leaky Bucket, and Token Bucket. 

It easy to setup, configure and is design to be easily scallable.


## Why ?

This is hobby project. The goal is to improve my knowledge and skills of the Rust programming language while also providing practical experience with some system design concepts. 

It addresses the common problem in system design aka 'design a rate limiter' but goes beyong the conceptual design by providing a concrete implementation.    



## Prerequisites

- [A redis instance](https://redis.io/)
- [Rust installed](https://www.rust-lang.org/)
- [Docker](https://www.docker.com/) if you want to run the rate limiter in a container. In which case the two previous points are not required


## Manual Installation

1. Clone the repository
```git clone https://github.com/GninninwokyOuattara/rrate-limiter.git```

2. Build the rate limiter prject
```cd rate_limiter && cargo build --release```

## Environment Variables
- `RL_REDIS_HOST`: The host of the redis instance. Default is `localhost`
- `RL_REDIS_PORT`: The port of the redis instance. Default is `6379`


# Usage

## Configuration File

The configuration file is a list of rules that should be applied to each route or endpoint. It should look like this:
```yaml
- route: "/" # The route or endpoint that should be rate limited
    limit: 1 # The maximum number of requests that can be made 
    expiration: 30 # The time window in seconds
    algorithm: "fw" # The algorithm to use (fw, swl, swc, lb, tb)
    tracking_type: "ip" # The type of tracking to use (ip, header)
    custom_tracking_key: "" # key required when tracking type is header
    active: true # Whether the rule is active or not
```

Dynamic `route` can be specified too. `- route : "api/v1/orders/{id}`. 


## Run

> **Warning:** Make sure your redis instance is up and running.


```zsh
# Load the configuration file
rate_limiter load --file <config_file_path>

# Run the rate limiter
rate_limiter run
```


## Quickstart with Docker Compose

> Edit the environment variables as needed

```zsh
docker compose up
```