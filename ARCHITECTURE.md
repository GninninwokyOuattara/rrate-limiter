# SYSTEM DESIGN
<img src="https://github.com/GninninwokyOuattara/rrate-limiter/raw/main/docs/architecture.png"/>


# Data Flow
The data flow intended through this system is the following:
- The client makes a request to backend through the api gateway
- The api gateway first checks with the rate limiter(s) if the request is allowed should be allowed or not.
- If the request is allowed, the api gateway forwards the request to the backend otherwise a 429 http response is returned to the client with the appropriate headers.


# Choices

## Why use Redis ?

The rate limiter is required to be fast in order to not slow down the use experience. Redis is a single threaded in-memory database. It is extremely fast and will ensure a valid state is maintain across all instances of the rate limiter.

I also needed a way to update active instance of the rate limiter with new or updates rules. Redis PubSub is a great fit for that purpose.

## OpenTelemetry

For observability, I choose opentelemetry for its incredible flexibility allowing me to integrate with a wide range of tools such asprometheus for storing metrics and gafana for visualization.

# Failure handling

## What happens if the rate limiter is down?

The rate limiter is a stateless component. It is expected to have multiple instances running so that when one is down the other can take over in order to maintain availability.

However in the case where all instances are down, the handling is dependent on the user api gateway configuration. The service has a fast start up time so in my opinion a fail-open strategy is appropriate. This means requests should be allowed to go through the backend while it reboots. 


## What happens if the redis instance is down?

Redis should have replicas in place to ensure availability.
That said, when it still fails and the rate limiter can no longer communicate with it an error is returned and the api gateway configuration which should be handled similarly as above. 


